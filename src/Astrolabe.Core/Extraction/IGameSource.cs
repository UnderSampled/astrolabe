namespace Astrolabe.Core.Extraction;

/// <summary>
/// Provides unified access to game files from either an ISO image or an extracted/mounted directory.
/// </summary>
public interface IGameSource : IDisposable
{
    /// <summary>
    /// Lists all files in the source.
    /// </summary>
    IEnumerable<string> ListFiles();

    /// <summary>
    /// Opens a file for reading.
    /// </summary>
    Stream OpenFile(string relativePath);

    /// <summary>
    /// Checks if a file exists.
    /// </summary>
    bool FileExists(string relativePath);

    /// <summary>
    /// Gets all files matching a pattern.
    /// </summary>
    IEnumerable<string> GetFiles(string pattern);

    /// <summary>
    /// Gets the source description (path to ISO or directory).
    /// </summary>
    string SourcePath { get; }

    /// <summary>
    /// Whether this source is an ISO file.
    /// </summary>
    bool IsIso { get; }
}
