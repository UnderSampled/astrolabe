using Astrolabe.Core.Rete;
using Xunit;

namespace Astrolabe.Core.Tests;

[CollectionDefinition("AstrolabeDisc")]
public sealed class AstrolabeDiscCollection : ICollectionFixture<AstrolabeDiscFixture>;

public sealed class AstrolabeDiscFixture : IDisposable
{
    private readonly string? _workspaceDir;

    public bool IsAvailable { get; }

    public string LevelDir { get; }

    public string PackageDir { get; }

    public string ExportDir { get; }

    public AstrolabeDiscFixture()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            IsAvailable = false;
            LevelDir = string.Empty;
            PackageDir = string.Empty;
            ExportDir = string.Empty;
            return;
        }

        IsAvailable = true;
        LevelDir = levelDir;
        _workspaceDir = OpenSpaceDiscTestHelper.CreateTempDir();
        PackageDir = Path.Combine(_workspaceDir, "rete");
        ExportDir = Path.Combine(_workspaceDir, "export");

        OpenSpaceImporter.ImportLevel(LevelDir, PackageDir);
        OpenSpaceExporter.ExportLevel(PackageDir, ExportDir);
    }

    public void Dispose()
    {
        if (_workspaceDir != null && Directory.Exists(_workspaceDir))
        {
            Directory.Delete(_workspaceDir, recursive: true);
        }
    }
}