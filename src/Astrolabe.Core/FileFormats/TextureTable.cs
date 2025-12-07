using System.Text;

namespace Astrolabe.Core.FileFormats;

/// <summary>
/// Texture info entry with name and flags.
/// </summary>
public class TextureEntry
{
    public string Name { get; set; } = "";
    public uint Flags { get; set; }

    // Flag bit 5 (0x20) = IsLight (additive blending)
    public bool IsLight => (Flags & 0x20) != 0;

    // Flag bit 3 (0x08) = IsTransparent
    public bool IsTransparent => (Flags & 0x08) != 0;
}

/// <summary>
/// Reads texture information from PTX file and SNA blocks.
/// PTX contains an array of pointers to TextureInfo structures in SNA memory.
/// </summary>
public class TextureTable
{
    private readonly LevelLoader _level;
    private readonly Dictionary<int, string> _textureNames = new();
    private readonly Dictionary<int, TextureEntry> _textureEntries = new();

    public IReadOnlyDictionary<int, string> TextureNames => _textureNames;
    public IReadOnlyDictionary<int, TextureEntry> TextureEntries => _textureEntries;

    public TextureTable(LevelLoader level, string ptxPath)
    {
        _level = level;
        LoadPtx(ptxPath);
    }

    private void LoadPtx(string ptxPath)
    {
        if (!File.Exists(ptxPath)) return;

        using var reader = new BinaryReader(File.OpenRead(ptxPath));

        // PTX header: first 4 bytes are count (little endian)
        // But examining the hex, it looks like 0x00000400 = 1024 in big endian, or 4 in little
        // Let's check by looking at the data pattern - pointers start immediately after
        int maxTextures = (int)reader.BaseStream.Length / 4;  // Maximum possible

        // Read pointers until we hit zeros or end of file
        var pointers = new List<int>();

        // Skip first 4 bytes (possible count or header)
        reader.BaseStream.Position = 4;

        while (reader.BaseStream.Position < reader.BaseStream.Length - 4)
        {
            int ptr = reader.ReadInt32();
            if (ptr == 0) break;  // End of table
            pointers.Add(ptr);
        }

        // Now resolve each pointer to find texture names and flags
        foreach (int ptr in pointers)
        {
            var entry = ReadTextureInfo(ptr);
            if (entry != null && !string.IsNullOrEmpty(entry.Name))
            {
                _textureNames[ptr] = entry.Name;
                _textureEntries[ptr] = entry;
            }
        }
    }

    private TextureEntry? ReadTextureInfo(int address)
    {
        // TextureInfo structure in Montreal engine (Hype):
        // The structure varies, but we look for the texture filename
        // which is typically a null-terminated string after some header fields.
        //
        // Based on raymap's reading, TextureInfo has:
        // - Various fields (offsets, dimensions, flags)
        // - Followed by the texture name string
        //
        // Montreal TextureInfo structure (approximate):
        // +0x00: uint32 - flags
        // +0x04: uint16 - width
        // +0x06: uint16 - height
        // +0x08...: more fields
        // +0x30: texture name (variable offset, need to search)

        var reader = _level.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            // TextureInfo structure for Montreal engine (Hype, R2):
            // 0x00: unknown (4 bytes)
            // 0x04: unknown (4 bytes)
            // 0x08: flags (4 bytes) <- IsLight is bit 5 (0x20)
            // 0x0C: height_ (4 bytes)
            // 0x10: width_ (4 bytes)
            // ... more fields, then name at variable offset

            byte[] buffer = new byte[128];
            int bytesRead = reader.Read(buffer, 0, buffer.Length);
            if (bytesRead < 0x14) return null;

            // Flags at offset 0x08 for Montreal engine
            uint flags = BitConverter.ToUInt32(buffer, 0x08);

            // Try to find a filename pattern (ends with .gf or starts with a letter)
            // The name is typically after offset 0x14
            for (int offset = 0x14; offset < bytesRead - 4; offset++)
            {
                // Look for potential filename start (letter character)
                if (buffer[offset] >= 'a' && buffer[offset] <= 'z' ||
                    buffer[offset] >= 'A' && buffer[offset] <= 'Z' ||
                    buffer[offset] >= '0' && buffer[offset] <= '9')
                {
                    // Try to read a null-terminated string
                    int end = offset;
                    while (end < bytesRead && buffer[end] != 0 && buffer[end] >= 0x20 && buffer[end] < 0x7F)
                    {
                        end++;
                    }

                    int len = end - offset;
                    if (len >= 4 && len <= 50)  // Reasonable filename length
                    {
                        string potential = Encoding.ASCII.GetString(buffer, offset, len);
                        // Check if it looks like a texture name (ends with txy, txz, gf, or common patterns)
                        if (potential.EndsWith("txy", StringComparison.OrdinalIgnoreCase) ||
                            potential.EndsWith("txynz", StringComparison.OrdinalIgnoreCase) ||
                            potential.EndsWith(".gf", StringComparison.OrdinalIgnoreCase) ||
                            potential.EndsWith("nz", StringComparison.OrdinalIgnoreCase) ||
                            potential.Contains("tex", StringComparison.OrdinalIgnoreCase) ||
                            (potential.Length > 3 && potential.All(c => char.IsLetterOrDigit(c) || c == '_' || c == '.')))
                        {
                            return new TextureEntry { Name = potential, Flags = flags };
                        }
                    }
                }
            }

            return null;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Gets the texture name for a given TextureInfo pointer address.
    /// </summary>
    public string? GetTextureName(int textureInfoAddress)
    {
        return _textureNames.GetValueOrDefault(textureInfoAddress);
    }

    /// <summary>
    /// Gets the full texture entry (name + flags) for a given TextureInfo pointer address.
    /// </summary>
    public TextureEntry? GetTextureEntry(int textureInfoAddress)
    {
        return _textureEntries.GetValueOrDefault(textureInfoAddress);
    }
}
