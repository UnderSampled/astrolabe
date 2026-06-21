using Astrolabe.Core;
using Astrolabe.Core.Hub;
using Astrolabe.Core.Rete;
using Xunit;

namespace Astrolabe.Core.Tests;

/// <summary>
/// Level hub tests that need a pre-imported astrolabe Rete package.
/// Import runs once via <see cref="AstrolabeDiscFixture"/>.
/// </summary>
[Trait("Category", "Disc")]
[Trait("Category", "Slow")]
[Collection("AstrolabeDisc")]
public sealed class LevelDiscTests(AstrolabeDiscFixture fixture)
{
    [Fact]
    public void Load_FromRetePackage_ReadsSceneGraph()
    {
        if (!fixture.IsAvailable)
        {
            Assert.Fail("Astrolabe disc fixture is required for Level disc tests.");
        }

        var openSpaceLevel = Level.Load(fixture.LevelDir);
        var reteLevel = Level.Load(fixture.PackageDir);

        Assert.Equal(openSpaceLevel.SceneGraph.AllNodes.Count, reteLevel.SceneGraph.AllNodes.Count);
        Assert.True(reteLevel.SceneGraph.AllNodes.Count > 0);
    }

    [Fact]
    public void Load_FromRetePackage_UsesHubWithoutLoader()
    {
        if (!fixture.IsAvailable)
        {
            Assert.Fail("Astrolabe disc fixture is required for Level disc tests.");
        }

        Assert.True(Directory.Exists(fixture.FixDir));
        Assert.True(File.Exists(Path.Combine(fixture.FixDir, OpenSpacePackageCodec.ManifestFileName)));

        var reteLevel = Level.Load(fixture.PackageDir);
        Assert.Equal("astrolabe", reteLevel.Name);
        Assert.Equal(LevelSourceKind.Rete, reteLevel.SourceKind);
        Assert.Null(reteLevel.Loader);
        Assert.NotNull(reteLevel.Catalog);
        Assert.NotNull(reteLevel.SiblingFix);
    }

    [Fact]
    public void Import_ProducesGeometricObjectEntries()
    {
        if (!fixture.IsAvailable)
        {
            Assert.Fail("Astrolabe disc fixture is required for Level disc tests.");
        }

        var catalog = HubCatalog.Load(fixture.PackageDir);
        var geoCount = catalog.GetElementsOfKind("geometricobject").Count();
        Assert.True(geoCount > 0, $"Expected geometricobject manifest entries, found {geoCount}.");
    }

    [Fact]
    public void ExportToGodot_FromRete_DoesNotThrow()
    {
        if (!fixture.IsAvailable)
        {
            Assert.Fail("Astrolabe disc fixture is required for Level disc tests.");
        }

        var godotDir = Path.Combine(fixture.WorkspaceDir, "godot");
        var level = Level.Load(fixture.PackageDir);
        var result = level.ExportToGodot(godotDir);

        Assert.True(result.ValidMeshCount > 0);
        Assert.True(result.ExportedMeshCount > 0);
        Assert.True(File.Exists(Path.Combine(godotDir, result.SceneFileName)));
        Assert.True(File.Exists(Path.Combine(godotDir, "project.godot")));
    }
}