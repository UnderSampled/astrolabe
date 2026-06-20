using Astrolabe.Core.Rete;
using Xunit;

namespace Astrolabe.Core.Tests;

[CollectionDefinition("AstrolabeDisc")]
public sealed class AstrolabeDiscCollection : ICollectionFixture<AstrolabeDiscFixture>;

/// <summary>
/// Shared astrolabe disc import for tests that need a Rete package as setup.
/// Export is deferred until <see cref="ExportDir"/> is first accessed.
/// </summary>
public sealed class AstrolabeDiscFixture : IDisposable
{
    private readonly string? _workspaceDir;
    private string? _exportDir;

    public bool IsAvailable { get; }

    public string LevelDir { get; }

    public string WorkspaceDir { get; }

    public string PackageDir { get; }

    public string FixDir { get; }

    public string ExportDir
    {
        get
        {
            if (!IsAvailable)
            {
                throw new InvalidOperationException("Astrolabe disc fixture is unavailable.");
            }

            if (_exportDir == null)
            {
                _exportDir = Path.Combine(WorkspaceDir, "export");
                OpenSpaceExporter.ExportLevel(PackageDir, _exportDir);
            }

            return _exportDir;
        }
    }

    public AstrolabeDiscFixture()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            IsAvailable = false;
            LevelDir = string.Empty;
            WorkspaceDir = string.Empty;
            PackageDir = string.Empty;
            FixDir = string.Empty;
            return;
        }

        IsAvailable = true;
        LevelDir = levelDir;
        _workspaceDir = OpenSpaceDiscTestHelper.CreateTempDir();
        WorkspaceDir = _workspaceDir;
        PackageDir = Path.Combine(_workspaceDir, "rete");
        FixDir = Path.Combine(_workspaceDir, "fix");

        OpenSpaceImporter.ImportLevel(LevelDir, PackageDir);
    }

    public void Dispose()
    {
        if (_workspaceDir != null && Directory.Exists(_workspaceDir))
        {
            Directory.Delete(_workspaceDir, recursive: true);
        }
    }
}