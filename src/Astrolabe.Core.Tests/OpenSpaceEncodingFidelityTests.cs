using Astrolabe.Core.Rete.OpenSpace;
using Xunit;

namespace Astrolabe.Core.Tests;

/// <summary>
/// Encoding fidelity gate against the astrolabe test level on disc.
/// Layer B/C measure decompressed plaintext parity (Step 7 gate).
/// Layer A compressed-byte checks are out of scope — see <see cref="CompressedDiscFidelityTests"/>.
/// Run with: dotnet test --filter "Category=Disc"
/// </summary>
[Trait("Category", "Disc")]
[Trait("Category", "Slow")]
[Collection("AstrolabeDisc")]
public sealed class OpenSpaceEncodingFidelityTests(AstrolabeDiscFixture fixture)
{
    private static readonly string[] RelocationFiles =
    [
        "astrolabe.rtb",
        "astrolabe.rtp",
        "astrolabe.rtt",
        "fixlvl.rtb"
    ];

    [Fact]
    public void LayerB_SnaPlaintext_MatchesExportPipeline()
    {
        if (!fixture.IsAvailable)
        {
            return;
        }

        var mismatches = new List<string>();
        var discPlaintext = OpenSpaceDiscTestHelper.IndexPlaintextSamples(
            OpenSpaceDiscTestHelper.EnumerateSnaPlaintextPayloads(fixture.LevelDir, "astrolabe.sna"));
        var exportPlaintext = OpenSpaceDiscTestHelper.IndexPlaintextSamples(
            OpenSpaceDiscTestHelper.EnumerateSnaPlaintextPayloads(fixture.ExportDir, "astrolabe.sna"));

        foreach (var (key, discData) in discPlaintext)
        {
            if (!exportPlaintext.TryGetValue(key, out var exportData))
            {
                mismatches.Add($"{key.FileName} {key.BlockKey}: missing from export");
                continue;
            }

            if (!discData.AsSpan().SequenceEqual(exportData))
            {
                mismatches.Add($"{key.FileName} {key.BlockKey}: plaintext differs");
            }
        }

        Assert.Empty(mismatches);
    }

    [Theory]
    [MemberData(nameof(RelocationFileNames))]
    public void LayerB_RelocationPlaintext_MatchesExportPipeline(string fileName)
    {
        if (!fixture.IsAvailable)
        {
            return;
        }

        if (!File.Exists(Path.Combine(fixture.LevelDir, fileName)))
        {
            return;
        }

        var mismatches = new List<string>();
        var discPlaintext = OpenSpaceDiscTestHelper.IndexPlaintextSamples(
            OpenSpaceDiscTestHelper.EnumerateRelocationPlaintextPayloads(fixture.LevelDir, fileName));
        var exportPlaintext = OpenSpaceDiscTestHelper.IndexPlaintextSamples(
            OpenSpaceDiscTestHelper.EnumerateRelocationPlaintextPayloads(fixture.ExportDir, fileName));

        foreach (var (key, discData) in discPlaintext)
        {
            if (!exportPlaintext.TryGetValue(key, out var exportData))
            {
                mismatches.Add($"{key.FileName} {key.BlockKey}: missing from export");
                continue;
            }

            if (!discData.AsSpan().SequenceEqual(exportData))
            {
                mismatches.Add($"{key.FileName} {key.BlockKey}: plaintext differs");
            }
        }

        Assert.Empty(mismatches);
    }

