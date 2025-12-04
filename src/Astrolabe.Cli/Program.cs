using Astrolabe.Core.Extraction;
using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Audio;
using Astrolabe.Core.FileFormats.Geometry;
using Astrolabe.Core.FileFormats.Godot;

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
            case "meshes":
                return RunMeshes(args[1..]);
            case "analyze":
                return RunAnalyze(args[1..]);
            case "export-gltf":
                return RunExportGltf(args[1..]);
            case "textures-sna":
                return RunTexturesSna(args[1..]);
            case "scene":
                return RunScene(args[1..]);
            case "export-godot":
                return RunExportGodot(args[1..]);
            case "audio":
                return RunAudio(args[1..]);
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
        bool extractAll = false;

        // Parse options
        for (int i = 0; i < args.Length; i++)
        {
            if (args[i] == "--pattern" && i + 1 < args.Length)
            {
                pattern = args[i + 1];
                i++;
            }
            else if (args[i] == "--all" || args[i] == "-a")
            {
                extractAll = true;
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
            else if (extractAll)
            {
                extractor.ExtractAll(outputDir, progress);
            }
            else
            {
                // Default: extract only game data folders
                string[] defaultFolders = ["Gamedata", "LangData", "Sound"];
                Console.WriteLine($"Extracting: {string.Join(", ", defaultFolders)} (use --all for everything)");
                foreach (var folder in defaultFolders)
                {
                    extractor.ExtractPattern($"{folder}/", outputDir, progress);
                }
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

    static int RunMeshes(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Level directory path required");
            Console.Error.WriteLine("Usage: astrolabe meshes <level-dir> [level-name]");
            return 1;
        }

        var levelDir = args[0];
        var levelName = args.Length > 1 ? args[1] : Path.GetFileName(levelDir.TrimEnd('/', '\\'));

        try
        {
            Console.WriteLine($"Loading level: {levelName}");
            var loader = new LevelLoader(levelDir, levelName);
            Console.WriteLine($"Loaded {loader.Sna.Blocks.Count} SNA blocks");

            // Load texture table from PTX
            TextureTable? textureTable = null;
            var ptxPath = Path.Combine(levelDir, $"{levelName}.ptx");
            if (!File.Exists(ptxPath))
            {
                ptxPath = Directory.GetFiles(levelDir, $"{levelName}.ptx*").FirstOrDefault() ?? "";
            }
            if (File.Exists(ptxPath))
            {
                textureTable = new TextureTable(loader, ptxPath);
                Console.WriteLine($"Loaded {textureTable.TextureNames.Count} texture references from PTX");
            }
            Console.WriteLine();

            var scanner = new MeshScanner(loader, textureTable);
            Console.WriteLine("Scanning for meshes...");
            var meshes = scanner.ScanForMeshes();

            Console.WriteLine($"\nFound {meshes.Count} potential meshes:");
            int withTriangles = meshes.Count(m => m.Indices != null && m.Indices.Length > 0);
            int withoutTriangles = meshes.Count - withTriangles;
            int withTextures = meshes.Count(m => !string.IsNullOrEmpty(m.TextureName));
            Console.WriteLine($"  With triangle indices: {withTriangles}");
            Console.WriteLine($"  Without triangle indices (using fallback): {withoutTriangles}");
            Console.WriteLine($"  With texture references: {withTextures}");
            Console.WriteLine();

            foreach (var mesh in meshes.Take(20))
            {
                int triCount = mesh.Indices != null ? mesh.Indices.Length / 3 : 0;
                string texInfo = !string.IsNullOrEmpty(mesh.TextureName) ? $" tex={mesh.TextureName}" : "";
                Console.WriteLine($"  {mesh.Name}: {mesh.NumVertices} verts, {mesh.NumElements} elems, {triCount} tris{texInfo}");
                if (mesh.Vertices.Length > 0)
                {
                    var minX = mesh.Vertices.Min(v => v.X);
                    var maxX = mesh.Vertices.Max(v => v.X);
                    var minY = mesh.Vertices.Min(v => v.Y);
                    var maxY = mesh.Vertices.Max(v => v.Y);
                    var minZ = mesh.Vertices.Min(v => v.Z);
                    var maxZ = mesh.Vertices.Max(v => v.Z);
                    Console.WriteLine($"    Bounds: X[{minX:F2}, {maxX:F2}] Y[{minY:F2}, {maxY:F2}] Z[{minZ:F2}, {maxZ:F2}]");
                }
            }

            if (meshes.Count > 20)
            {
                Console.WriteLine($"  ... and {meshes.Count - 20} more");
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

    static int RunAnalyze(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Level directory path required");
            Console.Error.WriteLine("Usage: astrolabe analyze <level-dir> [level-name]");
            return 1;
        }

        var levelDir = args[0];
        var levelName = args.Length > 1 ? args[1] : Path.GetFileName(levelDir.TrimEnd('/', '\\'));

        try
        {
            Console.WriteLine($"Loading level: {levelName}");
            var loader = new LevelLoader(levelDir, levelName);

            Console.WriteLine("\n=== SNA Block Summary ===");
            foreach (var block in loader.Sna.Blocks.OrderBy(b => b.BaseInMemory))
            {
                Console.WriteLine($"[{block.Module:X2}:{block.Id:X2}] Base=0x{block.BaseInMemory:X8} Size={block.Data?.Length ?? 0}");
            }

            // Block 05:01 is the "Fix" block in Montreal - contains the globals struct
            var fixBlock = loader.Sna.Blocks.FirstOrDefault(b => b.Module == 0x05 && b.Id == 0x01);
            if (fixBlock?.Data == null)
            {
                Console.WriteLine("Fix block not found!");
                return 1;
            }

            Console.WriteLine($"\n=== Fix Block [05:01] Analysis ===");
            Console.WriteLine($"Base: 0x{fixBlock.BaseInMemory:X8}");
            Console.WriteLine($"Size: {fixBlock.Data.Length} bytes");

            // Get pointer info from RTB
            var rtbBlock = loader.Rtb?.GetBlock(0x05, 0x01);
            if (rtbBlock != null)
            {
                Console.WriteLine($"RTB pointers: {rtbBlock.Count}");

                // Show first 30 pointers to understand the structure
                Console.WriteLine("\nFirst 30 RTB pointers from Fix block:");
                for (int i = 0; i < Math.Min(30, rtbBlock.Pointers.Length); i++)
                {
                    var ptr = rtbBlock.Pointers[i];
                    int offsetInBlock = (int)ptr.OffsetInMemory - fixBlock.BaseInMemory;

                    // Read the actual pointer value at this location
                    if (offsetInBlock >= 0 && offsetInBlock + 4 <= fixBlock.Data.Length)
                    {
                        int ptrValue = BitConverter.ToInt32(fixBlock.Data, offsetInBlock);
                        Console.WriteLine($"  [{i:D4}] Offset 0x{ptr.OffsetInMemory:X8} (block +0x{offsetInBlock:X}) -> [{ptr.TargetModule:X2}:{ptr.TargetId:X2}] value=0x{ptrValue:X8}");
                    }
                }
            }

            // Now let's look for GeometricObject structures in ALL blocks
            Console.WriteLine("\n=== Scanning ALL blocks for GeometricObject headers ===");
            int totalFound = 0;

            foreach (var block in loader.Sna.Blocks.Where(b => b.Data != null && b.Data.Length > 100))
            {
                int baseAddr = block.BaseInMemory;
                int endAddr = baseAddr + block.Data.Length;
                int found = 0;

                using var ms = new MemoryStream(block.Data);
                using var reader = new BinaryReader(ms);

                for (int offset = 0; offset < block.Data.Length - 64; offset += 4)
                {
                    ms.Position = offset;

                    uint numVertices = reader.ReadUInt32();
                    if (numVertices < 3 || numVertices > 10000) continue;

                    int offVerts = reader.ReadInt32();
                    int offNormals = reader.ReadInt32();
                    int offMaterials = reader.ReadInt32();
                    reader.ReadInt32(); // skip
                    uint numElements = reader.ReadUInt32();

                    if (numElements == 0 || numElements > 1000) continue;

                    // Check if the vertex/normal pointers look valid (pointing within this block)
                    bool vertsValid = offVerts >= baseAddr && offVerts < endAddr;
                    bool normalsValid = offNormals >= baseAddr && offNormals < endAddr;

                    if (vertsValid && normalsValid)
                    {
                        // Validate vertex data at the pointer location
                        int vertOffset = offVerts - baseAddr;
                        if (vertOffset >= 0 && vertOffset + numVertices * 12 <= block.Data.Length)
                        {
                            ms.Position = vertOffset;
                            float x = reader.ReadSingle();
                            float z = reader.ReadSingle();
                            float y = reader.ReadSingle();

                            // Only count meshes with non-trivial vertex data
                            if (!float.IsNaN(x) && !float.IsInfinity(x) &&
                                Math.Abs(x) < 100000 && Math.Abs(y) < 100000 && Math.Abs(z) < 100000)
                            {
                                // Calculate bounding box to filter out all-zero meshes
                                ms.Position = vertOffset;
                                float minX = float.MaxValue, maxX = float.MinValue;
                                float minY = float.MaxValue, maxY = float.MinValue;
                                float minZ = float.MaxValue, maxZ = float.MinValue;
                                bool hasVariation = false;

                                for (int v = 0; v < numVertices; v++)
                                {
                                    float vx = reader.ReadSingle();
                                    float vz = reader.ReadSingle();
                                    float vy = reader.ReadSingle();

                                    if (!float.IsNaN(vx) && !float.IsInfinity(vx))
                                    {
                                        minX = Math.Min(minX, vx); maxX = Math.Max(maxX, vx);
                                        minY = Math.Min(minY, vy); maxY = Math.Max(maxY, vy);
                                        minZ = Math.Min(minZ, vz); maxZ = Math.Max(maxZ, vz);
                                    }
                                }

                                float sizeX = maxX - minX;
                                float sizeY = maxY - minY;
                                float sizeZ = maxZ - minZ;
                                hasVariation = sizeX > 0.01f || sizeY > 0.01f || sizeZ > 0.01f;

                                if (hasVariation)
                                {
                                    if (found == 0)
                                    {
                                        Console.WriteLine($"\n[{block.Module:X2}:{block.Id:X2}] GeometricObjects:");
                                    }
                                    Console.WriteLine($"  +0x{offset:X}: {numVertices} verts, {numElements} elems, size=({sizeX:F2}, {sizeY:F2}, {sizeZ:F2})");
                                    found++;
                                    totalFound++;
                                    if (found >= 10) break; // Limit per block
                                }
                            }
                        }
                    }
                }
            }

            Console.WriteLine($"\nTotal GeometricObjects with variation: {totalFound}");

            // Debug: show pointer stats
            Console.WriteLine("\n=== Pointer validation stats ===");
            int vertPtrCount = 0, normPtrCount = 0, elemTypePtrCount = 0, elemsPtrCount = 0;
            foreach (var block in loader.Sna.Blocks.Where(b => b.Data != null && b.Data.Length > 100))
            {
                int baseAddr = block.BaseInMemory;
                for (int offset = 0; offset < block.Data.Length - 64; offset += 4)
                {
                    int memAddr = baseAddr + offset;
                    var memory = new Astrolabe.Core.FileFormats.MemoryContext(loader.Sna, loader.Rtb);
                    if (memory.GetPointerAt(memAddr + 4) != null) vertPtrCount++;
                    if (memory.GetPointerAt(memAddr + 8) != null) normPtrCount++;
                    if (memory.GetPointerAt(memAddr + 24) != null) elemTypePtrCount++;
                    if (memory.GetPointerAt(memAddr + 28) != null) elemsPtrCount++;
                }
            }
            Console.WriteLine($"Potential vert ptrs: {vertPtrCount}");
            Console.WriteLine($"Potential norm ptrs: {normPtrCount}");
            Console.WriteLine($"Potential elemType ptrs: {elemTypePtrCount}");
            Console.WriteLine($"Potential elems ptrs: {elemsPtrCount}");

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            Console.Error.WriteLine(ex.StackTrace);
            return 1;
        }
    }

    static int RunExportGltf(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Level directory path required");
            Console.Error.WriteLine("Usage: astrolabe export-gltf <level-dir> [level-name] [output.glb] [--texture <path>]");
            return 1;
        }

        // Filter out option arguments for positional arg parsing
        var positionalArgs = new List<string>();
        string? texturePath = null;
        for (int i = 0; i < args.Length; i++)
        {
            if ((args[i] == "--texture" || args[i] == "-t") && i + 1 < args.Length)
            {
                texturePath = args[i + 1];
                i++; // Skip next arg
            }
            else if (!args[i].StartsWith("-"))
            {
                positionalArgs.Add(args[i]);
            }
        }

        var levelDir = positionalArgs[0];
        var levelName = positionalArgs.Count > 1 && !positionalArgs[1].EndsWith(".glb")
            ? positionalArgs[1]
            : Path.GetFileName(levelDir.TrimEnd('/', '\\'));
        var outputPath = positionalArgs.FirstOrDefault(a => a.EndsWith(".glb")) ?? $"output/{levelName}_meshes.glb";

        try
        {
            Console.WriteLine($"Loading level: {levelName}");
            var loader = new LevelLoader(levelDir, levelName);
            Console.WriteLine($"Loaded {loader.Sna.Blocks.Count} SNA blocks");

            // Load texture table from PTX
            TextureTable? textureTable = null;
            var ptxPath = Path.Combine(levelDir, $"{levelName}.ptx");
            if (!File.Exists(ptxPath))
            {
                ptxPath = Directory.GetFiles(levelDir, $"{levelName}.ptx*").FirstOrDefault() ?? "";
            }
            if (File.Exists(ptxPath))
            {
                textureTable = new TextureTable(loader, ptxPath);
                Console.WriteLine($"Loaded {textureTable.TextureNames.Count} texture references from PTX");
            }

            var scanner = new MeshScanner(loader, textureTable);
            Console.WriteLine("Scanning for meshes...");
            var meshes = scanner.ScanForMeshes();
            Console.WriteLine($"Found {meshes.Count} potential meshes");

            // Filter to meshes with actual triangle data and reasonable size
            var validMeshes = meshes
                .Where(m => m.Vertices.Length >= 3)
                .Where(m => m.Indices != null && m.Indices.Length >= 3) // Must have triangles
                .Where(m =>
                {
                    var minX = m.Vertices.Min(v => v.X);
                    var maxX = m.Vertices.Max(v => v.X);
                    var minY = m.Vertices.Min(v => v.Y);
                    var maxY = m.Vertices.Max(v => v.Y);
                    var minZ = m.Vertices.Min(v => v.Z);
                    var maxZ = m.Vertices.Max(v => v.Z);

                    var sizeX = maxX - minX;
                    var sizeY = maxY - minY;
                    var sizeZ = maxZ - minZ;

                    // At least some dimension > 0.5 and no dimension > 1000
                    return (sizeX > 0.5f || sizeY > 0.5f || sizeZ > 0.5f) &&
                           sizeX < 1000 && sizeY < 1000 && sizeZ < 1000;
                })
                .Take(100) // Limit to 100 meshes
                .ToList();

            // Report UV statistics
            var meshesWithUVs = validMeshes.Count(m => m.UVs != null && m.UVs.Length > 0);
            var meshesWithTextures = validMeshes.Count(m => !string.IsNullOrEmpty(m.TextureName));
            Console.WriteLine($"Meshes with UV data: {meshesWithUVs} / {validMeshes.Count}");
            Console.WriteLine($"Meshes with texture refs: {meshesWithTextures} / {validMeshes.Count}");

            // Create output directory (use current directory if no path specified)
            string outputDir = Path.GetDirectoryName(outputPath);
            if (string.IsNullOrEmpty(outputDir)) outputDir = ".";
            string baseName = Path.GetFileNameWithoutExtension(outputPath);
            string meshOutputDir = Path.Combine(outputDir, baseName);
            Directory.CreateDirectory(meshOutputDir);

            Console.WriteLine($"Exporting {validMeshes.Count} meshes to {meshOutputDir}/...");

            // Build texture lookup from all extracted textures
            string textureBaseDir = "textures";
            var textureLookup = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            if (Directory.Exists(textureBaseDir))
            {
                foreach (var file in Directory.EnumerateFiles(textureBaseDir, "*.tga", SearchOption.AllDirectories))
                {
                    var fileName = Path.GetFileName(file);
                    if (!textureLookup.ContainsKey(fileName))
                        textureLookup[fileName] = file;
                }
                foreach (var file in Directory.EnumerateFiles(textureBaseDir, "*.png", SearchOption.AllDirectories))
                {
                    var fileName = Path.GetFileName(file);
                    if (!textureLookup.ContainsKey(fileName))
                        textureLookup[fileName] = file;
                }
                Console.WriteLine($"Indexed {textureLookup.Count} textures from {textureBaseDir}/");
            }

            if (!string.IsNullOrEmpty(texturePath) && File.Exists(texturePath))
            {
                Console.WriteLine($"Using override texture: {texturePath}");
            }

            // Create texture lookup function
            Func<string?, string?> lookupTexture = (texName) =>
            {
                if (!string.IsNullOrEmpty(texturePath))
                    return texturePath; // Override all textures

                if (string.IsNullOrEmpty(texName))
                    return null;

                string fileName = Path.GetFileName(texName);
                if (!fileName.EndsWith(".tga", StringComparison.OrdinalIgnoreCase) &&
                    !fileName.EndsWith(".png", StringComparison.OrdinalIgnoreCase))
                {
                    fileName += ".tga";
                }

                if (textureLookup.TryGetValue(fileName, out var foundPath))
                    return foundPath;

                // Try PNG extension
                var pngName = Path.ChangeExtension(fileName, ".png");
                if (textureLookup.TryGetValue(pngName, out foundPath))
                    return foundPath;

                return null;
            };

            // Export each mesh as a separate file
            int exported = 0;
            int withTextures = 0;
            int withTransparency = 0;
            foreach (var mesh in validMeshes)
            {
                string meshFileName = $"{mesh.Name}.glb";
                string meshPath = Path.Combine(meshOutputDir, meshFileName);

                // Count submeshes with textures
                int subMeshesWithTextures = mesh.SubMeshes.Count(sm => lookupTexture(sm.TextureName) != null);
                if (subMeshesWithTextures > 0)
                    withTextures++;

                // Count submeshes with transparency flags
                int transparentSubMeshes = mesh.SubMeshes.Count(sm =>
                    sm.MaterialFlags != 0 || (sm.VisualMaterial?.IsTransparent ?? false));
                if (transparentSubMeshes > 0)
                    withTransparency++;

                GltfExporter.ExportMesh(mesh, meshPath, lookupTexture);
                exported++;
            }

            Console.WriteLine($"Meshes exported with textures: {withTextures} / {exported}");
            Console.WriteLine($"Meshes with transparency flags: {withTransparency} / {exported}");

            // Show material stats
            var meshesWithVisualMat = validMeshes.Count(m => m.SubMeshes.Any(sm => sm.VisualMaterial != null));
            var meshesWithGameMat = validMeshes.Count(m => m.SubMeshes.Any(sm => sm.GameMaterial != null));
            Console.WriteLine($"Meshes with VisualMaterial: {meshesWithVisualMat} / {exported}");
            Console.WriteLine($"Meshes with GameMaterial: {meshesWithGameMat} / {exported}");

            Console.WriteLine($"Exported {exported} mesh files to: {meshOutputDir}/");
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            Console.Error.WriteLine(ex.StackTrace);
            return 1;
        }
    }

    static int RunTexturesSna(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Level directory path required");
            Console.Error.WriteLine("Usage: astrolabe textures-sna <level-dir> [level-name]");
            return 1;
        }

        var levelDir = args[0];
        var levelName = args.Length > 1 ? args[1] : Path.GetFileName(levelDir.TrimEnd('/', '\\'));

        try
        {
            Console.WriteLine($"Loading level: {levelName}");
            var loader = new LevelLoader(levelDir, levelName);
            Console.WriteLine($"Loaded {loader.Sna.Blocks.Count} SNA blocks");

            // Find PTX file
            var ptxPath = Path.Combine(levelDir, $"{levelName}.ptx");
            if (!File.Exists(ptxPath))
            {
                ptxPath = Directory.GetFiles(levelDir, $"{levelName}.ptx*").FirstOrDefault();
            }

            if (ptxPath == null || !File.Exists(ptxPath))
            {
                Console.WriteLine("PTX file not found");
                return 1;
            }

            Console.WriteLine($"Loading PTX from: {ptxPath}");
            var textureTable = new TextureTable(loader, ptxPath);

            Console.WriteLine($"\nFound {textureTable.TextureNames.Count} texture names:");
            foreach (var (addr, name) in textureTable.TextureNames.OrderBy(kv => kv.Key))
            {
                Console.WriteLine($"  0x{addr:X8}: {name}");
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

    static int RunScene(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Level directory path required");
            Console.Error.WriteLine("Usage: astrolabe scene <level-dir> [level-name]");
            return 1;
        }

        var levelDir = args[0];
        var levelName = args.Length > 1 ? args[1] : Path.GetFileName(levelDir.TrimEnd('/', '\\'));

        try
        {
            Console.WriteLine($"Loading level: {levelName}");
            var loader = new LevelLoader(levelDir, levelName);
            Console.WriteLine($"Loaded {loader.Sna.Blocks.Count} SNA blocks");

            // Find GPT file
            var gptPath = Path.Combine(levelDir, $"{levelName}.gpt");
            if (!File.Exists(gptPath))
            {
                gptPath = Directory.GetFiles(levelDir, $"{levelName}.gpt*").FirstOrDefault() ?? "";
            }

            if (!File.Exists(gptPath))
            {
                Console.Error.WriteLine("GPT file not found");
                return 1;
            }

            Console.WriteLine($"Loading GPT from: {gptPath}");
            var gpt = new GptReader(gptPath);
            gpt.PrintDebugInfo(Console.Out);
            Console.WriteLine();

            // Create memory context for pointer resolution
            var memory = new MemoryContext(loader.Sna, loader.Rtb);

            // Read scene graph
            Console.WriteLine("Reading scene graph...");
            var sceneReader = new SuperObjectReader(memory);
            var sceneGraph = sceneReader.ReadSceneGraph(gpt);

            // Print statistics
            Console.WriteLine($"\nScene Graph Statistics:");
            Console.WriteLine($"  Total nodes: {sceneGraph.AllNodes.Count}");

            var typeGroups = sceneGraph.AllNodes.GroupBy(n => n.Type).OrderBy(g => g.Key);
            foreach (var group in typeGroups)
            {
                Console.WriteLine($"  {group.Key}: {group.Count()}");
            }

            var geoNodes = sceneGraph.GetGeometryNodes().ToList();
            Console.WriteLine($"\n  Nodes with geometry: {geoNodes.Count}");

            // Print hierarchy (limited depth)
            Console.WriteLine("\nScene Hierarchy (ActualWorld):");
            if (sceneGraph.ActualWorld != null)
            {
                PrintHierarchy(sceneGraph.ActualWorld, 0, 3);
            }
            else
            {
                Console.WriteLine("  (no ActualWorld found)");
            }

            if (sceneGraph.DynamicWorld != null)
            {
                Console.WriteLine("\nDynamic World:");
                PrintHierarchy(sceneGraph.DynamicWorld, 0, 2);
            }

            if (sceneGraph.FatherSector != null)
            {
                Console.WriteLine("\nFather Sector:");
                PrintHierarchy(sceneGraph.FatherSector, 0, 2);
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

    static void PrintHierarchy(SceneNode node, int indent, int maxDepth)
    {
        if (indent > maxDepth) return;

        var prefix = new string(' ', indent * 2);
        string geoInfo = node.GeometricObjectAddress != 0 ? $" [geo@0x{node.GeometricObjectAddress:X8}]" : "";
        Console.WriteLine($"{prefix}{node.Type} @ 0x{node.Address:X8}{geoInfo}");

        if (indent < maxDepth)
        {
            foreach (var child in node.Children.Take(10))
            {
                PrintHierarchy(child, indent + 1, maxDepth);
            }
            if (node.Children.Count > 10)
            {
                Console.WriteLine($"{prefix}  ... and {node.Children.Count - 10} more children");
            }
        }
        else if (node.Children.Count > 0)
        {
            Console.WriteLine($"{prefix}  ({node.Children.Count} children)");
        }
    }

    static int RunExportGodot(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Level directory path required");
            Console.Error.WriteLine("Usage: astrolabe export-godot <level-dir> [output-dir]");
            return 1;
        }

        var levelDir = args[0];
        var levelName = Path.GetFileName(levelDir.TrimEnd('/', '\\'));
        var outputDir = args.Length > 1 ? args[1] : $"output/{levelName}";

        try
        {
            Console.WriteLine($"Loading level: {levelName}");
            var loader = new LevelLoader(levelDir, levelName);
            Console.WriteLine($"Loaded {loader.Sna.Blocks.Count} SNA blocks");

            // Load texture table from PTX
            TextureTable? textureTable = null;
            var ptxPath = Path.Combine(levelDir, $"{levelName}.ptx");
            if (!File.Exists(ptxPath))
            {
                ptxPath = Directory.GetFiles(levelDir, $"{levelName}.ptx*").FirstOrDefault() ?? "";
            }
            if (File.Exists(ptxPath))
            {
                textureTable = new TextureTable(loader, ptxPath);
                Console.WriteLine($"Loaded {textureTable.TextureNames.Count} texture references from PTX");
            }

            // Find GPT file
            var gptPath = Path.Combine(levelDir, $"{levelName}.gpt");
            if (!File.Exists(gptPath))
            {
                gptPath = Directory.GetFiles(levelDir, $"{levelName}.gpt*").FirstOrDefault() ?? "";
            }

            if (!File.Exists(gptPath))
            {
                Console.Error.WriteLine("GPT file not found - cannot build scene graph");
                return 1;
            }

            Console.WriteLine($"Loading GPT from: {gptPath}");
            var gpt = new GptReader(gptPath);

            // Create memory context for pointer resolution
            var memory = new MemoryContext(loader.Sna, loader.Rtb);

            // Read scene graph
            Console.WriteLine("Reading scene graph...");
            var sceneReader = new SuperObjectReader(memory);
            var sceneGraph = sceneReader.ReadSceneGraph(gpt);
            Console.WriteLine($"Found {sceneGraph.AllNodes.Count} scene nodes");

            // Scan for meshes
            Console.WriteLine("Scanning for meshes...");
            var scanner = new MeshScanner(loader, textureTable);
            var meshes = scanner.ScanForMeshes();

            // Filter valid meshes
            var validMeshes = meshes
                .Where(m => m.Vertices.Length >= 3)
                .Where(m => m.Indices != null && m.Indices.Length >= 3)
                .Where(m =>
                {
                    var minX = m.Vertices.Min(v => v.X);
                    var maxX = m.Vertices.Max(v => v.X);
                    var sizeX = maxX - minX;
                    return sizeX > 0.5f && sizeX < 1000;
                })
                .ToList();

            Console.WriteLine($"Found {validMeshes.Count} valid meshes");

            // Create output directories
            var meshDir = Path.Combine(outputDir, "meshes");
            var texturesDir = Path.Combine(outputDir, "textures");
            Directory.CreateDirectory(outputDir);
            Directory.CreateDirectory(meshDir);
            Directory.CreateDirectory(texturesDir);

            // Build texture lookup
            string textureBaseDir = "textures";
            var textureLookup = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            if (Directory.Exists(textureBaseDir))
            {
                foreach (var file in Directory.EnumerateFiles(textureBaseDir, "*.tga", SearchOption.AllDirectories))
                {
                    var fileName = Path.GetFileName(file);
                    if (!textureLookup.ContainsKey(fileName))
                        textureLookup[fileName] = file;
                }
                foreach (var file in Directory.EnumerateFiles(textureBaseDir, "*.png", SearchOption.AllDirectories))
                {
                    var fileName = Path.GetFileName(file);
                    if (!textureLookup.ContainsKey(fileName))
                        textureLookup[fileName] = file;
                }
                Console.WriteLine($"Indexed {textureLookup.Count} textures");
            }

            // Texture lookup function
            Func<string?, string?> lookupTexture = (texName) =>
            {
                if (string.IsNullOrEmpty(texName)) return null;

                string fileName = Path.GetFileName(texName);
                if (!fileName.EndsWith(".tga", StringComparison.OrdinalIgnoreCase) &&
                    !fileName.EndsWith(".png", StringComparison.OrdinalIgnoreCase))
                {
                    fileName += ".tga";
                }

                if (textureLookup.TryGetValue(fileName, out var foundPath))
                    return foundPath;

                var pngName = Path.ChangeExtension(fileName, ".png");
                if (textureLookup.TryGetValue(pngName, out foundPath))
                    return foundPath;

                return null;
            };

            // Build mapping from GeometricObject address to mesh data
            var geoAddrToMesh = new Dictionary<int, MeshData>();
            foreach (var mesh in validMeshes)
            {
                if (mesh.SourceBlock != null)
                {
                    int geoAddr = mesh.SourceBlock.BaseInMemory + mesh.SourceOffset;
                    geoAddrToMesh[geoAddr] = mesh;
                }
            }

            // Export meshes and build address-to-filename mapping
            Console.WriteLine("Exporting meshes as GLTF...");
            var geoAddrToMeshName = new Dictionary<int, string>();
            foreach (var (geoAddr, mesh) in geoAddrToMesh)
            {
                string meshFileName = $"mesh_{geoAddr:X8}";
                string meshPath = Path.Combine(meshDir, $"{meshFileName}.glb");

                GltfExporter.ExportMesh(mesh, meshPath, lookupTexture);
                geoAddrToMeshName[geoAddr] = meshFileName;
            }

            Console.WriteLine($"Exported {geoAddrToMeshName.Count} meshes");

            // Match scene nodes to meshes
            int matchedNodes = 0;
            foreach (var node in sceneGraph.AllNodes)
            {
                if (node.GeometricObjectAddress != 0 && geoAddrToMesh.ContainsKey(node.GeometricObjectAddress))
                {
                    matchedNodes++;
                }
            }
            Console.WriteLine($"Scene nodes with matched meshes: {matchedNodes}");

            // Export Godot scene
            Console.WriteLine("Exporting Godot scene...");
            var godotExporter = new GodotExporter();
            var tscnFileName = $"{levelName}.tscn";
            var tscnPath = Path.Combine(outputDir, tscnFileName);
            godotExporter.Export(sceneGraph, tscnPath, "meshes", geoAddrToMeshName);

            // Write Godot project file
            GodotExporter.WriteProjectFile(outputDir, levelName, tscnFileName);

            Console.WriteLine($"\nExported to: {outputDir}");
            Console.WriteLine($"  Project: project.godot");
            Console.WriteLine($"  Scene: {tscnFileName}");
            Console.WriteLine($"  Meshes: meshes/ ({geoAddrToMeshName.Count} files)");
            Console.WriteLine();
            Console.WriteLine("To open in Godot:");
            Console.WriteLine($"  godot --editor --path \"{outputDir}\"");
            Console.WriteLine("(First run will import all meshes, which may take a moment)");

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            Console.Error.WriteLine(ex.StackTrace);
            return 1;
        }
    }

    static int RunAudio(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Audio file path required");
            Console.Error.WriteLine("Usage: astrolabe audio <apm-path> [output-path]");
            Console.Error.WriteLine("       astrolabe audio <bnm-path> [output-dir]   # Extract BNM sound bank");
            Console.Error.WriteLine("       astrolabe audio <directory> [output-dir]  # Convert all APM/BNM files");
            return 1;
        }

        var inputPath = args[0];
        var outputPath = args.Length > 1 ? args[1] : null;

        try
        {
            // Check if input is a directory
            if (Directory.Exists(inputPath))
            {
                return ConvertAllAudio(inputPath, outputPath);
            }

            // Single file conversion
            if (!File.Exists(inputPath))
            {
                Console.Error.WriteLine($"Error: File not found: {inputPath}");
                return 1;
            }

            // Check file type
            var ext = Path.GetExtension(inputPath).ToLowerInvariant();
            if (ext == ".bnm")
            {
                return ExtractBnm(inputPath, outputPath);
            }

            // APM file
            outputPath ??= Path.ChangeExtension(inputPath, ".wav");

            Console.WriteLine($"Converting: {inputPath}");
            WavWriter.ConvertApmToWav(inputPath, outputPath);
            Console.WriteLine($"Output: {outputPath}");

            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            return 1;
        }
    }

    static int ExtractBnm(string bnmPath, string? outputDir)
    {
        var bnmName = Path.GetFileNameWithoutExtension(bnmPath);
        outputDir ??= Path.Combine(Path.GetDirectoryName(bnmPath) ?? ".", bnmName);

        Console.WriteLine($"Reading BNM: {bnmPath}");
        var bnm = new BnmReader(bnmPath);
        Console.WriteLine($"Version: 0x{bnm.Version:X8}");
        Console.WriteLine($"Entries: {bnm.Entries.Count}");

        if (bnm.Entries.Count == 0)
        {
            Console.WriteLine("No audio entries found.");
            return 0;
        }

        Console.WriteLine($"Extracting to: {outputDir}");
        int extracted = bnm.ExtractAll(outputDir, verbose: true);
        Console.WriteLine($"Extracted: {extracted}/{bnm.Entries.Count}");

        return extracted > 0 ? 0 : 1;
    }

    static int ConvertAllAudio(string inputDir, string? outputDir)
    {
        outputDir ??= inputDir;

        var apmFiles = Directory.GetFiles(inputDir, "*.apm", SearchOption.AllDirectories);
        var bnmFiles = Directory.GetFiles(inputDir, "*.bnm", SearchOption.AllDirectories);

        if (apmFiles.Length == 0 && bnmFiles.Length == 0)
        {
            Console.WriteLine("No APM or BNM files found.");
            return 0;
        }

        int converted = 0;
        int failed = 0;

        // Convert APM files
        if (apmFiles.Length > 0)
        {
            Console.WriteLine($"Found {apmFiles.Length} APM files");
            foreach (var apmPath in apmFiles)
            {
                var relativePath = Path.GetRelativePath(inputDir, apmPath);
                var wavPath = Path.Combine(outputDir, Path.ChangeExtension(relativePath, ".wav"));

                var wavDir = Path.GetDirectoryName(wavPath);
                if (wavDir != null && !Directory.Exists(wavDir))
                {
                    Directory.CreateDirectory(wavDir);
                }

                try
                {
                    WavWriter.ConvertApmToWav(apmPath, wavPath);
                    Console.WriteLine($"  {relativePath} -> {Path.GetFileName(wavPath)}");
                    converted++;
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine($"  {relativePath}: FAILED - {ex.Message}");
                    failed++;
                }
            }
        }

        // Extract BNM files
        if (bnmFiles.Length > 0)
        {
            Console.WriteLine($"Found {bnmFiles.Length} BNM files");
            foreach (var bnmPath in bnmFiles)
            {
                var relativePath = Path.GetRelativePath(inputDir, bnmPath);
                var bnmName = Path.GetFileNameWithoutExtension(bnmPath);
                var bnmOutputDir = Path.Combine(outputDir, Path.GetDirectoryName(relativePath) ?? "", bnmName);

                try
                {
                    var bnm = new BnmReader(bnmPath);
                    int extracted = bnm.ExtractAll(bnmOutputDir);
                    Console.WriteLine($"  {relativePath} -> {bnmName}/ ({extracted} files)");
                    converted += extracted;
                }
                catch (Exception ex)
                {
                    Console.Error.WriteLine($"  {relativePath}: FAILED - {ex.Message}");
                    failed++;
                }
            }
        }

        Console.WriteLine($"\nConverted: {converted}, Failed: {failed}");
        return failed > 0 ? 1 : 0;
    }
}
