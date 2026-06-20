using Astrolabe.Core.FileFormats;
using Astrolabe.Core.Rete.OpenSpace;

namespace Astrolabe.Core.Tests;

internal static class OpenSpaceDiscTestHelper
{
    internal readonly record struct CompressedPayloadSample(
        string SourceFile,
        string BlockKey,
        byte[] OriginalCompressed,
        byte[] Plaintext,
        uint DecompressedSize,
        uint DecompressedChecksum);

    internal readonly record struct PlaintextPayloadSample(
        string SourceFile,
        string BlockKey,
        byte[] Plaintext);

    internal static bool TryGetAstrolabeLevelDir(out string levelDir)
    {
        levelDir = string.Empty;

        var envSource = Environment.GetEnvironmentVariable("ASTROLABE_SOURCE_DIR");
        if (!string.IsNullOrWhiteSpace(envSource))
        {
            var candidate = Path.Combine(envSource, "astrolabe");
            if (IsAstrolabeLevelDir(candidate))
            {
                levelDir = candidate;
                return true;
            }
        }

        var repoRoot = FindRepositoryRoot();
        if (repoRoot != null)
        {
            var candidate = Path.Combine(repoRoot, "disc", "Gamedata", "World", "Levels", "astrolabe");
            if (IsAstrolabeLevelDir(candidate))
            {
                levelDir = candidate;
                return true;
            }
        }

        return false;
    }

    internal static IEnumerable<CompressedPayloadSample> EnumerateCompressedSnaPayloads(string levelDir, string snaFileName)
    {
        var reader = new SnaReader(Path.Combine(levelDir, snaFileName));
        foreach (var block in reader.Blocks)
        {
            if (!block.IsCompressed || block.CompressedData is not { Length: > 0 } compressed)
            {
                continue;
            }

            var plaintext = block.Data ?? OpenSpaceLzo.Decompress(compressed, (int)block.DecompressedSize);
            yield return new CompressedPayloadSample(
                snaFileName,
                $"{block.Module:X2}:{block.Id:X2}",
                compressed,
                plaintext,
                block.DecompressedSize,
                block.DecompressedChecksum);
        }
    }

    internal static IEnumerable<CompressedPayloadSample> EnumerateCompressedRelocationPayloads(
        string levelDir,
        string fileName)
    {
        var path = Path.Combine(levelDir, fileName);
        if (!File.Exists(path))
        {
            yield break;
        }

        var reader = new RelocationTableReader(path);
        foreach (var block in reader.PointerBlocks)
        {
            if (!block.IsCompressed || block.CompressedData.Length == 0)
            {
                continue;
            }

            var plaintext = block.PointerData.Length > 0
                ? block.PointerData
                : OpenSpaceLzo.Decompress(block.CompressedData, (int)block.DecompressedSize);

            yield return new CompressedPayloadSample(
                fileName,
                $"{block.Module:X2}:{block.Id:X2}",
                block.CompressedData,
                plaintext,
                block.DecompressedSize,
                block.DecompressedChecksum);
        }
    }

    internal static IEnumerable<PlaintextPayloadSample> EnumerateSnaPlaintextPayloads(string levelDir, string snaFileName)
    {
        var reader = new SnaReader(Path.Combine(levelDir, snaFileName));
        foreach (var block in reader.Blocks)
        {
            if (block.Size == 0 || block.Data is not { Length: > 0 } data)
            {
                continue;
            }

            yield return new PlaintextPayloadSample(
                snaFileName,
                $"{block.Module:X2}:{block.Id:X2}",
                data);
        }
    }

    internal static IEnumerable<PlaintextPayloadSample> EnumerateRelocationPlaintextPayloads(
        string levelDir,
        string fileName)
    {
        var path = Path.Combine(levelDir, fileName);
        if (!File.Exists(path))
        {
            yield break;
        }

        var reader = new RelocationTableReader(path);
        foreach (var block in reader.PointerBlocks)
        {
            if (block.Count == 0 || block.PointerData.Length == 0)
            {
                continue;
            }

            yield return new PlaintextPayloadSample(
                fileName,
                $"{block.Module:X2}:{block.Id:X2}",
                block.PointerData);
        }
    }

    internal static IReadOnlyDictionary<(string FileName, string BlockKey), byte[]> IndexPlaintextSamples(
        IEnumerable<PlaintextPayloadSample> samples) =>
        samples.ToDictionary(
            sample => (sample.SourceFile, sample.BlockKey),
            sample => sample.Plaintext);

    internal static string CreateTempDir()
    {
        var path = Path.Combine(Path.GetTempPath(), "astrolabe-tests", Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(path);
        return path;
    }

    private static bool IsAstrolabeLevelDir(string candidate) =>
        Directory.Exists(candidate) &&
        File.Exists(Path.Combine(candidate, "astrolabe.sna")) &&
        File.Exists(Path.Combine(candidate, "astrolabe.rtb"));

    internal static string GetRepositoryRoot() =>
        FindRepositoryRoot()
        ?? throw new InvalidOperationException("Repository root not found from test base directory.");

    private static string? FindRepositoryRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir != null)
        {
            if (Directory.Exists(Path.Combine(dir.FullName, "disc", "Gamedata", "World", "Levels")))
            {
                return dir.FullName;
            }

            dir = dir.Parent;
        }

        return null;
    }
}