using System.Text;

namespace Astrolabe.Core.FileFormats;

/// <summary>
/// Reads texture information from PTX file and SNA blocks.
/// PTX contains an array of pointers to TextureInfo structures in SNA memory.
/// </summary>
public class TextureTable
{
    private readonly LevelLoader _level;
    private readonly Dictionary<int, string> _textureNames = new();

    public IReadOnlyDictionary<int, string> TextureNames => _textureNames;

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

        // Now resolve each pointer to find texture names
        foreach (int ptr in pointers)
        {
            string? name = ReadTextureInfoName(ptr);
            if (!string.IsNullOrEmpty(name))
            {
                _textureNames[ptr] = name;
            }
        }
    }

    private string? ReadTextureInfoName(int address)
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
            // Read some data and look for the texture name
            // In Montreal, the name is often at a fixed offset, let's try common offsets
            byte[] buffer = new byte[128];
            int bytesRead = reader.Read(buffer, 0, buffer.Length);
            if (bytesRead < 32) return null;

            // Try to find a filename pattern (ends with .gf or starts with a letter)
            // The name is typically near the end of the structure
            for (int offset = 0; offset < bytesRead - 4; offset++)
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
                            return potential;
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
}
