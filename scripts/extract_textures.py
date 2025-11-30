#!/usr/bin/env python3
"""
Extract textures from Hype: The Time Quest CNT/GF files.

CNT format: Container file with XOR-encrypted file entries
GF format: Texture format with RLE compression
"""

import struct
import sys
from pathlib import Path
from dataclasses import dataclass
from io import BytesIO

# Try to use PIL for PNG output, fall back to simple TGA if not available
try:
    from PIL import Image
    HAS_PIL = True
except ImportError:
    HAS_PIL = False
    print("Note: PIL not available, will output raw TGA files")


@dataclass
class CNTFile:
    """A file entry in a CNT container."""
    name: str
    directory: str
    pointer: int
    size: int
    xor_key: bytes
    checksum: int

    @property
    def full_name(self) -> str:
        if self.directory:
            return f"{self.directory}/{self.name}"
        return self.name


@dataclass
class GFTexture:
    """A decoded GF texture."""
    width: int
    height: int
    channels: int
    pixels: bytes  # RGBA data
    name: str = ""


def read_cnt(path: Path) -> list[CNTFile]:
    """Read a CNT container and return file entries."""
    files = []

    with open(path, "rb") as f:
        dir_count = struct.unpack("<I", f.read(4))[0]
        file_count = struct.unpack("<I", f.read(4))[0]
        is_xor = f.read(1)[0]
        is_checksum = f.read(1)[0]
        xor_key = f.read(1)[0]

        # Read directories
        directories = []
        cur_checksum = 0
        for _ in range(dir_count):
            str_len = struct.unpack("<I", f.read(4))[0]
            dir_bytes = f.read(str_len)

            if is_xor or is_checksum:
                dir_chars = []
                for b in dir_bytes:
                    if is_xor:
                        b = xor_key ^ b
                    if is_checksum:
                        cur_checksum = (cur_checksum + b) % 256
                    dir_chars.append(chr(b))
                directory = "".join(dir_chars)
            else:
                directory = dir_bytes.decode("latin-1")

            directories.append(directory)

        # Read directory checksum
        dir_checksum = f.read(1)[0]

        # Read files
        for _ in range(file_count):
            dir_index = struct.unpack("<i", f.read(4))[0]
            str_len = struct.unpack("<I", f.read(4))[0]

            if is_xor:
                name_bytes = f.read(str_len)
                name = "".join(chr(xor_key ^ b) for b in name_bytes)
            else:
                name = f.read(str_len).decode("latin-1")

            file_xor_key = f.read(4)
            file_checksum = struct.unpack("<I", f.read(4))[0]
            data_pointer = struct.unpack("<I", f.read(4))[0]
            file_size = struct.unpack("<I", f.read(4))[0]

            directory = directories[dir_index] if dir_index >= 0 else ""

            files.append(CNTFile(
                name=name,
                directory=directory,
                pointer=data_pointer,
                size=file_size,
                xor_key=file_xor_key,
                checksum=file_checksum,
            ))

    return files


def read_file_data(cnt_path: Path, file_entry: CNTFile) -> bytes:
    """Read and decrypt a file from the CNT container."""
    with open(cnt_path, "rb") as f:
        f.seek(file_entry.pointer)
        data = bytearray(f.read(file_entry.size))

    # XOR decrypt
    for i in range(file_entry.size):
        if (file_entry.size % 4) + i < file_entry.size:
            data[i] ^= file_entry.xor_key[i % 4]

    return bytes(data)


def extract_bits(number: int, count: int, offset: int) -> int:
    """Extract bits from a number."""
    return ((1 << count) - 1) & (number >> offset)


