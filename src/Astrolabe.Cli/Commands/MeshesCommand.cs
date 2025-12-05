using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Geometry;

namespace Astrolabe.Cli.Commands;

public static class MeshesCommand
{
    public static int Run(string[] args)
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
}
