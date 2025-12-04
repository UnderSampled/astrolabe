namespace Astrolabe.Core.FileFormats.Audio;

/// <summary>
/// Reads Ubisoft APM audio files (used by Hype: The Time Quest and Rayman 2).
/// APM uses IMA-ADPCM compression with a custom header format.
/// </summary>
public class ApmReader
{
    public const ushort FormatTag = 0x2000;
    public const string Magic = "vs12";

    // Header fields
    public ushort Channels { get; private set; }
    public uint SampleRate { get; private set; }
    public uint ByteRate { get; private set; }
    public ushort BlockAlign { get; private set; }
    public ushort BitsPerSample { get; private set; }
    public uint FileSize { get; private set; }
    public uint NibbleCount { get; private set; }

    // Per-channel initial ADPCM state
    public ImaAdpcmDecoder.ChannelState[] ChannelStates { get; private set; } = [];

    // Raw ADPCM data
    public byte[] AdpcmData { get; private set; } = [];

    private readonly byte[] _data;

    public ApmReader(byte[] data)
    {
        _data = data;
        Parse();
    }

    public ApmReader(string filePath) : this(File.ReadAllBytes(filePath))
    {
    }

    private void Parse()
    {
        using var reader = new BinaryReader(new MemoryStream(_data));

        // Read header
        ushort formatTag = reader.ReadUInt16();
        if (formatTag != FormatTag)
            throw new InvalidDataException($"Invalid APM format tag: 0x{formatTag:X4} (expected 0x{FormatTag:X4})");

        Channels = reader.ReadUInt16();
        if (Channels < 1 || Channels > 2)
            throw new InvalidDataException($"Unsupported channel count: {Channels}");

        SampleRate = reader.ReadUInt32();
        ByteRate = reader.ReadUInt32();
        BlockAlign = reader.ReadUInt16();
        BitsPerSample = reader.ReadUInt16();

        // Extended header
        uint headerSize = reader.ReadUInt32(); // Should be 0x50 (80 bytes)

        byte[] magic = reader.ReadBytes(4);
        if (System.Text.Encoding.ASCII.GetString(magic) != Magic)
            throw new InvalidDataException($"Invalid APM magic: {System.Text.Encoding.ASCII.GetString(magic)}");

        FileSize = reader.ReadUInt32();
        NibbleCount = reader.ReadUInt32();

        // Skip unknown fields
        reader.ReadInt32();  // -1
        reader.ReadUInt32(); // 0
        reader.ReadUInt32(); // nibble flag (runtime state)

        // Read per-channel ADPCM state
        // Stored in reverse order (channel 1 first for stereo)
        ChannelStates = new ImaAdpcmDecoder.ChannelState[Channels];

        for (int i = Channels - 1; i >= 0; i--)
        {
            int predictor = reader.ReadInt32();
            int stepIndex = reader.ReadInt32();
            reader.ReadInt32(); // copy of first ADPCM byte (not needed)

            ChannelStates[i] = new ImaAdpcmDecoder.ChannelState((short)predictor, stepIndex);
        }

        // Seek to DATA chunk
        reader.BaseStream.Seek(0x60, SeekOrigin.Begin);
        byte[] dataMarker = reader.ReadBytes(4);
        if (System.Text.Encoding.ASCII.GetString(dataMarker) != "DATA")
            throw new InvalidDataException("Missing DATA marker in APM file");

        // Read remaining ADPCM data
        int dataSize = (int)(reader.BaseStream.Length - reader.BaseStream.Position);
        AdpcmData = reader.ReadBytes(dataSize);
    }

    /// <summary>
    /// Decodes the APM audio to 16-bit PCM samples.
    /// </summary>
    /// <returns>PCM samples (interleaved if stereo)</returns>
    public short[] Decode()
    {
        if (Channels == 1)
        {
            return ImaAdpcmDecoder.DecodeMono(
                AdpcmData,
                ChannelStates[0].Predictor,
                ChannelStates[0].StepIndex);
        }
        else
        {
            return ImaAdpcmDecoder.DecodeStereo(
                AdpcmData,
                ChannelStates[0],  // Left
                ChannelStates[1]); // Right
        }
    }

    /// <summary>
    /// Gets the duration in seconds.
    /// </summary>
    public double Duration => (double)NibbleCount / SampleRate;

    /// <summary>
    /// Gets the total sample count (per channel).
    /// </summary>
    public int SampleCount => (int)NibbleCount;
}
