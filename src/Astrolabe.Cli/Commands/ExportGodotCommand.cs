using Astrolabe.Core;

namespace Astrolabe.Cli.Commands;

public static class ExportGodotCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: OpenSpace level or Rete package path required");
            Console.Error.WriteLine("Usage: astrolabe export-godot <openspace-dir | rete-dir> [godot-dir]");
            return 1;
        }

        var inputDir = args[0];
        var inputName = Path.GetFileName(inputDir.TrimEnd('/', '\\'));
        var outputDir = args.Length > 1 ? args[1] : $"output/{inputName}";

        try
        {
            var level = Level.Load(inputDir);
            Console.WriteLine($"Loading level: {level.Name} ({level.SourceKind})");
            Console.WriteLine($"Loaded {level.Loader.Sna.Blocks.Count} SNA blocks");

            if (level.TextureTable != null)
            {
                Console.WriteLine($"Loaded {level.TextureTable.TextureNames.Count} texture references from PTX");
            }

            Console.WriteLine($"Found {level.SceneGraph.AllNodes.Count} scene nodes");

            Console.WriteLine("Exporting Godot project...");
            var result = level.ExportToGodot(outputDir);
            Console.WriteLine($"Found {result.ValidMeshCount} valid meshes");
            Console.WriteLine($"Exported {result.ExportedMeshCount} meshes");

            Console.WriteLine($"\nExported to: {outputDir}");
            Console.WriteLine($"  Project: project.godot");
            Console.WriteLine($"  Scene: {result.SceneFileName}");
            Console.WriteLine();
            Console.WriteLine("To open in Godot:");
            Console.WriteLine($"  godot --editor --path \"{outputDir}\"");
            Console.WriteLine("(First run will import referenced textures, which may take a moment)");

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            Console.Error.WriteLine(ex.StackTrace);
            return 1;
        }
    }
}