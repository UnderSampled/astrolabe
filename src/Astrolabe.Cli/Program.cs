using Astrolabe.Core.Extraction;
using Astrolabe.Core.FileFormats;

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

        switch (command)
        {
            case "extract":
                return RunExtract(args[1..]);
            case "list":
                return RunList(args[1..]);
            case "textures":
                return RunTextures(args[1..]);
            case "cnt":
                return RunCnt(args[1..]);
            case "debug-gf":
                return RunDebugGf(args[1..]);
            case "debug-sna":
                return RunDebugSna(args[1..]);
            case "help":
            case "--help":
            case "-h":
                PrintUsage();
                return 0;
            default:
                Console.Error.WriteLine($"Unknown command: {command}");
                PrintUsage();
                return 1;
        }
    }

    static void PrintUsage()
    {
        Console.WriteLine("""
            Astrolabe - Hype: The Time Quest Asset Extractor

            Usage:
                astrolabe <command> [options]

            Commands:
                extract <iso-path> [output-dir]    Extract files from ISO
                list <iso-path>                    List files in ISO
                textures <cnt-path> [output-dir]   Extract textures from CNT container
                cnt <cnt-path>                     List files in CNT container
                help                               Show this help message

            Options for 'extract':
                --pattern <pattern>    Only extract files matching pattern (e.g., "*.lvl")

            Examples:
                astrolabe list hype.iso
                astrolabe extract hype.iso ./extracted
                astrolabe extract hype.iso ./extracted --pattern "Gamedata/"
                astrolabe textures ./extracted/Gamedata/Textures.cnt ./textures
            """);
    }

    static int RunList(string[] args)
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

    static int RunExtract(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: ISO path required");
            return 1;
        }

        var isoPath = args[0];
        var outputDir = args.Length > 1 && !args[1].StartsWith("--")
            ? args[1]
            : "extracted";

        string? pattern = null;

        // Parse options
        for (int i = 0; i < args.Length; i++)
        {
            if (args[i] == "--pattern" && i + 1 < args.Length)
            {
                pattern = args[i + 1];
                i++;
            }
        }

        try
        {
            var extractor = new IsoExtractor(isoPath);

            var progress = new Progress<ExtractionProgress>(p =>
            {
                Console.Write($"\r[{p.ExtractedCount}/{p.TotalFiles}] {p.CurrentFile,-60}");
            });

            Console.WriteLine($"Extracting to: {outputDir}");

            if (pattern != null)
            {
                Console.WriteLine($"Pattern: {pattern}");
                extractor.ExtractPattern(pattern, outputDir, progress);
            }
            else
            {
                extractor.ExtractAll(outputDir, progress);
            }

            Console.WriteLine();
            Console.WriteLine("Extraction complete!");
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            return 1;
        }
    }

    static int RunCnt(string[] args)
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

    static int RunTextures(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: CNT file path required");
            return 1;
        }

        var cntPath = args[0];
        var outputDir = args.Length > 1 ? args[1] : "textures";

        try
        {
            var cnt = new CntReader(cntPath);

            Console.WriteLine($"Extracting {cnt.FileCount} textures from {Path.GetFileName(cntPath)}...");
            Directory.CreateDirectory(outputDir);

            int extracted = 0;
            int failed = 0;

            foreach (var file in cnt.Files)
            {
                try
                {
                    var data = cnt.ExtractFile(file);
                    var gf = new GfReader(data);

                    var outputPath = Path.Combine(outputDir, Path.ChangeExtension(file.FullPath, ".tga"));
                    var dir = Path.GetDirectoryName(outputPath);
                    if (!string.IsNullOrEmpty(dir))
                    {
                        Directory.CreateDirectory(dir);
                    }

                    gf.SaveAsTga(outputPath);
                    extracted++;

                    if (extracted % 100 == 0)
                    {
                        Console.Write($"\r[{extracted}/{cnt.FileCount}] Extracted...                    ");
                    }
                }
                catch
                {
                    failed++;
                }
            }

            Console.WriteLine();
            Console.WriteLine($"Extracted: {extracted} textures");
            if (failed > 0)
            {
                Console.WriteLine($"Failed: {failed} textures");
            }

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            return 1;
        }
    }

    static int RunDebugGf(string[] args)
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
            Console.WriteLine($"Files: {cnt.FileCount}");

            // Get first file
            var file = cnt.Files[0];
            Console.WriteLine($"\nFirst file: {file.FullPath} ({file.FileSize} bytes)");

            var data = cnt.ExtractFile(file);
            Console.WriteLine($"Extracted size: {data.Length} bytes");

            // Hex dump first 64 bytes
            Console.WriteLine("\nFirst 64 bytes:");
            for (int i = 0; i < Math.Min(64, data.Length); i++)
            {
                Console.Write($"{data[i]:X2} ");
                if ((i + 1) % 16 == 0) Console.WriteLine();
            }
            Console.WriteLine();

            // Try to parse as Montreal GF
            Console.WriteLine("\nParsing as Montreal GF:");
            using var reader = new BinaryReader(new MemoryStream(data));

            byte version = reader.ReadByte();
            Console.WriteLine($"Version: {version}");

            int width = reader.ReadInt32();
            int height = reader.ReadInt32();
            Console.WriteLine($"Dimensions: {width} x {height}");

            byte channels = reader.ReadByte();
            Console.WriteLine($"Channels: {channels}");

            // Montreal does NOT have mipmaps byte in header
            byte repeatByte = reader.ReadByte();
            Console.WriteLine($"RepeatByte: 0x{repeatByte:X2}");

            ushort paletteLength = reader.ReadUInt16();
            byte paletteBytesPerColor = reader.ReadByte();
            Console.WriteLine($"Palette: {paletteLength} entries x {paletteBytesPerColor} bytes");

            byte byte_0F = reader.ReadByte();
            byte byte_10 = reader.ReadByte();
            byte byte_11 = reader.ReadByte();
            uint uint_12 = reader.ReadUInt32();
            Console.WriteLine($"Unknown: 0x{byte_0F:X2} 0x{byte_10:X2} 0x{byte_11:X2} 0x{uint_12:X8}");

            int pixelCount = reader.ReadInt32();
            Console.WriteLine($"PixelCount: {pixelCount}");

            byte montrealType = reader.ReadByte();
            Console.WriteLine($"MontrealType: {montrealType}");

            Console.WriteLine($"\nRemaining bytes after header: {data.Length - reader.BaseStream.Position}");

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            Console.Error.WriteLine(ex.StackTrace);
            return 1;
        }
    }

    static int RunDebugSna(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Level directory path required");
            Console.Error.WriteLine("Usage: astrolabe debug-sna <level-dir> <level-name>");
            return 1;
        }

        var levelDir = args[0];
        var levelName = args.Length > 1 ? args[1] : Path.GetFileName(levelDir.TrimEnd('/', '\\'));

        try
        {
            Console.WriteLine($"Loading level: {levelName}");
            Console.WriteLine($"Directory: {levelDir}");
            Console.WriteLine();

            var loader = new LevelLoader(levelDir, levelName);
            loader.PrintDebugInfo(Console.Out);

            // Try to find and parse some structures
            Console.WriteLine();
            Console.WriteLine("Attempting to locate geometry data...");

            // Look for blocks that might contain geometry
            // In Montreal engine, geometry is typically in specific module/id combinations
            foreach (var block in loader.Sna.Blocks.Where(b => b.Data != null && b.Data.Length > 100))
            {
                // Try to identify potential GeometricObject structures by looking for patterns
                var data = block.Data!;
                using var reader = new BinaryReader(new MemoryStream(data));

                // Skip very small blocks
                if (data.Length < 50) continue;

                // Look for potential vertex count patterns (reasonable values)
                for (int offset = 0; offset < Math.Min(data.Length - 20, 100); offset += 4)
                {
                    reader.BaseStream.Position = offset;

                    // Montreal GeometricObject starts with num_vertices (uint32)
                    uint potentialVertCount = reader.ReadUInt32();
                    if (potentialVertCount > 0 && potentialVertCount < 10000)
                    {
                        // Read potential off_vertices pointer
                        int potentialPtr = reader.ReadInt32();

                        // Check if this pointer could be valid
                        if (potentialPtr > 0 && potentialPtr < 0x10000000)
                        {
                            // This might be a GeometricObject, note it
                            // Console.WriteLine($"  Potential mesh in [{block.Module:X2}:{block.Id:X2}] at offset {offset}: {potentialVertCount} verts, ptr=0x{potentialPtr:X8}");
                        }
                    }
                }
            }

            // Dump some raw hex from the first few blocks
            Console.WriteLine();
            Console.WriteLine("First 64 bytes of first 3 data blocks:");
            int shown = 0;
            foreach (var block in loader.Sna.Blocks.Where(b => b.Data != null).Take(5))
            {
                if (block.Data!.Length < 64) continue;
                if (shown++ >= 3) break;

                Console.WriteLine($"\nBlock [{block.Module:X2}:{block.Id:X2}] (Base=0x{block.BaseInMemory:X8}, {block.Data.Length} bytes):");
                for (int i = 0; i < 64; i++)
                {
                    Console.Write($"{block.Data[i]:X2} ");
                    if ((i + 1) % 16 == 0) Console.WriteLine();
                }
            }

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            Console.Error.WriteLine(ex.StackTrace);
            return 1;
        }
    }
}
