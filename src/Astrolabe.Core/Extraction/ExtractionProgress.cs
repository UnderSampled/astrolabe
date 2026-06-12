namespace Astrolabe.Core.Extraction;

/// <summary>
/// Progress information for file copy operations.
/// </summary>
public record ExtractionProgress(string CurrentFile, int ExtractedCount, int TotalFiles);
