using Astrolabe.Core.Intermediate;

namespace Astrolabe.Cli.Commands;

public static class ExtractIntermediateCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Level directory path required");
            Console.Error.WriteLine("Usage: astrolabe extract-intermediate <level-dir> [output-dir]");
            return 1;
        }

        var levelDir = args[0];
        var levelName = Path.GetFileName(levelDir.TrimEnd('/', '\\'));
        var outputDir = args.Length > 1
            ? args[1]
            : Path.Combine("output", "intermediate", levelName);

        try
        {
            var manifest = LevelIntermediateCodec.ExtractLevel(levelDir, outputDir);

            Console.WriteLine($"Extracted intermediate level: {manifest.LevelName}");
            Console.WriteLine($"Output: {outputDir}");
            Console.WriteLine($"SNA files: {manifest.SnaFiles.Count}");
            Console.WriteLine($"SNA blocks: {manifest.SnaFiles.Sum(f => f.Blocks.Count)}");
            Console.WriteLine($"Relocation tables: {manifest.RelocationTables.Count}");
            Console.WriteLine($"Loose files: {manifest.LooseFiles.Count}");

            if (manifest.Semantic?.Errors.Count > 0)
            {
                Console.WriteLine("Semantic metadata warnings:");
                foreach (var error in manifest.Semantic.Errors)
                {
                    Console.WriteLine($"  {error}");
                }
            }

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            return 1;
        }
    }
}
