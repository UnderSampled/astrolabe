using Astrolabe.Core.Extraction;

namespace Astrolabe.Cli.Commands;

public static class ExtractCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: ISO path required");
            return 1;
        }

        var isoPath = args[0];
        var outputDir = args.Length > 1 && !args[1].StartsWith("--")
            ? args[1]
            : "extracted";

        string? pattern = null;
        bool extractAll = false;

        // Parse options
        for (int i = 0; i < args.Length; i++)
        {
            if (args[i] == "--pattern" && i + 1 < args.Length)
            {
                pattern = args[i + 1];
                i++;
            }
            else if (args[i] == "--all" || args[i] == "-a")
            {
                extractAll = true;
            }
        }

        try
        {
            var extractor = new IsoExtractor(isoPath);

            var progress = new Progress<ExtractionProgress>(p =>
            {
                Console.Write($"\r[{p.ExtractedCount}/{p.TotalFiles}] {p.CurrentFile,-60}");
            });

            Console.WriteLine($"Extracting to: {outputDir}");

            if (pattern != null)
            {
                Console.WriteLine($"Pattern: {pattern}");
                extractor.ExtractPattern(pattern, outputDir, progress);
            }
            else if (extractAll)
            {
                extractor.ExtractAll(outputDir, progress);
            }
            else
            {
                // Default: extract only game data folders
                string[] defaultFolders = ["Gamedata", "LangData", "Sound"];
                Console.WriteLine($"Extracting: {string.Join(", ", defaultFolders)} (use --all for everything)");
                foreach (var folder in defaultFolders)
                {
                    extractor.ExtractPattern($"{folder}/", outputDir, progress);
                }
            }

            Console.WriteLine();
            Console.WriteLine("Extraction complete!");
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            return 1;
        }
    }
}
