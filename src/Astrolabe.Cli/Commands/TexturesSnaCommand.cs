using Astrolabe.Core.FileFormats;

namespace Astrolabe.Cli.Commands;

public static class TexturesSnaCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Level directory path required");
            Console.Error.WriteLine("Usage: astrolabe textures-sna <level-dir> [level-name]");
            return 1;
        }

        var levelDir = args[0];
        var levelName = args.Length > 1 ? args[1] : Path.GetFileName(levelDir.TrimEnd('/', '\\'));

        try
        {
            Console.WriteLine($"Loading level: {levelName}");
            var loader = new LevelLoader(levelDir, levelName);
            Console.WriteLine($"Loaded {loader.Sna.Blocks.Count} SNA blocks");

            // Find PTX file
            var ptxPath = Path.Combine(levelDir, $"{levelName}.ptx");
            if (!File.Exists(ptxPath))
            {
                ptxPath = Directory.GetFiles(levelDir, $"{levelName}.ptx*").FirstOrDefault();
            }

            if (ptxPath == null || !File.Exists(ptxPath))
            {
                Console.WriteLine("PTX file not found");
                return 1;
            }

            Console.WriteLine($"Loading PTX from: {ptxPath}");
            var textureTable = new TextureTable(loader, ptxPath);

            Console.WriteLine($"\nFound {textureTable.TextureNames.Count} texture names:");
            foreach (var (addr, name) in textureTable.TextureNames.OrderBy(kv => kv.Key))
            {
                Console.WriteLine($"  0x{addr:X8}: {name}");
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
}
