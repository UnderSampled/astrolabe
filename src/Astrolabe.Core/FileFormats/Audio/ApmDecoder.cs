namespace Astrolabe.Core.FileFormats.Audio;

/// <summary>
/// Decodes Ubisoft APM audio data (IMA-ADPCM with custom header).
/// Used by Hype: The Time Quest and Rayman 2.
/// </summary>
public class ApmDecoder : IAudioDecoder
{
    private readonly ApmReader _reader;

    public ApmDecoder(byte[] data)
    {
        _reader = new ApmReader(data);
    }

    public short[] Decode() => _reader.Decode();

    public uint SampleRate => _reader.SampleRate;

    public ushort Channels => _reader.Channels;
}
