using SixLabors.ImageSharp;
using SixLabors.ImageSharp.PixelFormats;

namespace Astrolabe.Core.FileFormats;

/// <summary>
/// Reads GF texture files from Hype (Montreal engine variant).
/// </summary>
public class GfReader
{
    public byte Version { get; private set; }
    public int Width { get; private set; }
    public int Height { get; private set; }
    public byte Channels { get; private set; }
    public byte RepeatByte { get; private set; }
    public ushort PaletteLength { get; private set; }
    public byte PaletteBytesPerColor { get; private set; }
    public int PixelCount { get; private set; }
    public GfFormat Format { get; private set; }
    public byte[]? Palette { get; private set; }
    public byte[] RawPixelData { get; private set; } = [];

    private readonly byte[] _data;

    public GfReader(byte[] data)
    {
        _data = data;
        Parse();
    }

    private void Parse()
    {
        using var reader = new BinaryReader(new MemoryStream(_data));

        // Montreal variant header
        Version = reader.ReadByte();
        Width = reader.ReadInt32();
        Height = reader.ReadInt32();
        Channels = reader.ReadByte();

        // Montreal does NOT have mipmaps byte - it's calculated from PixelCount later
        RepeatByte = reader.ReadByte();

        PaletteLength = reader.ReadUInt16();
        PaletteBytesPerColor = reader.ReadByte();

        // Unknown bytes
        reader.ReadByte(); // byte_0F
        reader.ReadByte(); // byte_10
        reader.ReadByte(); // byte_11
        reader.ReadUInt32(); // uint_12

        PixelCount = reader.ReadInt32();

        byte montrealType = reader.ReadByte();
        Format = montrealType switch
        {
            5 => GfFormat.Palette,
            10 => GfFormat.RGB565,
            11 => GfFormat.RGBA1555,
            12 => GfFormat.RGBA4444,
            _ => GfFormat.Unknown
        };

        // Read palette if present
        if (PaletteLength > 0 && PaletteBytesPerColor > 0)
        {
            Palette = reader.ReadBytes(PaletteLength * PaletteBytesPerColor);
        }

        // Read remaining RLE-encoded pixel data
        int remaining = (int)(reader.BaseStream.Length - reader.BaseStream.Position);
        RawPixelData = reader.ReadBytes(remaining);
    }

    /// <summary>
    /// Decodes the RLE-encoded pixel data.
    /// </summary>
    private byte[] DecodeRle()
    {
        int expectedSize = PixelCount * Channels;
        var result = new byte[expectedSize];
        int resultIndex = 0;

        using var reader = new BinaryReader(new MemoryStream(RawPixelData));

        // Decode each channel separately
        for (int channel = 0; channel < Channels; channel++)
        {
            int pixelsDecoded = 0;
            while (pixelsDecoded < PixelCount && reader.BaseStream.Position < reader.BaseStream.Length)
            {
                byte b = reader.ReadByte();

                if (b == RepeatByte && reader.BaseStream.Position < reader.BaseStream.Length - 1)
                {
                    // RLE: next byte is value, byte after is count
                    byte value = reader.ReadByte();
                    byte count = reader.ReadByte();

                    for (int i = 0; i < count && pixelsDecoded < PixelCount; i++)
                    {
                        result[channel * PixelCount + pixelsDecoded] = value;
                        pixelsDecoded++;
                    }
                }
                else
                {
                    // Literal byte
                    result[channel * PixelCount + pixelsDecoded] = b;
                    pixelsDecoded++;
                }
            }
        }

        return result;
    }

    /// <summary>
    /// Decodes the texture to RGBA8888 format (only the main texture, not mipmaps).
    /// </summary>
    public byte[] DecodeToRgba()
    {
        var decoded = DecodeRle();
        int mainPixels = Width * Height;
        var result = new byte[mainPixels * 4];

        switch (Format)
        {
            case GfFormat.Palette:
                DecodePalette(decoded, result, mainPixels);
                break;
            case GfFormat.RGB565:
                DecodeRgb565(decoded, result, mainPixels);
                break;
            case GfFormat.RGBA1555:
                DecodeRgba1555(decoded, result, mainPixels);
                break;
            case GfFormat.RGBA4444:
                DecodeRgba4444(decoded, result, mainPixels);
                break;
            default:
                // Unknown format, fill with gray
                for (int i = 0; i < mainPixels; i++)
                {
                    result[i * 4 + 0] = 128;
                    result[i * 4 + 1] = 128;
                    result[i * 4 + 2] = 128;
                    result[i * 4 + 3] = 255;
                }
                break;
        }

        return result;
    }

