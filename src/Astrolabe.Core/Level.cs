using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Geometry;
using Astrolabe.Core.FileFormats.Godot;
using Astrolabe.Core.Hub;
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
    public RetePackageManifest? Manifest { get; }
    public HubCatalog? Catalog { get; }
    public Fix? SiblingFix { get; }
    public LevelLoader? Loader { get; }
    public TextureTable? TextureTable { get; }

    private Level(
        string name,
        LevelSourceKind sourceKind,
        string sourcePath,
        SceneGraph sceneGraph,
        RetePackageManifest? manifest,
        HubCatalog? catalog,
        Fix? siblingFix,
        LevelLoader? loader,
        TextureTable? textureTable)
    {
        Name = name;
        SourceKind = sourceKind;
        SourcePath = sourcePath;
        SceneGraph = sceneGraph;
        Manifest = manifest;
        Catalog = catalog;
        SiblingFix = siblingFix;
        Loader = loader;
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

    public void ExportToOpenSpace(string levelDir) =>
        OpenSpacePackageCodec.ExportLevelFromHub(this, levelDir);

    public static void ExportToOpenSpace(string reteDir, string levelDir) =>
        Load(reteDir).ExportToOpenSpace(levelDir);

    public IReadOnlyList<MeshData> ScanMeshes()
    {
        if (Catalog != null)
        {
            return new HubMeshScanner(Catalog, TextureTable).ScanForMeshes();
        }

        if (Loader == null)
        {
            return [];
        }

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
        var sceneGraph = ReadSceneGraphFromOpenSpace(loader, levelDir, levelName);

        return new Level(
            levelName,
            LevelSourceKind.OpenSpace,
            levelDir,
            sceneGraph,
            manifest: null,
            catalog: null,
            siblingFix: null,
            loader,
            textureTable);
    }

    private static Level LoadFromRete(string packageDir)
    {
        var manifest = OpenSpacePackageCodec.ReadReteManifest(packageDir);
        if (!manifest.PackageRole.Equals("level", StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidDataException(
                $"Rete package at {packageDir} has packageRole '{manifest.PackageRole}'; expected 'level'.");
        }

        ValidateSiblingFixPackage(packageDir, manifest);

        var catalog = HubCatalog.Load(packageDir);
        var sceneGraph = ReteSceneLoader.Load(packageDir, catalog);
        var siblingFix = TryLoadSiblingFix(packageDir);
        var textureTable = TryLoadReteTextureTable(packageDir, manifest, catalog);

        return new Level(
            manifest.LevelName,
            LevelSourceKind.Rete,
            packageDir,
            sceneGraph,
            manifest,
            catalog,
            siblingFix,
            loader: null,
            textureTable);
    }

    private static void ValidateSiblingFixPackage(string levelPackageDir, RetePackageManifest manifest)
    {
        if (!manifest.RelocationTables.Any(table =>
                table.FileName.Equals("fixlvl.rtb", StringComparison.OrdinalIgnoreCase)))
        {
            return;
        }

        var fixDir = Path.Combine(Path.GetDirectoryName(levelPackageDir)!, "fix");
        if (!OpenSpacePackageCodec.IsRetePackageDirectory(fixDir))
        {
            throw new InvalidDataException("fixlvl.rtb requires a sibling Fix Rete package.");
        }
    }

    private static Fix? TryLoadSiblingFix(string levelPackageDir)
    {
        var fixDir = Path.Combine(Path.GetDirectoryName(levelPackageDir)!, "fix");
        if (!OpenSpacePackageCodec.IsRetePackageDirectory(fixDir))
        {
            return null;
        }

        try
        {
            return Fix.Load(fixDir);
        }
        catch (InvalidDataException)
        {
            return null;
        }
    }

    private static TextureTable? TryLoadReteTextureTable(
        string packageDir,
        RetePackageManifest manifest,
        HubCatalog catalog)
    {
        var ptxPath = OpenSpacePackageCodec.FindLooseFilePath(manifest, packageDir, $"{manifest.LevelName}.ptx");
        if (ptxPath == null || !File.Exists(ptxPath))
        {
            return null;
        }

        return new TextureTable(catalog, ptxPath);
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

    private static SceneGraph ReadSceneGraphFromOpenSpace(LevelLoader loader, string rootDir, string levelName)
    {
        var gptPath = FindOpenSpaceFile(rootDir, $"{levelName}.gpt");
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