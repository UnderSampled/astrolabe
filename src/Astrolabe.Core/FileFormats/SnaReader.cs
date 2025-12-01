using lzo.net;

namespace Astrolabe.Core.FileFormats;

/// <summary>
/// Reads SNA compressed level data files (Montreal engine variant).
/// </summary>
public class SnaReader
{
    public List<SnaBlock> Blocks { get; } = new();

    private readonly byte[] _data;

    public SnaReader(string filePath)
    {
        _data = File.ReadAllBytes(filePath);
        Parse();
    }

    public SnaReader(byte[] data)
    {
        _data = data;
        Parse();
    }

    private void Parse()
    {
        using var reader = new BinaryReader(new MemoryStream(_data));

        while (reader.BaseStream.Position < reader.BaseStream.Length - 4)
        {
            try
            {
                var block = ReadBlock(reader);
                if (block != null)
                {
                    Blocks.Add(block);
                }
            }
            catch (EndOfStreamException)
            {
                break;
            }
            catch
            {
                // Try to continue reading
                break;
            }
        }
    }

    private SnaBlock? ReadBlock(BinaryReader reader)
    {
        var block = new SnaBlock();

        block.Module = reader.ReadByte();
        block.Id = reader.ReadByte();
        block.Unk1 = reader.ReadByte(); // Montreal variant
        block.BaseInMemory = reader.ReadInt32();

        if (block.BaseInMemory == -1)
        {
            // Block not loaded, skip to next
            return null;
        }

        block.Unk2 = reader.ReadUInt32();
        block.Unk3 = reader.ReadUInt32();
        block.MaxPosMinus9 = reader.ReadUInt32();
        block.Size = reader.ReadUInt32();

        // Read compressed data
        block.IsCompressed = reader.ReadUInt32() == 1;
        block.CompressedSize = reader.ReadUInt32();
        block.CompressedChecksum = reader.ReadUInt32();
        block.DecompressedSize = reader.ReadUInt32();
        block.DecompressedChecksum = reader.ReadUInt32();

        block.FileOffset = reader.BaseStream.Position;

        if (block.CompressedSize > 0 && block.CompressedSize < reader.BaseStream.Length - reader.BaseStream.Position)
        {
            block.CompressedData = reader.ReadBytes((int)block.CompressedSize);

            if (block.IsCompressed)
            {
                try
                {
                    block.Data = DecompressLzo(block.CompressedData, (int)block.DecompressedSize);
                }
                catch
                {
                    // Decompression failed, use compressed data as-is
                    block.Data = block.CompressedData;
                }
            }
            else
            {
                block.Data = block.CompressedData;
            }
        }

        return block;
    }

    private static byte[] DecompressLzo(byte[] compressedData, int decompressedSize)
    {
        using var inputStream = new MemoryStream(compressedData);
        using var lzoStream = new LzoStream(inputStream, System.IO.Compression.CompressionMode.Decompress);
        using var outputStream = new MemoryStream();

        var buffer = new byte[4096];
        int bytesRead;
        while ((bytesRead = lzoStream.Read(buffer, 0, buffer.Length)) > 0)
        {
            outputStream.Write(buffer, 0, bytesRead);
        }

        return outputStream.ToArray();
    }

    /// <summary>
    /// Gets a block by module and ID.
    /// </summary>
    public SnaBlock? GetBlock(byte module, byte id)
    {
        return Blocks.FirstOrDefault(b => b.Module == module && b.Id == id);
    }

    /// <summary>
    /// Gets all decompressed data concatenated.
    /// </summary>
    public byte[] GetAllData()
    {
        using var output = new MemoryStream();
        foreach (var block in Blocks.Where(b => b.Data != null))
        {
            output.Write(block.Data!, 0, block.Data!.Length);
        }
        return output.ToArray();
    }
}

/// <summary>
/// A data block within an SNA file.
/// </summary>
public class SnaBlock
{
    public byte Module { get; set; }
    public byte Id { get; set; }
    public byte Unk1 { get; set; }
    public int BaseInMemory { get; set; }
    public uint Unk2 { get; set; }
    public uint Unk3 { get; set; }
    public uint MaxPosMinus9 { get; set; }
    public uint Size { get; set; }
    public bool IsCompressed { get; set; }
    public uint CompressedSize { get; set; }
    public uint CompressedChecksum { get; set; }
    public uint DecompressedSize { get; set; }
    public uint DecompressedChecksum { get; set; }
    public long FileOffset { get; set; }
    public byte[]? CompressedData { get; set; }
    public byte[]? Data { get; set; }

    /// <summary>
    /// Combined key for block identification.
    /// </summary>
    public ushort Key => (ushort)((Module << 8) | Id);
}
