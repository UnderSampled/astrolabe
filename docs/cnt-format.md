# CNT File Format (Texture Container)

CNT files are archive containers that hold multiple GF texture files.

## File Structure

CNT files have no fixed magic bytes.

### Header

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | int32 | directoryCount | Number of directory entries |
| 0x04 | int32 | fileCount | Number of files |
| 0x08 | byte | isXor | XOR encryption enabled (non-zero = enabled) |
| 0x09 | byte | isChecksum | Checksum validation enabled |
| 0x0A | byte | xorKey | XOR key for string encryption |

### Directory List

If `directoryCount > 0`, for each directory:

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | int32 | stringLength | Length of directory path string |
| 0x04 | byte[] | directoryPath | Directory path (XOR encrypted if isXor != 0) |

If `isChecksum != 0`:

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | byte | checksum | Sum of all directory bytes mod 256 |

### File Entries

For each file (`fileCount` entries):

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | int32 | dirIndex | Directory index (-1 for root) |
| 0x04 | int32 | filenameLength | Length of filename |
| 0x08 | byte[] | filename | Filename (XOR encrypted if isXor != 0) |
| +len | byte[4] | fileXorKey | Per-file XOR key (4 bytes) |
| +4 | uint32 | fileChecksum | File data checksum |
| +4 | int32 | filePointer | Absolute offset to file data |
| +4 | int32 | fileSize | Uncompressed file size |

## XOR Decryption

### String Decryption
```csharp
byte DecryptChar(byte encrypted, byte xorKey) {
    return (byte)(encrypted ^ xorKey);
}
```

### File Data Decryption
```csharp
byte[] DecryptFile(byte[] encrypted, byte[] fileXorKey) {
    byte[] result = new byte[encrypted.Length];
    for (int i = 0; i < encrypted.Length; i++) {
        result[i] = (byte)(encrypted[i] ^ fileXorKey[i % 4]);
    }
    return result;
}
```

## Contained Files

CNT containers typically hold:
- GF texture files (see [GF Format](gf-format.md))
- Other binary assets

## Raymap Code Reference

- `reference/raymap/Assets/Scripts/OpenSpace/FileFormat/Texture/CNT.cs` (lines 80-179)
- `reference/raymap/Assets/Scripts/Libraries/BinarySerializer.OpenSpace/src/DataTypes/CNT/CNT.cs`