def read_gf(data: bytes, name: str = "") -> GFTexture:
    """Read a GF texture from raw bytes."""
    reader = BytesIO(data)

    # Montreal format
    version = struct.unpack("<B", reader.read(1))[0]
    fmt = 1555  # Default for Montreal

    width = struct.unpack("<I", reader.read(4))[0]
    height = struct.unpack("<I", reader.read(4))[0]
    channel_pixels = width * height

    channels = struct.unpack("<B", reader.read(1))[0]
    repeat_byte = struct.unpack("<B", reader.read(1))[0]

    # Montreal specific
    palette_num_colors = struct.unpack("<H", reader.read(2))[0]
    palette_bytes_per_color = struct.unpack("<B", reader.read(1))[0]

    byte1 = struct.unpack("<B", reader.read(1))[0]
    byte2 = struct.unpack("<B", reader.read(1))[0]
    byte3 = struct.unpack("<B", reader.read(1))[0]
    num4 = struct.unpack("<I", reader.read(4))[0]
    channel_pixels = struct.unpack("<I", reader.read(4))[0]  # Includes mipmaps
    montreal_type = struct.unpack("<B", reader.read(1))[0]

    # Determine format
    if montreal_type == 5:
        fmt = 0  # palette
    elif montreal_type == 10:
        fmt = 565
    elif montreal_type == 11:
        fmt = 1555
    elif montreal_type == 12:
        fmt = 4444
    else:
        print(f"Warning: unknown Montreal GF format {montreal_type}")
        fmt = 1555

    # Read palette if present
    palette = None
    if palette_num_colors > 0 and palette_bytes_per_color > 0:
        palette = reader.read(palette_bytes_per_color * palette_num_colors)

    # Read channel data with RLE decompression
    pixel_data = bytearray(channels * channel_pixels)
    channel = 0
    while channel < channels:
        pixel = 0
        while pixel < channel_pixels:
            b1 = reader.read(1)
            if not b1:
                break
            b1 = b1[0]
            if b1 == repeat_byte:
                value = reader.read(1)[0]
                count = reader.read(1)[0]
                for _ in range(count):
                    if pixel < channel_pixels:
                        pixel_data[channel + pixel * channels] = value
                        pixel += 1
            else:
                pixel_data[channel + pixel * channels] = b1
                pixel += 1
        channel += 1

    # Convert to RGBA
    # Only use first width*height pixels (ignore mipmaps)
    main_pixels = width * height
    rgba = bytearray(main_pixels * 4)

    if channels >= 3:
        pos = 0
        for i in range(min(main_pixels, channel_pixels)):
            b = pixel_data[pos + 0]
            g = pixel_data[pos + 1]
            r = pixel_data[pos + 2]
            a = pixel_data[pos + 3] if channels == 4 else 255
            rgba[i * 4 + 0] = r
            rgba[i * 4 + 1] = g
            rgba[i * 4 + 2] = b
            rgba[i * 4 + 3] = a
            pos += channels

    elif channels == 2:
        pos = 0
        for i in range(min(main_pixels, channel_pixels)):
            pixel = struct.unpack("<H", pixel_data[pos:pos + 2])[0]

            if fmt == 4444:
                a = extract_bits(pixel, 4, 12) * 17
                r = extract_bits(pixel, 4, 8) * 17
                g = extract_bits(pixel, 4, 4) * 17
                b = extract_bits(pixel, 4, 0) * 17
            elif fmt == 1555:
                a = 255 if extract_bits(pixel, 1, 15) else 0
                r = int(extract_bits(pixel, 5, 10) * 255 / 31)
                g = int(extract_bits(pixel, 5, 5) * 255 / 31)
                b = int(extract_bits(pixel, 5, 0) * 255 / 31)
            else:  # 565
                a = 255
                r = int(extract_bits(pixel, 5, 11) * 255 / 31)
                g = int(extract_bits(pixel, 6, 5) * 255 / 63)
                b = int(extract_bits(pixel, 5, 0) * 255 / 31)

            rgba[i * 4 + 0] = r
            rgba[i * 4 + 1] = g
            rgba[i * 4 + 2] = b
            rgba[i * 4 + 3] = a
            pos += 2

    elif channels == 1:
        for i in range(min(main_pixels, channel_pixels)):
            idx = pixel_data[i]
            if palette and palette_bytes_per_color >= 3:
                b = palette[idx * palette_bytes_per_color + 0]
                g = palette[idx * palette_bytes_per_color + 1]
                r = palette[idx * palette_bytes_per_color + 2]
                a = palette[idx * palette_bytes_per_color + 3] if palette_bytes_per_color >= 4 else 255
            else:
                r = g = b = idx
                a = 255

            rgba[i * 4 + 0] = r
            rgba[i * 4 + 1] = g
            rgba[i * 4 + 2] = b
            rgba[i * 4 + 3] = a

    return GFTexture(
        width=width,
        height=height,
        channels=channels,
        pixels=bytes(rgba),
        name=name,
    )


