using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Audio;

namespace Astrolabe.Cli.Commands;

/// <summary>
/// Extracts all game assets, mirroring the ISO folder structure.
/// Containers (CNT, BNM) become folders with their extracted contents.
/// Audio files (APM) are converted to WAV in place.
/// </summary>
public static class ExtractAllCommand
{
    public static int Run(string[] args)
    {
        var extractedDir = args.Length > 0 ? args[0] : "extracted";
        var outputDir = args.Length > 1 ? args[1] : "output";

        if (!Directory.Exists(extractedDir))
        {
            Console.Error.WriteLine($"Error: Extracted directory not found: {extractedDir}");
            Console.Error.WriteLine("Run 'astrolabe extract <iso-path>' first to extract the ISO.");
            return 1;
        }

        Console.WriteLine($"Source: {extractedDir}");
        Console.WriteLine($"Output: {outputDir}");
        Console.WriteLine();

        int totalExtracted = 0;
        int totalFailed = 0;

        // Find and process texture CNT files (Textures.cnt, Vignette.cnt)
        // Note: fix.cnt and other CNT files are not texture containers
        var cntFiles = Directory.GetFiles(extractedDir, "*.cnt", SearchOption.AllDirectories)
            .Where(f => {
                var name = Path.GetFileName(f).ToLowerInvariant();
                return name == "textures.cnt" || name == "vignette.cnt";
            })
            .ToArray();
        if (cntFiles.Length > 0)
        {
            Console.WriteLine($"=== Extracting {cntFiles.Length} CNT texture containers ===");
            foreach (var cntPath in cntFiles.OrderBy(f => f))
            {
                var relativePath = Path.GetRelativePath(extractedDir, cntPath);
                var outputPath = Path.Combine(outputDir, Path.ChangeExtension(relativePath, null)); // Remove .cnt extension

                Console.WriteLine($"  {relativePath}");
                var (extracted, failed) = ExtractCnt(cntPath, outputPath);
                totalExtracted += extracted;
                totalFailed += failed;
            }
            Console.WriteLine();
        }

        // Find and process all BNM files
        var bnmFiles = Directory.GetFiles(extractedDir, "*.bnm", SearchOption.AllDirectories);
        if (bnmFiles.Length > 0)
        {
            Console.WriteLine($"=== Extracting {bnmFiles.Length} BNM sound banks ===");
            foreach (var bnmPath in bnmFiles.OrderBy(f => f))
            {
                var relativePath = Path.GetRelativePath(extractedDir, bnmPath);
                var outputPath = Path.Combine(outputDir, Path.ChangeExtension(relativePath, null)); // Remove .bnm extension

                try
                {
                    var bnm = new BnmReader(bnmPath);
                    if (bnm.Entries.Count > 0)
                    {
                        int extracted = bnm.ExtractAll(outputPath);
                        Console.WriteLine($"  {relativePath} -> {extracted} files");
                        totalExtracted += extracted;
                    }
                    else
                    {
                        Console.WriteLine($"  {relativePath} -> (empty)");
                    }
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine($"  {relativePath} -> FAILED: {ex.Message}");
                    totalFailed++;
                }
            }
            Console.WriteLine();
        }

        // Find and process all APM files
        var apmFiles = Directory.GetFiles(extractedDir, "*.apm", SearchOption.AllDirectories);
        if (apmFiles.Length > 0)
        {
            Console.WriteLine($"=== Converting {apmFiles.Length} APM audio files ===");
            foreach (var apmPath in apmFiles.OrderBy(f => f))
            {
                var relativePath = Path.GetRelativePath(extractedDir, apmPath);
                var wavRelativePath = Path.ChangeExtension(relativePath, ".wav");
                var wavPath = Path.Combine(outputDir, wavRelativePath);

                var wavDir = Path.GetDirectoryName(wavPath);
                if (!string.IsNullOrEmpty(wavDir))
                {
                    Directory.CreateDirectory(wavDir);
                }

                try
                {
                    WavWriter.ConvertApmToWav(apmPath, wavPath);
                    Console.WriteLine($"  {relativePath} -> .wav");
                    totalExtracted++;
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine($"  {relativePath} -> FAILED: {ex.Message}");
                    totalFailed++;
                }
            }
            Console.WriteLine();
        }

        // Summary
        Console.WriteLine("=== Summary ===");
        Console.WriteLine($"Total extracted: {totalExtracted}");
        if (totalFailed > 0)
        {
            Console.WriteLine($"Total failed: {totalFailed}");
        }
        Console.WriteLine();
        Console.WriteLine("Output mirrors ISO structure with containers expanded:");
        Console.WriteLine("  *.cnt -> folder/ (PNG textures)");
        Console.WriteLine("  *.bnm -> folder/ (WAV audio)");
        Console.WriteLine("  *.apm -> *.wav");

        return totalFailed > 0 ? 1 : 0;
    }

    private static (int extracted, int failed) ExtractCnt(string cntPath, string outputDir)
    {
        int extracted = 0;
        int failed = 0;

        try
        {
            var cnt = new CntReader(cntPath);
            Directory.CreateDirectory(outputDir);

            foreach (var file in cnt.Files)
            {
                try
                {
                    var data = cnt.ExtractFile(file);
                    var gf = new GfReader(data);

                    var outputPath = Path.Combine(outputDir, Path.ChangeExtension(file.FullPath, ".png"));
                    var dir = Path.GetDirectoryName(outputPath);
                    if (!string.IsNullOrEmpty(dir))
                    {
                        Directory.CreateDirectory(dir);
                    }

                    gf.SaveAsPng(outputPath);
                    extracted++;
                }
                catch
                {
                    failed++;
                }
            }

            Console.WriteLine($"    -> {extracted} textures" + (failed > 0 ? $" ({failed} failed)" : ""));
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"    -> FAILED: {ex.Message}");
            failed++;
        }

        return (extracted, failed);
    }
}
