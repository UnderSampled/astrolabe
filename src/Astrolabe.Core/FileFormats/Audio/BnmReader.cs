namespace Astrolabe.Core.FileFormats.Audio;

/// <summary>
/// Reads Ubisoft BNM sound bank files (used by Hype: The Time Quest PC).
/// BNM files contain multiple embedded audio samples (APM format).
/// </summary>
public class BnmReader
{
    // Header offsets
    private const int VersionOffset = 0x00;
    private const int Section1OffsetOffset = 0x04;
    private const int Section1CountOffset = 0x08;
    private const int Section2OffsetOffset = 0x0C;
    private const int Section2CountOffset = 0x10;
    // Block offsets in header: 0x14=MPDX, 0x18=MIDI, 0x1C=PCM, 0x20=APM, 0x24=streamed, 0x28=EOF
    private const int MpdxBlockOffset = 0x14;
    private const int PcmBlockOffset = 0x1C;
    private const int ApmBlockOffset = 0x20;

    // Entry structure (0x5C bytes for version 0, 0x60 for version 0x200)
    private const int EntrySize = 0x5C;
    private const int EntryHeaderTypeOffset = 0x04;
    private const int EntryStreamSizeOffset = 0x0C;   // Audio data size
    private const int EntryStreamOffsetOffset = 0x10; // Offset within block
    private const int EntrySampleRateOffset = 0x3C;
    private const int EntryChannelsOffset = 0x42;
    private const int EntryStreamTypeOffset = 0x44;
    private const int EntryNameOffset = 0x48;
    private const int EntryNameLength = 20;

    // Header type constants
    private const int HeaderTypeAudio = 0x01;

    // Stream type constants
    private const int StreamTypePcm = 0x01;
    private const int StreamTypeMpdx = 0x02;  // MPEG-like format, used for voice files
    private const int StreamTypeApm = 0x04;

    public uint Version { get; private set; }
    public List<BnmEntry> Entries { get; private set; } = [];

    private readonly byte[] _data;
    private uint _mpdxBlockStart;
    private uint _pcmBlockStart;
    private uint _apmBlockStart;

    public BnmReader(byte[] data)
    {
        _data = data;
        Parse();
    }

    public BnmReader(string filePath) : this(File.ReadAllBytes(filePath))
    {
    }

    private void Parse()
    {
        using var reader = new BinaryReader(new MemoryStream(_data));

        // Read header
        Version = reader.ReadUInt32();
        if (Version != 0x00000000 && Version != 0x00000200)
        {
            throw new InvalidDataException($"Unsupported BNM version: 0x{Version:X8}");
        }

        uint section1Offset = reader.ReadUInt32();
        uint section1Count = reader.ReadUInt32();
        uint section2Offset = reader.ReadUInt32();
        uint section2Count = reader.ReadUInt32();

        // Read block offsets
        reader.BaseStream.Seek(MpdxBlockOffset, SeekOrigin.Begin);
        _mpdxBlockStart = reader.ReadUInt32();

        reader.BaseStream.Seek(PcmBlockOffset, SeekOrigin.Begin);
        _pcmBlockStart = reader.ReadUInt32();

        reader.BaseStream.Seek(ApmBlockOffset, SeekOrigin.Begin);
        _apmBlockStart = reader.ReadUInt32();

        // Adjust entry size for version 0x200
        int entrySize = Version == 0x00000200 ? 0x60 : EntrySize;

        // Parse section2 entries (audio descriptors)
        Entries = new List<BnmEntry>();

        for (int i = 0; i < section2Count; i++)
        {
            uint entryOffset = section2Offset + (uint)(i * entrySize);
            reader.BaseStream.Seek(entryOffset, SeekOrigin.Begin);

            // Read header type
            reader.ReadUInt32(); // header_id
            uint headerType = reader.ReadUInt32();

            if (headerType != HeaderTypeAudio)
                continue; // Skip non-audio entries (sequences, etc.)

            // Read audio fields
            reader.BaseStream.Seek(entryOffset + EntryStreamOffsetOffset, SeekOrigin.Begin);
            uint streamOffset = reader.ReadUInt32();

            reader.BaseStream.Seek(entryOffset + EntrySampleRateOffset, SeekOrigin.Begin);
            uint sampleRate = reader.ReadUInt32();

            reader.BaseStream.Seek(entryOffset + EntryChannelsOffset, SeekOrigin.Begin);
            ushort channels = reader.ReadUInt16();

            reader.BaseStream.Seek(entryOffset + EntryStreamTypeOffset, SeekOrigin.Begin);
            uint streamType = reader.ReadUInt32();

            reader.BaseStream.Seek(entryOffset + EntryNameOffset, SeekOrigin.Begin);
            byte[] nameBytes = reader.ReadBytes(EntryNameLength);
            string name = System.Text.Encoding.Latin1.GetString(nameBytes).TrimEnd('\0');

            // Calculate actual data offset based on stream type
            uint blockStart = streamType switch
            {
                StreamTypePcm => _pcmBlockStart,
                StreamTypeMpdx => _mpdxBlockStart,
                StreamTypeApm => _apmBlockStart,
                _ => 0
            };

            // Skip entries with unknown stream types
            if (blockStart == 0 && streamType != StreamTypePcm && streamType != StreamTypeMpdx && streamType != StreamTypeApm)
                continue;

            uint actualOffset = blockStart + streamOffset;

            // Validate offset
            if (actualOffset >= _data.Length)
                continue;

            Entries.Add(new BnmEntry
            {
                Index = Entries.Count,
                Name = name,
                StreamType = streamType,
                SampleRate = sampleRate,
                Channels = channels,
                DataOffset = actualOffset
            });
        }
    }

