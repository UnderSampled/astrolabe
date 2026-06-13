using lzo.net;
using System.IO.Compression;

namespace Astrolabe.Core.FileFormats;

/// <summary>
/// Reads relocation tables (RTB, RTP, RTT) that map pointers between SNA memory blocks.
/// </summary>
public class RelocationTableReader
{
    public List<RelocationPointerBlock> PointerBlocks { get; } = new();

    private readonly byte[] _data;

    public RelocationTableReader(string filePath)
    {
        _data = File.ReadAllBytes(filePath);
        Parse();
    }

    public RelocationTableReader(byte[] data)
    {
        _data = data;
        Parse();
    }

    private void Parse()
    {
        using var reader = new BinaryReader(new MemoryStream(_data));

        // Montreal format: count byte, then blocks
        byte blockCount = reader.ReadByte();

        for (int i = 0; i < blockCount && reader.BaseStream.Position < reader.BaseStream.Length; i++)
        {
            var block = new RelocationPointerBlock
            {
                HeaderOffset = reader.BaseStream.Position,
                Module = reader.ReadByte(),
                Id = reader.ReadByte(),
                Count = reader.ReadUInt32()
            };

            if (block.Count > 0)
            {
                // Montreal uses compression for pointer blocks
                block.IsCompressed = reader.ReadUInt32() != 0;
                block.CompressedSize = reader.ReadUInt32();
                block.CompressedChecksum = reader.ReadUInt32();
                block.DecompressedSize = reader.ReadUInt32();
                block.DecompressedChecksum = reader.ReadUInt32();

                if (block.CompressedSize > reader.BaseStream.Length - reader.BaseStream.Position)
                {
                    break;
                }

                block.EncodedDataOffset = reader.BaseStream.Position;
                block.CompressedData = reader.ReadBytes((int)block.CompressedSize);

                byte[] pointerData;
                if (block.IsCompressed)
                {
                    pointerData = DecompressLzo(block.CompressedData, (int)block.DecompressedSize);
                }
                else
                {
                    pointerData = block.CompressedData;
                }

                block.PointerData = pointerData;
                block.EntrySize = GetPointerEntrySize(pointerData.Length, block.Count);

                // Parse pointers from decompressed data
                using var pointerReader = new BinaryReader(new MemoryStream(pointerData));
                block.Pointers = new RelocationPointerInfo[block.Count];

                for (int j = 0; j < block.Count; j++)
                {
                    block.Pointers[j] = new RelocationPointerInfo
                    {
                        OffsetInMemory = pointerReader.ReadUInt32(),
                        TargetModule = pointerReader.ReadByte(),
                        TargetId = pointerReader.ReadByte()
                    };

                    if (block.EntrySize >= 8)
                    {
                        block.Pointers[j].Byte6 = pointerReader.ReadByte();
                        block.Pointers[j].Byte7 = pointerReader.ReadByte();
                    }
                }

                var trailingLength = pointerData.Length - (int)(block.Count * block.EntrySize);
                if (trailingLength > 0)
                {
                    block.TrailingData = pointerReader.ReadBytes(trailingLength);
                }
            }
            else
            {
                block.Pointers = [];
                block.PointerData = [];
            }

            PointerBlocks.Add(block);
        }
    }

    private static int GetPointerEntrySize(int pointerDataLength, uint count)
    {
        if (count == 0)
        {
            return 0;
        }

        if (pointerDataLength >= count * 8)
        {
            return 8;
        }

        return 6;
    }

    private static byte[] DecompressLzo(byte[] compressedData, int decompressedSize)
    {
        using var inputStream = new MemoryStream(compressedData);
        using var lzoStream = new LzoStream(inputStream, CompressionMode.Decompress);
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
    /// Gets the pointer block for a specific module/id combination.
    /// </summary>
    public RelocationPointerBlock? GetBlock(byte module, byte id)
    {
        return PointerBlocks.FirstOrDefault(b => b.Module == module && b.Id == id);
    }

    /// <summary>
    /// Merges pointer blocks from another relocation table into this one.
    /// Blocks with the same key have their pointers merged.
    /// </summary>
    public void Merge(RelocationTableReader other)
    {
        foreach (var otherBlock in other.PointerBlocks)
        {
            var existingBlock = GetBlock(otherBlock.Module, otherBlock.Id);
            if (existingBlock != null)
            {
                // Merge pointers into existing block
                var mergedPointers = existingBlock.Pointers.ToList();
                var existingOffsets = new HashSet<uint>(mergedPointers.Select(p => p.OffsetInMemory));
                foreach (var ptr in otherBlock.Pointers)
                {
                    if (!existingOffsets.Contains(ptr.OffsetInMemory))
                    {
                        mergedPointers.Add(ptr);
                    }
                }
                existingBlock.Pointers = mergedPointers.ToArray();
                existingBlock.Count = (uint)existingBlock.Pointers.Length;
            }
            else
            {
                // Add new block
                PointerBlocks.Add(otherBlock);
            }
        }
    }
}

/// <summary>
/// A block of pointers for a specific SNA memory block.
/// </summary>
public class RelocationPointerBlock
{
    public byte Module { get; set; }
    public byte Id { get; set; }
    public uint Count { get; set; }
    public RelocationPointerInfo[] Pointers { get; set; } = [];
    public bool IsCompressed { get; set; }
    public uint CompressedSize { get; set; }
    public uint CompressedChecksum { get; set; }
    public uint DecompressedSize { get; set; }
    public uint DecompressedChecksum { get; set; }
    public byte[] CompressedData { get; set; } = [];
    public byte[] PointerData { get; set; } = [];
    public byte[] TrailingData { get; set; } = [];
    public int EntrySize { get; set; }
    public long HeaderOffset { get; set; }
    public long EncodedDataOffset { get; set; }

    public ushort Key => (ushort)((Module << 8) | Id);
}

/// <summary>
/// Information about a single pointer in memory.
/// </summary>
public class RelocationPointerInfo
{
    /// <summary>
    /// Offset in the source block where the pointer is located.
    /// </summary>
    public uint OffsetInMemory { get; set; }

    /// <summary>
    /// Target module this pointer points to.
    /// </summary>
    public byte TargetModule { get; set; }

    /// <summary>
    /// Target block ID this pointer points to.
    /// </summary>
    public byte TargetId { get; set; }

    public byte Byte6 { get; set; }

    public byte Byte7 { get; set; }

    public ushort TargetKey => (ushort)((TargetModule << 8) | TargetId);
}
