using Astrolabe.Core.Rete;

namespace Astrolabe.Cli.Commands;

public static class DebugPointerProbeCommand
{
    public static int Run(string[] args)
    {
        if (args.Length < 3)
        {
            Console.Error.WriteLine("Usage: astrolabe debug-pointer-probe <rete-dir> <kind> <relative-data-path>");
            return 1;
        }

        try
        {
            var bytes = OpenSpaceExporter.PreviewStructuredElementBytes(args[0], args[1], args[2]);
            Console.WriteLine($"Length: {bytes.Length}");
            Console.WriteLine(Convert.ToHexString(bytes).ToLowerInvariant());
            for (var offset = 0; offset <= bytes.Length - 4; offset += 4)
            {
                var value = BitConverter.ToInt32(bytes, offset);
                var target = OpenSpaceExporter.FindTargetBlockKey(args[0], value);
                Console.WriteLine($"  0x{offset:X2}: 0x{unchecked((uint)value):X8} -> {target ?? "none"}");
            }

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex}");
            return 1;
        }
    }
}