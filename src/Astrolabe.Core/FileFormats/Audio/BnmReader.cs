namespace Astrolabe.Core.FileFormats.Audio;

/// <summary>
/// Reads Ubisoft BNM sound bank files (used by Hype: The Time Quest PC).
/// BNM files contain multiple embedded audio samples in various formats:
/// - APM: IMA-ADPCM compressed audio (music, ambient sounds)
/// - PCM: Raw 16-bit signed little-endian PCM (sound effects)
/// - MPDX: Ubi-MPEG compressed audio (voice/dialogue)
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
    private const int EntryEmbeddedFlagOffset = 0x08; // 1 = embedded, 0 = streamed/external
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
    private bool _mpdxOffsetsAbsolute;

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

        // Determine if MPDX offsets are absolute or relative by checking first MPDX entry
        _mpdxOffsetsAbsolute = DetectMpdxOffsetMode(reader, section2Offset, section2Count, entrySize);

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

            // Check if embedded (1) or streamed/external (0)
            reader.BaseStream.Seek(entryOffset + EntryEmbeddedFlagOffset, SeekOrigin.Begin);
            uint embeddedFlag = reader.ReadUInt32();
            if (embeddedFlag != 1)
                continue; // Skip streamed entries - audio data is in external files

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
            uint actualOffset;
            if (streamType == StreamTypeMpdx)
            {
                // MPDX offsets can be absolute or relative - check per-entry
                // Some files mix both modes (absolute for some entries, relative for others)
                if (IsValidMpdxStart(streamOffset))
                    actualOffset = streamOffset;
                else if (IsValidMpdxStart(_mpdxBlockStart + streamOffset))
                    actualOffset = _mpdxBlockStart + streamOffset;
                else
                    actualOffset = _mpdxOffsetsAbsolute ? streamOffset : _mpdxBlockStart + streamOffset;
            }
            else
            {
                uint blockStart = streamType switch
                {
                    StreamTypePcm => _pcmBlockStart,
                    StreamTypeApm => _apmBlockStart,
                    _ => 0
                };

                // Skip entries with unknown stream types
                if (blockStart == 0 && streamType != StreamTypePcm && streamType != StreamTypeApm)
                    continue;

                actualOffset = blockStart + streamOffset;
            }

            // Validate offset
            if (actualOffset >= _data.Length)
                continue;

            // Read stream size
            reader.BaseStream.Seek(entryOffset + EntryStreamSizeOffset, SeekOrigin.Begin);
            uint streamSize = reader.ReadUInt32();

            Entries.Add(new BnmEntry
            {
                Index = Entries.Count,
                Name = name,
                StreamType = streamType,
                SampleRate = sampleRate,
                Channels = channels,
                DataOffset = actualOffset,
                DataSize = streamSize
            });
        }
    }

    /// <summary>
    /// Detects whether MPDX offsets in this file are absolute or relative to the MPDX block.
    /// Returns true if absolute.
    /// </summary>
    private bool DetectMpdxOffsetMode(BinaryReader reader, uint section2Offset, uint section2Count, int entrySize)
    {
        for (int i = 0; i < section2Count; i++)
        {
            uint entryOffset = section2Offset + (uint)(i * entrySize);

            reader.BaseStream.Seek(entryOffset + EntryHeaderTypeOffset, SeekOrigin.Begin);
            uint headerType = reader.ReadUInt32();

            if (headerType != HeaderTypeAudio)
                continue;

            reader.BaseStream.Seek(entryOffset + EntryStreamTypeOffset, SeekOrigin.Begin);
            uint streamType = reader.ReadUInt32();

            if (streamType != StreamTypeMpdx)
                continue;

            reader.BaseStream.Seek(entryOffset + EntryEmbeddedFlagOffset, SeekOrigin.Begin);
            uint embeddedFlag = reader.ReadUInt32();

            if (embeddedFlag != 1)
                continue;

            // Found first embedded MPDX entry
            reader.BaseStream.Seek(entryOffset + EntryStreamOffsetOffset, SeekOrigin.Begin);
            uint streamOffset = reader.ReadUInt32();

            // Try to detect by checking what's at each offset interpretation
            // MPDX data starts with either:
            // - 4-byte header + Ubi-MPEG sync (FF Fx)
            // - 4-byte header + "2RUS"/"1RUS" surround marker + Ubi-MPEG sync
            if (IsValidMpdxStart(streamOffset))
                return true; // Absolute
            if (IsValidMpdxStart(_mpdxBlockStart + streamOffset))
                return false; // Relative

            // Fallback: if offset equals block start, it's absolute
            return streamOffset == _mpdxBlockStart;
        }

        // No MPDX entries found, default to absolute
        return true;
    }

    /// <summary>
    /// Checks if the given offset contains valid MPDX data start markers.
    /// </summary>
    private bool IsValidMpdxStart(uint offset)
    {
        if (offset + 8 >= _data.Length)
            return false;

        // Skip 4-byte header at offset
        uint dataStart = offset + 4;
        if (dataStart + 4 >= _data.Length)
            return false;

        // Check for surround marker "2RUS" or "1RUS" (0x53555232 or 0x53555231)
        uint marker = BitConverter.ToUInt32(_data, (int)dataStart);
        if (marker == 0x53555232 || marker == 0x53555231)
            return true;

        // Check for Ubi-MPEG sync (12-bit 0xFFF)
        if (_data[dataStart] == 0xFF && (_data[dataStart + 1] & 0xF0) == 0xF0)
            return true;

        return false;
    }

    /// <summary>
    /// Gets the raw audio data for an entry (APM, PCM, or MPDX).
    /// </summary>
    public byte[] GetEntryData(BnmEntry entry)
    {
        return entry.StreamType switch
        {
            StreamTypeApm => GetApmData(entry),
            StreamTypePcm => GetPcmData(entry),
            StreamTypeMpdx => GetMpdxData(entry),
            _ => throw new NotSupportedException($"Unsupported stream type: {entry.StreamTypeName}")
        };
    }

    /// <summary>
    /// Gets raw APM data for an entry.
    /// </summary>
    private byte[] GetApmData(BnmEntry entry)
    {
        using var reader = new BinaryReader(new MemoryStream(_data));
        reader.BaseStream.Seek(entry.DataOffset, SeekOrigin.Begin);

        // Verify APM signature
        ushort formatTag = reader.ReadUInt16();
        if (formatTag != 0x2000)
            throw new InvalidDataException($"Invalid APM format tag at entry {entry.Index}: 0x{formatTag:X4}");

        // Read file size from APM header
        reader.BaseStream.Seek(entry.DataOffset + 0x18, SeekOrigin.Begin);
        uint fileSize = reader.ReadUInt32();

        // Read full APM data
        reader.BaseStream.Seek(entry.DataOffset, SeekOrigin.Begin);
        return reader.ReadBytes((int)fileSize);
    }

    /// <summary>
    /// Gets raw PCM data for an entry. PCM data is 16-bit signed little-endian.
    /// </summary>
    private byte[] GetPcmData(BnmEntry entry)
    {
        if (entry.DataSize == 0)
            throw new InvalidDataException($"PCM entry {entry.Index} has zero data size");

        using var reader = new BinaryReader(new MemoryStream(_data));
        reader.BaseStream.Seek(entry.DataOffset, SeekOrigin.Begin);
        return reader.ReadBytes((int)entry.DataSize);
    }

    /// <summary>
    /// Gets raw MPDX data for an entry.
    /// MPDX data has a 4-byte header followed by optional surround marker and Ubi-MPEG data.
    /// We skip the 4-byte header here since UbiMpegDecoder expects data starting at the surround marker.
    /// </summary>
    private byte[] GetMpdxData(BnmEntry entry)
    {
        if (entry.DataSize == 0)
            throw new InvalidDataException($"MPDX entry {entry.Index} has zero data size");
        if (entry.DataSize <= 4)
            throw new InvalidDataException($"MPDX entry {entry.Index} data size too small: {entry.DataSize}");

        using var reader = new BinaryReader(new MemoryStream(_data));

        // Skip the 4-byte header at the start of MPDX data
        reader.BaseStream.Seek(entry.DataOffset + 4, SeekOrigin.Begin);
        return reader.ReadBytes((int)(entry.DataSize - 4));
    }

    /// <summary>
    /// Creates a decoder for the specified entry.
    /// </summary>
    public IAudioDecoder CreateDecoder(BnmEntry entry)
    {
        byte[] data = GetEntryData(entry);

        return entry.StreamType switch
        {
            StreamTypeApm => new ApmDecoder(data),
            StreamTypePcm => new PcmDecoder(data, entry.SampleRate, entry.Channels),
            StreamTypeMpdx => new UbiMpegDecoder(data, entry.SampleRate, entry.Channels),
            _ => throw new NotSupportedException($"Unsupported stream type: {entry.StreamTypeName}")
        };
    }

    /// <summary>
    /// Decodes an entry to PCM samples.
    /// </summary>
    public short[] DecodeEntry(BnmEntry entry)
    {
        var decoder = CreateDecoder(entry);
        return decoder.Decode();
    }

    /// <summary>
    /// Extracts an entry to a WAV file.
    /// </summary>
    public void ExtractEntry(BnmEntry entry, string outputPath)
    {
        var decoder = CreateDecoder(entry);
        short[] samples = decoder.Decode();

        // Use decoder's metadata (more accurate than entry metadata)
        WavWriter.Write(outputPath, samples, decoder.SampleRate, decoder.Channels);
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
                // Always include index to avoid name collisions (multiple entries can have same name)
                string baseName = !string.IsNullOrEmpty(entry.Name)
                    ? Path.GetFileNameWithoutExtension(entry.Name)
                    : "audio";
                string fileName = $"{entry.Index:D4}_{baseName}.wav";

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
    public uint DataSize { get; set; }

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
