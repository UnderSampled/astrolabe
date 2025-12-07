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
            "list" => ListCommand.Run(args[1..]),
            "textures" => TexturesCommand.Run(args[1..]),
            "cnt" => CntCommand.Run(args[1..]),
            "debug-gf" => DebugGfCommand.Run(args[1..]),
            "debug-sna" => DebugSnaCommand.Run(args[1..]),
            "debug-names" => DebugNamesCommand.Run(args[1..]),
            "meshes" => MeshesCommand.Run(args[1..]),
            "analyze" => AnalyzeCommand.Run(args[1..]),
            "export-gltf" => ExportGltfCommand.Run(args[1..]),
            "export-families" => ExportFamiliesCommand.Run(args[1..]),
            "textures-sna" => TexturesSnaCommand.Run(args[1..]),
            "scene" => SceneCommand.Run(args[1..]),
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
                list <source>                      List files in ISO or directory
                textures <cnt-path> [output-dir]   Extract textures from CNT container
                cnt <cnt-path>                     List files in CNT container
                audio <apm-path|bnm-path> [out]    Convert APM/BNM audio to WAV
                export-gltf <level-dir> [output]   Export level meshes to GLTF
                export-families <level-dir> [out]  Export character Families (meshes + animations) to GLTF
                export-godot <level-dir> [output]  Export level to Godot scene
                help                               Show this help message

            The <source> can be either:
                - An ISO file (hype.iso)
                - An extracted/mounted directory containing game files

            Options for 'extract':
                --raw, -r              Copy raw files without conversion
                --all, -a              Include all files (with --raw only)
                --pattern <pattern>    Only extract files matching pattern (with --raw only)

            Examples:
                astrolabe extract hype.iso ./output
                astrolabe extract ./disc ./output
                astrolabe extract hype.iso ./disc --raw
                astrolabe list hype.iso
            """);
    }
}
