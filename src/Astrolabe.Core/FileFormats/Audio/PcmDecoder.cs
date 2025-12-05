namespace Astrolabe.Core.FileFormats.Audio;

/// <summary>
/// Decodes raw PCM audio data (16-bit signed little-endian).
/// </summary>
public class PcmDecoder : IAudioDecoder
{
    private readonly byte[] _data;

    public PcmDecoder(byte[] data, uint sampleRate, ushort channels)
    {
        _data = data;
        SampleRate = sampleRate;
        Channels = channels;
    }

    public short[] Decode()
    {
        int sampleCount = _data.Length / 2;
        short[] samples = new short[sampleCount];

        for (int i = 0; i < sampleCount; i++)
        {
            samples[i] = BitConverter.ToInt16(_data, i * 2);
        }

        return samples;
    }

    public uint SampleRate { get; }

    public ushort Channels { get; }
}
