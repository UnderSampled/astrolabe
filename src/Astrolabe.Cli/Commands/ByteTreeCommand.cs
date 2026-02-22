using Astrolabe.Core.Extraction;
using Astrolabe.Core.FileFormats;

namespace Astrolabe.Cli.Commands;

public static class ByteTreeCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Path required");
            Console.Error.WriteLine("Usage: astrolabe byte-tree <level-path> [options]");
            Console.Error.WriteLine();
            Console.Error.WriteLine("The path should be a level directory (e.g., disc/Gamedata/World/Levels/brigand)");
            Console.Error.WriteLine();
            Console.Error.WriteLine("Options:");
            Console.Error.WriteLine("  --depth, -d <n>    Maximum depth to display (default: unlimited)");
            Console.Error.WriteLine("  --actual           Show only ActualWorld");
            Console.Error.WriteLine("  --dynamic          Show only DynamicWorld");
            Console.Error.WriteLine("  --sector           Show only FatherSector");
            Console.Error.WriteLine("  --gpt              Show GPT pointer roots (references to orphan data)");
            Console.Error.WriteLine("  --orphans          Show orphan (uncovered) regions");
            Console.Error.WriteLine("  --blocks           Show per-block coverage");
            Console.Error.WriteLine("  --summary          Show only summary statistics");
            return 1;
        }

        var path = args[0];
        int? maxDepth = null;
        bool showActual = false, showDynamic = false, showSector = false;
        bool showOrphans = false;
        bool showBlocks = false;
        bool summaryOnly = false;
        bool showGpt = false;

        // Parse options
        for (int i = 1; i < args.Length; i++)
        {
            switch (args[i])
            {
                case "--depth" or "-d":
                    if (i + 1 < args.Length && int.TryParse(args[i + 1], out int d))
                    {
                        maxDepth = d;
                        i++;
                    }
                    break;
                case "--actual":
                    showActual = true;
                    break;
                case "--dynamic":
                    showDynamic = true;
                    break;
                case "--sector":
                    showSector = true;
                    break;
                case "--gpt":
                    showGpt = true;
                    break;
                case "--orphans":
                    showOrphans = true;
                    break;
                case "--blocks":
                    showBlocks = true;
                    break;
                case "--summary":
                    summaryOnly = true;
                    break;
            }
        }

        // Default: show all roots if none specified
        if (!showActual && !showDynamic && !showSector)
        {
            showActual = showDynamic = showSector = true;
        }

        try
        {
            if (!IsLevelDirectory(path))
            {
                Console.Error.WriteLine($"Error: {path} does not appear to be a level directory");
                Console.Error.WriteLine("A level directory should contain .sna and .gpt files");
                return 1;
            }

            return ProcessLevel(path, maxDepth, showActual, showDynamic, showSector,
                showOrphans, showBlocks, summaryOnly, showGpt);
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            return 1;
        }
    }

    private static bool IsLevelDirectory(string path)
    {
        if (!Directory.Exists(path))
            return false;

        var dirName = Path.GetFileName(path.TrimEnd('/', '\\'));
        var snaPath = Path.Combine(path, $"{dirName}.sna");
        var gptPath = Path.Combine(path, $"{dirName}.gpt");

        if (File.Exists(snaPath) || Directory.GetFiles(path, $"{dirName}.sna*").Length > 0)
        {
            if (File.Exists(gptPath) || Directory.GetFiles(path, $"{dirName}.gpt*").Length > 0)
            {
                return true;
            }
        }

        return false;
    }

    private static int ProcessLevel(string levelDir, int? maxDepth, bool showActual,
        bool showDynamic, bool showSector, bool showOrphans, bool showBlocks, bool summaryOnly,
        bool showGpt)
    {
        var levelName = Path.GetFileName(levelDir.TrimEnd('/', '\\'));

        var loader = new LevelLoader(levelDir, levelName);

        var gptPath = Path.Combine(levelDir, $"{levelName}.gpt");
        if (!File.Exists(gptPath))
        {
            gptPath = Directory.GetFiles(levelDir, $"{levelName}.gpt*").FirstOrDefault() ?? "";
        }

        if (!File.Exists(gptPath))
        {
            Console.Error.WriteLine($"GPT file not found in {levelDir}");
            return 1;
        }

        var gpt = new GptReader(gptPath);
        var memory = new MemoryContext(loader.Sna, loader.Rtb);

        // Use tracking reader
        var tracker = new ByteRangeTracker();
        var sceneReader = new TrackingSuperObjectReader(memory, tracker);
        var sceneGraph = sceneReader.ReadSceneGraph(gpt);

        // Compute coverage
        var coverage = tracker.ComputeCoverage(loader.Sna.Blocks);

        // Print summary
        Console.WriteLine($"Level: {levelName}");
        Console.WriteLine($"Total SNA bytes: {coverage.TotalBytes:N0}");
        Console.WriteLine($"Covered bytes:   {coverage.CoveredBytes:N0} ({coverage.CoveragePercent:F1}%)");
        Console.WriteLine($"Uncovered bytes: {coverage.UncoveredBytes:N0} ({100 - coverage.CoveragePercent:F1}%)");
        Console.WriteLine($"Scene nodes:     {sceneGraph.AllNodes.Count}");
        Console.WriteLine();

        if (showBlocks)
        {
            PrintBlockCoverage(coverage);
            Console.WriteLine();
        }

        if (!summaryOnly)
        {
            if (showActual && sceneGraph.ActualWorld != null)
            {
                Console.WriteLine("ActualWorld");
                PrintTree(sceneGraph.ActualWorld, tracker, "", true, maxDepth, 0);
                Console.WriteLine();
            }

            if (showDynamic && sceneGraph.DynamicWorld != null)
            {
                Console.WriteLine("DynamicWorld");
                PrintTree(sceneGraph.DynamicWorld, tracker, "", true, maxDepth, 0);
                Console.WriteLine();
            }

            if (showSector && sceneGraph.FatherSector != null)
            {
                Console.WriteLine("FatherSector");
                PrintTree(sceneGraph.FatherSector, tracker, "", true, maxDepth, 0);
                Console.WriteLine();
            }
        }

        if (showGpt)
        {
            // Scan GPT for pointers that reference orphan data
            gpt.ScanForPointers();
            PrintGptPointers(gpt, tracker, coverage, loader.Sna.Blocks.ToList());
            Console.WriteLine();
        }

        if (showOrphans)
        {
            PrintOrphans(coverage, loader.Sna.Blocks);
        }

        return 0;
    }

    private static void PrintBlockCoverage(CoverageStats coverage)
    {
        Console.WriteLine("Block Coverage:");
        Console.WriteLine("  Block          Total       Covered     Uncovered   %");
        Console.WriteLine("  ─────────────────────────────────────────────────────");

        foreach (var block in coverage.BlockStats.OrderBy(b => b.Block.BaseInMemory))
        {
            var pct = block.CoveragePercent;
            var bar = new string('█', (int)(pct / 5)) + new string('░', 20 - (int)(pct / 5));

            Console.WriteLine($"  [{block.Block.Module:X2}:{block.Block.Id:X2}]  " +
                $"{block.TotalBytes,10:N0}  {block.CoveredBytes,10:N0}  {block.UncoveredBytes,10:N0}  " +
                $"{pct,5:F1}% {bar}");
        }
    }

    private static void PrintOrphans(CoverageStats coverage, IEnumerable<SnaBlock> blocks)
    {
        Console.WriteLine("Uncovered Regions (Orphans):");
        Console.WriteLine();

        var blockList = blocks.ToList();

        // Group by block and show significant orphans
        var orphansByBlock = coverage.UncoveredRegions
            .Where(r => r.Length >= 16) // Skip tiny gaps (likely padding)
            .GroupBy(r => r.Label)
            .OrderBy(g => g.Key);

        foreach (var group in orphansByBlock)
        {
            var totalBytes = group.Sum(r => r.Length);
            var regions = group.OrderByDescending(r => r.Length).ToList();

            Console.WriteLine($"  {group.Key}: {totalBytes:N0} bytes in {regions.Count} region(s)");

            // Show largest orphan regions
            foreach (var region in regions.Take(5))
            {
                var block = blockList.FirstOrDefault(b =>
                    b.BaseInMemory <= region.Start &&
                    b.BaseInMemory + (b.Data?.Length ?? 0) > region.Start);

                if (block?.Data != null)
                {
                    var offset = region.Start - block.BaseInMemory;
                    var preview = GetHexPreview(block.Data, offset, Math.Min(region.Length, 32));
                    Console.WriteLine($"    0x{region.Start:X8} ({region.Length,6:N0} bytes): {preview}");
                }
                else
                {
                    Console.WriteLine($"    0x{region.Start:X8} ({region.Length,6:N0} bytes)");
                }
            }

            if (regions.Count > 5)
            {
                Console.WriteLine($"    ... and {regions.Count - 5} more regions");
            }

            Console.WriteLine();
        }
    }

    private static string GetHexPreview(byte[] data, int offset, int length)
    {
        var bytes = new List<string>();
        for (int i = 0; i < length && offset + i < data.Length; i++)
        {
            bytes.Add(data[offset + i].ToString("X2"));
        }

        var hex = string.Join(" ", bytes);
        if (length < 32 || offset + length > data.Length)
            return hex;
        return hex + "...";
    }

    private static void PrintGptPointers(GptReader gpt, ByteRangeTracker tracker,
        CoverageStats coverage, List<SnaBlock> blocks)
    {
        Console.WriteLine("GPT Pointer Roots (references to orphan data):");
        Console.WriteLine();

        // Build set of covered ranges for quick lookup
        var coveredRanges = tracker.Ranges.ToList();

        // Group pointers by which orphan region they point into
        var orphanHits = new List<(int GptOffset, int Pointer, string Label, ByteRange? Orphan, int OrphanBytes)>();

        foreach (var (offset, pointer, label) in gpt.AllPointers)
        {
            // Skip the main scene graph roots (already tracked)
            if (label == "ActualWorld" || label == "DynamicWorld" || label == "FatherSector")
                continue;

            // Find if this pointer points to an orphan region
            var orphan = coverage.UncoveredRegions
                .Where(r => r.Length >= 16) // Skip tiny gaps
                .FirstOrDefault(r => pointer >= r.Start && pointer < r.End);

            if (orphan.Length > 0)
            {
                // Calculate how many bytes from this pointer to end of orphan region
                var bytesInOrphan = orphan.End - pointer;
                orphanHits.Add((offset, pointer, label, orphan, bytesInOrphan));
            }
        }

        if (orphanHits.Count == 0)
        {
            Console.WriteLine("  No GPT pointers reference orphan regions.");
            return;
        }

        // Group by the orphan region they point into
        var byOrphan = orphanHits
            .GroupBy(h => h.Orphan!.Value.Start)
            .OrderByDescending(g => g.First().Orphan!.Value.Length);

        foreach (var group in byOrphan)
        {
            var orphan = group.First().Orphan!.Value;
            Console.WriteLine($"  Orphan region @ 0x{orphan.Start:X8} ({orphan.Length:N0} bytes):");

            // Find the block containing this orphan
            var block = blocks.FirstOrDefault(b =>
                b.BaseInMemory <= orphan.Start &&
                b.BaseInMemory + (b.Data?.Length ?? 0) > orphan.Start);

            if (block?.Data != null)
            {
                var offset = orphan.Start - block.BaseInMemory;
                var preview = GetHexPreview(block.Data, offset, Math.Min(orphan.Length, 32));
                Console.WriteLine($"    Data: {preview}");
            }

            Console.WriteLine($"    Referenced by {group.Count()} GPT pointer(s):");
            foreach (var hit in group.OrderBy(h => h.GptOffset))
            {
                var relOffset = hit.Pointer - orphan.Start;
                Console.WriteLine($"      {hit.Label} (GPT+0x{hit.GptOffset:X3}) -> 0x{hit.Pointer:X8} (+{relOffset} into orphan)");
            }
            Console.WriteLine();
        }

        // Summary
        var totalOrphanBytes = byOrphan.Sum(g => g.First().Orphan!.Value.Length);
        var pctOfUncovered = coverage.UncoveredBytes > 0
            ? (double)totalOrphanBytes / coverage.UncoveredBytes * 100 : 0;
        Console.WriteLine($"  Total orphan bytes referenced by GPT: {totalOrphanBytes:N0} ({pctOfUncovered:F1}% of uncovered)");
    }

    private static void PrintTree(SceneNode node, ByteRangeTracker tracker, string prefix,
        bool isLast, int? maxDepth, int depth)
    {
        var label = FormatNode(node, tracker);

        var connector = isLast ? "└── " : "├── ";
        Console.WriteLine($"{prefix}{connector}{label}");

        if (maxDepth.HasValue && depth >= maxDepth.Value)
        {
            if (node.Children.Count > 0)
            {
                var childPrefix = prefix + (isLast ? "    " : "│   ");
                Console.WriteLine($"{childPrefix}└── ({node.Children.Count} children...)");
            }
            return;
        }

        var newPrefix = prefix + (isLast ? "    " : "│   ");
        for (int i = 0; i < node.Children.Count; i++)
        {
            var child = node.Children[i];
            var childIsLast = i == node.Children.Count - 1;
            PrintTree(child, tracker, newPrefix, childIsLast, maxDepth, depth + 1);
        }
    }

    private static string FormatNode(SceneNode node, ByteRangeTracker tracker)
    {
        var parts = new List<string>();

        parts.Add(node.Type.ToString());

        if (!string.IsNullOrEmpty(node.Name))
        {
            parts.Add($"\"{node.Name}\"");
        }

        // Add byte count
        var bytes = tracker.GetBytesForNode(node.Address);
        parts.Add($"({bytes} bytes)");

        // Add address
        parts.Add($"@0x{node.Address:X8}");

        // Add geometry indicator
        if (node.GeometricObjectAddress != 0)
        {
            parts.Add("[geo]");
        }

        return string.Join(" ", parts);
    }
}
