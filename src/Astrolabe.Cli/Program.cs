using Astrolabe.Core.Extraction;
using Astrolabe.Core.FileFormats;

namespace Astrolabe.Cli;

class Program
{
    static int Main(string[] args)
    {
        if (args.Length == 0)
        {
            PrintUsage();
            return 1;
        }

        var command = args[0].ToLowerInvariant();

        switch (command)
        {
            case "extract":
                return RunExtract(args[1..]);
            case "list":
                return RunList(args[1..]);
            case "textures":
                return RunTextures(args[1..]);
            case "cnt":
                return RunCnt(args[1..]);
            case "help":
            case "--help":
            case "-h":
                PrintUsage();
                return 0;
            default:
                Console.Error.WriteLine($"Unknown command: {command}");
                PrintUsage();
                return 1;
        }
    }

    static void PrintUsage()
    {
        Console.WriteLine("""
            Astrolabe - Hype: The Time Quest Asset Extractor

            Usage:
                astrolabe <command> [options]

            Commands:
                extract <iso-path> [output-dir]    Extract files from ISO
                list <iso-path>                    List files in ISO
                textures <cnt-path> [output-dir]   Extract textures from CNT container
                cnt <cnt-path>                     List files in CNT container
                help                               Show this help message

            Options for 'extract':
                --pattern <pattern>    Only extract files matching pattern (e.g., "*.lvl")

            Examples:
                astrolabe list hype.iso
                astrolabe extract hype.iso ./extracted
                astrolabe extract hype.iso ./extracted --pattern "Gamedata/"
                astrolabe textures ./extracted/Gamedata/Textures.cnt ./textures
            """);
    }

    static int RunList(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: ISO path required");
            return 1;
        }

        var isoPath = args[0];

        try
        {
            var extractor = new IsoExtractor(isoPath);
            var files = extractor.ListFiles();

            foreach (var file in files)
            {
                Console.WriteLine(file);
            }

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            return 1;
        }
    }

    static int RunExtract(string[] args)
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

        // Parse options
        for (int i = 0; i < args.Length; i++)
        {
            if (args[i] == "--pattern" && i + 1 < args.Length)
            {
                pattern = args[i + 1];
                i++;
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
            else
            {
                extractor.ExtractAll(outputDir, progress);
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

    static int RunCnt(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: CNT file path required");
            return 1;
        }

        var cntPath = args[0];

        try
        {
            var cnt = new CntReader(cntPath);

            Console.WriteLine($"CNT Container: {Path.GetFileName(cntPath)}");
            Console.WriteLine($"Directories: {cnt.DirectoryCount}");
            Console.WriteLine($"Files: {cnt.FileCount}");
            Console.WriteLine($"XOR Encrypted: {cnt.IsXorEncrypted}");
            Console.WriteLine($"Has Checksum: {cnt.HasChecksum}");
            Console.WriteLine();

            Console.WriteLine("Directories:");
            for (int i = 0; i < Math.Min(cnt.Directories.Length, 20); i++)
            {
                Console.WriteLine($"  [{i}] {cnt.Directories[i]}");
            }
            if (cnt.Directories.Length > 20)
            {
                Console.WriteLine($"  ... and {cnt.Directories.Length - 20} more");
            }

            Console.WriteLine();
            Console.WriteLine("Files (first 20):");
            for (int i = 0; i < Math.Min(cnt.Files.Length, 20); i++)
            {
                var f = cnt.Files[i];
                Console.WriteLine($"  {f.FullPath} ({f.FileSize} bytes)");
            }
            if (cnt.Files.Length > 20)
            {
                Console.WriteLine($"  ... and {cnt.Files.Length - 20} more");
            }

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            return 1;
        }
    }

    static int RunTextures(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: CNT file path required");
            return 1;
        }

        var cntPath = args[0];
        var outputDir = args.Length > 1 ? args[1] : "textures";

        try
        {
            var cnt = new CntReader(cntPath);

            Console.WriteLine($"Extracting {cnt.FileCount} textures from {Path.GetFileName(cntPath)}...");
            Directory.CreateDirectory(outputDir);

            int extracted = 0;
            int failed = 0;

            foreach (var file in cnt.Files)
            {
                try
                {
                    var data = cnt.ExtractFile(file);
                    var gf = new GfReader(data);

                    var outputPath = Path.Combine(outputDir, Path.ChangeExtension(file.FullPath, ".tga"));
                    var dir = Path.GetDirectoryName(outputPath);
                    if (!string.IsNullOrEmpty(dir))
                    {
                        Directory.CreateDirectory(dir);
                    }

                    gf.SaveAsTga(outputPath);
                    extracted++;

                    if (extracted % 100 == 0)
                    {
                        Console.Write($"\r[{extracted}/{cnt.FileCount}] Extracted...                    ");
                    }
                }
                catch
                {
                    failed++;
                }
            }

            Console.WriteLine();
            Console.WriteLine($"Extracted: {extracted} textures");
            if (failed > 0)
            {
                Console.WriteLine($"Failed: {failed} textures");
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
