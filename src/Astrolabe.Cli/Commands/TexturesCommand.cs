using Astrolabe.Core.FileFormats;

namespace Astrolabe.Cli.Commands;

public static class TexturesCommand
{
    public static int Run(string[] args)
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

            // Vignette.cnt contains images for direct display (BGR, no flip)
            // Textures.cnt contains GPU textures (RGB, flipped), except 640x480 images
            bool isVignetteCnt = Path.GetFileName(cntPath).Equals("Vignette.cnt", StringComparison.OrdinalIgnoreCase);

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

                    // Vignettes are either from Vignette.cnt or are 640x480 (full-screen images)
                    gf.IsVignette = isVignetteCnt || (gf.Width == 640 && gf.Height == 480);

                    var outputPath = Path.Combine(outputDir, Path.ChangeExtension(file.FullPath, ".png"));
                    var dir = Path.GetDirectoryName(outputPath);
                    if (!string.IsNullOrEmpty(dir))
                    {
                        Directory.CreateDirectory(dir);
                    }

                    gf.SaveAsPng(outputPath);
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
