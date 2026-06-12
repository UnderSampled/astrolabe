using Astrolabe.Core.Extraction;

namespace Astrolabe.Cli.Commands;

public static class ListCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Source directory required");
            return 1;
        }

        var sourcePath = args[0];

        try
        {
            using var source = GameSourceFactory.Create(sourcePath);

            Console.WriteLine($"# Source: {source.SourcePath}");

            foreach (var file in source.ListFiles())
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
