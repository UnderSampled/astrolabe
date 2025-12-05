using Astrolabe.Core.FileFormats;

namespace Astrolabe.Cli.Commands;

public static class CntCommand
{
    public static int Run(string[] args)
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
}