    [Fact]
    public void LayerC_DecompressedContent_MatchesSourceDisc()
    {
        if (!fixture.IsAvailable)
        {
            return;
        }

        // Empty RTB pointer-data blocks are skipped here; presence is checked via FixlvlBlockKeys + generator tests.
        var mismatches = new List<string>();

        foreach (var (fileName, blockKey, discData) in OpenSpaceDiscTestHelper.EnumerateDecompressedDiscContent(fixture.LevelDir))
        {
            if (!OpenSpaceDiscTestHelper.TryGetExportDecompressedBlock(
                    fixture.ExportDir,
                    fileName,
                    blockKey,
                    out var exportData))
            {
                mismatches.Add($"{fileName} {blockKey}: missing from export");
                continue;
            }

            if (!discData.AsSpan().SequenceEqual(exportData))
            {
                mismatches.Add($"{fileName} {blockKey}: decompressed content differs");
            }
        }

        foreach (var fileName in OpenSpaceDiscTestHelper.EnumerateNonGeneratedSidecarFiles(fixture.LevelDir))
        {
            var sourcePath = Path.Combine(fixture.LevelDir, fileName);
            var exportPath = Path.Combine(fixture.ExportDir, fileName);
            if (!File.Exists(exportPath))
            {
                mismatches.Add($"{fileName}: missing from export");
                continue;
            }

            var sourceBytes = File.ReadAllBytes(sourcePath);
            var exportBytes = File.ReadAllBytes(exportPath);
            if (!sourceBytes.AsSpan().SequenceEqual(exportBytes))
            {
                mismatches.Add($"{fileName}: byte mismatch ({sourceBytes.Length} vs {exportBytes.Length})");
            }
        }

        Assert.Empty(mismatches);
    }

    public static TheoryData<string> RelocationFileNames => new(RelocationFiles);
}

/// <summary>
/// Compressed-byte fidelity probes. Out of scope for Step 7 decompressed parity gate.
/// </summary>
[Trait("Category", "CompressedDisc")]
[Trait("Category", "Slow")]
[Collection("AstrolabeDisc")]
public sealed class CompressedDiscFidelityTests(AstrolabeDiscFixture fixture)
{
    private static readonly string[] RelocationFiles =
    [
        "astrolabe.rtb",
        "astrolabe.rtp",
        "astrolabe.rtt",
        "fixlvl.rtb"
    ];

    [Fact(Skip = "Step 7 gate is decompressed plaintext parity only; LZO recompression can differ on valid alternate encodings.")]
    public void LayerA_SnaCompressedBlocks_RecompressMatchesOriginal()
    {
        if (!fixture.IsAvailable)
        {
            return;
        }

        var mismatches = new List<string>();
        var checkedCount = 0;

        foreach (var sample in OpenSpaceDiscTestHelper.EnumerateCompressedSnaPayloads(fixture.LevelDir, "astrolabe.sna"))
        {
            checkedCount++;
            var recompressed = OpenSpaceLzo.Compress(sample.Plaintext);
            if (!recompressed.AsSpan().SequenceEqual(sample.OriginalCompressed))
            {
                mismatches.Add(
                    $"{sample.SourceFile} {sample.BlockKey}: " +
                    $"original={sample.OriginalCompressed.Length} recompressed={recompressed.Length}");
            }
        }

        Assert.True(checkedCount > 0, "Expected at least one compressed SNA block in astrolabe.sna.");
        Assert.Empty(mismatches);
    }

    [Theory(Skip = "Step 7 gate is decompressed plaintext parity only; LZO recompression can differ on valid alternate encodings.")]
    [MemberData(nameof(RelocationFileNames))]
    public void LayerA_RelocationCompressedBlocks_RecompressMatchesOriginal(string fileName)
    {
        if (!fixture.IsAvailable)
        {
            return;
        }

        var mismatches = new List<string>();

        foreach (var sample in OpenSpaceDiscTestHelper.EnumerateCompressedRelocationPayloads(fixture.LevelDir, fileName))
        {
            var recompressed = OpenSpaceLzo.Compress(sample.Plaintext);
            if (!recompressed.AsSpan().SequenceEqual(sample.OriginalCompressed))
            {
                mismatches.Add(
                    $"{sample.SourceFile} {sample.BlockKey}: " +
                    $"original={sample.OriginalCompressed.Length} recompressed={recompressed.Length}");
            }
        }

        Assert.Empty(mismatches);
    }

    public static TheoryData<string> RelocationFileNames => new(RelocationFiles);
}
