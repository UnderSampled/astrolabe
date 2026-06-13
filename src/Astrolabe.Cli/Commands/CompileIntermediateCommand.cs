using Astrolabe.Core.Intermediate;

namespace Astrolabe.Cli.Commands;

public static class CompileIntermediateCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Intermediate directory path required");
            Console.Error.WriteLine("Usage: astrolabe compile-intermediate <intermediate-dir> [output-level-dir]");
            return 1;
        }

        var intermediateDir = args[0];
        var outputDir = args.Length > 1
            ? args[1]
            : Path.Combine("output", "compiled", Path.GetFileName(intermediateDir.TrimEnd('/', '\\')));

        try
        {
            LevelIntermediateCodec.CompileLevel(intermediateDir, outputDir);
            Console.WriteLine($"Compiled intermediate level: {intermediateDir}");
            Console.WriteLine($"Output: {outputDir}");
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            return 1;
        }
    }
}
