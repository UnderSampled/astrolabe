using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Audio;

namespace Astrolabe.Cli.Commands;

/// <summary>
/// Extracts all game assets to a well-organized output folder structure.
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

        // 1. Extract Textures.cnt -> output/Textures/
        var texturesCnt = Path.Combine(extractedDir, "Gamedata", "Textures.cnt");
        if (File.Exists(texturesCnt))
        {
            Console.WriteLine("=== Extracting Textures.cnt ===");
            var (extracted, failed) = ExtractCnt(texturesCnt, Path.Combine(outputDir, "Textures"));
            totalExtracted += extracted;
            totalFailed += failed;
            Console.WriteLine();
        }

        // 2. Extract Vignette.cnt -> output/Vignette/
        var vignetteCnt = Path.Combine(extractedDir, "Gamedata", "Vignette.cnt");
        if (File.Exists(vignetteCnt))
        {
            Console.WriteLine("=== Extracting Vignette.cnt ===");
            var (extracted, failed) = ExtractCnt(vignetteCnt, Path.Combine(outputDir, "Vignette"));
            totalExtracted += extracted;
            totalFailed += failed;
            Console.WriteLine();
        }

        // 3. Extract all BNM sound banks -> output/Sound/<bankname>/
        var soundDir = Path.Combine(extractedDir, "Gamedata", "World", "Sound");
        if (Directory.Exists(soundDir))
        {
            Console.WriteLine("=== Extracting BNM Sound Banks ===");
            var bnmFiles = Directory.GetFiles(soundDir, "*.bnm");
            Console.WriteLine($"Found {bnmFiles.Length} BNM files");

            foreach (var bnmPath in bnmFiles.OrderBy(f => f))
            {
                var bankName = Path.GetFileNameWithoutExtension(bnmPath);
                var bankOutputDir = Path.Combine(outputDir, "Sound", bankName);

                try
                {
                    var bnm = new BnmReader(bnmPath);
                    if (bnm.Entries.Count > 0)
                    {
                        int extracted = bnm.ExtractAll(bankOutputDir);
                        Console.WriteLine($"  {bankName}: {extracted} files");
                        totalExtracted += extracted;
                    }
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine($"  {bankName}: FAILED - {ex.Message}");
                    totalFailed++;
                }
            }
            Console.WriteLine();
        }

        // 4. Convert APM ambient audio -> output/Sound/Ambient/
        if (Directory.Exists(soundDir))
        {
            Console.WriteLine("=== Converting APM Audio ===");
            var apmFiles = Directory.GetFiles(soundDir, "*.apm");
            Console.WriteLine($"Found {apmFiles.Length} APM files");

            var ambientDir = Path.Combine(outputDir, "Sound", "Ambient");
            Directory.CreateDirectory(ambientDir);

            foreach (var apmPath in apmFiles.OrderBy(f => f))
            {
                var apmName = Path.GetFileNameWithoutExtension(apmPath);
                var wavPath = Path.Combine(ambientDir, $"{apmName}.wav");

                try
                {
                    WavWriter.ConvertApmToWav(apmPath, wavPath);
                    Console.WriteLine($"  {apmName}.apm -> {apmName}.wav");
                    totalExtracted++;
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine($"  {apmName}: FAILED - {ex.Message}");
                    totalFailed++;
                }
            }
            Console.WriteLine();
        }

        // 5. Convert music CSB files -> output/Music/
        var musicDir = Path.Combine(extractedDir, "Sound");
        if (Directory.Exists(musicDir))
        {
            Console.WriteLine("=== Converting Music (CSB/APM) ===");

            // CSB files contain references, but there may be APM files alongside
            var musicApmFiles = Directory.GetFiles(musicDir, "*.apm", SearchOption.AllDirectories);
            if (musicApmFiles.Length > 0)
            {
                Console.WriteLine($"Found {musicApmFiles.Length} music APM files");
                var musicOutputDir = Path.Combine(outputDir, "Music");
                Directory.CreateDirectory(musicOutputDir);

                foreach (var apmPath in musicApmFiles.OrderBy(f => f))
                {
                    var apmName = Path.GetFileNameWithoutExtension(apmPath);
                    var wavPath = Path.Combine(musicOutputDir, $"{apmName}.wav");

                    try
                    {
                        WavWriter.ConvertApmToWav(apmPath, wavPath);
                        Console.WriteLine($"  {apmName}.apm -> {apmName}.wav");
                        totalExtracted++;
                    }
                    catch (Exception ex)
                    {
                        Console.Error.WriteLine($"  {apmName}: FAILED - {ex.Message}");
                        totalFailed++;
                    }
                }
            }
            else
            {
                Console.WriteLine("  No standalone APM music files found (music may be in CSB containers)");
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
        Console.WriteLine($"Output structure:");
        PrintOutputStructure(outputDir);

        return totalFailed > 0 ? 1 : 0;
    }

    private static (int extracted, int failed) ExtractCnt(string cntPath, string outputDir)
    {
        int extracted = 0;
        int failed = 0;

        try
        {
            var cnt = new CntReader(cntPath);
            Console.WriteLine($"Found {cnt.FileCount} files");
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

            Console.WriteLine($"Extracted: {extracted}, Failed: {failed}");
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error reading CNT: {ex.Message}");
            failed++;
        }

        return (extracted, failed);
    }

    private static void PrintOutputStructure(string outputDir)
    {
        if (!Directory.Exists(outputDir))
        {
            Console.WriteLine("  (output directory not created)");
            return;
        }

        foreach (var dir in Directory.GetDirectories(outputDir).OrderBy(d => d))
        {
            var dirName = Path.GetFileName(dir);
            var fileCount = Directory.GetFiles(dir, "*", SearchOption.AllDirectories).Length;
            var subDirCount = Directory.GetDirectories(dir, "*", SearchOption.AllDirectories).Length;

            if (subDirCount > 0)
            {
                Console.WriteLine($"  {dirName}/ ({fileCount} files in {subDirCount + 1} folders)");
            }
            else
            {
                Console.WriteLine($"  {dirName}/ ({fileCount} files)");
            }
        }
    }
}
