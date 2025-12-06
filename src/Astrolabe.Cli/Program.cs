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
            "meshes" => MeshesCommand.Run(args[1..]),
            "analyze" => AnalyzeCommand.Run(args[1..]),
            "export-gltf" => ExportGltfCommand.Run(args[1..]),
            "textures-sna" => TexturesSnaCommand.Run(args[1..]),
            "scene" => SceneCommand.Run(args[1..]),
            "export-godot" => ExportGodotCommand.Run(args[1..]),
            "audio" => AudioCommand.Run(args[1..]),
            "extract-all" => ExtractAllCommand.Run(args[1..]),
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
                extract <iso-path> [output-dir]    Extract files from ISO
                extract-all [extracted] [output]   Extract all assets to organized layout
                list <iso-path>                    List files in ISO
                textures <cnt-path> [output-dir]   Extract textures from CNT container
                cnt <cnt-path>                     List files in CNT container
                audio <apm-path|bnm-path> [out]    Convert APM/BNM audio to WAV
                help                               Show this help message

            Options for 'extract':
                --all, -a              Extract all files (default: only Gamedata, LangData, Sound)
                --pattern <pattern>    Only extract files matching pattern (e.g., "*.lvl")

            Examples:
                astrolabe list hype.iso
                astrolabe extract hype.iso ./extracted
                astrolabe extract hype.iso ./extracted --pattern "Gamedata/"
                astrolabe textures ./extracted/Gamedata/Textures.cnt ./textures
            """);
    }
}
