using Astrolabe.Core;
using Astrolabe.Core.Rete;
using Xunit;

namespace Astrolabe.Core.Tests;

/// <summary>
/// Level hub tests that need a pre-imported astrolabe Rete package.
/// Import runs once via <see cref="AstrolabeDiscFixture"/>.
/// </summary>
[Trait("Category", "Disc")]
[Collection("AstrolabeDisc")]
public sealed class LevelDiscTests(AstrolabeDiscFixture fixture)
{
    [Fact]
    public void Load_FromRetePackage_MatchesOpenSpaceRtbBlockCount()
    {
        if (!fixture.IsAvailable)
        {
            return;
        }

        var openSpaceLevel = Level.Load(fixture.LevelDir);
        var reteLevel = Level.Load(fixture.PackageDir);

        Assert.NotNull(openSpaceLevel.Loader.Rtb);
        Assert.NotNull(reteLevel.Loader.Rtb);

        var openSpaceBlocks = openSpaceLevel.Loader.Rtb.PointerBlocks
            .Select(block => block.Key)
            .OrderBy(key => key)
            .ToList();
        var reteBlocks = reteLevel.Loader.Rtb.PointerBlocks
            .Select(block => block.Key)
            .OrderBy(key => key)
            .ToList();

        Assert.Equal(openSpaceBlocks, reteBlocks);
        Assert.Equal(openSpaceLevel.Loader.Rtb.PointerBlocks.Count, reteLevel.Loader.Rtb.PointerBlocks.Count);
    }

    [Fact]
    public void Load_FromRetePackage_ReadsSceneGraph()
    {
        if (!fixture.IsAvailable)
        {
            return;
        }

        var openSpaceLevel = Level.Load(fixture.LevelDir);
        var reteLevel = Level.Load(fixture.PackageDir);

        Assert.Equal(openSpaceLevel.SceneGraph.AllNodes.Count, reteLevel.SceneGraph.AllNodes.Count);
        Assert.True(reteLevel.SceneGraph.AllNodes.Count > 0);
    }

    [Fact]
    public void Load_FromRetePackage_ResolvesFixSibling()
    {
        if (!fixture.IsAvailable)
        {
            return;
        }

        Assert.True(Directory.Exists(fixture.FixDir));
        Assert.True(File.Exists(Path.Combine(fixture.FixDir, OpenSpacePackageCodec.ManifestFileName)));

        var reteLevel = Level.Load(fixture.PackageDir);
        Assert.Equal("astrolabe", reteLevel.Name);
        Assert.Equal(LevelSourceKind.Rete, reteLevel.SourceKind);
        Assert.NotNull(reteLevel.Loader.Rtb);
    }

    [Fact]
    public void ExportToGodot_FromRete_DoesNotThrow()
    {
        if (!fixture.IsAvailable)
        {
            return;
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