namespace Astrolabe.Core.FileFormats.Audio;

/// <summary>
/// Interface for audio decoders that convert encoded audio data to PCM samples.
/// </summary>
public interface IAudioDecoder
{
    /// <summary>
    /// Decodes the audio data to 16-bit PCM samples.
    /// </summary>
    /// <returns>PCM samples (interleaved if stereo)</returns>
    short[] Decode();

    /// <summary>
    /// The sample rate in Hz.
    /// </summary>
    uint SampleRate { get; }

    /// <summary>
    /// The number of channels (1 for mono, 2 for stereo).
    /// </summary>
    ushort Channels { get; }
}
