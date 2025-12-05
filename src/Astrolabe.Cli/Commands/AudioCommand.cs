using Astrolabe.Core.FileFormats.Audio;

namespace Astrolabe.Cli.Commands;

public static class AudioCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Audio file path required");
            Console.Error.WriteLine("Usage: astrolabe audio <apm-path> [output-path]");
            Console.Error.WriteLine("       astrolabe audio <bnm-path> [output-dir]   # Extract BNM sound bank");
            Console.Error.WriteLine("       astrolabe audio <directory> [output-dir]  # Convert all APM/BNM files");
            return 1;
        }

        var inputPath = args[0];
        var outputPath = args.Length > 1 ? args[1] : null;

        try
        {
            // Check if input is a directory
            if (Directory.Exists(inputPath))
            {
                return ConvertAllAudio(inputPath, outputPath);
            }

            // Single file conversion
            if (!File.Exists(inputPath))
            {
                Console.Error.WriteLine($"Error: File not found: {inputPath}");
                return 1;
            }

            // Check file type
            var ext = Path.GetExtension(inputPath).ToLowerInvariant();
            if (ext == ".bnm")
            {
                return ExtractBnm(inputPath, outputPath);
            }

            // APM file
            outputPath ??= Path.ChangeExtension(inputPath, ".wav");

            Console.WriteLine($"Converting: {inputPath}");
            WavWriter.ConvertApmToWav(inputPath, outputPath);
            Console.WriteLine($"Output: {outputPath}");

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            return 1;
        }
    }

    private static int ExtractBnm(string bnmPath, string? outputDir)
    {
        var bnmName = Path.GetFileNameWithoutExtension(bnmPath);
        outputDir ??= Path.Combine(Path.GetDirectoryName(bnmPath) ?? ".", bnmName);

        Console.WriteLine($"Reading BNM: {bnmPath}");
        var bnm = new BnmReader(bnmPath);
        Console.WriteLine($"Version: 0x{bnm.Version:X8}");
        Console.WriteLine($"Entries: {bnm.Entries.Count}");

        if (bnm.Entries.Count == 0)
        {
            Console.WriteLine("No audio entries found.");
            return 0;
        }

        Console.WriteLine($"Extracting to: {outputDir}");
        int extracted = bnm.ExtractAll(outputDir, verbose: true);
        Console.WriteLine($"Extracted: {extracted}/{bnm.Entries.Count}");

        return extracted > 0 ? 0 : 1;
    }

    private static int ConvertAllAudio(string inputDir, string? outputDir)
    {
        outputDir ??= inputDir;

        var apmFiles = Directory.GetFiles(inputDir, "*.apm", SearchOption.AllDirectories);
        var bnmFiles = Directory.GetFiles(inputDir, "*.bnm", SearchOption.AllDirectories);

        if (apmFiles.Length == 0 && bnmFiles.Length == 0)
        {
            Console.WriteLine("No APM or BNM files found.");
            return 0;
        }

        int converted = 0;
        int failed = 0;

        // Convert APM files
        if (apmFiles.Length > 0)
        {
            Console.WriteLine($"Found {apmFiles.Length} APM files");
            foreach (var apmPath in apmFiles)
            {
                var relativePath = Path.GetRelativePath(inputDir, apmPath);
                var wavPath = Path.Combine(outputDir, Path.ChangeExtension(relativePath, ".wav"));

                var wavDir = Path.GetDirectoryName(wavPath);
                if (wavDir != null && !Directory.Exists(wavDir))
                {
                    Directory.CreateDirectory(wavDir);
                }

                try
                {
                    WavWriter.ConvertApmToWav(apmPath, wavPath);
                    Console.WriteLine($"  {relativePath} -> {Path.GetFileName(wavPath)}");
                    converted++;
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine($"  {relativePath}: FAILED - {ex.Message}");
                    failed++;
                }
            }
        }

        // Extract BNM files
        if (bnmFiles.Length > 0)
        {
            Console.WriteLine($"Found {bnmFiles.Length} BNM files");
            foreach (var bnmPath in bnmFiles)
            {
                var relativePath = Path.GetRelativePath(inputDir, bnmPath);
                var bnmName = Path.GetFileNameWithoutExtension(bnmPath);
                var bnmOutputDir = Path.Combine(outputDir, Path.GetDirectoryName(relativePath) ?? "", bnmName);

                try
                {
                    var bnm = new BnmReader(bnmPath);
                    int extracted = bnm.ExtractAll(bnmOutputDir);
                    Console.WriteLine($"  {relativePath} -> {bnmName}/ ({extracted} files)");
                    converted += extracted;
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine($"  {relativePath}: FAILED - {ex.Message}");
                    failed++;
                }
            }
        }

        Console.WriteLine($"\nConverted: {converted}, Failed: {failed}");
        return failed > 0 ? 1 : 0;
    }
}
