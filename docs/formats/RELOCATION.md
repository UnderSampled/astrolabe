# Relocation Tables

**Extensions:** `.rtb`, `.rtp`, `.rtt`, `.rtd`, `.rtg`, `.rts`, `.rtl`, `.rtv`
**Purpose:** Map pointer addresses between memory blocks and files
**Encryption:** XOR number masking (mask read from first 4 bytes)
**Compression:** LZO per pointer block entry

## Overview

The OpenSpace engine uses a sophisticated pointer relocation system to resolve references between data blocks. Since game data is serialized from memory, pointers need to be translated from their original memory addresses to file offsets.

## Relocation Table Types

| Extension | Enum | Description |
|-----------|------|-------------|
| `.rtb` | RTB (0) | SNA main blocks |
| `.rtp` | RTP (1) | Pointer file (GPT) |
| `.rts` | RTS (2) | Sound files |
| `.rtt` | RTT (3) | Texture file (PTX) |
| `.rtl` | RTL (4) | Standard relocation |
| `.rtd` | RTD (5) | Language pointer file (DLG) |
| `.rtg` | RTG (6) | Language-specific SNA blocks (LNG) |
| `.rtv` | RTV (7) | Video files |

## File Structure

```
┌─────────────────────────────────────┐
│ XOR Mask (4 bytes)                  │
├─────────────────────────────────────┤
│ Block count (1 byte)                │
├─────────────────────────────────────┤
│ Unknown (4 bytes) [R2/R3 only]      │
├─────────────────────────────────────┤
│ Pointer Block 0                     │
├─────────────────────────────────────┤
│ Pointer Block 1                     │
├─────────────────────────────────────┤
│ ...                                 │
└─────────────────────────────────────┘
```

## Pointer Block Structure

Each pointer block contains pointers located within a specific SNA memory block:

```
Offset  Size  Type   Description
------  ----  ----   -----------
0x00    1     u8     Module ID (matches SNA block)
0x01    1     u8     Block ID (matches SNA block)
0x02    4     u32    Pointer count

If snaCompression enabled:
0x06    4     u32    Is compressed (0=no, 1=yes)
0x0A    4     u32    Compressed size
0x0E    4     u32    Compressed checksum
0x12    4     u32    Decompressed size
0x16    4     u32    Decompressed checksum
0x1A    N     bytes  Pointer info array (compressed or not)

Otherwise:
0x06    N     bytes  Pointer info array (8 bytes each)
```

## Pointer Info Structure

Each pointer info entry describes one pointer in the source block:

### Montreal/TT Engine

```
Offset  Size  Type   Description
------  ----  ----   -----------
0x00    4     u32    Offset in memory (where pointer is located)
0x04    1     u8     Target module ID (block the pointer points to)
0x05    1     u8     Target block ID
```

### Post-TT Engine (R2, R3)

```
Offset  Size  Type   Description
------  ----  ----   -----------
0x00    4     u32    Offset in memory
0x04    1     u8     Target module ID
0x05    1     u8     Target block ID
0x06    1     u8     Byte 6 (additional flags)
0x07    1     u8     Byte 7 (additional flags)
```

## Pointer Resolution Algorithm

To resolve a pointer at a given location:

1. Find the pointer info entry matching the memory offset
2. Look up the target block (module, block_id) in the SNA
3. Read the raw pointer value at the memory offset
4. Subtract the target block's `baseInMemory`
5. Add the target block's `dataPosition` in the file

```python
def resolve_pointer(
    pointer_info: RelocationPointerInfo,
    source_block: SNAMemoryBlock,
    all_blocks: dict[int, SNAMemoryBlock]
) -> int:
    """Resolve a pointer to a file offset."""
    # Get the raw pointer value from source block
    relative_offset = pointer_info.offset_in_memory - source_block.base_in_memory
    raw_ptr = int.from_bytes(
        source_block.data[relative_offset:relative_offset + 4],
        'little'
    )

    # Find target block
    target_key = (pointer_info.target_module * 0x100) + pointer_info.target_block_id
    target_block = all_blocks[target_key]

    # Translate to file offset
    file_offset = raw_ptr - target_block.base_in_memory + target_block.data_position

    return file_offset
```

