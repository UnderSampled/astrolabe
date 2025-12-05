using Astrolabe.Core.FileFormats;

namespace Astrolabe.Cli.Commands;

public static class SceneCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Level directory path required");
            Console.Error.WriteLine("Usage: astrolabe scene <level-dir> [level-name]");
            return 1;
        }

        var levelDir = args[0];
        var levelName = args.Length > 1 ? args[1] : Path.GetFileName(levelDir.TrimEnd('/', '\\'));

        try
        {
            Console.WriteLine($"Loading level: {levelName}");
            var loader = new LevelLoader(levelDir, levelName);
            Console.WriteLine($"Loaded {loader.Sna.Blocks.Count} SNA blocks");

            // Find GPT file
            var gptPath = Path.Combine(levelDir, $"{levelName}.gpt");
            if (!File.Exists(gptPath))
            {
                gptPath = Directory.GetFiles(levelDir, $"{levelName}.gpt*").FirstOrDefault() ?? "";
            }

            if (!File.Exists(gptPath))
            {
                Console.Error.WriteLine("GPT file not found");
                return 1;
            }

            Console.WriteLine($"Loading GPT from: {gptPath}");
            var gpt = new GptReader(gptPath);
            gpt.PrintDebugInfo(Console.Out);
            Console.WriteLine();

            // Create memory context for pointer resolution
            var memory = new MemoryContext(loader.Sna, loader.Rtb);

            // Read scene graph
            Console.WriteLine("Reading scene graph...");
            var sceneReader = new SuperObjectReader(memory);
            var sceneGraph = sceneReader.ReadSceneGraph(gpt);

            // Print statistics
            Console.WriteLine($"\nScene Graph Statistics:");
            Console.WriteLine($"  Total nodes: {sceneGraph.AllNodes.Count}");

            var typeGroups = sceneGraph.AllNodes.GroupBy(n => n.Type).OrderBy(g => g.Key);
            foreach (var group in typeGroups)
            {
                Console.WriteLine($"  {group.Key}: {group.Count()}");
            }

            var geoNodes = sceneGraph.GetGeometryNodes().ToList();
            Console.WriteLine($"\n  Nodes with geometry: {geoNodes.Count}");

            // Print hierarchy (limited depth)
            Console.WriteLine("\nScene Hierarchy (ActualWorld):");
            if (sceneGraph.ActualWorld != null)
            {
                PrintHierarchy(sceneGraph.ActualWorld, 0, 3);
            }
            else
            {
                Console.WriteLine("  (no ActualWorld found)");
            }

            if (sceneGraph.DynamicWorld != null)
            {
                Console.WriteLine("\nDynamic World:");
                PrintHierarchy(sceneGraph.DynamicWorld, 0, 2);
            }

            if (sceneGraph.FatherSector != null)
            {
                Console.WriteLine("\nFather Sector:");
                PrintHierarchy(sceneGraph.FatherSector, 0, 2);
            }

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            Console.Error.WriteLine(ex.StackTrace);
            return 1;
        }
    }

    private static void PrintHierarchy(SceneNode node, int indent, int maxDepth)
    {
        if (indent > maxDepth) return;

        var prefix = new string(' ', indent * 2);
        string geoInfo = node.GeometricObjectAddress != 0 ? $" [geo@0x{node.GeometricObjectAddress:X8}]" : "";
        Console.WriteLine($"{prefix}{node.Type} @ 0x{node.Address:X8}{geoInfo}");

        if (indent < maxDepth)
        {
            foreach (var child in node.Children.Take(10))
            {
                PrintHierarchy(child, indent + 1, maxDepth);
            }
            if (node.Children.Count > 10)
            {
                Console.WriteLine($"{prefix}  ... and {node.Children.Count - 10} more children");
            }
        }
        else if (node.Children.Count > 0)
        {
            Console.WriteLine($"{prefix}  ({node.Children.Count} children)");
        }
    }
}
