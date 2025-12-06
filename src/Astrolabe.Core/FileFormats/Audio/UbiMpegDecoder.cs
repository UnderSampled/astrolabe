using System.Diagnostics;
using NLayer;

namespace Astrolabe.Core.FileFormats.Audio;

/// <summary>
/// Decodes Ubisoft MPEG (Ubi-MPEG) audio data, a modified VBR MP2 format.
/// Used for voice/dialogue in BNM sound banks.
///
/// The format has a different sync word (12-bit 0xFFF vs 11-bit), fixed codec settings,
/// and frames are not byte-aligned. This decoder transforms Ubi-MPEG frames to standard
/// MP2 frames before decoding with NLayer.
/// </summary>
public class UbiMpegDecoder : IAudioDecoder
{
    // Ubi-MPEG constants
    private const int SamplesPerFrame = 1152;
    private const int MaxChannels = 2;
    private const int FixedFrameSize = 0x300; // ~256kbps at 48000hz
    private const int EncoderDelay = 480; // MP2 encoder delay samples to discard

    // Allocation table for 27 subbands (table 0)
    private static readonly byte[] BitAllocTable =
    [
        4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 0, 0, 0, 0, 0
    ];

    // Quantization index table [band][index]
    private static readonly byte[][] QIndexTable =
    [
        [0, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        [0, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        [0, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 16],
        [0, 1, 2, 3, 4, 5, 16],
        [0, 1, 2, 3, 4, 5, 16],
        [0, 1, 2, 3, 4, 5, 16],
        [0, 1, 2, 3, 4, 5, 16],
        [0, 1, 2, 3, 4, 5, 16],
        [0, 1, 2, 3, 4, 5, 16],
        [0, 1, 2, 3, 4, 5, 16],
        [0, 1, 2, 3, 4, 5, 16],
        [0, 1, 2, 3, 4, 5, 16],
        [0, 1, 2, 3, 4, 5, 16],
        [0, 1, 2, 3, 4, 5, 16],
        [0, 1, 2, 3, 4, 5, 16],
        [0, 1, 16],
        [0, 1, 16],
        [0, 1, 16],
        [0, 1, 16],
        [0, 1, 16],
        [0, 1, 16],
        [0, 1, 16]
    ];

    // Bits per codeword table (negative = grouping of 3)
    private static readonly sbyte[] QBitsTable =
    [
        -5, -7, 3, -10, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16
    ];

    // Joint stereo bounds based on mode extension
    private static readonly int[] JointBounds = [4, 8, 12, 16];
    private const int MaxSubbands = 27;
    private const int MaxGranules = 12;

    private readonly byte[] _data;
    private readonly bool _isSurround2;
    private readonly bool _isSurround1;

    public UbiMpegDecoder(byte[] data, uint sampleRate, ushort channels)
    {
        _data = data;
        SampleRate = sampleRate;

        // Check for surround mode headers
        if (data.Length >= 4)
        {
            uint header = BitConverter.ToUInt32(data, 0);
            if (header == 0x53555232) // "2RUS" in little-endian
                _isSurround2 = true;
            else if (header == 0x53555231) // "1RUS" in little-endian
                _isSurround1 = true;
        }

        // Detect actual channel count from first Ubi-MPEG frame header
        // BNM metadata often says stereo when frames are actually mono
        Channels = (ushort)DetectChannels();
    }

    /// <summary>
    /// Detects channel count from the first Ubi-MPEG frame header.
    /// </summary>
    private int DetectChannels()
    {
        var bitReader = new BitReader(_data);

        // Skip surround header if present
        if (_isSurround1 || _isSurround2)
            bitReader.Skip(32);

        // Find sync (12-bit 0xFFF)
        for (int i = 0; i < 32 && bitReader.BitsRemaining > 16; i++)
        {
            int sync = bitReader.ReadBits(12);
            if (sync == 0xFFF)
            {
                // Read 4-bit mode field
                int mode = bitReader.ReadBits(4);
                int chMode = (mode >> 2) & 0x03;
                // Mode 3 = mono, others = stereo
                return chMode == 3 ? 1 : 2;
            }
            if (sync != 0)
                break;
        }

        // Default to mono if detection fails
        return 1;
    }

    public short[] Decode()
    {
        // Transform Ubi-MPEG to standard MP2 and decode
        byte[] mp2Data = TransformToMp2();

        if (mp2Data.Length == 0)
            return [];

        // Try NLayer first, fall back to ffmpeg if it fails or returns empty
        try
        {
            var result = DecodeWithNLayer(mp2Data);
            if (result.Length > 0)
                return result;
            // NLayer returned empty, try ffmpeg
            return DecodeWithFfmpeg(mp2Data);
        }
        catch (Exception ex) when (ex is InvalidDataException or IndexOutOfRangeException)
        {
            // NLayer rejected the file or crashed, try ffmpeg as fallback
            return DecodeWithFfmpeg(mp2Data);
        }
    }

    private short[] DecodeWithNLayer(byte[] mp2Data)
    {
        using var inputStream = new MemoryStream(mp2Data);
        using var mpegFile = new MpegFile(inputStream);

        // Read all samples
        var samples = new List<float>();
        float[] buffer = new float[SamplesPerFrame * MaxChannels];

        int samplesRead;
        while ((samplesRead = mpegFile.ReadSamples(buffer, 0, buffer.Length)) > 0)
        {
            for (int i = 0; i < samplesRead; i++)
                samples.Add(buffer[i]);
        }

        // Skip encoder delay (480 samples per channel)
        int skipSamples = EncoderDelay * Channels;
        int outputCount = Math.Max(0, samples.Count - skipSamples);

        // Convert float samples to short (16-bit PCM)
        short[] result = new short[outputCount];
        for (int i = 0; i < outputCount; i++)
        {
            float sample = samples[skipSamples + i] * 32767f;
            result[i] = (short)Math.Clamp(sample, short.MinValue, short.MaxValue);
        }

        return result;
    }

    private short[] DecodeWithFfmpeg(byte[] mp2Data)
    {
        // Write MP2 data to temp file
        string tempMp2 = Path.GetTempFileName();
        string tempRaw = Path.GetTempFileName();

        try
        {
            File.WriteAllBytes(tempMp2, mp2Data);

            // Run ffmpeg to decode to raw PCM
            var psi = new ProcessStartInfo
            {
                FileName = "ffmpeg",
                Arguments = $"-hide_banner -y -i \"{tempMp2}\" -f s16le -acodec pcm_s16le -ar 48000 -ac 1 \"{tempRaw}\"",
                UseShellExecute = false,
                RedirectStandardError = true,
                CreateNoWindow = true
            };

            using var process = Process.Start(psi);
            if (process == null)
                throw new InvalidOperationException("Failed to start ffmpeg");

            process.WaitForExit(30000);

            if (!File.Exists(tempRaw) || new FileInfo(tempRaw).Length == 0)
                throw new InvalidOperationException("ffmpeg produced no output");

            // Read raw PCM data
            byte[] rawData = File.ReadAllBytes(tempRaw);
            short[] samples = new short[rawData.Length / 2];

            for (int i = 0; i < samples.Length; i++)
            {
                samples[i] = BitConverter.ToInt16(rawData, i * 2);
            }

            // Skip encoder delay
            int skipSamples = EncoderDelay;
            if (skipSamples >= samples.Length)
                return samples;

            short[] result = new short[samples.Length - skipSamples];
            Array.Copy(samples, skipSamples, result, 0, result.Length);

            return result;
        }
        finally
        {
            // Cleanup temp files
            try { File.Delete(tempMp2); } catch { }
            try { File.Delete(tempRaw); } catch { }
        }
    }

    public uint SampleRate { get; }
    public ushort Channels { get; }

    /// <summary>
    /// Transforms Ubi-MPEG data to standard MP2 format.
    /// </summary>
    private byte[] TransformToMp2()
    {
        var outputStream = new MemoryStream();
        var bitReader = new BitReader(_data);

        // Skip surround header if present
        if (_isSurround1 || _isSurround2)
            bitReader.Skip(32);

        while (bitReader.BitsRemaining > 16)
        {
            byte[] frame = TransformFrame(bitReader);
            if (frame.Length == 0)
                break;

            outputStream.Write(frame, 0, frame.Length);

            // In surround mode, skip the second (mono) frame
            if (_isSurround1 || _isSurround2)
            {
                // Consume the mono frame but don't output it
                // (proper surround mixing is not implemented)
                TransformFrame(bitReader);
            }
        }

        return outputStream.ToArray();
    }

    /// <summary>
    /// Transforms a single Ubi-MPEG frame to standard MP2 format.
    /// </summary>
    private byte[] TransformFrame(BitReader input)
    {
        // Find sync (12-bit 0xFFF)
        int sync = FindSync(input);
        if (sync != 0xFFF)
            return [];

        // Read 4-bit mode
        int mode = input.ReadBits(4);
        int extMode = mode & 0x03;
        int chMode = (mode >> 2) & 0x03;
        int channels = chMode == 3 ? 1 : 2; // Mode 3 = mono
        int jointBound = chMode != 1 ? MaxSubbands : JointBounds[extMode];

        // Output buffer with bit writer
        var output = new BitWriter(FixedFrameSize);

        // Write standard MP2 header
        output.WriteBits(11, 0x7FF);  // Sync
        output.WriteBits(2, 3);       // MPEG 1
        output.WriteBits(2, 2);       // Layer II
        output.WriteBits(1, 1);       // No CRC
        output.WriteBits(4, 12);      // Bitrate index (256kbps)
        output.WriteBits(2, 1);       // Sample rate index (48000)
        output.WriteBits(1, 0);       // No padding
        output.WriteBits(1, 0);       // Private
        output.WriteBits(2, chMode);  // Channel mode
        output.WriteBits(2, extMode); // Mode extension
        output.WriteBits(1, 1);       // Copyright
        output.WriteBits(1, 1);       // Original
        output.WriteBits(2, 0);       // Emphasis

        // Bit allocation
        var bitAlloc = new byte[32, 2];
        var scfsi = new int[32, 2];

        // Read/write allocation bits for non-joint bands
        for (int i = 0; i < jointBound; i++)
        {
            int baBits = BitAllocTable[i];
            for (int ch = 0; ch < channels; ch++)
            {
                bitAlloc[i, ch] = (byte)input.ReadBits(baBits);
                output.WriteBits(baBits, bitAlloc[i, ch]);
            }
        }

        // Read/write allocation bits for joint stereo bands
        for (int i = jointBound; i < MaxSubbands; i++)
        {
            int baBits = BitAllocTable[i];
            bitAlloc[i, 0] = (byte)input.ReadBits(baBits);
            bitAlloc[i, 1] = bitAlloc[i, 0];
            output.WriteBits(baBits, bitAlloc[i, 0]);
        }

        // Read/write scalefactor selector information
        for (int i = 0; i < MaxSubbands; i++)
        {
            for (int ch = 0; ch < channels; ch++)
            {
                if (bitAlloc[i, ch] == 0)
                    continue;

                scfsi[i, ch] = input.ReadBits(2);
                output.WriteBits(2, scfsi[i, ch]);
            }
        }

        // Read/write scalefactors
        for (int i = 0; i < MaxSubbands; i++)
        {
            for (int ch = 0; ch < channels; ch++)
            {
                if (bitAlloc[i, ch] == 0)
                    continue;

                switch (scfsi[i, ch])
                {
                    case 0: // 3 scalefactors
                        CopyBits(input, output, 6);
                        CopyBits(input, output, 6);
                        CopyBits(input, output, 6);
                        break;
                    case 1: // 2 scalefactors
                    case 3:
                        CopyBits(input, output, 6);
                        CopyBits(input, output, 6);
                        break;
                    case 2: // 1 scalefactor
                        CopyBits(input, output, 6);
                        break;
                }
            }
        }

        // Read/write quantized samples
        for (int gr = 0; gr < MaxGranules; gr++)
        {
            // Non-joint bands
            for (int i = 0; i < jointBound; i++)
            {
                for (int ch = 0; ch < channels; ch++)
                {
                    int baIndex = bitAlloc[i, ch];
                    if (baIndex == 0)
                        continue;

                    if (i >= QIndexTable.Length || baIndex - 1 >= QIndexTable[i].Length)
                        continue; // Skip invalid index
                    int qbIndex = QIndexTable[i][baIndex - 1];
                    if (qbIndex >= QBitsTable.Length)
                        continue;
                    int qbits = QBitsTable[qbIndex];
                    int qs = qbits < 0 ? 1 : 3;
                    if (qbits < 0) qbits = -qbits;

                    for (int q = 0; q < qs; q++)
                        CopyBits(input, output, qbits);
                }
            }

            // Joint stereo bands
            for (int i = jointBound; i < MaxSubbands; i++)
            {
                int baIndex = bitAlloc[i, 0];
                if (baIndex == 0)
                    continue;

                if (i >= QIndexTable.Length || baIndex - 1 >= QIndexTable[i].Length)
                    continue;
                int qbIndex = QIndexTable[i][baIndex - 1];
                if (qbIndex >= QBitsTable.Length)
                    continue;
                int qbits = QBitsTable[qbIndex];
                int qs = qbits < 0 ? 1 : 3;
                if (qbits < 0) qbits = -qbits;

                for (int q = 0; q < qs; q++)
                    CopyBits(input, output, qbits);
            }
        }

        // Byte-align output
        output.ByteAlign();

        return output.ToArray(FixedFrameSize);
    }

    private static int FindSync(BitReader reader)
    {
        for (int i = 0; i < 32; i++)
        {
            int sync = reader.ReadBits(12);
            if (sync == 0xFFF)
                return sync;
            if (sync != 0)
                return sync;
        }
        return 0;
    }

    private static void CopyBits(BitReader input, BitWriter output, int bits)
    {
        int value = input.ReadBits(bits);
        output.WriteBits(bits, value);
    }

    /// <summary>
    /// Bit reader for MSB-first bitstreams.
    /// </summary>
    private class BitReader
    {
        private readonly byte[] _data;
        private int _bitPos;

        public BitReader(byte[] data)
        {
            _data = data;
            _bitPos = 0;
        }

        public int BitsRemaining => (_data.Length * 8) - _bitPos;

        public void Skip(int bits)
        {
            _bitPos += bits;
        }

        public int ReadBits(int count)
        {
            if (count == 0 || _bitPos + count > _data.Length * 8)
                return 0;

            int result = 0;
            for (int i = 0; i < count; i++)
            {
                int byteIndex = _bitPos / 8;
                int bitIndex = 7 - (_bitPos % 8); // MSB first
                int bit = (_data[byteIndex] >> bitIndex) & 1;
                result = (result << 1) | bit;
                _bitPos++;
            }
            return result;
        }
    }

    /// <summary>
    /// Bit writer for MSB-first bitstreams.
    /// </summary>
    private class BitWriter
    {
        private readonly byte[] _data;
        private int _bitPos;

        public BitWriter(int capacity)
        {
            _data = new byte[capacity];
            _bitPos = 0;
        }

        public void WriteBits(int count, int value)
        {
            for (int i = count - 1; i >= 0; i--)
            {
                int byteIndex = _bitPos / 8;
                int bitIndex = 7 - (_bitPos % 8); // MSB first

                if (byteIndex < _data.Length)
                {
                    int bit = (value >> i) & 1;
                    _data[byteIndex] |= (byte)(bit << bitIndex);
                }
                _bitPos++;
            }
        }

        public void ByteAlign()
        {
            if (_bitPos % 8 != 0)
                _bitPos += 8 - (_bitPos % 8);
        }

        public byte[] ToArray(int size)
        {
            byte[] result = new byte[size];
            int copyLen = Math.Min(size, (_bitPos + 7) / 8);
            Array.Copy(_data, result, copyLen);
            return result;
        }
    }
}
