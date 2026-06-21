using Astrolabe.Core.FileFormats;
using Astrolabe.Core.Hub;
using Astrolabe.Core.Rete;
using Astrolabe.Core.Serialization.Codecs;
using Xunit;

namespace Astrolabe.Core.Tests;

public sealed class GeometricObjectImportDebugTests
{
    [Fact]
    public void GeometricObject_ElementTypesPointer_CanBeInlineInHeader()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            return;
        }

        var loader = new LevelLoader(levelDir, "astrolabe");
        var block = loader.Sna.Blocks.First(b => b.Module == 0x06 && b.Id == 0x02);
        var data = block.Data ?? throw new InvalidOperationException("Block data missing.");
        const int geoOffset = 691504;
        var geo = GeometricObjectCodec.Instance.Read(data, geoOffset, GeometricObjectCodec.Size);
        var typesAddress = HubReferenceIO.Materialize(geo.ElementTypes);
        var geoAddress = block.BaseInMemory + geoOffset;

        Assert.True(typesAddress >= geoAddress);
        Assert.True(typesAddress < geoAddress + GeometricObjectCodec.Size);
        Assert.True(geo.NumElements > 0);
        Assert.Equal(52, typesAddress - geoAddress);
        Assert.Equal(checked((int)(geo.NumElements * 2)), 6);
        Assert.False(geo.ElementTypes.IsNull);
    }

    [Fact]
    [Trait("Category", "Slow")]
    public void Import_CarvesInlineElementTypesForGeometricObject()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            return;
        }

        var workspace = Path.Combine(Path.GetTempPath(), "astrolabe-geo-carve-" + Guid.NewGuid().ToString("N"));
        try
        {
            var packageDir = Path.Combine(workspace, "rete");
            OpenSpaceImporter.ImportLevel(levelDir, packageDir);

            var elementTypeCount = Directory.Exists(Path.Combine(packageDir, "types", "elementtypes"))
                ? Directory.GetFiles(Path.Combine(packageDir, "types", "elementtypes"), "*.json").Length
                : 0;

            Assert.True(elementTypeCount > 0, $"Expected carved elementtypes entries, found {elementTypeCount}.");
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