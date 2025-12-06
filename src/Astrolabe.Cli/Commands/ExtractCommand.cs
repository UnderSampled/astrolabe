using Astrolabe.Core.Extraction;
using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Audio;

namespace Astrolabe.Cli.Commands;

/// <summary>
/// Extracts game assets from an ISO or directory.
/// By default, converts assets to usable formats (PNG, WAV).
/// With --raw, copies files without conversion.
/// </summary>
public static class ExtractCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Source path required (ISO file or directory)");
            Console.Error.WriteLine("Usage: astrolabe extract <source> [output] [options]");
            Console.Error.WriteLine();
            Console.Error.WriteLine("Options:");
            Console.Error.WriteLine("  --raw                  Copy raw files without conversion");
            Console.Error.WriteLine("  --all, -a              Include all files (default: only Gamedata, LangData, Sound)");
            Console.Error.WriteLine("  --pattern <pattern>    Only extract files matching pattern");
            return 1;
        }

        var sourcePath = args[0];
        var outputDir = "output";
        bool rawMode = false;
        bool extractAll = false;
        string? pattern = null;

        // Parse arguments
        for (int i = 1; i < args.Length; i++)
        {
            if (args[i] == "--raw" || args[i] == "-r")
            {
                rawMode = true;
            }
            else if (args[i] == "--all" || args[i] == "-a")
            {
                extractAll = true;
            }
            else if (args[i] == "--pattern" && i + 1 < args.Length)
            {
                pattern = args[++i];
            }
            else if (!args[i].StartsWith("-"))
            {
                outputDir = args[i];
            }
        }

        try
        {
            using var source = GameSourceFactory.Create(sourcePath);

            Console.WriteLine($"Source: {source.SourcePath} ({(source.IsIso ? "ISO" : "Directory")})");
            Console.WriteLine($"Output: {outputDir}");
            Console.WriteLine($"Mode: {(rawMode ? "raw (copy files as-is)" : "convert (PNG/WAV)")}");
            Console.WriteLine();

            if (rawMode)
            {
                return ExtractRaw(source, outputDir, extractAll, pattern);
            }
            else
            {
                return ExtractConverted(source, outputDir);
            }
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            return 1;
        }
    }

    private static int ExtractRaw(IGameSource source, string outputDir, bool extractAll, string? pattern)
    {
        var progress = new Progress<ExtractionProgress>(p =>
        {
            Console.Write($"\r[{p.ExtractedCount}/{p.TotalFiles}] {p.CurrentFile,-60}");
        });

        IEnumerable<string> filesToExtract;

        if (pattern != null)
        {
            Console.WriteLine($"Pattern: {pattern}");
            filesToExtract = source.GetFiles(pattern);
        }
        else if (extractAll)
        {
            Console.WriteLine("Extracting all files...");
            filesToExtract = source.ListFiles();
        }
        else
        {
            string[] defaultFolders = ["Gamedata/", "LangData/", "Sound/"];
            Console.WriteLine($"Extracting: {string.Join(", ", defaultFolders)} (use --all for everything)");
            filesToExtract = defaultFolders.SelectMany(folder => source.GetFiles(folder));
        }

        GameSourceFactory.ExtractFiles(source, outputDir, filesToExtract, progress);

        Console.WriteLine();
        Console.WriteLine("Extraction complete!");
        return 0;
    }

    private static int ExtractConverted(IGameSource source, string outputDir)
    {
        int totalExtracted = 0;
        int totalFailed = 0;

        // Process CNT files (texture containers)
        var cntFiles = source.GetFiles("*.cnt")
            .Where(f =>
            {
                var name = Path.GetFileName(f).ToLowerInvariant();
                return name == "textures.cnt" || name == "vignette.cnt";
            })
            .ToArray();

        if (cntFiles.Length > 0)
        {
            Console.WriteLine($"=== Extracting {cntFiles.Length} texture containers ===");
            foreach (var cntPath in cntFiles.OrderBy(f => f))
            {
                // Output to textures/ or vignette/ folder
                var name = Path.GetFileNameWithoutExtension(cntPath).ToLowerInvariant();
                var outputPath = Path.Combine(outputDir, name);

                Console.WriteLine($"  {cntPath}");
                var (extracted, failed) = ExtractCnt(source, cntPath, outputPath);
                totalExtracted += extracted;
                totalFailed += failed;
            }
            Console.WriteLine();
        }

        // Process BNM files (sound banks)
        var bnmFiles = source.GetFiles("*.bnm").ToArray();
        if (bnmFiles.Length > 0)
        {
            Console.WriteLine($"=== Extracting {bnmFiles.Length} sound banks ===");
            foreach (var bnmPath in bnmFiles.OrderBy(f => f))
            {
                var outputPath = Path.Combine(outputDir, Path.ChangeExtension(bnmPath, null));

                try
                {
                    using var stream = source.OpenFile(bnmPath);
                    var bnm = new BnmReader(stream);
                    if (bnm.Entries.Count > 0)
                    {
                        int extracted = bnm.ExtractAll(outputPath);
                        Console.WriteLine($"  {bnmPath} -> {extracted} files");
                        totalExtracted += extracted;
                    }
                    else
                    {
                        Console.WriteLine($"  {bnmPath} -> (empty)");
                    }
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine($"  {bnmPath} -> FAILED: {ex.Message}");
                    totalFailed++;
                }
            }
            Console.WriteLine();
        }

        // Process APM files (streaming audio)
        var apmFiles = source.GetFiles("*.apm").ToArray();
        if (apmFiles.Length > 0)
        {
            Console.WriteLine($"=== Converting {apmFiles.Length} audio files ===");
            foreach (var apmPath in apmFiles.OrderBy(f => f))
            {
                var wavRelativePath = Path.ChangeExtension(apmPath, ".wav");
                var wavPath = Path.Combine(outputDir, wavRelativePath);

                var wavDir = Path.GetDirectoryName(wavPath);
                if (!string.IsNullOrEmpty(wavDir))
                {
                    Directory.CreateDirectory(wavDir);
                }

                try
                {
                    using var stream = source.OpenFile(apmPath);
                    WavWriter.ConvertApmToWav(stream, wavPath);
                    Console.WriteLine($"  {apmPath} -> .wav");
                    totalExtracted++;
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine($"  {apmPath} -> FAILED: {ex.Message}");
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

        return totalFailed > 0 ? 1 : 0;
    }

    private static (int extracted, int failed) ExtractCnt(IGameSource source, string cntPath, string outputDir)
    {
        int extracted = 0;
        int failed = 0;

        try
        {
            using var stream = source.OpenFile(cntPath);
            var cnt = new CntReader(stream);
            Directory.CreateDirectory(outputDir);

            bool isVignette = Path.GetFileName(cntPath).Equals("Vignette.cnt", StringComparison.OrdinalIgnoreCase);

            foreach (var file in cnt.Files)
            {
                try
                {
                    var data = cnt.ExtractFile(file);
                    var gf = new GfReader(data);
                    gf.IsVignette = isVignette || (gf.Width == 640 && gf.Height == 480);

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
