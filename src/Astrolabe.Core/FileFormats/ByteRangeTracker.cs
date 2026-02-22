namespace Astrolabe.Core.FileFormats;

/// <summary>
/// A range of bytes in memory with an associated label.
/// </summary>
public readonly record struct ByteRange(int Start, int Length, string Label)
{
    public int End => Start + Length;

    public bool Overlaps(ByteRange other)
        => Start < other.End && End > other.Start;

    public override string ToString()
        => $"0x{Start:X8}..0x{End:X8} ({Length} bytes) [{Label}]";
}

/// <summary>
/// Tracks which byte ranges have been read from memory, for coverage analysis.
/// </summary>
public class ByteRangeTracker
{
    private readonly List<ByteRange> _ranges = new();
    private readonly Dictionary<int, List<ByteRange>> _nodeRanges = new();

    /// <summary>
    /// All recorded byte ranges.
    /// </summary>
    public IReadOnlyList<ByteRange> Ranges => _ranges;

    /// <summary>
    /// Records a byte range as being read.
    /// </summary>
    public void Record(int start, int length, string label)
    {
        if (length <= 0) return;
        _ranges.Add(new ByteRange(start, length, label));
    }

    /// <summary>
    /// Records a byte range associated with a specific node address.
    /// </summary>
    public void RecordForNode(int nodeAddress, int start, int length, string label)
    {
        if (length <= 0) return;
        var range = new ByteRange(start, length, label);
        _ranges.Add(range);

        if (!_nodeRanges.TryGetValue(nodeAddress, out var list))
        {
            list = new List<ByteRange>();
            _nodeRanges[nodeAddress] = list;
        }
        list.Add(range);
    }

    /// <summary>
    /// Gets all byte ranges for a specific node.
    /// </summary>
    public IReadOnlyList<ByteRange> GetRangesForNode(int nodeAddress)
    {
        return _nodeRanges.GetValueOrDefault(nodeAddress) ?? [];
    }

    /// <summary>
    /// Gets the total bytes covered for a node (including all its recorded ranges).
    /// </summary>
    public int GetBytesForNode(int nodeAddress)
    {
        var ranges = GetRangesForNode(nodeAddress);
        if (ranges.Count == 0) return 0;

        // Merge overlapping ranges and sum
        return MergeAndSum(ranges);
    }

    /// <summary>
    /// Computes coverage statistics for a set of memory blocks.
    /// </summary>
    public CoverageStats ComputeCoverage(IEnumerable<SnaBlock> blocks)
    {
        var stats = new CoverageStats();

        foreach (var block in blocks)
        {
            if (block.Data == null) continue;

            var blockStart = block.BaseInMemory;
            var blockEnd = blockStart + block.Data.Length;
            stats.TotalBytes += block.Data.Length;

            // Create a bitmap of covered bytes for this block
            var covered = new bool[block.Data.Length];

            foreach (var range in _ranges)
            {
                // Check if range overlaps with this block
                if (range.End <= blockStart || range.Start >= blockEnd)
                    continue;

                // Calculate overlap
                var overlapStart = Math.Max(range.Start, blockStart);
                var overlapEnd = Math.Min(range.End, blockEnd);

                for (int i = overlapStart - blockStart; i < overlapEnd - blockStart; i++)
                {
                    covered[i] = true;
                }
            }

            // Count covered bytes and find uncovered regions
            int blockCovered = 0;
            int? uncoveredStart = null;

            for (int i = 0; i < covered.Length; i++)
            {
                if (covered[i])
                {
                    blockCovered++;
                    if (uncoveredStart.HasValue)
                    {
                        // End of uncovered region
                        stats.UncoveredRegions.Add(new ByteRange(
                            blockStart + uncoveredStart.Value,
                            i - uncoveredStart.Value,
                            $"Block [{block.Module:X2}:{block.Id:X2}]"));
                        uncoveredStart = null;
                    }
                }
                else
                {
                    if (!uncoveredStart.HasValue)
                    {
                        uncoveredStart = i;
                    }
                }
            }

            // Handle trailing uncovered region
            if (uncoveredStart.HasValue)
            {
                stats.UncoveredRegions.Add(new ByteRange(
                    blockStart + uncoveredStart.Value,
                    covered.Length - uncoveredStart.Value,
                    $"Block [{block.Module:X2}:{block.Id:X2}]"));
            }

            stats.CoveredBytes += blockCovered;
            stats.BlockStats.Add(new BlockCoverageStats
            {
                Block = block,
                TotalBytes = block.Data.Length,
                CoveredBytes = blockCovered
            });
        }

        return stats;
    }

    private static int MergeAndSum(IReadOnlyList<ByteRange> ranges)
    {
        if (ranges.Count == 0) return 0;

        var sorted = ranges.OrderBy(r => r.Start).ToList();
        var merged = new List<(int Start, int End)>();

        var current = (sorted[0].Start, sorted[0].End);
        for (int i = 1; i < sorted.Count; i++)
        {
            var r = sorted[i];
            if (r.Start <= current.End)
            {
                // Overlapping or adjacent - extend
                current.End = Math.Max(current.End, r.End);
            }
            else
            {
                // Gap - save current and start new
                merged.Add(current);
                current = (r.Start, r.End);
            }
        }
        merged.Add(current);

        return merged.Sum(r => r.End - r.Start);
    }

    /// <summary>
    /// Clears all recorded ranges.
    /// </summary>
    public void Clear()
    {
        _ranges.Clear();
        _nodeRanges.Clear();
    }
}

/// <summary>
/// Coverage statistics for SNA blocks.
/// </summary>
public class CoverageStats
{
    public int TotalBytes { get; set; }
    public int CoveredBytes { get; set; }
    public List<ByteRange> UncoveredRegions { get; } = new();
    public List<BlockCoverageStats> BlockStats { get; } = new();

    public double CoveragePercent => TotalBytes > 0 ? (double)CoveredBytes / TotalBytes * 100 : 0;
    public int UncoveredBytes => TotalBytes - CoveredBytes;
}

/// <summary>
/// Coverage statistics for a single SNA block.
/// </summary>
public class BlockCoverageStats
{
    public SnaBlock Block { get; set; } = null!;
    public int TotalBytes { get; set; }
    public int CoveredBytes { get; set; }

    public double CoveragePercent => TotalBytes > 0 ? (double)CoveredBytes / TotalBytes * 100 : 0;
    public int UncoveredBytes => TotalBytes - CoveredBytes;
}
