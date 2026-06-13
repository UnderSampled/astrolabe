using Astrolabe.Core.Rete;

namespace Astrolabe.Cli.Commands;

public static class DebugRelocationsCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Rete package directory path required");
            Console.Error.WriteLine("Usage: astrolabe debug-relocations <rete-dir> [--details]");
            return 1;
        }

        try
        {
            var packageDir = args[0];
            var showDetails = args.Any(a => a.Equals("--details", StringComparison.OrdinalIgnoreCase));
            if (Environment.GetEnvironmentVariable("ASTROLABE_DEBUG_RELOC") == "1")
            {
                var generated = OpenSpaceExporter.GenerateRtb(
                    packageDir,
                    "astrolabe.rtb",
                    [Path.Combine(Path.GetDirectoryName(Path.GetFullPath(packageDir))!, "fix")]);
                var total = generated.Blocks.Sum(b => b.Pointers.Count);
                foreach (var block in generated.Blocks)
                {
                    Console.WriteLine($"debug block {block.Key}: {block.Pointers.Count}");
                }

                Console.WriteLine($"debug generated total: {total}");
            }

            var results = OpenSpaceExporter.CompareGeneratedRelocations(packageDir);
            foreach (var result in results)
            {
                var status = result.Supported ? "generated" : "unsupported";
                Console.WriteLine($"{result.FileName}: {status}");
                Console.WriteLine($"  preserved: {result.PreservedPointerCount}");
                Console.WriteLine($"  generated: {result.GeneratedPointerCount}");
                Console.WriteLine($"  matching:  {result.MatchingPointerCount}");
                Console.WriteLine($"  missing:   {result.MissingPointerCount}");
                Console.WriteLine($"  extra:     {result.ExtraPointerCount}");
                Console.WriteLine($"  pointer data: {(result.PointerDataMatches ? "match" : "diff")}");
                if (!string.IsNullOrWhiteSpace(result.Note))
                {
                    Console.WriteLine($"  note:      {result.Note}");
                }

                if (showDetails && result.MissingSamples.Count > 0)
                {
                    Console.WriteLine("  missing samples:");
                    foreach (var sample in result.MissingSamples)
                    {
                        Console.WriteLine($"    {sample}");
                    }
                }

                if (showDetails && result.ExtraSamples.Count > 0)
                {
                    Console.WriteLine("  extra samples:");
                    foreach (var sample in result.ExtraSamples)
                    {
                        Console.WriteLine($"    {sample}");
                    }
                }
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