    private void DecodePalette(byte[] decoded, byte[] result, int mainPixels)
    {
        if (Palette == null) return;

        for (int i = 0; i < mainPixels && i < decoded.Length; i++)
        {
            int paletteIndex = decoded[i];
            int paletteOffset = paletteIndex * PaletteBytesPerColor;

            if (paletteOffset + PaletteBytesPerColor <= Palette.Length)
            {
                // Palette is BGR or BGRA
                result[i * 4 + 2] = Palette[paletteOffset + 0]; // B
                result[i * 4 + 1] = Palette[paletteOffset + 1]; // G
                result[i * 4 + 0] = Palette[paletteOffset + 2]; // R
                result[i * 4 + 3] = PaletteBytesPerColor >= 4 ? Palette[paletteOffset + 3] : (byte)255; // A
            }
        }
    }

    private void DecodeRgb565(byte[] decoded, byte[] result, int mainPixels)
    {
        // Channels are stored separately: all lo bytes, then all hi bytes
        for (int i = 0; i < mainPixels; i++)
        {
            byte lo = decoded[i];
            byte hi = decoded[PixelCount + i];
            ushort pixel = (ushort)(lo | (hi << 8));

            // Format is RGB565:
            // bits 11-15: red (5 bits)
            // bits 5-10: green (6 bits)
            // bits 0-4: blue (5 bits)
            uint r = (uint)((pixel >> 11) & 0x1F);
            uint g = (uint)((pixel >> 5) & 0x3F);
            uint b = (uint)(pixel & 0x1F);

            // Scale to 8-bit
            result[i * 4 + 0] = (byte)((r * 255) / 31);
            result[i * 4 + 1] = (byte)((g * 255) / 63);
            result[i * 4 + 2] = (byte)((b * 255) / 31);
            result[i * 4 + 3] = 255; // No alpha in RGB565
        }
    }

    private void DecodeRgba1555(byte[] decoded, byte[] result, int mainPixels)
    {
        // Channels are stored separately: all lo bytes, then all hi bytes
        for (int i = 0; i < mainPixels; i++)
        {
            byte lo = decoded[i];
            byte hi = decoded[PixelCount + i];
            ushort pixel = (ushort)(lo | (hi << 8));

            // Format is ARGB1555:
            // bit 15: alpha (1 bit)
            // bits 10-14: red (5 bits)
            // bits 5-9: green (5 bits)
            // bits 0-4: blue (5 bits)
            uint a = (uint)((pixel >> 15) & 0x1);
            uint r = (uint)((pixel >> 10) & 0x1F);
            uint g = (uint)((pixel >> 5) & 0x1F);
            uint b = (uint)(pixel & 0x1F);

            // Scale 5-bit values to 8-bit
            result[i * 4 + 0] = (byte)((r * 255) / 31);
            result[i * 4 + 1] = (byte)((g * 255) / 31);
            result[i * 4 + 2] = (byte)((b * 255) / 31);
            result[i * 4 + 3] = (byte)(a * 255);
        }
    }

    private void DecodeRgba4444(byte[] decoded, byte[] result, int mainPixels)
    {
        // Channels are stored separately: all lo bytes, then all hi bytes
        for (int i = 0; i < mainPixels; i++)
        {
            byte lo = decoded[i];
            byte hi = decoded[PixelCount + i];
            ushort pixel = (ushort)(lo | (hi << 8));

            // Format is ARGB4444:
            // bits 12-15: alpha (4 bits)
            // bits 8-11: red (4 bits)
            // bits 4-7: green (4 bits)
            // bits 0-3: blue (4 bits)
            uint a = (uint)((pixel >> 12) & 0xF);
            uint r = (uint)((pixel >> 8) & 0xF);
            uint g = (uint)((pixel >> 4) & 0xF);
            uint b = (uint)(pixel & 0xF);

            // Scale 4-bit values to 8-bit
            result[i * 4 + 0] = (byte)((r * 255) / 15);
            result[i * 4 + 1] = (byte)((g * 255) / 15);
            result[i * 4 + 2] = (byte)((b * 255) / 15);
            result[i * 4 + 3] = (byte)((a * 255) / 15);
        }
    }

    /// <summary>
    /// Saves the texture as a PNG file.
    /// </summary>
    public void SaveAsPng(string outputPath)
    {
        var rgba = DecodeToRgba();

        using var image = Image.LoadPixelData<Rgba32>(rgba, Width, Height);
        image.SaveAsPng(outputPath);
    }
}

public enum GfFormat
{
    Unknown,
    Palette,
    RGB565,
    RGBA1555,
    RGBA4444,
    RGB888,
    RGBA8888
}
