# LVL Format

**Extension:** `.lvl`
**Purpose:** Level container file with raw data and pointer references
**Encryption:** None
**Compression:** None

## Overview

LVL files are level containers used in some OpenSpace variants. They contain raw game data along with pointer reference files (PTR) for resolving cross-file pointers.

Hype: The Time Quest primarily uses SNA files, but understanding LVL is useful for compatibility with other OpenSpace games.

## File Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       u32         Unknown (skipped, baseOffset=4)
0x04    ...     bytes       Level data
```

## PTR File Format

The associated `.ptr` file maps pointers between files.

### Largo Winch Format

```
For each pointer (file_size / 8 entries):
    i32     File ID (source file containing pointer)
    u32     Pointer offset (location in source file)
```

### Standard Format

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       u32         Number of pointers
0x04    ...     entries     Pointer entries

Per pointer:
    i32     File ID
    u32     Pointer offset
```

### Revolution/R2 Extended Format

After basic pointers:
```
u16     Number of special blocks
u16[]   Special block IDs
u8      Number of levels

If Fix file:
    string[num_levels]  Level names (0x1E bytes each)

If Level file:
    u8      Unknown
    u32     Unknown (x3)
    u32     Number of meshes
    u32     Number of materials
    u32     Number of textures
    u32     Unknown (x2)
    u32     Level index
    u32     Number of special pointers
    u32[]   Special pointer data
    u32     Unknown
    u32     Number of lightmapped objects
    u8      Unknown
```

### Rayman 3 Extended Format

After basic pointers:
```
For each fill-in pointer (remaining_size / 16):
    u32     Pointer offset (where pointer is located)
    i32     Source file ID
    u32     Target value (pointer value)
    i32     Target file ID
```

## File IDs

Files are identified by integer IDs for pointer resolution:

| ID | File Type |
|----|-----------|
| 0 | Fix |
| 1 | Level |
| 2+ | Additional data files |

## Pointer Resolution

```python
def resolve_lvl_pointer(
    ptr_offset: int,
    file_id: int,
    files: dict[int, 'LVLFile']
) -> tuple['LVLFile', int]:
    """Resolve a pointer to its target file and offset."""
    source_file = files[file_id]

    # Read pointer value at offset
    ptr_value = struct.unpack_from(
        '<I',
        source_file.data,
        ptr_offset + source_file.base_offset
    )[0]

    # Find target file containing this address
    target_file = files[file_id]  # Usually same file

    return target_file, ptr_value
```

## Python Implementation

```python
from dataclasses import dataclass
import struct

@dataclass
class LVLPointer:
    """Pointer reference from PTR file."""
    file_id: int
    offset: int
    target_value: int = 0
    target_file_id: int = 0

@dataclass
class LVLFile:
    """Parsed LVL file."""
    name: str
    file_id: int
    base_offset: int
    data: bytes
    pointers: dict[int, 'Pointer']  # offset -> resolved pointer

class LVLReader:
    """Reader for LVL and PTR files."""

    def __init__(self, lvl_data: bytes, file_id: int = 0):
        self.base_offset = 4  # Skip first 4 bytes
        self.data = lvl_data
        self.file_id = file_id
        self.pointers: dict[int, 'Pointer'] = {}

    def read_ptr(self, ptr_data: bytes, is_largo: bool = False) -> list[LVLPointer]:
        """Read pointer references from PTR file."""
        pointers = []
        pos = 0

        if is_largo:
            num_ptrs = len(ptr_data) // 8
        else:
            num_ptrs = struct.unpack_from('<I', ptr_data, pos)[0]
            pos += 4

        for _ in range(num_ptrs):
            file_id = struct.unpack_from('<i', ptr_data, pos)[0]
            ptr_offset = struct.unpack_from('<I', ptr_data, pos + 4)[0]
            pos += 8

            # Read pointer value from LVL data
            ptr_value = struct.unpack_from(
                '<I',
                self.data,
                ptr_offset + self.base_offset
            )[0]

            pointers.append(LVLPointer(
                file_id=file_id,
                offset=ptr_offset,
                target_value=ptr_value
            ))

        return pointers

    def create_pointers(self, ptr_list: list[LVLPointer], files: dict[int, 'LVLFile']):
        """Create resolved pointer map."""
        for ptr_info in ptr_list:
            if ptr_info.file_id != self.file_id:
                continue

            target_file = files.get(ptr_info.file_id, self)
            self.pointers[ptr_info.offset] = Pointer(
                offset=ptr_info.target_value,
                file=target_file
            )

@dataclass
class Pointer:
    """Resolved pointer."""
    offset: int
    file: LVLFile
```

## Usage with SNA

In Montreal engine games like Hype, SNA files are the primary data format. LVL may be used for:
- Compatibility layers
- Alternative data organization
- Debug/development builds

When both formats exist, prefer SNA with RTB relocation.

## References

- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/FileFormat/LVL.cs`
