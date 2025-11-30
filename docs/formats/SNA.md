# SNA Format

**Extension:** `.sna`
**Purpose:** Serialized game data containing level geometry, objects, textures, and AI
**Encryption:** XOR number masking (mask read from first 4 bytes)
**Compression:** LZO per memory block

## Overview

SNA (SNApshot) files contain the main game data serialized as memory blocks. Each block represents a portion of the game's memory layout and can be individually compressed.

## File Structure

```
┌─────────────────────────────────────┐
│ XOR Mask (4 bytes)                  │  ← Read first, used for decryption
├─────────────────────────────────────┤
│ Memory Block 0                      │
├─────────────────────────────────────┤
│ Memory Block 1                      │
├─────────────────────────────────────┤
│ ...                                 │
├─────────────────────────────────────┤
│ Memory Block N                      │
├─────────────────────────────────────┤
│ Terminator Block (baseInMemory=-1)  │
└─────────────────────────────────────┘
```

## Memory Block Header

### Montreal Engine (Hype)

```
Offset  Size  Type   Description
------  ----  ----   -----------
0x00    1     u8     Module ID
0x01    1     u8     Block ID
0x02    4     i32    Base address in memory (-1 = terminator)

If baseInMemory != -1:
0x06    4     u32    Unknown 2
0x0A    4     u32    Unknown 3
0x0E    4     u32    Max position minus 9
0x12    4     u32    Block size (uncompressed)
0x16    ...         Block data (possibly compressed)
```

### Post-Montreal Engine (R2, R3)

```
Offset  Size  Type   Description
------  ----  ----   -----------
0x00    1     u8     Module ID
0x01    1     u8     Block ID
0x02    1     u8     Unknown 1
0x03    4     i32    Base address in memory (-1 = terminator)

If baseInMemory != -1:
0x07    4     u32    Unknown 2
0x0B    4     u32    Unknown 3
0x0F    4     u32    Max position minus 9
0x13    4     u32    Block size
0x17    ...         Block data (possibly compressed)
```

## Block Data (When Compressed)

If `Settings.snaCompression` is enabled:

```
Offset  Size  Type    Description
------  ----  ----    -----------
0x00    4     u32     Is compressed (0=no, 1=yes)
0x04    4     u32     Compressed size
0x08    4     u32     Compressed checksum
0x0C    4     u32     Decompressed size
0x10    4     u32     Decompressed checksum
0x14    N     bytes   Data (compressed or uncompressed)
```

## Module/Block IDs

The module and block IDs identify different memory regions:

| Module | Description |
|--------|-------------|
| 0x00 | Fix (global) data |
| 0x01 | Level-specific data |
| 0x0A | Temporary/special blocks |
| 0xFF | End marker |

Common block IDs vary by module and contain different data types (geometry, AI, textures, etc.).

## Relocation Key

Blocks are identified by a relocation key for pointer resolution:
```python
relocation_key = (module * 0x100) + block_id
```

## Associated Files

SNA files work with several companion files:

| Extension | Purpose |
|-----------|---------|
| `.rtb` | Relocation table for SNA blocks |
| `.gpt` | Global pointer table |
| `.rtp` | Relocation table for GPT |
| `.ptx` | Texture data |
| `.rtt` | Relocation table for PTX |
| `.lng` | Language-specific SNA blocks |
| `.rtg` | Relocation table for LNG |
| `.dlg` | Language pointer file |
| `.rtd` | Relocation table for DLG |

## Python Implementation

```python
from dataclasses import dataclass
import lzo

@dataclass
class SNAMemoryBlock:
    """A single memory block from an SNA file."""
    module: int
    block_id: int
    base_in_memory: int
    size: int
    data: bytes
    data_position: int  # Position in file for relocation

    @property
    def relocation_key(self) -> int:
        return (self.module * 0x100) + self.block_id

class SNAReader:
    """Reader for SNA files."""

    def __init__(self, data: bytes, is_montreal: bool = True):
        self.data = data
        self.is_montreal = is_montreal
        self.blocks: list[SNAMemoryBlock] = []

    def read(self) -> list[SNAMemoryBlock]:
        pos = 0

        while pos < len(self.data):
            module = self.data[pos]
            block_id = self.data[pos + 1]

            if self.is_montreal:
                base_in_memory = int.from_bytes(
                    self.data[pos + 2:pos + 6], 'little', signed=True
                )
                pos += 6
            else:
                # Skip unk1 byte
                base_in_memory = int.from_bytes(
                    self.data[pos + 3:pos + 7], 'little', signed=True
                )
                pos += 7

            if base_in_memory == -1:
                # Terminator block
                self.blocks.append(SNAMemoryBlock(
                    module=module,
                    block_id=block_id,
                    base_in_memory=-1,
                    size=0,
                    data=b'',
                    data_position=pos
                ))
                break

            # Read block metadata
            unk2 = int.from_bytes(self.data[pos:pos + 4], 'little')
            unk3 = int.from_bytes(self.data[pos + 4:pos + 8], 'little')
            max_pos_minus_9 = int.from_bytes(self.data[pos + 8:pos + 12], 'little')
            block_size = int.from_bytes(self.data[pos + 12:pos + 16], 'little')
            pos += 16

            data_position = pos
            block_data = self._read_block_data(pos, block_size)
            pos += self._get_raw_block_size(pos, block_size)

            self.blocks.append(SNAMemoryBlock(
                module=module,
                block_id=block_id,
                base_in_memory=base_in_memory,
                size=block_size,
                data=block_data,
                data_position=data_position
            ))

        return self.blocks

    def _read_block_data(self, pos: int, size: int) -> bytes:
        """Read and decompress block data if necessary."""
        if size == 0:
            return b''

        is_compressed = int.from_bytes(self.data[pos:pos + 4], 'little')
        compressed_size = int.from_bytes(self.data[pos + 4:pos + 8], 'little')
        decompressed_size = int.from_bytes(self.data[pos + 12:pos + 16], 'little')

        raw_data = self.data[pos + 20:pos + 20 + compressed_size]

        if is_compressed:
            return lzo.decompress(raw_data, False, decompressed_size)
        else:
            return raw_data[:size]

    def _get_raw_block_size(self, pos: int, size: int) -> int:
        """Get the raw size of block data in file (including compression header)."""
        if size == 0:
            return 0
        compressed_size = int.from_bytes(self.data[pos + 4:pos + 8], 'little')
        return 20 + compressed_size
```

## Data Contents

After parsing, SNA blocks contain serialized game structures:
- Geometry (vertices, triangles, materials)
- Scene hierarchy (SuperObjects, Sectors)
- Game objects (Persos, IPOs)
- AI data (Brains, Behaviors, Scripts)
- Animations and states

Pointer references between blocks are resolved using the relocation table (RTB).

## References

- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/FileFormat/SNA.cs`
- Relocation: `reference/raymap/Assets/Scripts/OpenSpace/FileFormat/RelocationTable.cs`