    /// <summary>
    /// Gets the raw APM data for an entry.
    /// </summary>
    public byte[] GetEntryData(BnmEntry entry)
    {
        if (entry.StreamType != StreamTypeApm)
            throw new NotSupportedException($"Only APM entries are supported, got {entry.StreamTypeName}");

        // Read APM header to get file size
        using var reader = new BinaryReader(new MemoryStream(_data));
        reader.BaseStream.Seek(entry.DataOffset, SeekOrigin.Begin);

        // Verify APM signature
        ushort formatTag = reader.ReadUInt16();
        if (formatTag != 0x2000)
            throw new InvalidDataException($"Invalid APM format tag at entry {entry.Index}: 0x{formatTag:X4}");

        // Read file size from APM header
        reader.BaseStream.Seek(entry.DataOffset + 0x18, SeekOrigin.Begin);
        uint fileSize = reader.ReadUInt32();

        // Include header (0x64 bytes) + data
        uint totalSize = 0x64 + fileSize - 0x64; // fileSize includes header

        // Read full APM data
        reader.BaseStream.Seek(entry.DataOffset, SeekOrigin.Begin);
        return reader.ReadBytes((int)fileSize);
    }

    /// <summary>
    /// Decodes an entry to PCM samples.
    /// </summary>
    public short[] DecodeEntry(BnmEntry entry)
    {
        byte[] apmData = GetEntryData(entry);
        var apm = new ApmReader(apmData);
        return apm.Decode();
    }

    /// <summary>
    /// Extracts an entry to a WAV file.
    /// </summary>
    public void ExtractEntry(BnmEntry entry, string outputPath)
    {
        byte[] apmData = GetEntryData(entry);
        var apm = new ApmReader(apmData);
        var samples = apm.Decode();
        WavWriter.Write(outputPath, samples, apm.SampleRate, apm.Channels);
    }

    /// <summary>
    /// Extracts all entries to a directory.
    /// </summary>
    public int ExtractAll(string outputDir, bool verbose = false)
    {
        Directory.CreateDirectory(outputDir);
        int extracted = 0;

        foreach (var entry in Entries)
        {
            try
            {
                string fileName = !string.IsNullOrEmpty(entry.Name)
                    ? Path.ChangeExtension(entry.Name, ".wav")
                    : $"{entry.Index:D4}.wav";

                string outputPath = Path.Combine(outputDir, fileName);
                ExtractEntry(entry, outputPath);
                extracted++;
            }
            catch (Exception ex)
            {
                if (verbose)
                    Console.Error.WriteLine($"  [{entry.Index}] {entry.Name}: {ex.Message}");
            }
        }

        return extracted;
    }
}

/// <summary>
/// Represents an audio entry in a BNM file.
/// </summary>
public class BnmEntry
{
    public int Index { get; set; }
    public string Name { get; set; } = "";
    public uint StreamType { get; set; }
    public uint SampleRate { get; set; }
    public ushort Channels { get; set; }
    public uint DataOffset { get; set; }

    public bool IsApm => StreamType == 0x04;
    public bool IsPcm => StreamType == 0x01;
    public bool IsMpdx => StreamType == 0x02;

    public string StreamTypeName => StreamType switch
    {
        0x01 => "PCM",
        0x02 => "MPDX",
        0x04 => "APM",
        _ => $"Unknown(0x{StreamType:X2})"
    };
}
