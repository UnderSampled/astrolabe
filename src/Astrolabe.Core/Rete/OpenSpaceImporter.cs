using System.Text.Json;

namespace Astrolabe.Core.Rete;

public static class OpenSpaceImporter
{
    public static RetePackageManifest ImportLevel(string levelDir, string outputDir) =>
        OpenSpacePackageCodec.ImportLevel(levelDir, outputDir);

    /// <summary>
    /// Run animation tree aggregation on an already-imported level package
    /// (families + transforms + content segment rewrite).
    /// </summary>
    public static void AggregateAnimationTree(string packageDir)
    {
        var manifestPath = Path.Combine(packageDir, OpenSpacePackageCodec.ManifestFileName);
        var options = new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true,
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase
        };
        var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                           File.ReadAllText(manifestPath), options)
                       ?? throw new InvalidDataException($"Could not read {manifestPath}");
        AnimationTreeImporter.AggregateLevelPackage(packageDir, manifest);
    }

    /// <summary>
    /// Collapse remaining non-animation domains into dual-layer semantic trees
    /// (scene, geometry, AI, characters, sectors, sidecars).
    /// </summary>
    public static void AggregateSemanticTrees(string packageDir)
    {
        var manifestPath = Path.Combine(packageDir, OpenSpacePackageCodec.ManifestFileName);
        var options = new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true,
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase
        };
        var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                           File.ReadAllText(manifestPath), options)
                       ?? throw new InvalidDataException($"Could not read {manifestPath}");
        SemanticDomainAggregator.AggregateAll(packageDir, manifest);
    }
}