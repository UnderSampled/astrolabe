using Astrolabe.Core.Rete;
using Xunit;

namespace Astrolabe.Core.Tests;

public sealed class ReferenceUriTests
{
    [Fact]
    public void TryResolve_FixScheme_FromLevelPackage()
    {
        var workspace = CreateWorkspace();
        try
        {
            var levelDir = Path.Combine(workspace, "astrolabe");
            var fixDir = Path.Combine(workspace, "fix");
            CreatePackage(levelDir, "level");
            CreatePackage(fixDir, "fix");
            var target = Path.Combine(fixDir, "types", "raw", "target.json");
            Directory.CreateDirectory(Path.GetDirectoryName(target)!);
            File.WriteAllText(target, "{}");

            Assert.True(ReferenceUri.TryResolve(
                levelDir,
                "fix:/types/raw/target.json",
                out var resolved,
                out var fragment));
            Assert.Null(fragment);
            Assert.Equal(target, resolved);
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    [Fact]
    public void TryResolve_LegacyRelativeFix_StillWorks()
    {
        var workspace = CreateWorkspace();
        try
        {
            var levelDir = Path.Combine(workspace, "astrolabe");
            var fixDir = Path.Combine(workspace, "fix");
            CreatePackage(levelDir, "level");
            CreatePackage(fixDir, "fix");
            var target = Path.Combine(fixDir, "types", "raw", "target.json");
            Directory.CreateDirectory(Path.GetDirectoryName(target)!);
            File.WriteAllText(target, "{}");

            Assert.True(ReferenceUri.TryResolve(
                levelDir,
                "../fix/types/raw/target.json",
                out var resolved,
                out _));
            Assert.Equal(target, resolved);
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    [Fact]
    public void TryResolve_LevelScheme_FromLevelPackage()
    {
        var workspace = CreateWorkspace();
        try
        {
            var levelDir = Path.Combine(workspace, "astrolabe");
            CreatePackage(levelDir, "level");
            var target = Path.Combine(levelDir, "slots", "0x0262C4C0.json");
            Directory.CreateDirectory(Path.GetDirectoryName(target)!);
            File.WriteAllText(target, "{}");

            Assert.True(ReferenceUri.TryResolve(
                levelDir,
                "level:/slots/0x0262C4C0.json",
                out var resolved,
                out _));
            Assert.Equal(target, resolved);
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    [Fact]
    public void TryResolve_LevelScheme_FromFixPackage_WithExplicitLevelRoot()
    {
        var workspace = CreateWorkspace();
        try
        {
            var levelDir = Path.Combine(workspace, "astrolabe");
            var fixDir = Path.Combine(workspace, "fix");
            CreatePackage(levelDir, "level");
            CreatePackage(fixDir, "fix");
            var target = Path.Combine(levelDir, "slots", "0x0262C4C0.json");
            Directory.CreateDirectory(Path.GetDirectoryName(target)!);
            File.WriteAllText(target, "{}");

            Assert.True(ReferenceUri.TryResolve(
                fixDir,
                "level:/slots/0x0262C4C0.json",
                out var resolved,
                out _,
                levelPackageRoot: levelDir));
            Assert.Equal(target, resolved);
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    [Fact]
    public void MakeReference_EmitsFixScheme_ForSiblingFixTarget()
    {
        var workspace = CreateWorkspace();
        try
        {
            var levelDir = Path.Combine(workspace, "astrolabe");
            var fixDir = Path.Combine(workspace, "fix");
            CreatePackage(levelDir, "level");
            CreatePackage(fixDir, "fix");
            var target = Path.Combine(fixDir, "types", "raw", "target.json");

            var uri = ReferenceUri.MakeReference(levelDir, target);

            Assert.Equal("fix:/types/raw/target.json", uri);
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    [Fact]
    public void MakeReference_EmitsLevelScheme_ForSamePackageTarget()
    {
        var workspace = CreateWorkspace();
        try
        {
            var levelDir = Path.Combine(workspace, "astrolabe");
            CreatePackage(levelDir, "level");
            var target = Path.Combine(levelDir, "types", "objectlist", "default.json");

            var uri = ReferenceUri.MakeReference(levelDir, target);

            Assert.Equal("types/objectlist/default.json", uri);
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    [Fact]
    public void MakeReference_EmitsLevelScheme_FromFixReferrer()
    {
        var workspace = CreateWorkspace();
        try
        {
            var levelDir = Path.Combine(workspace, "astrolabe");
            var fixDir = Path.Combine(workspace, "fix");
            CreatePackage(levelDir, "level");
            CreatePackage(fixDir, "fix");
            var target = Path.Combine(levelDir, "slots", "0x0262C4C0.json");

            var uri = ReferenceUri.MakeReference(fixDir, target);

            Assert.Equal("level:/slots/0x0262C4C0.json", uri);
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    [Fact]
    public void TryResolve_PreservesByteOffsetFragment()
    {
        var workspace = CreateWorkspace();
        try
        {
            var levelDir = Path.Combine(workspace, "astrolabe");
            CreatePackage(levelDir, "level");
            var target = Path.Combine(levelDir, "types", "raw", "target.json");
            Directory.CreateDirectory(Path.GetDirectoryName(target)!);
            File.WriteAllText(target, "{}");

            Assert.True(ReferenceUri.TryResolve(
                levelDir,
                "level:/types/raw/target.json#byteOffset=4",
                out var resolved,
                out var fragment));
            Assert.Equal(target, resolved);
            Assert.Equal("byteOffset=4", fragment);
        }
        finally
        {
            Directory.Delete(workspace, true);
        }
    }

    private static string CreateWorkspace()
    {
        var workspace = Path.Combine(Path.GetTempPath(), "astrolabe-ref-uri-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(workspace);
        return workspace;
    }

    private static void CreatePackage(string packageDir, string role)
    {
        Directory.CreateDirectory(packageDir);
        File.WriteAllText(
            Path.Combine(packageDir, OpenSpacePackageCodec.ManifestFileName),
            $$"""
            {
              "schema": "astrolabe.rete.v1",
              "packageRole": "{{role}}",
              "levelName": "test",
              "snaFiles": [],
              "relocationTables": [],
              "looseFiles": []
            }
            """);
    }
}