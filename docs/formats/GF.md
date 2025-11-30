# GF Texture Format

**Extension:** `.gf` (embedded in SNA/PTX)
**Purpose:** OpenSpace texture data with RLE compression
**Encryption:** None (contained within encrypted SNA)
**Compression:** Run-Length Encoding (RLE) per channel

## Overview

GF (Graphics Format) is the OpenSpace engine's native texture format. It supports multiple color formats and uses a simple RLE compression scheme per color channel.

## File Structure

### Montreal Engine Header (Hype)

```
Offset  Size  Type   Description
------  ----  ----   -----------
0x00    1     u8     Version
0x01    4     u32    Width
0x05    4     u32    Height
0x09    1     u8     Channel count (1, 2, 3, or 4)
0x0A    1     u8     Repeat byte (RLE marker)
0x0B    2     u16    Palette color count (0 if no palette)
0x0D    1     u8     Palette bytes per color
0x0E    1     u8     Byte 1 (unknown)
0x0F    1     u8     Byte 2 (unknown)
0x10    1     u8     Byte 3 (unknown)
0x11    4     u32    Num 4 (unknown)
0x15    4     u32    Channel pixels (including mipmaps)
0x19    1     u8     Montreal type (format identifier)
--- If palette ---
0x1A    N     bytes  Palette data (bytes_per_color * color_count)
--- Then ---
...     ...   ...    RLE-compressed channel data
```

### R2/R3 Header

```
Offset  Size  Type   Description
------  ----  ----   -----------
0x00    4     u32    Format code (8888, etc.)
0x04    4     u32    Width
0x08    4     u32    Height
0x0C    1     u8     Channel count
0x0D    1     u8     Enlarge byte (mipmap levels)
0x0E    1     u8     Repeat byte
--- R3 with palette (channels=1) ---
...     1024  bytes  256-color BGRA palette
--- Then ---
...     ...   ...    RLE-compressed channel data
```

## Montreal Format Types

| Type | Format | Description |
|------|--------|-------------|
| 5 | Indexed | Palette-based (uses palette data) |
| 10 | 565 | 16-bit RGB (5-6-5 bits) |
| 11 | 1555 | 16-bit ARGB (1-5-5-5 bits) |
| 12 | 4444 | 16-bit ARGB (4-4-4-4 bits) |

## Color Formats

### 8888 (32-bit BGRA)
- Channels: 4
- Per pixel: Blue, Green, Red, Alpha (1 byte each)
- Used by R2/R3

### 1555 (16-bit ARGB)
- Channels: 2
- Bit layout: `A RRRRR GGGGG BBBBB`
- Alpha: 1 bit (on/off)
- RGB: 5 bits each
- Common in Montreal engine

### 4444 (16-bit ARGB)
- Channels: 2
- Bit layout: `AAAA RRRR GGGG BBBB`
- 4 bits per component

### 565 (16-bit RGB)
- Channels: 2
- Bit layout: `RRRRR GGGGGG BBBBB`
- No alpha channel
- Green has extra bit for human eye sensitivity

### Indexed (8-bit)
- Channels: 1
- Pixel values index into palette
- Palette: BGRA (4 bytes) or BGR (3 bytes) per entry

## RLE Compression

Each color channel is compressed independently using run-length encoding.

### Algorithm

```
For each pixel in channel:
    Read byte B1
    If B1 == repeat_byte:
        Read byte VALUE
        Read byte COUNT
        Output VALUE repeated COUNT times
    Else:
        Output B1 as single pixel
```

### Decompression Example

```python
def decompress_channel(data: bytes, repeat_byte: int, pixel_count: int) -> bytes:
    """Decompress a single RLE-compressed channel."""
    output = bytearray()
    pos = 0

    while len(output) < pixel_count:
        b1 = data[pos]
        pos += 1

        if b1 == repeat_byte:
            value = data[pos]
            count = data[pos + 1]
            pos += 2
            output.extend([value] * count)
        else:
            output.append(b1)

    return bytes(output[:pixel_count])
```

## Multi-Channel Reading

Channels are stored sequentially, all pixels of one channel before the next:

```python
def read_channels(data: bytes, channels: int, pixels: int, repeat_byte: int) -> bytes:
    """Read and interleave all channels."""
    result = bytearray(channels * pixels)
    pos = 0

    for channel in range(channels):
        pixel = 0
        while pixel < pixels:
            b1 = data[pos]
            pos += 1

            if b1 == repeat_byte:
                value = data[pos]
                count = data[pos + 1]
                pos += 2
                for _ in range(count):
                    result[channel + pixel * channels] = value
                    pixel += 1
            else:
                result[channel + pixel * channels] = b1
                pixel += 1

    return bytes(result)
```

