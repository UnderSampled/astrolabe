using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Geometry;
using Astrolabe.Core.FileFormats.Godot;
using Astrolabe.Core.Rete;

namespace Astrolabe.Core;

public enum LevelSourceKind
{
    OpenSpace,
    Rete
}

/// <summary>
/// In-memory hub for canonical level data. Fix is a separate hub/package (see plan.md); cross-package
/// pointers use fix:/ and level:/ URIs. Import/export and Godot pipelines hydrate level data through Level.
/// </summary>
public sealed class Level
{
    public string Name { get; }
    public LevelSourceKind SourceKind { get; }
    public string SourcePath { get; }
    public SceneGraph SceneGraph { get; }
    public LevelLoader Loader { get; }
    public TextureTable? TextureTable { get; }

    private Level(
        string name,
        LevelSourceKind sourceKind,
        string sourcePath,
        LevelLoader loader,
        SceneGraph sceneGraph,
        TextureTable? textureTable)
    {
        Name = name;
        SourceKind = sourceKind;
        SourcePath = sourcePath;
        Loader = loader;
        SceneGraph = sceneGraph;
        TextureTable = textureTable;
    }

    public static Level Load(string path)
    {
        var fullPath = Path.GetFullPath(path.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
        if (!Directory.Exists(fullPath))
        {
            throw new DirectoryNotFoundException($"Level path not found: {fullPath}");
        }

        return OpenSpacePackageCodec.IsRetePackageDirectory(fullPath)
            ? LoadFromRete(fullPath)
            : LoadFromOpenSpace(fullPath);
    }

    public static RetePackageManifest ImportFromOpenSpace(string levelDir, string reteDir) =>
        OpenSpacePackageCodec.ImportLevel(levelDir, reteDir);

    public static void ExportToOpenSpace(string reteDir, string levelDir) =>
        OpenSpacePackageCodec.ExportLevel(reteDir, levelDir);

    public IReadOnlyList<MeshData> ScanMeshes()
    {
        var scanner = new MeshScanner(Loader, TextureTable);
        return scanner.ScanForMeshes();
    }

    public IReadOnlyList<MeshData> GetValidMeshes() =>
        GodotLevelExporter.FilterValidMeshes(ScanMeshes());

    public GodotExportResult ExportToGodot(string outputDir, IEnumerable<string>? textureSearchRoots = null) =>
        GodotLevelExporter.Export(this, outputDir, textureSearchRoots);

    private static Level LoadFromOpenSpace(string levelDir)
    {
        var levelName = Path.GetFileName(levelDir);
        var loader = new LevelLoader(levelDir, levelName);
        var textureTable = TryLoadTextureTable(loader, levelDir, levelName, null);
        var sceneGraph = ReadSceneGraph(loader, levelDir, levelName, null);

        return new Level(levelName, LevelSourceKind.OpenSpace, levelDir, loader, sceneGraph, textureTable);
    }

    private static Level LoadFromRete(string packageDir)
    {
        var hydration = OpenSpacePackageCodec.HydrateFromRetePackage(packageDir);
        var manifest = hydration.Manifest;
        var loader = new LevelLoader(hydration.Sna, hydration.Rtb, hydration.Rtp, hydration.Rtt);
        var textureTable = TryLoadTextureTable(loader, packageDir, manifest.LevelName, manifest);
        var sceneGraph = ReadSceneGraph(loader, packageDir, manifest.LevelName, manifest);

        return new Level(
            manifest.LevelName,
            LevelSourceKind.Rete,
            packageDir,
            loader,
            sceneGraph,
            textureTable);
    }

    private static TextureTable? TryLoadTextureTable(
        LevelLoader loader,
        string rootDir,
        string levelName,
        RetePackageManifest? manifest)
    {
        var ptxPath = manifest == null
            ? FindOpenSpaceFile(rootDir, $"{levelName}.ptx")
            : OpenSpacePackageCodec.FindLooseFilePath(manifest, rootDir, $"{levelName}.ptx")
                ?? manifest.LooseFiles
                    .Select(file => OpenSpacePackageCodec.FindLooseFilePath(manifest, rootDir, file.FileName))
                    .FirstOrDefault(path =>
                        path != null &&
                        fileNameEndsWith(path, ".ptx"));

        if (ptxPath == null || !File.Exists(ptxPath))
        {
            return null;
        }

        return new TextureTable(loader, ptxPath);
    }

    private static SceneGraph ReadSceneGraph(
        LevelLoader loader,
        string rootDir,
        string levelName,
        RetePackageManifest? manifest)
    {
        var gptPath = manifest == null
            ? FindOpenSpaceFile(rootDir, $"{levelName}.gpt")
            : OpenSpacePackageCodec.FindLooseFilePath(manifest, rootDir, $"{levelName}.gpt");

        if (gptPath == null || !File.Exists(gptPath))
        {
            throw new FileNotFoundException($"GPT file not found for level {levelName}.");
        }

        var gpt = new GptReader(gptPath);
        var memory = new MemoryContext(loader.Sna, loader.Rtb);
        var sceneReader = new SuperObjectReader(memory);
        return sceneReader.ReadSceneGraph(gpt);
    }

    private static string? FindOpenSpaceFile(string dir, string fileName)
    {
        var exact = Path.Combine(dir, fileName);
        if (File.Exists(exact))
        {
            return exact;
        }

        if (fileName.EndsWith('*'))
        {
            return Directory.GetFiles(dir, fileName, SearchOption.TopDirectoryOnly).FirstOrDefault();
        }

        return Directory.GetFiles(dir, fileName + "*", SearchOption.TopDirectoryOnly).FirstOrDefault();
    }

    private static bool fileNameEndsWith(string path, string extension) =>
        Path.GetFileName(path).EndsWith(extension, StringComparison.OrdinalIgnoreCase);
}