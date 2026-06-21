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
            // Empty RTB pointer-data blocks (e.g. fixlvl 07:00) are validated via FixlvlBlockKeys/generator paths.
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

    internal static IEnumerable<(string FileName, string BlockKey, byte[] Plaintext)> EnumerateDecompressedDiscContent(
        string levelDir)
    {
        foreach (var sample in EnumerateSnaPlaintextPayloads(levelDir, "astrolabe.sna"))
        {
            yield return (sample.SourceFile, sample.BlockKey, sample.Plaintext);
        }

        foreach (var fileName in new[] { "astrolabe.rtb", "astrolabe.rtp", "astrolabe.rtt", "fixlvl.rtb" })
        {
            foreach (var sample in EnumerateRelocationPlaintextPayloads(levelDir, fileName))
            {
                yield return (sample.SourceFile, sample.BlockKey, sample.Plaintext);
            }
        }
    }

    internal static bool TryGetExportDecompressedBlock(
        string exportDir,
        string fileName,
        string blockKey,
        out byte[] plaintext)
    {
        plaintext = [];
        if (fileName.Equals("astrolabe.sna", StringComparison.OrdinalIgnoreCase))
        {
            var reader = new SnaReader(Path.Combine(exportDir, fileName));
            var block = reader.Blocks.FirstOrDefault(b => $"{b.Module:X2}:{b.Id:X2}" == blockKey);
            if (block?.Data is not { Length: > 0 } data)
            {
                return false;
            }

            plaintext = data;
            return true;
        }

        if (!fileName.EndsWith(".rtb", StringComparison.OrdinalIgnoreCase) &&
            !fileName.EndsWith(".rtp", StringComparison.OrdinalIgnoreCase) &&
            !fileName.EndsWith(".rtt", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        var path = Path.Combine(exportDir, fileName);
        if (!File.Exists(path))
        {
            return false;
        }

        var relocationReader = new RelocationTableReader(path);
        var relocationBlock = relocationReader.PointerBlocks
            .FirstOrDefault(b => $"{b.Module:X2}:{b.Id:X2}" == blockKey);
        if (relocationBlock == null || relocationBlock.PointerData.Length == 0)
        {
            return false;
        }

        plaintext = relocationBlock.PointerData;
        return true;
    }

    internal static IEnumerable<string> EnumerateNonGeneratedSidecarFiles(string levelDir)
    {
        var generated = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            "astrolabe.sna",
            "astrolabe.rtb",
            "astrolabe.rtp",
            "astrolabe.rtt",
            "fixlvl.rtb"
        };

        foreach (var fileName in Directory.EnumerateFiles(levelDir).Select(Path.GetFileName).OrderBy(name => name))
        {
            if (fileName == null || generated.Contains(fileName))
            {
                continue;
            }

            yield return fileName;
        }
    }

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