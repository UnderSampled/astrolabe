# GF File Format (Texture)

GF files contain texture data, typically stored inside CNT containers.

## File Structure

GF files have no fixed magic bytes. The format varies by engine version.

### Montreal Engine Header (Hype)

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | byte | version | Format version |
| 0x01 | uint16 | paletteLength | Color palette entry count |
| 0x03 | byte | paletteBytesPerColor | Bytes per palette entry (3 or 4) |
| 0x04 | byte | byte_0F | Unknown |
| 0x05 | byte | byte_10 | Unknown |
| 0x06 | byte | byte_11 | Unknown |
| 0x07 | uint32 | uint_12 | Unknown |
| 0x0B | int32 | pixelCount | Total pixel count (including mipmaps) |
| 0x0F | byte | montrealType | Format type code |
| 0x10 | byte[] | palette | Palette data (paletteLength * paletteBytesPerColor) |

### Montreal Type Codes

| Code | Format | Description |
|------|--------|-------------|
| 5 | Palette | 8-bit indexed with palette |
| 10 | RGB565 | 16-bit BGR (5-6-5) |
| 11 | RGBA1555 | 16-bit BGRA (5-5-5-1) |
| 12 | RGBA4444 | 16-bit BGRA (4-4-4-4) |

### Rayman 3+ Engine Header

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | uint32 | format | Format code |
| 0x04 | int32 | width | Texture width in pixels |
| 0x08 | int32 | height | Texture height in pixels |
| 0x0C | byte | channels | Channels per pixel (1, 2, 3, or 4) |
| 0x0D | byte | mipmapsCount | Number of mipmap levels |
| 0x0E | byte | repeatByte | RLE repeat marker byte |
| 0x0F | byte[] | pixelData | RLE-encoded pixel data |

### Format Codes

| Code | Format | Bytes/Pixel | Description |
|------|--------|-------------|-------------|
| 8888 | BGRA32 | 4 | 8 bits per channel |
| 888 | BGR24 | 3 | No alpha |
| 565 | BGR16 | 2 | B:5, G:6, R:5 |
| 1555 | BGRA16 | 2 | B:5, G:5, R:5, A:1 |
| 4444 | BGRA16 | 2 | 4 bits per channel |
| 88 | GrayAlpha | 2 | Gray + Alpha |
| 8 | Gray | 1 | Grayscale only |
| 0 | Indexed | 1 | 8-bit with palette |

## RLE Encoding

Pixel data is RLE (Run-Length Encoding) compressed per channel.

### Decoding Algorithm

```csharp
byte[] DecodeChannel(BinaryReader reader, int pixelCount, byte repeatByte) {
    byte[] result = new byte[pixelCount];
    int i = 0;
    while (i < pixelCount) {
        byte b = reader.ReadByte();
        if (b == repeatByte) {
            byte value = reader.ReadByte();
            byte count = reader.ReadByte();
            for (int j = 0; j < count && i < pixelCount; j++) {
                result[i++] = value;
            }
        } else {
            result[i++] = b;
        }
    }
    return result;
}
```

### Channel Order

Channels are stored sequentially (not interleaved):
1. All Blue values
2. All Green values
3. All Red values
4. All Alpha values (if present)

## Pixel Format Decoding

### RGB565
```csharp
ushort pixel = reader.ReadUInt16();
byte b = (byte)((pixel & 0x001F) << 3);
byte g = (byte)((pixel & 0x07E0) >> 3);
byte r = (byte)((pixel & 0xF800) >> 8);
```

### RGBA1555
```csharp
ushort pixel = reader.ReadUInt16();
byte b = (byte)((pixel & 0x001F) << 3);
byte g = (byte)((pixel & 0x03E0) >> 2);
byte r = (byte)((pixel & 0x7C00) >> 7);
byte a = (byte)((pixel & 0x8000) != 0 ? 255 : 0);
```

### RGBA4444
```csharp
ushort pixel = reader.ReadUInt16();
byte b = (byte)((pixel & 0x000F) << 4);
byte g = (byte)((pixel & 0x00F0));
byte r = (byte)((pixel & 0x0F00) >> 4);
byte a = (byte)((pixel & 0xF000) >> 8);
```

## Raymap Code Reference

- `reference/raymap/Assets/Scripts/Libraries/BinarySerializer.OpenSpace/src/DataTypes/Graphics/GF/GF.cs` (lines 370-458)
- `reference/raymap/Assets/Scripts/Libraries/BinarySerializer.OpenSpace/src/DataTypes/Graphics/GF/GF_Encoder.cs`
