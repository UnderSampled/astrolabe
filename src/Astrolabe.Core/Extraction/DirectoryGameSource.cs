namespace Astrolabe.Core.Extraction;

/// <summary>
/// Provides access to game files from an extracted or mounted directory.
/// </summary>
public class DirectoryGameSource : IGameSource
{
    public string SourcePath { get; }
    public bool IsIso => false;

    public DirectoryGameSource(string directoryPath)
    {
        if (!Directory.Exists(directoryPath))
            throw new DirectoryNotFoundException($"Directory not found: {directoryPath}");

        SourcePath = Path.GetFullPath(directoryPath);
    }

    public IEnumerable<string> ListFiles()
    {
        return Directory.EnumerateFiles(SourcePath, "*", SearchOption.AllDirectories)
            .Select(f => Path.GetRelativePath(SourcePath, f).Replace('\\', '/'));
    }

    public Stream OpenFile(string relativePath)
    {
        var fullPath = Path.Combine(SourcePath, relativePath.Replace('/', Path.DirectorySeparatorChar));
        return File.OpenRead(fullPath);
    }

    public bool FileExists(string relativePath)
    {
        var fullPath = Path.Combine(SourcePath, relativePath.Replace('/', Path.DirectorySeparatorChar));
        return File.Exists(fullPath);
    }

    public IEnumerable<string> GetFiles(string pattern)
    {
        return ListFiles().Where(f => MatchesPattern(f, pattern));
    }

    public void Dispose()
    {
        // Nothing to dispose for directory access
    }

    private static bool MatchesPattern(string path, string pattern)
    {
        if (pattern == "*" || pattern == "*.*")
            return true;

        // Extension pattern (e.g., "*.cnt")
        if (pattern.StartsWith("*."))
        {
            var extension = pattern[1..];
            return path.EndsWith(extension, StringComparison.OrdinalIgnoreCase);
        }

        // Directory prefix pattern (e.g., "Gamedata/**" or "Gamedata/")
        if (pattern.EndsWith("/**") || pattern.EndsWith("/"))
        {
            var prefix = pattern.TrimEnd('*', '/');
            return path.StartsWith(prefix, StringComparison.OrdinalIgnoreCase);
        }

        // Path contains pattern
        if (pattern.Contains('/'))
        {
            return path.Contains(pattern, StringComparison.OrdinalIgnoreCase);
        }

        // Filename match
        var fileName = Path.GetFileName(path);
        return fileName.Equals(pattern, StringComparison.OrdinalIgnoreCase);
    }
}
