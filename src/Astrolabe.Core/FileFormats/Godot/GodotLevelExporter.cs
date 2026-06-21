using System.Security.Cryptography;
using System.Text;
using Astrolabe.Core.FileFormats.Geometry;

namespace Astrolabe.Core.FileFormats.Godot;

public sealed record GodotExportResult(int ValidMeshCount, int ExportedMeshCount, string SceneFileName);

public static class GodotLevelExporter
{
    public static IReadOnlyList<MeshData> FilterValidMeshes(IEnumerable<MeshData> meshes) =>
        meshes
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

    public static GodotExportResult Export(Level level, string outputDir, IEnumerable<string>? textureSearchRoots = null)
    {
        var validMeshes = FilterValidMeshes(level.ScanMeshes());

        var meshDir = Path.Combine(outputDir, "meshes");
        var texturesDir = Path.Combine(outputDir, "textures");
        Directory.CreateDirectory(outputDir);
        Directory.CreateDirectory(meshDir);
        Directory.CreateDirectory(texturesDir);

        var textureLookup = BuildTextureLookup(textureSearchRoots);
        var copiedTextures = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        var textureFileClaims = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);

        Func<string?, string?> lookupGodotTexture = texName =>
        {
            var texturePath = LookupTexture(textureLookup, texName);
            return texturePath == null
                ? null
                : CopyTextureForGodot(texturePath, texturesDir, copiedTextures, textureFileClaims);
        };

        var geoAddrToMesh = new Dictionary<int, MeshData>();
        foreach (var mesh in validMeshes)
        {
            int? geoAddr = mesh.SourceBlock != null
                ? mesh.SourceBlock.BaseInMemory + mesh.SourceOffset
                : mesh.VirtualAddress != 0
                    ? mesh.VirtualAddress
                    : null;

            if (geoAddr.HasValue)
            {
                geoAddrToMesh[geoAddr.Value] = mesh;
            }
        }

        var geoAddrToMeshName = new Dictionary<int, string>();
        foreach (var (geoAddr, mesh) in geoAddrToMesh)
        {
            string meshFileName = $"mesh_{geoAddr:X8}";
            string meshPath = Path.Combine(meshDir, $"{meshFileName}.tres");
            GodotMeshExporter.ExportMesh(mesh, meshPath, lookupGodotTexture);
            geoAddrToMeshName[geoAddr] = meshFileName;
        }

        var godotExporter = new GodotExporter();
        var tscnFileName = $"{level.Name}.tscn";
        var tscnPath = Path.Combine(outputDir, tscnFileName);
        godotExporter.Export(level.SceneGraph, tscnPath, "meshes", geoAddrToMeshName);
        GodotExporter.WriteProjectFile(outputDir, level.Name, tscnFileName);

        return new GodotExportResult(validMeshes.Count, geoAddrToMeshName.Count, tscnFileName);
    }

    private static Dictionary<string, string> BuildTextureLookup(IEnumerable<string>? textureSearchRoots)
    {
        var textureLookup = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        var roots = textureSearchRoots?.ToList() ??
        [
            "output/Gamedata/Textures",
            "output/textures",
            "textures"
        ];

        foreach (var textureBaseDir in roots)
        {
            if (!Directory.Exists(textureBaseDir))
            {
                continue;
            }

            foreach (var file in Directory.EnumerateFiles(textureBaseDir, "*.tga", SearchOption.AllDirectories))
            {
                var fileName = Path.GetFileName(file);
                textureLookup.TryAdd(fileName, file);
            }

            foreach (var file in Directory.EnumerateFiles(textureBaseDir, "*.png", SearchOption.AllDirectories))
            {
                var fileName = Path.GetFileName(file);
                textureLookup.TryAdd(fileName, file);
            }
        }

        return textureLookup;
    }

    private static string? LookupTexture(IReadOnlyDictionary<string, string> textureLookup, string? texName)
    {
        if (string.IsNullOrEmpty(texName))
        {
            return null;
        }

        string fileName = Path.GetFileName(texName);
        if (!fileName.EndsWith(".tga", StringComparison.OrdinalIgnoreCase) &&
            !fileName.EndsWith(".png", StringComparison.OrdinalIgnoreCase))
        {
            fileName += ".png";
        }

        if (textureLookup.TryGetValue(fileName, out var foundPath))
        {
            return foundPath;
        }

        var pngName = Path.ChangeExtension(fileName, ".png");
        return textureLookup.TryGetValue(pngName, out foundPath) ? foundPath : null;
    }

    private static string CopyTextureForGodot(
        string sourcePath,
        string texturesDir,
        Dictionary<string, string> copiedTextures,
        Dictionary<string, string> textureFileClaims)
    {
        var fullSourcePath = Path.GetFullPath(sourcePath);
        if (copiedTextures.TryGetValue(fullSourcePath, out var resourcePath))
        {
            return resourcePath;
        }

        var fileName = Path.GetFileName(sourcePath);
        if (textureFileClaims.TryGetValue(fileName, out var claimedSource) &&
            !string.Equals(claimedSource, fullSourcePath, StringComparison.OrdinalIgnoreCase))
        {
            var stem = Path.GetFileNameWithoutExtension(fileName);
            var extension = Path.GetExtension(fileName);
            fileName = $"{stem}_{ShortHash(fullSourcePath)}{extension}";
        }

        textureFileClaims[fileName] = fullSourcePath;
        Directory.CreateDirectory(texturesDir);
        var destinationPath = Path.Combine(texturesDir, fileName);
        File.Copy(fullSourcePath, destinationPath, overwrite: true);

        resourcePath = $"res://textures/{fileName.Replace('\\', '/')}";
        copiedTextures[fullSourcePath] = resourcePath;
        return resourcePath;
    }

    private static string ShortHash(string value)
    {
        var hash = SHA1.HashData(Encoding.UTF8.GetBytes(value));
        return Convert.ToHexString(hash, 0, 4).ToLowerInvariant();
    }
}