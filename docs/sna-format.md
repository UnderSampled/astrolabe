# SNA File Format

SNA files contain compressed level and game data organized into memory blocks.

## File Structure

SNA files have no fixed magic bytes. The file consists of a sequence of memory blocks.

### Block Header

For the Montreal Engine variant (Hype):

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | byte | module | Module ID (0-255) |
| 0x01 | byte | id | Block ID (0-255) |
| 0x02 | byte | unk1 | Unknown (Montreal+ only) |
| 0x03 | int32 | baseInMemory | Base memory address (-1 if not loaded) |

If `baseInMemory != -1`:

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x07 | uint32 | unk2 | Unknown field 2 |
| 0x0B | uint32 | unk3 | Unknown field 3 |
| 0x0F | uint32 | maxPosMinus9 | Maximum position minus 9 |
| 0x13 | uint32 | size | Uncompressed size of block data |

### Compressed Block Data

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | uint32 | isCompressed | 1 if LZO compressed, 0 if uncompressed |
| 0x04 | uint32 | compressedSize | Size of compressed data |
| 0x08 | uint32 | compressedChecksum | Checksum of compressed data |
| 0x0C | uint32 | decompressedSize | Expected decompressed size |
| 0x10 | uint32 | decompressedChecksum | Expected checksum after decompression |
| 0x14 | byte[] | data | Compressed or uncompressed data |

## Compression

SNA files use LZO compression:

- If `isCompressed == 1`: Data is LZO compressed
- If `isCompressed == 0`: Data is stored uncompressed

The checksum algorithm is Adler32-like with polynomial 0xFFF1.

## Block Identification

Blocks are identified by a `(module, id)` tuple:
- `module`: Identifies the subsystem (e.g., geometry, AI, textures)
- `id`: Identifies the specific block within that module

The combined key is: `key = (module << 8) | id`

## Memory Layout

Each block has a virtual memory address (`baseInMemory`) that represents where it would be loaded in the original game's memory. Pointers within the data reference these virtual addresses and must be relocated to file offsets.

## Raymap Code Reference

- `reference/raymap/Assets/Scripts/OpenSpace/FileFormat/SNA.cs` (lines 105-261)
