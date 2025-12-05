using Astrolabe.Core.Extraction;

namespace Astrolabe.Cli.Commands;

public static class ListCommand
{
    public static int Run(string[] args)
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
}
