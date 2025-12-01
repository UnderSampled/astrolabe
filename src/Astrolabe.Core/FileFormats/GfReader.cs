namespace Astrolabe.Core.FileFormats;

/// <summary>
/// Reads GF texture files from Hype (Montreal engine variant).
/// </summary>
public class GfReader
{
    public byte Version { get; private set; }
    public ushort PaletteLength { get; private set; }
    public byte PaletteBytesPerColor { get; private set; }
    public int PixelCount { get; private set; }
    public GfFormat Format { get; private set; }
    public int Width { get; private set; }
    public int Height { get; private set; }
    public byte[]? Palette { get; private set; }
    public byte[] PixelData { get; private set; } = [];

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
        if (PaletteLength > 0)
        {
            Palette = reader.ReadBytes(PaletteLength * PaletteBytesPerColor);
        }

        // Try to determine dimensions from pixel count
        // Assume square or power-of-2 dimensions
        Width = (int)Math.Sqrt(PixelCount);
        Height = PixelCount / Width;

        // Adjust for non-square textures
        if (Width * Height != PixelCount)
        {
            // Try common aspect ratios
            for (int w = 1; w <= PixelCount; w++)
            {
                if (PixelCount % w == 0)
                {
                    int h = PixelCount / w;
                    if (IsPowerOfTwo(w) && IsPowerOfTwo(h))
                    {
                        Width = w;
                        Height = h;
                        break;
                    }
                }
            }
        }

        // Read remaining pixel data
        int remaining = (int)(reader.BaseStream.Length - reader.BaseStream.Position);
        PixelData = reader.ReadBytes(remaining);
    }

    private static bool IsPowerOfTwo(int x) => x > 0 && (x & (x - 1)) == 0;

    /// <summary>
    /// Decodes the texture to RGBA8888 format.
    /// </summary>
    public byte[] DecodeToRgba()
    {
        var result = new byte[PixelCount * 4];

        switch (Format)
        {
            case GfFormat.Palette:
                DecodePalette(result);
                break;
            case GfFormat.RGB565:
                DecodeRgb565(result);
                break;
            case GfFormat.RGBA1555:
                DecodeRgba1555(result);
                break;
            case GfFormat.RGBA4444:
                DecodeRgba4444(result);
                break;
            default:
                // Unknown format, try raw interpretation
                DecodeRaw(result);
                break;
        }

        return result;
    }

    private void DecodePalette(byte[] result)
    {
        if (Palette == null) return;

        for (int i = 0; i < PixelCount && i < PixelData.Length; i++)
        {
            int paletteIndex = PixelData[i];
            int paletteOffset = paletteIndex * PaletteBytesPerColor;

            if (paletteOffset + PaletteBytesPerColor <= Palette.Length)
            {
                result[i * 4 + 0] = Palette[paletteOffset + 0]; // R (or B)
                result[i * 4 + 1] = Palette[paletteOffset + 1]; // G
                result[i * 4 + 2] = PaletteBytesPerColor >= 3 ? Palette[paletteOffset + 2] : (byte)0; // B (or R)
                result[i * 4 + 3] = PaletteBytesPerColor >= 4 ? Palette[paletteOffset + 3] : (byte)255; // A
            }
        }
    }

    private void DecodeRgb565(byte[] result)
    {
        using var reader = new BinaryReader(new MemoryStream(PixelData));

        for (int i = 0; i < PixelCount && reader.BaseStream.Position + 2 <= reader.BaseStream.Length; i++)
        {
            ushort pixel = reader.ReadUInt16();
            result[i * 4 + 2] = (byte)((pixel & 0x001F) << 3); // B
            result[i * 4 + 1] = (byte)((pixel & 0x07E0) >> 3); // G
            result[i * 4 + 0] = (byte)((pixel & 0xF800) >> 8); // R
            result[i * 4 + 3] = 255; // A
        }
    }

    private void DecodeRgba1555(byte[] result)
    {
        using var reader = new BinaryReader(new MemoryStream(PixelData));

        for (int i = 0; i < PixelCount && reader.BaseStream.Position + 2 <= reader.BaseStream.Length; i++)
        {
            ushort pixel = reader.ReadUInt16();
            result[i * 4 + 2] = (byte)((pixel & 0x001F) << 3); // B
            result[i * 4 + 1] = (byte)((pixel & 0x03E0) >> 2); // G
            result[i * 4 + 0] = (byte)((pixel & 0x7C00) >> 7); // R
            result[i * 4 + 3] = (byte)((pixel & 0x8000) != 0 ? 255 : 0); // A
        }
    }

    private void DecodeRgba4444(byte[] result)
    {
        using var reader = new BinaryReader(new MemoryStream(PixelData));

        for (int i = 0; i < PixelCount && reader.BaseStream.Position + 2 <= reader.BaseStream.Length; i++)
        {
            ushort pixel = reader.ReadUInt16();
            result[i * 4 + 2] = (byte)((pixel & 0x000F) << 4); // B
            result[i * 4 + 1] = (byte)((pixel & 0x00F0)); // G
            result[i * 4 + 0] = (byte)((pixel & 0x0F00) >> 4); // R
            result[i * 4 + 3] = (byte)((pixel & 0xF000) >> 8); // A
        }
    }

    private void DecodeRaw(byte[] result)
    {
        // Try to interpret as raw BGRA or BGR data
        int bytesPerPixel = PixelData.Length / PixelCount;

        if (bytesPerPixel >= 3)
        {
            for (int i = 0; i < PixelCount; i++)
            {
                int offset = i * bytesPerPixel;
                if (offset + bytesPerPixel <= PixelData.Length)
                {
                    result[i * 4 + 2] = PixelData[offset + 0]; // B
                    result[i * 4 + 1] = PixelData[offset + 1]; // G
                    result[i * 4 + 0] = PixelData[offset + 2]; // R
                    result[i * 4 + 3] = bytesPerPixel >= 4 ? PixelData[offset + 3] : (byte)255; // A
                }
            }
        }
    }

    /// <summary>
    /// Saves the texture as a TGA file.
    /// </summary>
    public void SaveAsTga(string outputPath)
    {
        var rgba = DecodeToRgba();

        using var writer = new BinaryWriter(File.Create(outputPath));

        // TGA header
        writer.Write((byte)0);  // ID length
        writer.Write((byte)0);  // Color map type
        writer.Write((byte)2);  // Image type (uncompressed true-color)
        writer.Write((short)0); // Color map first entry
        writer.Write((short)0); // Color map length
        writer.Write((byte)0);  // Color map entry size
        writer.Write((short)0); // X origin
        writer.Write((short)0); // Y origin
        writer.Write((short)Width);  // Width
        writer.Write((short)Height); // Height
        writer.Write((byte)32); // Bits per pixel
        writer.Write((byte)0x28); // Image descriptor (top-left origin, 8 alpha bits)

        // Write pixel data as BGRA
        for (int y = 0; y < Height; y++)
        {
            for (int x = 0; x < Width; x++)
            {
                int i = (y * Width + x) * 4;
                if (i + 3 < rgba.Length)
                {
                    writer.Write(rgba[i + 2]); // B
                    writer.Write(rgba[i + 1]); // G
                    writer.Write(rgba[i + 0]); // R
                    writer.Write(rgba[i + 3]); // A
                }
            }
        }
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
