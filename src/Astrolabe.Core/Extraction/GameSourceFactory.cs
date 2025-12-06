namespace Astrolabe.Core.Extraction;

/// <summary>
/// Factory for creating game sources from ISOs or directories.
/// </summary>
public static class GameSourceFactory
{
    /// <summary>
    /// Creates a game source from the given path, auto-detecting whether it's an ISO or directory.
    /// </summary>
    public static IGameSource Create(string path)
    {
        if (File.Exists(path))
        {
            // Check if it's an ISO file
            var extension = Path.GetExtension(path).ToLowerInvariant();
            if (extension == ".iso" || extension == ".bin" || extension == ".img")
            {
                return new IsoGameSource(path);
            }

            // Try to detect ISO by reading magic bytes
            if (IsIsoFile(path))
            {
                return new IsoGameSource(path);
            }

            throw new ArgumentException($"File exists but is not a recognized ISO format: {path}");
        }

        if (Directory.Exists(path))
        {
            return new DirectoryGameSource(path);
        }

        throw new FileNotFoundException($"Path not found: {path}");
    }

    /// <summary>
    /// Checks if a file is an ISO by looking for ISO 9660 signature.
    /// </summary>
    private static bool IsIsoFile(string path)
    {
        try
        {
            using var fs = File.OpenRead(path);
            // ISO 9660 signature "CD001" is at sector 16 (offset 0x8000 + 1)
            if (fs.Length < 0x8006)
                return false;

            fs.Seek(0x8001, SeekOrigin.Begin);
            var buffer = new byte[5];
            if (fs.Read(buffer, 0, 5) != 5)
                return false;

            return buffer[0] == 'C' && buffer[1] == 'D' &&
                   buffer[2] == '0' && buffer[3] == '0' && buffer[4] == '1';
        }
        catch
        {
            return false;
        }
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
