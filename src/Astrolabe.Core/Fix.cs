using Astrolabe.Core.Hub;
using Astrolabe.Core.Rete;

namespace Astrolabe.Core;

public enum FixSourceKind
{
    Rete
}

/// <summary>
/// In-memory hub for canonical Fix data. URIs on disk, object references in memory.
/// </summary>
public sealed class Fix
{
    public string Name { get; }
    public FixSourceKind SourceKind { get; }
    public string SourcePath { get; }
    public RetePackageManifest Manifest { get; }
    public HubCatalog Catalog { get; }

    private Fix(
        string name,
        FixSourceKind sourceKind,
        string sourcePath,
        RetePackageManifest manifest,
        HubCatalog catalog)
    {
        Name = name;
        SourceKind = sourceKind;
        SourcePath = sourcePath;
        Manifest = manifest;
        Catalog = catalog;
    }

    public static Fix Load(string path)
    {
        var fullPath = Path.GetFullPath(path.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
        if (!Directory.Exists(fullPath))
        {
            throw new DirectoryNotFoundException($"Fix path not found: {fullPath}");
        }

        if (!OpenSpacePackageCodec.IsRetePackageDirectory(fullPath))
        {
            throw new InvalidDataException($"Fix path is not a Rete package: {fullPath}");
        }

        var manifest = OpenSpacePackageCodec.ReadReteManifest(fullPath);
        if (!manifest.PackageRole.Equals("fix", StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidDataException(
                $"Rete package at {fullPath} has packageRole '{manifest.PackageRole}'; expected 'fix'.");
        }

        var catalog = HubCatalog.Load(fullPath);
        return new Fix(manifest.LevelName, FixSourceKind.Rete, fullPath, manifest, catalog);
    }

    public void ExportToOpenSpace(string outputDir) =>
        OpenSpacePackageCodec.ExportFixFromHub(SourcePath, Manifest, Catalog, outputDir);
}