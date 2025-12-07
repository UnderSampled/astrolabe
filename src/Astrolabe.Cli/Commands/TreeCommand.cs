using Astrolabe.Core.Extraction;
using Astrolabe.Core.FileFormats;

namespace Astrolabe.Cli.Commands;

public static class TreeCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Path required");
            Console.Error.WriteLine("Usage: astrolabe tree <path> [options]");
            Console.Error.WriteLine();
            Console.Error.WriteLine("The path can be:");
            Console.Error.WriteLine("  - A level directory (e.g., disc/Gamedata/World/Levels/brigand)");
            Console.Error.WriteLine("  - An extracted game directory (e.g., disc/)");
            Console.Error.WriteLine("  - An ISO file (e.g., hype.iso)");
            Console.Error.WriteLine();
            Console.Error.WriteLine("Options:");
            Console.Error.WriteLine("  --depth, -d <n>    Maximum depth to display (default: unlimited)");
            Console.Error.WriteLine("  --actual           Show only ActualWorld");
            Console.Error.WriteLine("  --dynamic          Show only DynamicWorld");
            Console.Error.WriteLine("  --sector           Show only FatherSector");
            Console.Error.WriteLine("  --stats            Show statistics summary");
            return 1;
        }

        var path = args[0];
        int? maxDepth = null;
        bool showActual = false, showDynamic = false, showSector = false;
        bool showStats = false;

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
                case "--stats":
                    showStats = true;
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
            // Detect if this is a level directory or a game source
            if (IsLevelDirectory(path))
            {
                return ProcessLevel(path, maxDepth, showActual, showDynamic, showSector, showStats);
            }
            else
            {
                return ProcessGameSource(path, maxDepth, showActual, showDynamic, showSector, showStats);
            }
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

        // A level directory has .sna and .gpt files with the same base name
        var dirName = Path.GetFileName(path.TrimEnd('/', '\\'));
        var snaPath = Path.Combine(path, $"{dirName}.sna");
        var gptPath = Path.Combine(path, $"{dirName}.gpt");

        // Case-insensitive check
        if (File.Exists(snaPath) || Directory.GetFiles(path, $"{dirName}.sna*").Length > 0)
        {
            if (File.Exists(gptPath) || Directory.GetFiles(path, $"{dirName}.gpt*").Length > 0)
            {
                return true;
            }
        }

        return false;
    }

    private static int ProcessLevel(string levelDir, int? maxDepth, bool showActual, bool showDynamic, bool showSector, bool showStats)
    {
        var levelName = Path.GetFileName(levelDir.TrimEnd('/', '\\'));

        var loader = new LevelLoader(levelDir, levelName);

        // Find GPT file
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
        var sceneReader = new SuperObjectReader(memory);
        var sceneGraph = sceneReader.ReadSceneGraph(gpt);

        if (showStats)
        {
            PrintStats(sceneGraph);
            Console.WriteLine();
        }

        if (showActual && sceneGraph.ActualWorld != null)
        {
            Console.WriteLine("ActualWorld");
            PrintTree(sceneGraph.ActualWorld, "", true, maxDepth, 0);
            Console.WriteLine();
        }

        if (showDynamic && sceneGraph.DynamicWorld != null)
        {
            Console.WriteLine("DynamicWorld");
            PrintTree(sceneGraph.DynamicWorld, "", true, maxDepth, 0);
            Console.WriteLine();
        }

        if (showSector && sceneGraph.FatherSector != null)
        {
            Console.WriteLine("FatherSector");
            PrintTree(sceneGraph.FatherSector, "", true, maxDepth, 0);
        }

        return 0;
    }

    private static int ProcessGameSource(string sourcePath, int? maxDepth, bool showActual, bool showDynamic, bool showSector, bool showStats)
    {
        using var source = GameSourceFactory.Create(sourcePath);
        Console.WriteLine($"Source: {source.SourcePath} ({(source.IsIso ? "ISO" : "Directory")})");
        Console.WriteLine();

        // Find all level directories by looking for .gpt files
        var gptFiles = source.GetFiles("*.gpt")
            .Where(f => !Path.GetFileName(f).Equals("Fix.gpt", StringComparison.OrdinalIgnoreCase))
            .OrderBy(f => f)
            .ToList();

        if (gptFiles.Count == 0)
        {
            Console.Error.WriteLine("No levels found (no .gpt files)");
            return 1;
        }

        Console.WriteLine($"Found {gptFiles.Count} levels");
        Console.WriteLine();

        // For ISO sources, we need to extract level files to temp directory
        if (source.IsIso)
        {
            return ProcessLevelsFromIso(source, gptFiles, maxDepth, showActual, showDynamic, showSector, showStats);
        }

        // For directory sources, process directly
        foreach (var gptFile in gptFiles)
        {
            var levelDir = Path.Combine(source.SourcePath, Path.GetDirectoryName(gptFile) ?? "");
            var levelName = Path.GetFileNameWithoutExtension(gptFile);

            Console.WriteLine($"=== {levelName} ===");

            try
            {
                ProcessLevelDirect(levelDir, levelName, maxDepth, showActual, showDynamic, showSector, showStats);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"  Error: {ex.Message}");
            }

            Console.WriteLine();
        }

        return 0;
    }

    private static int ProcessLevelsFromIso(IGameSource source, List<string> gptFiles, int? maxDepth, bool showActual, bool showDynamic, bool showSector, bool showStats)
    {
        var tempDir = Path.Combine(Path.GetTempPath(), "astrolabe-tree-" + Guid.NewGuid().ToString("N")[..8]);

        try
        {
            Directory.CreateDirectory(tempDir);

            foreach (var gptFile in gptFiles)
            {
                var levelDir = Path.GetDirectoryName(gptFile) ?? "";
                var levelName = Path.GetFileNameWithoutExtension(gptFile);

                Console.WriteLine($"=== {levelName} ===");

                try
                {
                    // Extract required level files to temp
                    var levelTempDir = Path.Combine(tempDir, levelName);
                    Directory.CreateDirectory(levelTempDir);

                    string[] extensions = [".sna", ".gpt", ".rtb", ".rtp", ".rtt", ".ptx"];
                    foreach (var ext in extensions)
                    {
                        var filePath = Path.Combine(levelDir, levelName + ext);
                        if (source.FileExists(filePath))
                        {
                            using var srcStream = source.OpenFile(filePath);
                            var destPath = Path.Combine(levelTempDir, levelName + ext);
                            using var destStream = File.Create(destPath);
                            srcStream.CopyTo(destStream);
                        }
                    }

                    ProcessLevelDirect(levelTempDir, levelName, maxDepth, showActual, showDynamic, showSector, showStats);
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine($"  Error: {ex.Message}");
                }

                Console.WriteLine();
            }
        }
        finally
        {
            // Cleanup temp directory
            try { Directory.Delete(tempDir, true); } catch { }
        }

        return 0;
    }

    private static void ProcessLevelDirect(string levelDir, string levelName, int? maxDepth, bool showActual, bool showDynamic, bool showSector, bool showStats)
    {
        var loader = new LevelLoader(levelDir, levelName);

        var gptPath = Path.Combine(levelDir, $"{levelName}.gpt");
        if (!File.Exists(gptPath))
        {
            gptPath = Directory.GetFiles(levelDir, $"{levelName}.gpt*").FirstOrDefault() ?? "";
        }

        var gpt = new GptReader(gptPath);
        var memory = new MemoryContext(loader.Sna, loader.Rtb);
        var sceneReader = new SuperObjectReader(memory);
        var sceneGraph = sceneReader.ReadSceneGraph(gpt);

        if (showStats)
        {
            PrintStats(sceneGraph);
            Console.WriteLine();
        }

        if (showActual && sceneGraph.ActualWorld != null)
        {
            Console.WriteLine("ActualWorld");
            PrintTree(sceneGraph.ActualWorld, "", true, maxDepth, 0);
        }

        if (showDynamic && sceneGraph.DynamicWorld != null)
        {
            if (showActual) Console.WriteLine();
            Console.WriteLine("DynamicWorld");
            PrintTree(sceneGraph.DynamicWorld, "", true, maxDepth, 0);
        }

        if (showSector && sceneGraph.FatherSector != null)
        {
            if (showActual || showDynamic) Console.WriteLine();
            Console.WriteLine("FatherSector");
            PrintTree(sceneGraph.FatherSector, "", true, maxDepth, 0);
        }
    }

    private static void PrintStats(SceneGraph graph)
    {
        Console.WriteLine($"Total nodes: {graph.AllNodes.Count}");

        var typeGroups = graph.AllNodes
            .GroupBy(n => n.Type)
            .OrderByDescending(g => g.Count());

        foreach (var group in typeGroups)
        {
            Console.WriteLine($"  {group.Key,-20} {group.Count(),5}");
        }

        var geoCount = graph.GetGeometryNodes().Count();
        Console.WriteLine($"  {"(with geometry)",-20} {geoCount,5}");
    }

    private static void PrintTree(SceneNode node, string prefix, bool isLast, int? maxDepth, int depth)
    {
        // Build node label
        var label = FormatNode(node);

        // Print this node
        var connector = isLast ? "└── " : "├── ";
        Console.WriteLine($"{prefix}{connector}{label}");

        // Check depth limit
        if (maxDepth.HasValue && depth >= maxDepth.Value)
        {
            if (node.Children.Count > 0)
            {
                var childPrefix = prefix + (isLast ? "    " : "│   ");
                Console.WriteLine($"{childPrefix}└── ({node.Children.Count} children...)");
            }
            return;
        }

        // Print children
        var newPrefix = prefix + (isLast ? "    " : "│   ");
        for (int i = 0; i < node.Children.Count; i++)
        {
            var child = node.Children[i];
            var childIsLast = i == node.Children.Count - 1;
            PrintTree(child, newPrefix, childIsLast, maxDepth, depth + 1);
        }
    }

    private static string FormatNode(SceneNode node)
    {
        var parts = new List<string>();

        // Type comes first
        parts.Add(node.Type.ToString());

        // Add name if present (this is the primary identifier)
        if (!string.IsNullOrEmpty(node.Name))
        {
            parts.Add($"\"{node.Name}\"");
        }

        // Add geometry indicator for IPO nodes
        if (node.GeometricObjectAddress != 0)
        {
            parts.Add($"[geo]");
        }

        return string.Join(" ", parts);
    }
}
