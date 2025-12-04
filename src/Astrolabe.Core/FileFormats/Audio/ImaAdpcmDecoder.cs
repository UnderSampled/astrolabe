namespace Astrolabe.Core.FileFormats.Audio;

/// <summary>
/// Decodes IMA-ADPCM audio data to 16-bit PCM.
/// This is a pure managed implementation with no external dependencies.
/// </summary>
public static class ImaAdpcmDecoder
{
    /// <summary>
    /// IMA ADPCM step index adjustment table.
    /// Indexed by the lower 3 bits of the nibble.
    /// </summary>
    private static readonly int[] IndexTable =
    [
        -1, -1, -1, -1, 2, 4, 6, 8
    ];

    /// <summary>
    /// IMA ADPCM step size table (89 entries).
    /// </summary>
    private static readonly int[] StepTable =
    [
        7, 8, 9, 10, 11, 12, 13, 14, 16, 17,
        19, 21, 23, 25, 28, 31, 34, 37, 41, 45,
        50, 55, 60, 66, 73, 80, 88, 97, 107, 118,
        130, 143, 157, 173, 190, 209, 230, 253, 279, 307,
        337, 371, 408, 449, 494, 544, 598, 658, 724, 796,
        876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066,
        2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358,
        5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899,
        15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767
    ];

    /// <summary>
    /// Holds the state for one channel of ADPCM decoding.
    /// </summary>
    public struct ChannelState
    {
        public short Predictor;
        public int StepIndex;

        public ChannelState(short predictor, int stepIndex)
        {
            Predictor = predictor;
            StepIndex = Math.Clamp(stepIndex, 0, 88);
        }
    }

    /// <summary>
    /// Decodes a single ADPCM nibble to a PCM sample.
    /// </summary>
    /// <param name="nibble">4-bit ADPCM nibble (0-15)</param>
    /// <param name="state">Current decoder state (modified in place)</param>
    /// <returns>Decoded 16-bit PCM sample</returns>
    public static short DecodeNibble(int nibble, ref ChannelState state)
    {
        int step = StepTable[state.StepIndex];

        // Calculate difference
        int diff = step >> 3;
        if ((nibble & 1) != 0) diff += step >> 2;
        if ((nibble & 2) != 0) diff += step >> 1;
        if ((nibble & 4) != 0) diff += step;
        if ((nibble & 8) != 0) diff = -diff;

        // Update predictor with clamping
        int predictor = state.Predictor + diff;
        predictor = Math.Clamp(predictor, -32768, 32767);
        state.Predictor = (short)predictor;

        // Update step index
        state.StepIndex += IndexTable[nibble & 7];
        state.StepIndex = Math.Clamp(state.StepIndex, 0, 88);

        return state.Predictor;
    }

    /// <summary>
    /// Decodes mono IMA-ADPCM data to 16-bit PCM.
    /// </summary>
    /// <param name="adpcmData">Raw ADPCM nibble data (2 nibbles per byte, high nibble first)</param>
    /// <param name="initialPredictor">Initial predictor value</param>
    /// <param name="initialStepIndex">Initial step table index</param>
    /// <returns>Decoded 16-bit PCM samples</returns>
    public static short[] DecodeMono(byte[] adpcmData, short initialPredictor, int initialStepIndex)
    {
        var state = new ChannelState(initialPredictor, initialStepIndex);
        var samples = new short[adpcmData.Length * 2];
        int sampleIndex = 0;

        foreach (byte b in adpcmData)
        {
            // High nibble first, then low nibble (APM format)
            samples[sampleIndex++] = DecodeNibble((b >> 4) & 0x0F, ref state);
            samples[sampleIndex++] = DecodeNibble(b & 0x0F, ref state);
        }

        return samples;
    }

    /// <summary>
    /// Decodes stereo IMA-ADPCM data to interleaved 16-bit PCM.
    /// APM format uses byte-interleaved stereo (1 byte L, 1 byte R, repeat).
    /// </summary>
    /// <param name="adpcmData">Raw ADPCM data (byte-interleaved stereo)</param>
    /// <param name="leftState">Initial state for left channel</param>
    /// <param name="rightState">Initial state for right channel</param>
    /// <returns>Interleaved stereo PCM samples (L, R, L, R, ...)</returns>
    public static short[] DecodeStereo(byte[] adpcmData, ChannelState leftState, ChannelState rightState)
    {
        // Each byte pair produces 4 samples (2 per channel)
        int bytePairs = adpcmData.Length / 2;
        var samples = new short[bytePairs * 4];
        int sampleIndex = 0;

        for (int i = 0; i < adpcmData.Length - 1; i += 2)
        {
            byte leftByte = adpcmData[i];
            byte rightByte = adpcmData[i + 1];

            // Process HIGH nibbles first (both channels), then LOW nibbles
            // This matches the APM format where nibbles are processed in pairs
            short l0 = DecodeNibble((leftByte >> 4) & 0x0F, ref leftState);
            short r0 = DecodeNibble((rightByte >> 4) & 0x0F, ref rightState);
            short l1 = DecodeNibble(leftByte & 0x0F, ref leftState);
            short r1 = DecodeNibble(rightByte & 0x0F, ref rightState);

            // Interleave: L, R, L, R
            samples[sampleIndex++] = l0;
            samples[sampleIndex++] = r0;
            samples[sampleIndex++] = l1;
            samples[sampleIndex++] = r1;
        }

        return samples;
    }
}
