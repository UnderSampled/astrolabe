using Astrolabe.Core.Rete;

namespace Astrolabe.Cli.Commands;

public static class CompileIntermediateCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Rete package directory path required");
            Console.Error.WriteLine("Usage: astrolabe compile-intermediate <rete-dir> [output-level-dir]");
            return 1;
        }

        var packageDir = args[0];
        var outputDir = args.Length > 1
            ? args[1]
            : Path.Combine("output", "compiled", Path.GetFileName(packageDir.TrimEnd('/', '\\')));

        try
        {
            OpenSpaceExporter.ExportLevel(packageDir, outputDir);
            Console.WriteLine($"Exported OpenSpace level from Rete package: {packageDir}");
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