## Global Pointer Table (GPT)

The GPT file contains a flat array of pointers (no headers per entry). Resolution uses the RTP relocation table differently:

For Montreal engine:
- Iterate through all pointer values in GPT
- Match each value against RTP's `offsetInMemory` entries
- Use matching entry's module/block to find target

For R2/R3:
- RTP entries are in same order as GPT entries
- Sequential matching rather than value matching

## Python Implementation

```python
from dataclasses import dataclass
import lzo

@dataclass
class RelocationPointerInfo:
    """Information about a single pointer."""
    offset_in_memory: int
    target_module: int
    target_block_id: int
    byte6: int = 0
    byte7: int = 0

@dataclass
class RelocationPointerList:
    """List of pointers within a single SNA block."""
    module: int
    block_id: int
    pointers: list[RelocationPointerInfo]

class RelocationTableReader:
    """Reader for relocation table files."""

    def __init__(self, data: bytes, is_montreal: bool = True):
        self.data = data
        self.is_montreal = is_montreal
        self.pointer_blocks: list[RelocationPointerList] = []

    def read(self) -> list[RelocationPointerList]:
        pos = 0

        block_count = self.data[pos]
        pos += 1

        if not self.is_montreal:
            pos += 4  # Skip unknown u32

        for _ in range(block_count):
            if pos >= len(self.data):
                break

            module = self.data[pos]
            block_id = self.data[pos + 1]
            pointer_count = int.from_bytes(self.data[pos + 2:pos + 6], 'little')
            pos += 6

            pointers = []

            if pointer_count > 0:
                # Read compression header
                is_compressed = int.from_bytes(self.data[pos:pos + 4], 'little')
                compressed_size = int.from_bytes(self.data[pos + 4:pos + 8], 'little')
                decompressed_size = int.from_bytes(self.data[pos + 12:pos + 16], 'little')
                pos += 20

                raw_data = self.data[pos:pos + compressed_size]
                pos += compressed_size

                if is_compressed:
                    ptr_data = lzo.decompress(raw_data, False, decompressed_size)
                else:
                    ptr_data = raw_data

                # Parse pointer entries
                ptr_pos = 0
                for _ in range(pointer_count):
                    offset = int.from_bytes(ptr_data[ptr_pos:ptr_pos + 4], 'little')
                    target_module = ptr_data[ptr_pos + 4]
                    target_block = ptr_data[ptr_pos + 5]

                    if self.is_montreal:
                        ptr_pos += 6
                        pointers.append(RelocationPointerInfo(
                            offset_in_memory=offset,
                            target_module=target_module,
                            target_block_id=target_block
                        ))
                    else:
                        byte6 = ptr_data[ptr_pos + 6]
                        byte7 = ptr_data[ptr_pos + 7]
                        ptr_pos += 8
                        pointers.append(RelocationPointerInfo(
                            offset_in_memory=offset,
                            target_module=target_module,
                            target_block_id=target_block,
                            byte6=byte6,
                            byte7=byte7
                        ))

            self.pointer_blocks.append(RelocationPointerList(
                module=module,
                block_id=block_id,
                pointers=pointers
            ))

        return self.pointer_blocks
```

## Usage Pattern

```python
# Load SNA and RTB together
sna_data = decode_masked_data(read_file("level.sna"))
rtb_data = decode_masked_data(read_file("level.rtb"))

sna_reader = SNAReader(sna_data)
blocks = sna_reader.read()
blocks_by_key = {b.relocation_key: b for b in blocks}

rtb_reader = RelocationTableReader(rtb_data)
relocation = rtb_reader.read()

# Now pointers can be resolved when parsing block contents
```

## References

- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/FileFormat/RelocationTable.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/FileFormat/SNA.cs` (CreatePointers method)
