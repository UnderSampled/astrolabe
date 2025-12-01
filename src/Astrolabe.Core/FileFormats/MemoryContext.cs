using System.Numerics;

namespace Astrolabe.Core.FileFormats;

/// <summary>
/// Provides unified memory access across SNA blocks with pointer resolution.
/// Based on raymap's SNA.CreatePointers approach.
/// </summary>
public class MemoryContext
{
    private readonly Dictionary<ushort, SnaBlock> _blocks = new();
    private readonly Dictionary<int, PointerInfo> _pointers = new();

    public SnaReader Sna { get; }
    public RelocationTableReader? Rtb { get; }

    public MemoryContext(SnaReader sna, RelocationTableReader? rtb)
    {
        Sna = sna;
        Rtb = rtb;

        // Build block lookup
        foreach (var block in sna.Blocks)
        {
            _blocks[block.Key] = block;
        }

        // Build pointer map from relocation table
        if (rtb != null)
        {
            BuildPointerMap(rtb);
        }
    }

    private void BuildPointerMap(RelocationTableReader rtb)
    {
        foreach (var ptrBlock in rtb.PointerBlocks)
        {
            var sourceBlock = _blocks.GetValueOrDefault(ptrBlock.Key);
            if (sourceBlock?.Data == null) continue;

            foreach (var ptr in ptrBlock.Pointers)
            {
                // Memory address where the pointer is stored
                int ptrLocation = (int)ptr.OffsetInMemory;

                // Calculate offset within source block
                int offsetInBlock = ptrLocation - sourceBlock.BaseInMemory;
                if (offsetInBlock < 0 || offsetInBlock + 4 > sourceBlock.Data.Length)
                    continue;

                // Read the pointer value (memory address in target block)
                int ptrValue = BitConverter.ToInt32(sourceBlock.Data, offsetInBlock);

                // Get target block
                ushort targetKey = (ushort)((ptr.TargetModule << 8) | ptr.TargetId);

                _pointers[ptrLocation] = new PointerInfo
                {
                    SourceBlock = sourceBlock,
                    OffsetInSourceBlock = offsetInBlock,
                    TargetKey = targetKey,
                    TargetModule = ptr.TargetModule,
                    TargetId = ptr.TargetId,
                    RawValue = ptrValue
                };
            }
        }
    }

    /// <summary>
    /// Reads data at a memory address.
    /// </summary>
    public byte[]? ReadBytes(int memoryAddress, int length)
    {
        // Find which block contains this address
        foreach (var block in Sna.Blocks)
        {
            if (block.Data == null) continue;

            int endAddr = block.BaseInMemory + block.Data.Length;
            if (memoryAddress >= block.BaseInMemory && memoryAddress < endAddr)
            {
                int offset = memoryAddress - block.BaseInMemory;
                if (offset + length > block.Data.Length)
                    return null;

                var result = new byte[length];
                Array.Copy(block.Data, offset, result, 0, length);
                return result;
            }
        }
        return null;
    }

    /// <summary>
    /// Gets a BinaryReader positioned at a memory address.
    /// </summary>
    public BinaryReader? GetReaderAt(int memoryAddress)
    {
        foreach (var block in Sna.Blocks)
        {
            if (block.Data == null) continue;

            int endAddr = block.BaseInMemory + block.Data.Length;
            if (memoryAddress >= block.BaseInMemory && memoryAddress < endAddr)
            {
                int offset = memoryAddress - block.BaseInMemory;
                var ms = new MemoryStream(block.Data);
                ms.Position = offset;
                return new BinaryReader(ms);
            }
        }
        return null;
    }

    /// <summary>
    /// Gets pointer info at a memory address (if it's a known pointer location).
    /// </summary>
    public PointerInfo? GetPointerAt(int memoryAddress)
    {
        return _pointers.GetValueOrDefault(memoryAddress);
    }

    /// <summary>
    /// Follows a pointer at a memory address and returns a reader at the target.
    /// </summary>
    public BinaryReader? FollowPointer(int memoryAddress)
    {
        var ptr = GetPointerAt(memoryAddress);
        if (ptr == null) return null;

        return GetReaderAt(ptr.RawValue);
    }

    /// <summary>
    /// Gets all pointers that target a specific block.
    /// </summary>
    public IEnumerable<(int sourceAddr, PointerInfo ptr)> GetPointersToBlock(byte module, byte id)
    {
        ushort targetKey = (ushort)((module << 8) | id);
        return _pointers
            .Where(kvp => kvp.Value.TargetKey == targetKey)
            .Select(kvp => (kvp.Key, kvp.Value));
    }

    /// <summary>
    /// Gets block by module/id.
    /// </summary>
    public SnaBlock? GetBlock(byte module, byte id)
    {
        ushort key = (ushort)((module << 8) | id);
        return _blocks.GetValueOrDefault(key);
    }

    /// <summary>
    /// Checks if a memory address is a valid pointer location.
    /// </summary>
    public bool IsPointer(int memoryAddress) => _pointers.ContainsKey(memoryAddress);
}

/// <summary>
/// Information about a pointer in memory.
/// </summary>
public class PointerInfo
{
    public SnaBlock SourceBlock { get; set; } = null!;
    public int OffsetInSourceBlock { get; set; }
    public ushort TargetKey { get; set; }
    public byte TargetModule { get; set; }
    public byte TargetId { get; set; }
    public int RawValue { get; set; }
}
