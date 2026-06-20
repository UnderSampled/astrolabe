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
    public void Load_FromRetePackage_MatchesOpenSpaceRtbBlockCount()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            return;
        }

        var workspace = CreateWorkspace();
        try
        {
            var packageDir = Path.Combine(workspace, "astrolabe");
            Level.ImportFromOpenSpace(levelDir, packageDir);

            var openSpaceLevel = Level.Load(levelDir);
            var reteLevel = Level.Load(packageDir);

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
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    [Fact]
    public void Load_FromRetePackage_ReadsSceneGraph()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            return;
        }

        var workspace = CreateWorkspace();
        try
        {
            var packageDir = Path.Combine(workspace, "astrolabe");
            Level.ImportFromOpenSpace(levelDir, packageDir);

            var openSpaceLevel = Level.Load(levelDir);
            var reteLevel = Level.Load(packageDir);

            Assert.Equal(openSpaceLevel.SceneGraph.AllNodes.Count, reteLevel.SceneGraph.AllNodes.Count);
            Assert.True(reteLevel.SceneGraph.AllNodes.Count > 0);
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    [Fact]
    public void Load_FromRetePackage_ResolvesFixSibling()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            return;
        }

        var workspace = CreateWorkspace();
        try
        {
            var packageDir = Path.Combine(workspace, "astrolabe");
            Level.ImportFromOpenSpace(levelDir, packageDir);

            var fixDir = Path.Combine(workspace, "fix");
            Assert.True(Directory.Exists(fixDir));
            Assert.True(File.Exists(Path.Combine(fixDir, OpenSpacePackageCodec.ManifestFileName)));

            var reteLevel = Level.Load(packageDir);
            Assert.Equal("astrolabe", reteLevel.Name);
            Assert.Equal(LevelSourceKind.Rete, reteLevel.SourceKind);
            Assert.NotNull(reteLevel.Loader.Rtb);
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    [Fact]
    public void ExportToGodot_FromRete_DoesNotThrow()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            return;
        }

        var workspace = CreateWorkspace();
        try
        {
            var packageDir = Path.Combine(workspace, "astrolabe");
            var godotDir = Path.Combine(workspace, "godot");
            Level.ImportFromOpenSpace(levelDir, packageDir);

            var level = Level.Load(packageDir);
            var result = level.ExportToGodot(godotDir);

            Assert.True(result.ValidMeshCount > 0);
            Assert.True(result.ExportedMeshCount > 0);
            Assert.True(File.Exists(Path.Combine(godotDir, result.SceneFileName)));
            Assert.True(File.Exists(Path.Combine(godotDir, "project.godot")));
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