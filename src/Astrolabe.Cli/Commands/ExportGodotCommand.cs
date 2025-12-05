using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Geometry;
using Astrolabe.Core.FileFormats.Godot;

namespace Astrolabe.Cli.Commands;

public static class ExportGodotCommand
{
    public static int Run(string[] args)
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
                    fileName += ".png";
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
}
