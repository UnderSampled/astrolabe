using Astrolabe.Core;
using Astrolabe.Core.Hub;
using Astrolabe.Core.Rete;
using Xunit;

namespace Astrolabe.Core.Tests;

[Trait("Category", "Slow")]
public sealed class HubCatalogLazyTests
{
    [Fact]
    public void Load_IndexesWithoutHydratingRecords()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            Assert.Fail("Astrolabe disc fixture is required for hub lazy-loading tests.");
        }

        var workspace = Path.Combine(Path.GetTempPath(), "astrolabe-hub-lazy-" + Guid.NewGuid().ToString("N"));
        try
        {
            var packageDir = Path.Combine(workspace, "rete");
            OpenSpaceImporter.ImportLevel(levelDir, packageDir);

            var catalog = HubCatalog.Load(packageDir);

            Assert.True(catalog.Elements.Count > 0);
            Assert.All(catalog.Elements, element => Assert.False(element.IsHydrated));
        }
        finally
        {
            if (Directory.Exists(workspace))
            {
                Directory.Delete(workspace, recursive: true);
            }
        }
    }

    [Fact]
    public void TryHydrate_LoadsSingleRecordOnDemand()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            Assert.Fail("Astrolabe disc fixture is required for hub lazy-loading tests.");
        }

        var workspace = Path.Combine(Path.GetTempPath(), "astrolabe-hub-lazy-" + Guid.NewGuid().ToString("N"));
        try
        {
            var packageDir = Path.Combine(workspace, "rete");
            OpenSpaceImporter.ImportLevel(levelDir, packageDir);

            var catalog = HubCatalog.Load(packageDir);
            var stub = catalog.Elements.First(element =>
                element.Kind is "ipo" or "physicalobject" or "visualmaterial" or "gamematerial");

            Assert.False(stub.IsHydrated);
            var hydratedBefore = catalog.Elements.Count(element => element.IsHydrated);
            Assert.True(catalog.TryHydrate(stub));
            Assert.True(stub.IsHydrated);
            Assert.NotNull(stub.Value);
            Assert.True(catalog.Elements.Count(element => element.IsHydrated) < catalog.Elements.Count);
            Assert.True(catalog.Elements.Count(element => element.IsHydrated) > hydratedBefore);
        }
        finally
        {
            if (Directory.Exists(workspace))
            {
                Directory.Delete(workspace, recursive: true);
            }
        }
    }

    [Fact]
    public void Import_ProducesGeometricObjectManifestEntries()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            Assert.Fail("Astrolabe disc fixture is required for geometricobject import tests.");
        }

        var workspace = Path.Combine(Path.GetTempPath(), "astrolabe-geo-import-" + Guid.NewGuid().ToString("N"));
        try
        {
            var packageDir = Path.Combine(workspace, "rete");
            OpenSpaceImporter.ImportLevel(levelDir, packageDir);

            var catalog = HubCatalog.Load(packageDir);
            var geoCount = catalog.GetElementsOfKind("geometricobject").Count();

            Assert.True(geoCount > 0, $"Expected geometricobject manifest entries after import, found {geoCount}.");
        }
        finally
        {
            if (Directory.Exists(workspace))
            {
                Directory.Delete(workspace, recursive: true);
            }
        }
    }

    [Fact]
    public void ScanMeshes_FromRetePackage_FindsGeometricMeshes()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            Assert.Fail("Astrolabe disc fixture is required for hub mesh scan tests.");
        }

        var workspace = Path.Combine(Path.GetTempPath(), "astrolabe-hub-mesh-" + Guid.NewGuid().ToString("N"));
        try
        {
            var packageDir = Path.Combine(workspace, "rete");
            OpenSpaceImporter.ImportLevel(levelDir, packageDir);

            var level = Level.Load(packageDir);
            var meshes = level.ScanMeshes();

            Assert.NotEmpty(meshes);
            Assert.Contains(meshes, mesh => mesh.VirtualAddress != 0 && mesh.Vertices.Length >= 3);
        }
        finally
        {
            if (Directory.Exists(workspace))
            {
                Directory.Delete(workspace, recursive: true);
            }
        }
    }
}