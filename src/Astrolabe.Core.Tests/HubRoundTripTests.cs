using Astrolabe.Core;
using Astrolabe.Core.Hub;
using Astrolabe.Core.Rete;
using Xunit;

namespace Astrolabe.Core.Tests;

[Trait("Category", "Disc")]
[Trait("Category", "Slow")]
[Collection("AstrolabeDisc")]
public sealed class HubRoundTripTests(AstrolabeDiscFixture fixture)
{
    [Fact]
    public void Load_FromRete_DoesNotHydrateVmLayout()
    {
        if (!fixture.IsAvailable)
        {
            Assert.Fail("Astrolabe disc fixture is required for hub round-trip tests.");
        }

        var level = Level.Load(fixture.PackageDir);

        Assert.Equal(LevelSourceKind.Rete, level.SourceKind);
        Assert.Null(level.Loader);
        Assert.NotNull(level.Catalog);
        Assert.True(level.Catalog!.Elements.Count > 0);
        Assert.All(level.Catalog.Elements, element => Assert.False(element.IsHydrated));
    }

    [Fact]
    public void Import_Load_Export_WithoutVmRehydrationOnLoad()
    {
        if (!fixture.IsAvailable)
        {
            Assert.Fail("Astrolabe disc fixture is required for hub round-trip tests.");
        }

        var level = Level.Load(fixture.PackageDir);
        Assert.All(level.Catalog!.Elements, element => Assert.False(element.IsHydrated));

        var exportDir = Path.Combine(fixture.WorkspaceDir, "hub-export");
        level.ExportToOpenSpace(exportDir);

        Assert.True(File.Exists(Path.Combine(exportDir, "astrolabe.sna")));
        Assert.True(File.Exists(Path.Combine(exportDir, "astrolabe.rtb")));
        Assert.True(Directory.GetFiles(exportDir).Length > 0);
    }

    [Fact]
    public void HubCatalog_ResolvesTypedReferences()
    {
        if (!fixture.IsAvailable)
        {
            Assert.Fail("Astrolabe disc fixture is required for hub round-trip tests.");
        }

        var catalog = HubCatalog.Load(fixture.PackageDir);
        Assert.True(catalog.Elements.Count > 0);
        var promoted = catalog.Elements.FirstOrDefault(element =>
            element.Kind is "ipo" or "physicalobject" or "visualmaterial" or "gamematerial");
        Assert.NotNull(promoted);
        Assert.False(string.IsNullOrWhiteSpace(promoted!.DataPath));
        Assert.False(promoted.IsHydrated);
        Assert.True(catalog.TryHydrate(promoted));
        Assert.True(promoted.IsHydrated);
    }

    [Fact]
    public void Fix_LoadsAsSeparateHub()
    {
        if (!fixture.IsAvailable)
        {
            Assert.Fail("Astrolabe disc fixture is required for hub round-trip tests.");
        }

        var fix = Fix.Load(fixture.FixDir);
        Assert.Equal(FixSourceKind.Rete, fix.SourceKind);
        Assert.NotNull(fix.Catalog);
        Assert.True(fix.Catalog.Elements.Count > 0);
        Assert.All(fix.Catalog.Elements, element => Assert.False(element.IsHydrated));
    }
}