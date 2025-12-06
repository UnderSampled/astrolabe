using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Geometry;

namespace Astrolabe.Cli.Commands;

public static class ExportGltfCommand
{
    public static int Run(string[] args)
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
            string outputDir = Path.GetDirectoryName(outputPath) ?? ".";
            if (string.IsNullOrEmpty(outputDir)) outputDir = ".";
            string baseName = Path.GetFileNameWithoutExtension(outputPath);
            string meshOutputDir = Path.Combine(outputDir, baseName);
            Directory.CreateDirectory(meshOutputDir);

            Console.WriteLine($"Exporting {validMeshes.Count} meshes to {meshOutputDir}/...");

            // Build texture lookup from all extracted textures
            // Check both legacy "textures/" and new "output/textures/" locations
            var textureLookup = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            foreach (var textureBaseDir in new[] { "output/textures", "textures" })
            {
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
                    fileName += ".png";
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
}