def save_texture(texture: GFTexture, path: Path) -> None:
    """Save texture to file (PNG if PIL available, otherwise TGA)."""
    if HAS_PIL:
        img = Image.frombytes("RGBA", (texture.width, texture.height), texture.pixels)
        img = img.transpose(Image.FLIP_TOP_BOTTOM)  # Flip for correct orientation
        path = path.with_suffix(".png")
        img.save(path)
    else:
        # Write simple TGA
        path = path.with_suffix(".tga")
        with open(path, "wb") as f:
            # TGA header
            f.write(bytes([
                0,  # ID length
                0,  # Color map type
                2,  # Image type (uncompressed true-color)
                0, 0, 0, 0, 0,  # Color map spec
                0, 0,  # X origin
                0, 0,  # Y origin
            ]))
            f.write(struct.pack("<HH", texture.width, texture.height))
            f.write(bytes([32, 0]))  # Pixel depth, image descriptor

            # Write BGRA pixels
            for y in range(texture.height):
                for x in range(texture.width):
                    i = (y * texture.width + x) * 4
                    r, g, b, a = texture.pixels[i:i + 4]
                    f.write(bytes([b, g, r, a]))


def main():
    output_dir = Path("/home/deck/code/astrolabe/output")
    texture_output = output_dir / "textures"
    texture_output.mkdir(exist_ok=True)

    # Find CNT files
    cnt_files = [
        output_dir / "Gamedata/Textures.cnt",
        output_dir / "Gamedata/World/Levels/fix.cnt",
        output_dir / "Gamedata/Vignette.cnt",
    ]

    total_extracted = 0

    for cnt_path in cnt_files:
        if not cnt_path.exists():
            print(f"Skipping {cnt_path} (not found)")
            continue

        print(f"\n{'=' * 60}")
        print(f"Processing {cnt_path.name}...")
        print("=" * 60)

        try:
            files = read_cnt(cnt_path)
            print(f"Found {len(files)} files")

            gf_files = [f for f in files if f.name.lower().endswith(".gf")]
            print(f"Found {len(gf_files)} GF textures")

            for file_entry in gf_files[:50]:  # Limit to first 50 per container
                try:
                    data = read_file_data(cnt_path, file_entry)
                    texture = read_gf(data, file_entry.full_name)

                    # Create output path
                    safe_name = file_entry.name.replace("/", "_").replace("\\", "_")
                    out_path = texture_output / safe_name

                    save_texture(texture, out_path)
                    total_extracted += 1

                    if total_extracted <= 20:
                        print(f"  Extracted {file_entry.full_name} ({texture.width}x{texture.height})")

                except Exception as e:
                    print(f"  Error extracting {file_entry.full_name}: {e}")

            if len(gf_files) > 50:
                print(f"  ... and {len(gf_files) - 50} more textures")

        except Exception as e:
            print(f"Error reading {cnt_path}: {e}")
            import traceback
            traceback.print_exc()

    print(f"\n\nTotal textures extracted: {total_extracted}")
    print(f"Output directory: {texture_output}")


if __name__ == "__main__":
    main()
