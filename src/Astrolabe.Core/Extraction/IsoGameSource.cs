using DiscUtils.Iso9660;

namespace Astrolabe.Core.Extraction;

/// <summary>
/// Provides access to game files from an ISO image.
/// </summary>
public class IsoGameSource : IGameSource
{
    private readonly FileStream _isoStream;
    private readonly CDReader _cdReader;
    private readonly List<string> _fileCache;

    public string SourcePath { get; }
    public bool IsIso => true;

    public IsoGameSource(string isoPath)
    {
        if (!File.Exists(isoPath))
            throw new FileNotFoundException("ISO file not found", isoPath);

        SourcePath = isoPath;
        _isoStream = File.OpenRead(isoPath);
        _cdReader = new CDReader(_isoStream, true);
        _fileCache = ListFilesRecursive(_cdReader, "\\").ToList();
    }

    public IEnumerable<string> ListFiles()
    {
        return _fileCache.Select(NormalizePath);
    }

    public Stream OpenFile(string relativePath)
    {
        var isoPath = ToIsoPath(relativePath);
        return _cdReader.OpenFile(isoPath, FileMode.Open);
    }

    public bool FileExists(string relativePath)
    {
        var isoPath = ToIsoPath(relativePath);
        return _cdReader.FileExists(isoPath) ||
               _cdReader.FileExists(isoPath + ";1");
    }

    public IEnumerable<string> GetFiles(string pattern)
    {
        return _fileCache
            .Where(f => MatchesPattern(f, pattern))
            .Select(NormalizePath);
    }

    public void Dispose()
    {
        _cdReader.Dispose();
        _isoStream.Dispose();
    }

    private static IEnumerable<string> ListFilesRecursive(CDReader cd, string directory)
    {
        foreach (var file in cd.GetFiles(directory))
        {
            yield return file;
        }

        foreach (var dir in cd.GetDirectories(directory))
        {
            foreach (var file in ListFilesRecursive(cd, dir))
            {
                yield return file;
            }
        }
    }

    private static string ToIsoPath(string relativePath)
    {
        return "\\" + relativePath.Replace('/', '\\').TrimStart('\\');
    }

    private static string NormalizePath(string isoPath)
    {
        var path = isoPath.TrimStart('\\').Replace('\\', '/');
        // Strip ISO 9660 version suffix (e.g., ";1")
        var semicolonIndex = path.LastIndexOf(';');
        if (semicolonIndex > 0 && semicolonIndex > path.LastIndexOf('/'))
        {
            path = path[..semicolonIndex];
        }
        return path;
    }

    private static bool MatchesPattern(string fullPath, string pattern)
    {
        var normalizedPath = NormalizePath(fullPath);

        if (pattern == "*" || pattern == "*.*")
            return true;

        // Extension pattern (e.g., "*.lvl")
        if (pattern.StartsWith("*."))
        {
            var extension = pattern[1..];
            return normalizedPath.EndsWith(extension, StringComparison.OrdinalIgnoreCase);
        }

        // Directory prefix pattern (e.g., "Gamedata/**" or "Gamedata/")
        if (pattern.EndsWith("/**") || pattern.EndsWith("/"))
        {
            var prefix = pattern.TrimEnd('*', '/');
            return normalizedPath.StartsWith(prefix, StringComparison.OrdinalIgnoreCase);
        }

        // Path contains pattern
        if (pattern.Contains('/'))
        {
            return normalizedPath.Contains(pattern, StringComparison.OrdinalIgnoreCase);
        }

        // Filename match
        var fileName = Path.GetFileName(normalizedPath);
        return fileName.Equals(pattern, StringComparison.OrdinalIgnoreCase);
    }
}
