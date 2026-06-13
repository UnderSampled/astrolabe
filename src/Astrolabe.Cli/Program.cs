using Astrolabe.Cli.Commands;

namespace Astrolabe.Cli;

class Program
{
    static int Main(string[] args)
    {
        if (args.Length == 0)
        {
            PrintUsage();
            return 1;
        }

        var command = args[0].ToLowerInvariant();

        return command switch
        {
            "extract" => ExtractCommand.Run(args[1..]),
            "extract-intermediate" => ExtractIntermediateCommand.Run(args[1..]),
            "compile-intermediate" => CompileIntermediateCommand.Run(args[1..]),
            "list" => ListCommand.Run(args[1..]),
            "textures" => TexturesCommand.Run(args[1..]),
            "cnt" => CntCommand.Run(args[1..]),
            "debug-gf" => DebugGfCommand.Run(args[1..]),
            "debug-sna" => DebugSnaCommand.Run(args[1..]),
            "debug-names" => DebugNamesCommand.Run(args[1..]),
            "meshes" => MeshesCommand.Run(args[1..]),
            "analyze" => AnalyzeCommand.Run(args[1..]),
            "textures-sna" => TexturesSnaCommand.Run(args[1..]),
            "scene" => SceneCommand.Run(args[1..]),
            "tree" => TreeCommand.Run(args[1..]),
            "byte-tree" => ByteTreeCommand.Run(args[1..]),
            "export-godot" => ExportGodotCommand.Run(args[1..]),
            "audio" => AudioCommand.Run(args[1..]),
            "scripts" => ScriptsCommand.Run(args[1..]),
            "help" or "--help" or "-h" => Help(),
            _ => UnknownCommand(command)
        };
    }

    static int Help()
    {
        PrintUsage();
        return 0;
    }

    static int UnknownCommand(string command)
    {
        Console.Error.WriteLine($"Unknown command: {command}");
        PrintUsage();
        return 1;
    }

    static void PrintUsage()
    {
        Console.WriteLine("""
            Astrolabe - Hype: The Time Quest Asset Extractor

            Usage:
                astrolabe <command> [options]

            Commands:
                extract <source> [output]          Extract and convert assets (PNG/WAV)
                extract-intermediate <level> [out] Extract a reversible intermediate level package
                compile-intermediate <dir> [out]   Compile an intermediate package back to level files
                list <source>                      List files in a directory
                textures <cnt-path> [output-dir]   Extract textures from CNT container
                cnt <cnt-path>                     List files in CNT container
                audio <apm-path|bnm-path> [out]    Convert APM/BNM audio to WAV
                tree <path> [options]              Display scene graph as tree
                byte-tree <level-dir> [options]   Show tree with byte coverage analysis
                export-godot <level-dir> [output]  Export level to a Godot project
                help                               Show this help message

            The <source> is an extracted or mounted directory containing game files.

            Options for 'extract':
                --raw, -r              Copy raw files without conversion
                --all, -a              Include all files (with --raw only)
                --pattern <pattern>    Only extract files matching pattern (with --raw only)

            Examples:
                astrolabe extract ./disc ./output
                astrolabe extract ./disc ./raw --raw
                astrolabe list ./disc
            """);
    }
}
