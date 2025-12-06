using System.Text;

namespace Astrolabe.Core.FileFormats;

// Register Windows-1252 encoding provider
static class EncodingInit
{
    static EncodingInit()
    {
        Encoding.RegisterProvider(CodePagesEncodingProvider.Instance);
    }

    public static void EnsureInitialized() { }
}

/// <summary>
/// Reads CNT texture container files.
/// </summary>
public class CntReader
{
    public int DirectoryCount { get; private set; }
    public int FileCount { get; private set; }
    public bool IsXorEncrypted { get; private set; }
    public bool HasChecksum { get; private set; }
    public byte XorKey { get; private set; }
    public string[] Directories { get; private set; } = [];
    public CntFileEntry[] Files { get; private set; } = [];

    private readonly string _filePath;
    private readonly byte[] _data;

    public CntReader(string filePath)
    {
        EncodingInit.EnsureInitialized();
        _filePath = filePath;
        _data = File.ReadAllBytes(filePath);
        Parse();
    }

    public CntReader(byte[] data)
    {
        EncodingInit.EnsureInitialized();
        _filePath = "<memory>";
        _data = data;
        Parse();
    }

    public CntReader(Stream stream)
    {
        EncodingInit.EnsureInitialized();
        _filePath = "<stream>";
        using var ms = new MemoryStream();
        stream.CopyTo(ms);
        _data = ms.ToArray();
        Parse();
    }

    private void Parse()
    {
        using var reader = new BinaryReader(new MemoryStream(_data));

        // Read header
        DirectoryCount = reader.ReadInt32();
        FileCount = reader.ReadInt32();
        IsXorEncrypted = reader.ReadByte() != 0;
        HasChecksum = reader.ReadByte() != 0;
        XorKey = reader.ReadByte();

        // Read directories
        Directories = new string[DirectoryCount];
        for (int i = 0; i < DirectoryCount; i++)
        {
            int stringLength = reader.ReadInt32();
            var nameBytes = reader.ReadBytes(stringLength);

            if (IsXorEncrypted)
            {
                for (int j = 0; j < nameBytes.Length; j++)
                {
                    nameBytes[j] ^= XorKey;
                }
            }

            Directories[i] = Encoding.GetEncoding(1252).GetString(nameBytes).TrimEnd('\0');
        }

        // Read directory checksum if present
        if (HasChecksum && DirectoryCount > 0)
        {
            reader.ReadByte(); // Checksum byte, we skip validation
        }

        // Read file entries
        Files = new CntFileEntry[FileCount];
        for (int i = 0; i < FileCount; i++)
        {
            var entry = new CntFileEntry();

            entry.DirectoryIndex = reader.ReadInt32();
            int filenameLength = reader.ReadInt32();
            var filenameBytes = reader.ReadBytes(filenameLength);

            if (IsXorEncrypted)
            {
                for (int j = 0; j < filenameBytes.Length; j++)
                {
                    filenameBytes[j] ^= XorKey;
                }
            }

            entry.Filename = Encoding.GetEncoding(1252).GetString(filenameBytes).TrimEnd('\0');
            entry.FileXorKey = reader.ReadBytes(4);
            entry.Checksum = reader.ReadUInt32();
            entry.FilePointer = reader.ReadInt32();
            entry.FileSize = reader.ReadInt32();

            // Build full path
            if (entry.DirectoryIndex >= 0 && entry.DirectoryIndex < Directories.Length)
            {
                entry.FullPath = Path.Combine(Directories[entry.DirectoryIndex], entry.Filename);
            }
            else
            {
                entry.FullPath = entry.Filename;
            }

            Files[i] = entry;
        }
    }

    /// <summary>
    /// Extracts a file from the container.
    /// </summary>
    public byte[] ExtractFile(CntFileEntry entry)
    {
        var data = new byte[entry.FileSize];
        Array.Copy(_data, entry.FilePointer, data, 0, entry.FileSize);

        // Decrypt with per-file XOR key
        for (int i = 0; i < data.Length; i++)
        {
            data[i] ^= entry.FileXorKey[i % 4];
        }

        return data;
    }

    /// <summary>
    /// Extracts a file by name.
    /// </summary>
    public byte[]? ExtractFile(string filename)
    {
        var entry = Files.FirstOrDefault(f =>
            f.Filename.Equals(filename, StringComparison.OrdinalIgnoreCase) ||
            f.FullPath.Equals(filename, StringComparison.OrdinalIgnoreCase));

        return entry != null ? ExtractFile(entry) : null;
    }

    /// <summary>
    /// Extracts all files to a directory.
    /// </summary>
    public void ExtractAll(string outputDirectory, IProgress<(int current, int total, string filename)>? progress = null)
    {
        for (int i = 0; i < Files.Length; i++)
        {
            var entry = Files[i];
            var outputPath = Path.Combine(outputDirectory, entry.FullPath);

            var dir = Path.GetDirectoryName(outputPath);
            if (!string.IsNullOrEmpty(dir))
            {
                Directory.CreateDirectory(dir);
            }

            var data = ExtractFile(entry);
            File.WriteAllBytes(outputPath, data);

            progress?.Report((i + 1, Files.Length, entry.FullPath));
        }
    }
}

/// <summary>
/// Entry for a file in a CNT container.
/// </summary>
public class CntFileEntry
{
    public int DirectoryIndex { get; set; }
    public string Filename { get; set; } = "";
    public string FullPath { get; set; } = "";
    public byte[] FileXorKey { get; set; } = new byte[4];
    public uint Checksum { get; set; }
    public int FilePointer { get; set; }
    public int FileSize { get; set; }
}
