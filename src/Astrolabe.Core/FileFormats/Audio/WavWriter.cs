namespace Astrolabe.Core.FileFormats.Audio;

/// <summary>
/// Writes 16-bit PCM WAV files.
/// </summary>
public static class WavWriter
{
    /// <summary>
    /// Writes PCM samples to a WAV file.
    /// </summary>
    /// <param name="filePath">Output file path</param>
    /// <param name="samples">16-bit PCM samples (interleaved if stereo)</param>
    /// <param name="sampleRate">Sample rate in Hz</param>
    /// <param name="channels">Number of channels (1 or 2)</param>
    public static void Write(string filePath, short[] samples, uint sampleRate, ushort channels)
    {
        using var stream = File.Create(filePath);
        Write(stream, samples, sampleRate, channels);
    }

    /// <summary>
    /// Writes PCM samples to a stream as WAV format.
    /// </summary>
    public static void Write(Stream stream, short[] samples, uint sampleRate, ushort channels)
    {
        using var writer = new BinaryWriter(stream);

        int dataSize = samples.Length * 2; // 16-bit samples = 2 bytes each
        int fileSize = 36 + dataSize;

        // RIFF header
        writer.Write("RIFF"u8);
        writer.Write(fileSize);
        writer.Write("WAVE"u8);

        // fmt chunk
        writer.Write("fmt "u8);
        writer.Write(16);                           // Chunk size
        writer.Write((ushort)1);                    // Format = PCM
        writer.Write(channels);                     // Channels
        writer.Write(sampleRate);                   // Sample rate
        writer.Write(sampleRate * channels * 2);    // Byte rate
        writer.Write((ushort)(channels * 2));       // Block align
        writer.Write((ushort)16);                   // Bits per sample

        // data chunk
        writer.Write("data"u8);
        writer.Write(dataSize);

        // Write samples
        foreach (short sample in samples)
        {
            writer.Write(sample);
        }
    }

    /// <summary>
    /// Converts an APM file to WAV.
    /// </summary>
    /// <param name="apmPath">Input APM file path</param>
    /// <param name="wavPath">Output WAV file path</param>
    public static void ConvertApmToWav(string apmPath, string wavPath)
    {
        var apm = new ApmReader(apmPath);
        var samples = apm.Decode();
        Write(wavPath, samples, apm.SampleRate, apm.Channels);
    }

    /// <summary>
    /// Converts an APM stream to WAV.
    /// </summary>
    /// <param name="apmStream">Input APM stream</param>
    /// <param name="wavPath">Output WAV file path</param>
    public static void ConvertApmToWav(Stream apmStream, string wavPath)
    {
        var apm = new ApmReader(apmStream);
        var samples = apm.Decode();
        Write(wavPath, samples, apm.SampleRate, apm.Channels);
    }
}
