using Astrolabe.Core;
using Astrolabe.Core.Rete;
using Xunit;

namespace Astrolabe.Core.Tests;

public sealed class LevelTests
{
    [Fact]
    public void ExportToOpenSpace_RejectsLegacyManifestSchema()
    {
        var workspace = CreateWorkspace();
        try
        {
            var packageDir = Path.Combine(workspace, "astrolabe");
            Directory.CreateDirectory(packageDir);
            File.WriteAllText(
                Path.Combine(packageDir, OpenSpacePackageCodec.ManifestFileName),
                """
                {
                  "schema": "astrolabe.level-intermediate.v1",
                  "packageRole": "level",
                  "levelName": "astrolabe",
                  "snaFiles": [],
                  "relocationTables": [],
                  "looseFiles": []
                }
                """);

            var ex = Assert.Throws<InvalidDataException>(() =>
                Level.ExportToOpenSpace(packageDir, Path.Combine(workspace, "export")));
            Assert.Contains("Unsupported Rete manifest schema", ex.Message, StringComparison.OrdinalIgnoreCase);
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    [Fact]
    public void Load_RejectsLegacyManifestSchema()
    {
        var workspace = CreateWorkspace();
        try
        {
            var packageDir = Path.Combine(workspace, "astrolabe");
            Directory.CreateDirectory(packageDir);
            File.WriteAllText(
                Path.Combine(packageDir, OpenSpacePackageCodec.ManifestFileName),
                """
                {
                  "schema": "astrolabe.level-intermediate.v1",
                  "packageRole": "level",
                  "levelName": "astrolabe",
                  "snaFiles": [],
                  "relocationTables": [],
                  "looseFiles": []
                }
                """);

            var ex = Assert.Throws<InvalidDataException>(() => Level.Load(packageDir));
            Assert.Contains("Unsupported Rete manifest schema", ex.Message, StringComparison.OrdinalIgnoreCase);
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    [Fact]
    [Trait("Category", "Slow")]
    public void ImportFromOpenSpace_DelegatesToReteImporter()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            return;
        }

        var workspace = CreateWorkspace();
        try
        {
            var packageDir = Path.Combine(workspace, "astrolabe");
            var manifest = Level.ImportFromOpenSpace(levelDir, packageDir);

            Assert.Equal("astrolabe.rete.v1", manifest.Schema);
            Assert.Equal("level", manifest.PackageRole);
            Assert.True(manifest.SnaFiles.Count > 0);
            Assert.True(File.Exists(Path.Combine(packageDir, OpenSpacePackageCodec.ManifestFileName)));
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    [Fact]
    public void Load_WithoutSiblingFix_ThrowsWhenFixlvlListed()
    {
        var workspace = CreateWorkspace();
        try
        {
            var packageDir = Path.Combine(workspace, "level");
            Directory.CreateDirectory(packageDir);
            File.WriteAllText(
                Path.Combine(packageDir, OpenSpacePackageCodec.ManifestFileName),
                """
                {
                  "schema": "astrolabe.rete.v1",
                  "packageRole": "level",
                  "levelName": "test",
                  "sourceDirectoryName": "test",
                  "snaFiles": [
                    {
                      "fileName": "test.sna",
                      "blocks": [
                        {
                          "order": 0,
                          "key": "05:01",
                          "module": 5,
                          "id": 1,
                          "baseInMemory": 134217728,
                          "unk2": 0,
                          "unk3": 0,
                          "maxPosMinus9": 134217735,
                          "hasPayload": false
                        }
                      ]
                    }
                  ],
                  "relocationTables": [
                    { "fileName": "fixlvl.rtb" }
                  ],
                  "looseFiles": []
                }
                """);

            var ex = Assert.Throws<InvalidDataException>(() => Level.Load(packageDir));
            Assert.Contains("fixlvl.rtb requires a sibling Fix Rete package", ex.Message, StringComparison.OrdinalIgnoreCase);
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    [Fact]
    public void ExportToOpenSpace_WithoutSiblingFix_ThrowsWhenFixlvlListed()
    {
        var workspace = CreateWorkspace();
        try
        {
            var packageDir = Path.Combine(workspace, "level");
            Directory.CreateDirectory(packageDir);
            File.WriteAllText(
                Path.Combine(packageDir, OpenSpacePackageCodec.ManifestFileName),
                """
                {
                  "schema": "astrolabe.rete.v1",
                  "packageRole": "level",
                  "levelName": "test",
                  "sourceDirectoryName": "test",
                  "snaFiles": [
                    {
                      "fileName": "test.sna",
                      "blocks": [
                        {
                          "order": 0,
                          "key": "05:01",
                          "module": 5,
                          "id": 1,
                          "baseInMemory": 134217728,
                          "unk2": 0,
                          "unk3": 0,
                          "maxPosMinus9": 134217735,
                          "hasPayload": false
                        }
                      ]
                    }
                  ],
                  "relocationTables": [
                    { "fileName": "fixlvl.rtb" }
                  ],
                  "looseFiles": []
                }
                """);

            var ex = Assert.Throws<InvalidDataException>(() =>
                Level.ExportToOpenSpace(packageDir, Path.Combine(workspace, "export")));
            Assert.Contains("fixlvl.rtb requires a sibling Fix Rete package", ex.Message, StringComparison.OrdinalIgnoreCase);
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    private static string CreateWorkspace()
    {
        var workspace = Path.Combine(Path.GetTempPath(), "astrolabe-level-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(workspace);
        return workspace;
    }
}