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
        // NOTE: Montreal engine does NOT have unk1 byte here (only later engine versions do)
        block.BaseInMemory = reader.ReadInt32();

        if (block.BaseInMemory == -1)
        {
            // Block not loaded. It still belongs to the container and must be
            // preserved by reversible intermediate exports.
            return block;
        }

        block.Unk2 = reader.ReadUInt32();
        block.Unk3 = reader.ReadUInt32();
        block.MaxPosMinus9 = reader.ReadUInt32();
        block.Size = reader.ReadUInt32();

        // Only read compressed data if size > 0
        if (block.Size > 0)
        {
            block.IsCompressed = reader.ReadUInt32() == 1;
            block.CompressedSize = reader.ReadUInt32();
            block.CompressedChecksum = reader.ReadUInt32();
            block.DecompressedSize = reader.ReadUInt32();
            block.DecompressedChecksum = reader.ReadUInt32();

            if (block.CompressedSize > 0 && block.CompressedSize <= reader.BaseStream.Length - reader.BaseStream.Position)
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

    /// <summary>
    /// Merges blocks from another SNA reader into this one.
    /// Blocks with the same key are not duplicated.
    /// </summary>
    public void Merge(SnaReader other)
    {
        int addedCount = 0;
        int replacedCount = 0;
        int skippedCount = 0;

        // Build a map of existing blocks by key, so we can replace empty blocks with data blocks
        var existingBlocksByKey = Blocks.ToDictionary(b => b.Key, b => b);

        foreach (var block in other.Blocks)
        {
            if (!existingBlocksByKey.ContainsKey(block.Key))
            {
                // New key - add the block
                Blocks.Add(block);
                existingBlocksByKey[block.Key] = block;
                addedCount++;
            }
            else
            {
                // Key exists - check if we should replace (prefer blocks with data)
                var existingBlock = existingBlocksByKey[block.Key];
                bool existingHasData = existingBlock.Data != null && existingBlock.Data.Length > 0;
                bool newHasData = block.Data != null && block.Data.Length > 0;

                if (!existingHasData && newHasData)
                {
                    // Replace empty block with block that has data
                    int idx = Blocks.IndexOf(existingBlock);
                    if (idx >= 0)
                    {
                        Blocks[idx] = block;
                        existingBlocksByKey[block.Key] = block;
                        replacedCount++;
                    }
                }
                else
                {
                    skippedCount++;
                }
            }
        }
        Console.WriteLine($"    Merge: added {addedCount}, replaced {replacedCount} empty blocks, skipped {skippedCount}");
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
    public byte[]? CompressedData { get; set; }
    public byte[]? Data { get; set; }

    /// <summary>
    /// Combined key for block identification.
    /// </summary>
    public ushort Key => (ushort)((Module << 8) | Id);
}
