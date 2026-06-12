namespace Astrolabe.Core.Extraction;

/// <summary>
/// Factory for creating game sources from extracted or mounted directories.
/// </summary>
public static class GameSourceFactory
{
    /// <summary>
    /// Creates a game source from the given directory path.
    /// </summary>
    public static IGameSource Create(string path)
    {
        if (File.Exists(path))
        {
            throw new ArgumentException($"Expected an extracted or mounted game directory, not a file: {path}");
        }

        if (Directory.Exists(path))
        {
            return new DirectoryGameSource(path);
        }

        throw new DirectoryNotFoundException($"Directory not found: {path}");
    }

    /// <summary>
    /// Extracts files from a source to a directory.
    /// </summary>
    public static void ExtractFiles(
        IGameSource source,
        string outputDirectory,
        IEnumerable<string>? filesToExtract = null,
        IProgress<ExtractionProgress>? progress = null)
    {
        var files = (filesToExtract ?? source.ListFiles()).ToList();
        var totalFiles = files.Count;
        var extractedCount = 0;

        foreach (var file in files)
        {
            var outputPath = Path.Combine(outputDirectory, file.Replace('/', Path.DirectorySeparatorChar));
            var outputDir = Path.GetDirectoryName(outputPath);

            if (!string.IsNullOrEmpty(outputDir))
            {
                Directory.CreateDirectory(outputDir);
            }

            using var sourceStream = source.OpenFile(file);
            using var destStream = File.Create(outputPath);
            sourceStream.CopyTo(destStream);

            extractedCount++;
            progress?.Report(new ExtractionProgress(file, extractedCount, totalFiles));
        }
    }
}
