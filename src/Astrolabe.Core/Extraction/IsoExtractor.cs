using DiscUtils.Iso9660;

namespace Astrolabe.Core.Extraction;

/// <summary>
/// Extracts files from a Hype: The Time Quest ISO image.
/// </summary>
public class IsoExtractor
{
    private readonly string _isoPath;

    public IsoExtractor(string isoPath)
    {
        if (!File.Exists(isoPath))
            throw new FileNotFoundException("ISO file not found", isoPath);

        _isoPath = isoPath;
    }

    /// <summary>
    /// Lists all files in the ISO image.
    /// </summary>
    public IEnumerable<string> ListFiles()
    {
        using var isoStream = File.OpenRead(_isoPath);
        using var cd = new CDReader(isoStream, true);

        return ListFilesRecursive(cd, "\\").ToList();
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

    /// <summary>
    /// Extracts all files from the ISO to the specified output directory.
    /// </summary>
    public void ExtractAll(string outputDirectory, IProgress<ExtractionProgress>? progress = null)
    {
        using var isoStream = File.OpenRead(_isoPath);
        using var cd = new CDReader(isoStream, true);

        var files = ListFilesRecursive(cd, "\\").ToList();
        var totalFiles = files.Count;
        var extractedCount = 0;

        foreach (var file in files)
        {
            ExtractFile(cd, file, outputDirectory);
            extractedCount++;
            progress?.Report(new ExtractionProgress(file, extractedCount, totalFiles));
        }
    }

    /// <summary>
    /// Extracts a specific file from the ISO.
    /// </summary>
    public void ExtractFile(string isoPath, string outputDirectory)
    {
        using var isoStream = File.OpenRead(_isoPath);
        using var cd = new CDReader(isoStream, true);

        ExtractFile(cd, isoPath, outputDirectory);
    }

    private static void ExtractFile(CDReader cd, string isoPath, string outputDirectory)
    {
        // Convert ISO path (backslash) to local path and strip ISO 9660 version suffix
        var relativePath = isoPath.TrimStart('\\').Replace('\\', Path.DirectorySeparatorChar);
        relativePath = StripVersionSuffix(relativePath);
        var outputPath = Path.Combine(outputDirectory, relativePath);

        // Ensure directory exists
        var outputDir = Path.GetDirectoryName(outputPath);
        if (!string.IsNullOrEmpty(outputDir))
        {
            Directory.CreateDirectory(outputDir);
        }

        // Extract file
        using var sourceStream = cd.OpenFile(isoPath, FileMode.Open);
        using var destStream = File.Create(outputPath);
        sourceStream.CopyTo(destStream);
    }

    /// <summary>
    /// Extracts files matching a pattern (e.g., "*.lvl", "*.cnt", "Gamedata/**").
    /// </summary>
    public void ExtractPattern(string pattern, string outputDirectory, IProgress<ExtractionProgress>? progress = null)
    {
        using var isoStream = File.OpenRead(_isoPath);
        using var cd = new CDReader(isoStream, true);

        var files = ListFilesRecursive(cd, "\\")
            .Where(f => MatchesPattern(f, pattern))
            .ToList();

        var totalFiles = files.Count;
        var extractedCount = 0;

        foreach (var file in files)
        {
            ExtractFile(cd, file, outputDirectory);
            extractedCount++;
            progress?.Report(new ExtractionProgress(file, extractedCount, totalFiles));
        }
    }

    /// <summary>
    /// Extracts files from a specific directory in the ISO.
    /// </summary>
    public void ExtractDirectory(string isoDirectory, string outputDirectory, IProgress<ExtractionProgress>? progress = null)
    {
        using var isoStream = File.OpenRead(_isoPath);
        using var cd = new CDReader(isoStream, true);

        // Normalize the directory path
        var normalizedDir = "\\" + isoDirectory.Trim('\\', '/').Replace('/', '\\');

        var files = ListFilesRecursive(cd, "\\")
            .Where(f => f.StartsWith(normalizedDir, StringComparison.OrdinalIgnoreCase))
            .ToList();

        var totalFiles = files.Count;
        var extractedCount = 0;

        foreach (var file in files)
        {
            ExtractFile(cd, file, outputDirectory);
            extractedCount++;
            progress?.Report(new ExtractionProgress(file, extractedCount, totalFiles));
        }
    }

    /// <summary>
    /// Strips the ISO 9660 version suffix (e.g., ";1") from a filename.
    /// </summary>
    private static string StripVersionSuffix(string path)
    {
        var semicolonIndex = path.LastIndexOf(';');
        if (semicolonIndex > 0 && semicolonIndex > path.LastIndexOf(Path.DirectorySeparatorChar))
        {
            return path[..semicolonIndex];
        }
        return path;
    }

    private static bool MatchesPattern(string fullPath, string pattern)
    {
        // Simple wildcard matching
        if (pattern == "*" || pattern == "*.*")
            return true;

        // Extension pattern (e.g., "*.lvl")
        if (pattern.StartsWith("*."))
        {
            var extension = pattern[1..]; // includes the dot
            return fullPath.EndsWith(extension, StringComparison.OrdinalIgnoreCase) ||
                   fullPath.EndsWith(extension + ";1", StringComparison.OrdinalIgnoreCase);
        }

        // Directory prefix pattern (e.g., "Gamedata/**" or "Gamedata/")
        if (pattern.EndsWith("/**") || pattern.EndsWith("/"))
        {
            var prefix = "\\" + pattern.TrimEnd('*', '/').Replace('/', '\\');
            return fullPath.StartsWith(prefix, StringComparison.OrdinalIgnoreCase);
        }

        // Path contains pattern
        if (pattern.Contains('/') || pattern.Contains('\\'))
        {
            var normalizedPattern = pattern.Replace('/', '\\');
            return fullPath.Contains(normalizedPattern, StringComparison.OrdinalIgnoreCase);
        }

        // Filename match
        var fileName = Path.GetFileName(fullPath);
        return fileName.Equals(pattern, StringComparison.OrdinalIgnoreCase) ||
               fileName.Equals(pattern + ";1", StringComparison.OrdinalIgnoreCase);
    }
}

/// <summary>
/// Progress information for extraction operations.
/// </summary>
public record ExtractionProgress(string CurrentFile, int ExtractedCount, int TotalFiles)
{
    public double Percentage => TotalFiles > 0 ? (double)ExtractedCount / TotalFiles * 100 : 0;
}
