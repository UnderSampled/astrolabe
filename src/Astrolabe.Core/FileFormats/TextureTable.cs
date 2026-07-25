using System.Text;
using Astrolabe.Core.Hub;
using Astrolabe.Core.Serialization.Codecs;

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
    private readonly LevelLoader? _level;
    private readonly Dictionary<int, string> _textureNames = new();
    private readonly Dictionary<int, TextureEntry> _textureEntries = new();

    public IReadOnlyDictionary<int, string> TextureNames => _textureNames;
    public IReadOnlyDictionary<int, TextureEntry> TextureEntries => _textureEntries;

    public TextureTable(LevelLoader level, string ptxPath)
    {
        _level = level;
        LoadPtx(ptxPath);
    }

    public TextureTable(HubCatalog catalog, string ptxPath)
    {
        _level = null;
        LoadPtxFromHub(catalog, ptxPath);
    }

    private void LoadPtx(string ptxPath)
    {
        foreach (var ptr in ReadPtxPointers(ptxPath))
        {
            var entry = ReadTextureInfo(ptr);
            if (entry != null && !string.IsNullOrEmpty(entry.Name))
            {
                _textureNames[ptr] = entry.Name;
                _textureEntries[ptr] = entry;
            }
        }

        LogAddressRange();
    }

    private void LoadPtxFromHub(HubCatalog catalog, string ptxPath)
    {
        foreach (var ptr in ReadPtxPointers(ptxPath))
        {
            var entry = ReadTextureInfoFromHub(catalog, ptr);
            if (entry != null && !string.IsNullOrEmpty(entry.Name))
            {
                _textureNames[ptr] = entry.Name;
                _textureEntries[ptr] = entry;
            }
        }

        LogAddressRange();
    }

    private static List<int> ReadPtxPointers(string ptxPath)
    {
        var pointers = new List<int>();
        if (!File.Exists(ptxPath))
        {
            return pointers;
        }

        using var reader = new BinaryReader(File.OpenRead(ptxPath));
        if (reader.BaseStream.Length < 8)
        {
            return pointers;
        }

        // Hype / Montreal PTX: [capacity:u32][count:u32][pointers count]
        // Older scanners started at offset 4 and treated count as a pointer — skip that.
        _ = reader.ReadInt32(); // capacity / max slots
        int countOrPtr = reader.ReadInt32();

        if (countOrPtr > 0 && countOrPtr < 0x01000000)
        {
            // count field: read up to count entries (zeros allowed; keep non-zero only)
            var remaining = (reader.BaseStream.Length - reader.BaseStream.Position) / 4;
            var toRead = (int)Math.Min(countOrPtr, remaining);
            for (var i = 0; i < toRead; i++)
            {
                int ptr = reader.ReadInt32();
                if (ptr != 0 && ptr >= 0x01000000)
                {
                    pointers.Add(ptr);
                }
            }

            return pointers;
        }

        // Fallback: countOrPtr looked like an address — include it and scan until zero.
        if (countOrPtr != 0)
        {
            pointers.Add(countOrPtr);
        }

        while (reader.BaseStream.Position + 4 <= reader.BaseStream.Length)
        {
            int ptr = reader.ReadInt32();
            if (ptr == 0)
            {
                break;
            }

            if (ptr >= 0x01000000)
            {
                pointers.Add(ptr);
            }
        }

        return pointers;
    }

    private void LogAddressRange()
    {
        if (_textureNames.Count == 0)
        {
            return;
        }

        int minAddr = _textureNames.Keys.Min();
        int maxAddr = _textureNames.Keys.Max();
        Console.WriteLine($"  TextureTable: {_textureNames.Count} entries, address range 0x{minAddr:X8} - 0x{maxAddr:X8}");
    }

    private TextureEntry? ReadTextureInfo(int address)
    {
        if (_level == null)
        {
            return null;
        }

        var reader = _level.GetReaderAt(address);
        if (reader == null)
        {
            return null;
        }

        try
        {
            byte[] buffer = new byte[128];
            int bytesRead = reader.Read(buffer, 0, buffer.Length);
            return ParseTextureInfoBuffer(buffer, bytesRead);
        }
        catch
        {
            return null;
        }
    }

    private static TextureEntry? ReadTextureInfoFromHub(HubCatalog catalog, int address)
    {
        if (!catalog.TryGetByVirtualAddress(address, out var element) ||
            !catalog.TryHydrate(element))
        {
            return null;
        }

        byte[]? buffer = element.Value switch
        {
            OpaqueBinaryRecord raw => raw.Data,
            byte[] bytes => bytes,
            _ => null
        };

        return buffer == null ? null : ParseTextureInfoBuffer(buffer, buffer.Length);
    }

    private static TextureEntry? ParseTextureInfoBuffer(byte[] buffer, int bytesRead)
    {
        if (bytesRead < 0x14)
        {
            return null;
        }

        uint flags = BitConverter.ToUInt32(buffer, 0x08);

        for (int offset = 0x14; offset < bytesRead - 4; offset++)
        {
            if (buffer[offset] >= 'a' && buffer[offset] <= 'z' ||
                buffer[offset] >= 'A' && buffer[offset] <= 'Z' ||
                buffer[offset] >= '0' && buffer[offset] <= '9')
            {
                int end = offset;
                while (end < bytesRead && buffer[end] != 0 && buffer[end] >= 0x20 && buffer[end] < 0x7F)
                {
                    end++;
                }

                int len = end - offset;
                if (len >= 4 && len <= 50)
                {
                    string potential = Encoding.ASCII.GetString(buffer, offset, len);
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

    /// <summary>
    /// Merges texture entries from another PTX file.
    /// </summary>
    public void MergeFromPtx(string ptxPath)
    {
        if (!File.Exists(ptxPath)) return;

        int countBefore = _textureNames.Count;

        using var reader = new BinaryReader(File.OpenRead(ptxPath));

        // Fix.ptx has different format: header at 0, unknown at 4, pointers start at 8
        // Try both formats - skip first pointer if it's too small (< 0x01000000)
        reader.BaseStream.Position = 4;
        int testPtr = reader.ReadInt32();

        // If first pointer is too small, skip it and start at offset 8
        if (testPtr < 0x01000000)
        {
            reader.BaseStream.Position = 8;
        }
        else
        {
            reader.BaseStream.Position = 4;
        }

        var pointers = new List<int>();
        while (reader.BaseStream.Position < reader.BaseStream.Length - 4)
        {
            int ptr = reader.ReadInt32();
            if (ptr == 0) break;
            if (ptr < 0x01000000) continue; // Skip invalid pointers
            pointers.Add(ptr);
        }

        int resolvedCount = 0;
        int failedCount = 0;
        foreach (int ptr in pointers)
        {
            if (_textureNames.ContainsKey(ptr)) continue;

            var entry = ReadTextureInfo(ptr);
            if (entry != null && !string.IsNullOrEmpty(entry.Name))
            {
                _textureNames[ptr] = entry.Name;
                _textureEntries[ptr] = entry;
                resolvedCount++;
            }
            else
            {
                failedCount++;
            }
        }

        Console.WriteLine($"  Fix.ptx: {pointers.Count} pointers, resolved {resolvedCount}, failed {failedCount}");
        if (pointers.Count > 0)
        {
            var readerStatus = _level?.GetReaderAt(pointers[0]) != null ? "OK" : "NULL";
            Console.WriteLine($"  First pointer: 0x{pointers[0]:X8}, reader={readerStatus}");
        }

        int added = _textureNames.Count - countBefore;
        if (added > 0)
        {
            int minAddr = _textureNames.Keys.Min();
            int maxAddr = _textureNames.Keys.Max();
            Console.WriteLine($"  Merged {added} texture entries, total {_textureNames.Count}, range 0x{minAddr:X8} - 0x{maxAddr:X8}");
        }
    }
}