## Python Implementation

```python
from dataclasses import dataclass
import struct

@dataclass
class GFTexture:
    """Parsed GF texture."""
    width: int
    height: int
    channels: int
    format: int  # 8888, 1555, 4444, 565, or 0 (indexed)
    is_transparent: bool
    pixels: bytes  # RGBA pixel data

def parse_gf_montreal(data: bytes) -> GFTexture:
    """Parse a Montreal-engine GF texture."""
    pos = 0

    version = data[pos]
    pos += 1

    width = struct.unpack_from('<I', data, pos)[0]
    pos += 4
    height = struct.unpack_from('<I', data, pos)[0]
    pos += 4

    channels = data[pos]
    pos += 1
    repeat_byte = data[pos]
    pos += 1

    palette_colors = struct.unpack_from('<H', data, pos)[0]
    pos += 2
    palette_bpc = data[pos]
    pos += 1

    # Skip unknown bytes
    pos += 3
    pos += 4  # num4

    channel_pixels = struct.unpack_from('<I', data, pos)[0]
    pos += 4

    montreal_type = data[pos]
    pos += 1

    # Determine format
    format_map = {5: 0, 10: 565, 11: 1555, 12: 4444}
    tex_format = format_map.get(montreal_type, 1555)

    # Read palette if present
    palette = None
    if palette_colors > 0 and palette_bpc > 0:
        palette_size = palette_colors * palette_bpc
        palette = data[pos:pos + palette_size]
        pos += palette_size

    # Read RLE-compressed pixel data
    pixel_data = _read_channels(data[pos:], channels, channel_pixels, repeat_byte)

    # Convert to RGBA
    rgba = _convert_to_rgba(pixel_data, channels, tex_format, width, height, palette, palette_bpc)

    is_transparent = (channels == 4) or (tex_format in [1555, 4444]) or (palette_bpc == 4)

    return GFTexture(
        width=width,
        height=height,
        channels=channels,
        format=tex_format,
        is_transparent=is_transparent,
        pixels=rgba
    )

def _convert_to_rgba(data: bytes, channels: int, fmt: int,
                     width: int, height: int,
                     palette: bytes | None, palette_bpc: int) -> bytes:
    """Convert pixel data to RGBA format."""
    pixels = width * height
    rgba = bytearray(pixels * 4)

    if channels >= 3:
        # BGR(A) format
        for i in range(pixels):
            b = data[i * channels + 0]
            g = data[i * channels + 1]
            r = data[i * channels + 2]
            a = data[i * channels + 3] if channels == 4 else 255
            rgba[i * 4:i * 4 + 4] = bytes([r, g, b, a])

    elif channels == 2:
        # 16-bit format
        for i in range(pixels):
            pixel = struct.unpack_from('<H', data, i * 2)[0]

            if fmt == 1555:
                a = ((pixel >> 15) & 1) * 255
                r = ((pixel >> 10) & 0x1F) * 255 // 31
                g = ((pixel >> 5) & 0x1F) * 255 // 31
                b = (pixel & 0x1F) * 255 // 31
            elif fmt == 4444:
                a = ((pixel >> 12) & 0xF) * 255 // 15
                r = ((pixel >> 8) & 0xF) * 255 // 15
                g = ((pixel >> 4) & 0xF) * 255 // 15
                b = (pixel & 0xF) * 255 // 15
            else:  # 565
                r = ((pixel >> 11) & 0x1F) * 255 // 31
                g = ((pixel >> 5) & 0x3F) * 255 // 63
                b = (pixel & 0x1F) * 255 // 31
                a = 255

            rgba[i * 4:i * 4 + 4] = bytes([r, g, b, a])

    elif channels == 1 and palette is not None:
        # Indexed format
        for i in range(pixels):
            idx = data[i]
            if palette_bpc == 4:
                b = palette[idx * 4 + 0]
                g = palette[idx * 4 + 1]
                r = palette[idx * 4 + 2]
                a = palette[idx * 4 + 3]
            else:
                b = palette[idx * 3 + 0]
                g = palette[idx * 3 + 1]
                r = palette[idx * 3 + 2]
                a = 255
            rgba[i * 4:i * 4 + 4] = bytes([r, g, b, a])

    return bytes(rgba)
```

## Bit Extraction Helper

```python
def extract_bits(value: int, count: int, offset: int) -> int:
    """Extract 'count' bits starting at 'offset' from value."""
    return (value >> offset) & ((1 << count) - 1)
```

## References

- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/FileFormat/Texture/GF.cs`
- Original Rayman2Lib: https://github.com/szymski/Rayman2Lib
