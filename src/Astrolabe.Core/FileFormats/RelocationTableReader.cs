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
                Module = reader.ReadByte(),
                Id = reader.ReadByte(),
                Count = reader.ReadUInt32()
            };

            if (block.Count > 0)
            {
                // Montreal uses compression for pointer blocks
                uint isCompressed = reader.ReadUInt32();
                uint compressedSize = reader.ReadUInt32();
                uint compressedChecksum = reader.ReadUInt32();
                uint decompressedSize = reader.ReadUInt32();
                uint decompressedChecksum = reader.ReadUInt32();

                if (compressedSize > reader.BaseStream.Length - reader.BaseStream.Position)
                {
                    break;
                }

                byte[] compressedData = reader.ReadBytes((int)compressedSize);

                byte[] pointerData;
                if (isCompressed != 0)
                {
                    pointerData = DecompressLzo(compressedData, (int)decompressedSize);
                }
                else
                {
                    pointerData = compressedData;
                }

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
                    // Montreal uses 6-byte pointer entries (no extra 2 bytes like R2/R3)
                }
            }
            else
            {
                block.Pointers = [];
            }

            PointerBlocks.Add(block);
        }
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

    public ushort TargetKey => (ushort)((TargetModule << 8) | TargetId);
}
