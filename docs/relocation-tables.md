# Relocation Table Formats

Relocation tables map virtual memory pointers to file offsets.

## File Types

| Extension | Type ID | Purpose |
|-----------|---------|---------|
| `.rtb` | 0 | SNA block relocations (main level data) |
| `.rtp` | 1 | GPT pointer file relocations |
| `.rts` | 2 | Sound data relocations |
| `.rtt` | 3 | PTX/Texture file relocations |
| `.rtl` | 4 | Unknown (not used in Rayman 2) |
| `.rtd` | 5 | Dialog/language data relocations |
| `.rtg` | 6 | Language-specific SNA blocks |
| `.rtv` | 7 | Video data relocations |

## File Structure

### Header

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | byte | blockCount | Number of pointer blocks |
| 0x01 | uint32 | reserved | Unused (Montreal+ only) |

### Pointer Blocks

For each block (`blockCount` entries):

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | byte | module | Module ID |
| 0x01 | byte | id | Block ID |
| 0x02 | uint32 | pointerCount | Number of pointers in this block |

### Compressed Block Data (if snaCompression enabled)

If `pointerCount > 0` and compression is enabled:

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | uint32 | isCompressed | Compression flag |
| 0x04 | uint32 | compressedSize | Compressed data size |
| 0x08 | uint32 | compressedChecksum | Checksum of compressed data |
| 0x0C | uint32 | decompressedSize | Decompressed size |
| 0x10 | uint32 | decompressedChecksum | Checksum after decompression |
| 0x14 | byte[] | data | LZO compressed pointer data |

### Pointer Entries

For each pointer (`pointerCount` entries, may be compressed):

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | uint32 | offsetInMemory | Virtual memory address of pointer |
| 0x04 | byte | module | Target module ID |
| 0x05 | byte | id | Target block ID |
| 0x06 | byte | byte6 | Unknown (Montreal+ only) |
| 0x07 | byte | byte7 | Unknown (Montreal+ only) |

## Relocation Algorithm

```csharp
void RelocatePointer(PointerEntry entry, Block sourceBlock, Block[] allBlocks) {
    // 1. Find the pointer location in the source block
    long pointerLocation = sourceBlock.FileOffset +
        (entry.OffsetInMemory - sourceBlock.BaseInMemory);

    // 2. Read the 32-bit pointer value
    uint pointerValue = ReadUInt32(pointerLocation);

    // 3. Find the target block
    Block targetBlock = FindBlock(allBlocks, entry.Module, entry.Id);

    // 4. Calculate relocated offset
    long relocatedOffset = (pointerValue - targetBlock.BaseInMemory) +
        targetBlock.FileOffset;

    // 5. Write back the relocated pointer
    WriteUInt32(pointerLocation, (uint)relocatedOffset);
}
```

## Block Identification

Blocks are identified by a combined key:
```csharp
ushort blockKey = (ushort)((module << 8) | id);
```

## Special Cases

- **Module 0x0A**: Handled differently in some engine versions
- **BaseInMemory == -1**: Block not loaded, use global relocation
- **Cross-block pointers**: Target block may be different from source block

## Raymap Code Reference

- `reference/raymap/Assets/Scripts/OpenSpace/FileFormat/RelocationTable.cs` (lines 53-234)
