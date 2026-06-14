using System.IO.Compression;
using lzo.net;

namespace Astrolabe.Core.Rete.OpenSpace;

/// <summary>
/// LZO1X compression for OpenSpace RT*/SNA payloads. Decompression uses lzo.net; compression uses MiniLZO
/// because lzo.net only supports <see cref="CompressionMode.Decompress"/>.
/// </summary>
internal static class OpenSpaceLzo
{
    public static byte[] Compress(ReadOnlySpan<byte> data)
    {
        if (data.IsEmpty)
        {
            return [];
        }

        var compressed = MiniLZO.MiniLZO.Compress(data.ToArray());
        if (!TryDecompress(compressed, data.Length, out var roundTrip) ||
            !roundTrip.AsSpan().SequenceEqual(data))
        {
            throw new InvalidDataException("LZO compression round-trip validation failed.");
        }

        return compressed;
    }

    public static bool TryCompress(ReadOnlySpan<byte> data, out byte[] compressed)
    {
        compressed = [];
        if (data.IsEmpty)
        {
            return true;
        }

        try
        {
            compressed = Compress(data);
            return true;
        }
        catch (InvalidDataException)
        {
            return false;
        }
    }

    public static byte[] Decompress(ReadOnlySpan<byte> compressedData, int decompressedSize)
    {
        if (!TryDecompress(compressedData, decompressedSize, out var data))
        {
            throw new InvalidDataException("LZO decompression failed.");
        }

        return data;
    }

    public static bool TryDecompress(ReadOnlySpan<byte> compressedData, int decompressedSize, out byte[] data)
    {
        data = [];
        try
        {
            using var inputStream = new MemoryStream(compressedData.ToArray());
            using var lzoStream = new LzoStream(inputStream, CompressionMode.Decompress);
            using var outputStream = new MemoryStream();

            var buffer = new byte[4096];
            int bytesRead;
            while ((bytesRead = lzoStream.Read(buffer, 0, buffer.Length)) > 0)
            {
                outputStream.Write(buffer, 0, bytesRead);
            }

            data = outputStream.ToArray();
            if (data.Length > decompressedSize)
            {
                Array.Resize(ref data, decompressedSize);
            }

            return data.Length == decompressedSize;
        }
        catch
        {
            return false;
        }
    }
}