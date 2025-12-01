namespace Astrolabe.Core.FileFormats;

/// <summary>
/// Loads a complete level from SNA + relocation tables, providing pointer resolution.
/// </summary>
public class LevelLoader
{
    public SnaReader Sna { get; }
    public RelocationTableReader? Rtb { get; }
    public RelocationTableReader? Rtp { get; }
    public RelocationTableReader? Rtt { get; }

    // Map from virtual memory address to (block, offset within block)
    private readonly Dictionary<int, (SnaBlock block, int offset)> _memoryMap = new();

    // Map from block key to SnaBlock
    private readonly Dictionary<ushort, SnaBlock> _blockMap = new();

    public LevelLoader(string levelDir, string levelName)
    {
        string snaPath = Path.Combine(levelDir, $"{levelName}.sna;1");
        string rtbPath = Path.Combine(levelDir, $"{levelName}.rtb;1");
        string rtpPath = Path.Combine(levelDir, $"{levelName}.rtp;1");
        string rttPath = Path.Combine(levelDir, $"{levelName}.rtt;1");

        // Handle case-insensitive file extensions
        if (!File.Exists(snaPath))
        {
            snaPath = FindFile(levelDir, $"{levelName}.sna");
        }

        Sna = new SnaReader(snaPath);

        if (File.Exists(rtbPath) || (rtbPath = FindFile(levelDir, $"{levelName}.rtb")) != null)
        {
            Rtb = new RelocationTableReader(rtbPath);
        }

        if (File.Exists(rtpPath) || (rtpPath = FindFile(levelDir, $"{levelName}.rtp")) != null)
        {
            Rtp = new RelocationTableReader(rtpPath);
        }

        if (File.Exists(rttPath) || (rttPath = FindFile(levelDir, $"{levelName}.rtt")) != null)
        {
            Rtt = new RelocationTableReader(rttPath);
        }

        BuildMemoryMap();
    }

    private static string? FindFile(string dir, string baseName)
    {
        var files = Directory.GetFiles(dir, baseName + "*", SearchOption.TopDirectoryOnly);
        return files.FirstOrDefault();
    }

    private void BuildMemoryMap()
    {
        foreach (var block in Sna.Blocks)
        {
            _blockMap[block.Key] = block;

            if (block.BaseInMemory >= 0 && block.Data != null)
            {
                // Map each byte in this block's virtual memory range
                for (int i = 0; i < block.Data.Length; i++)
                {
                    _memoryMap[block.BaseInMemory + i] = (block, i);
                }
            }
        }
    }

    /// <summary>
    /// Gets a block by module and ID.
    /// </summary>
    public SnaBlock? GetBlock(byte module, byte id)
    {
        ushort key = (ushort)((module << 8) | id);
        return _blockMap.GetValueOrDefault(key);
    }

    /// <summary>
    /// Reads data at a virtual memory address.
    /// </summary>
    public byte[]? ReadAt(int virtualAddress, int length)
    {
        if (!_memoryMap.TryGetValue(virtualAddress, out var location))
        {
            return null;
        }

        var (block, offset) = location;
        if (block.Data == null || offset + length > block.Data.Length)
        {
            return null;
        }

        var result = new byte[length];
        Array.Copy(block.Data, offset, result, 0, length);
        return result;
    }

    /// <summary>
    /// Resolves a pointer at a given virtual address using the relocation table.
    /// </summary>
    public int? ResolvePointer(int virtualAddress, byte sourceModule, byte sourceId)
    {
        if (Rtb == null) return null;

        var pointerBlock = Rtb.GetBlock(sourceModule, sourceId);
        if (pointerBlock == null) return null;

        // Find the pointer info for this offset
        int offsetInBlock = virtualAddress - (GetBlock(sourceModule, sourceId)?.BaseInMemory ?? 0);

        foreach (var ptr in pointerBlock.Pointers)
        {
            if (ptr.OffsetInMemory == offsetInBlock)
            {
                // Read the raw pointer value
                var data = ReadAt(virtualAddress, 4);
                if (data == null) return null;

                int rawPointer = BitConverter.ToInt32(data, 0);

                // The pointer points to the target block
                var targetBlock = GetBlock(ptr.TargetModule, ptr.TargetId);
                if (targetBlock == null) return null;

                // The raw pointer is relative to the target block's base
                return rawPointer;
            }
        }

        return null;
    }

    /// <summary>
    /// Creates a BinaryReader positioned at a specific block.
    /// </summary>
    public BinaryReader? GetBlockReader(byte module, byte id)
    {
        var block = GetBlock(module, id);
        if (block?.Data == null) return null;

        return new BinaryReader(new MemoryStream(block.Data));
    }

    /// <summary>
    /// Creates a BinaryReader for reading data at a virtual address.
    /// </summary>
    public BinaryReader? GetReaderAt(int virtualAddress)
    {
        if (!_memoryMap.TryGetValue(virtualAddress, out var location))
        {
            return null;
        }

        var (block, offset) = location;
        if (block.Data == null) return null;

        var ms = new MemoryStream(block.Data);
        ms.Position = offset;
        return new BinaryReader(ms);
    }

    /// <summary>
    /// Prints debug information about the loaded level.
    /// </summary>
    public void PrintDebugInfo(TextWriter writer)
    {
        writer.WriteLine($"SNA Blocks: {Sna.Blocks.Count}");
        writer.WriteLine($"RTB Pointer Blocks: {Rtb?.PointerBlocks.Count ?? 0}");
        writer.WriteLine($"RTP Pointer Blocks: {Rtp?.PointerBlocks.Count ?? 0}");
        writer.WriteLine($"RTT Pointer Blocks: {Rtt?.PointerBlocks.Count ?? 0}");
        writer.WriteLine();

        writer.WriteLine("SNA Memory Blocks:");
        foreach (var block in Sna.Blocks.OrderBy(b => b.BaseInMemory))
        {
            var dataSize = block.Data?.Length ?? 0;
            var compressed = block.IsCompressed ? "LZO" : "raw";
            writer.WriteLine($"  [{block.Module:X2}:{block.Id:X2}] Base=0x{block.BaseInMemory:X8} Size={dataSize,8} ({compressed})");
        }

        if (Rtb != null)
        {
            writer.WriteLine();
            writer.WriteLine("RTB Pointer Blocks:");
            foreach (var block in Rtb.PointerBlocks)
            {
                writer.WriteLine($"  [{block.Module:X2}:{block.Id:X2}] {block.Count} pointers");
            }
        }
    }
}
