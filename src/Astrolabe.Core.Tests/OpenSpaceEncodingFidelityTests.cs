using Astrolabe.Core.Rete.OpenSpace;
using Xunit;

namespace Astrolabe.Core.Tests;

/// <summary>
/// Three-layer encoding fidelity gate against the astrolabe test level on disc.
/// Run with: dotnet test --filter "Category=Disc"
/// </summary>
[Trait("Category", "Disc")]
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
    public void LayerA_SnaCompressedBlocks_RecompressMatchesOriginal()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            return;
        }

        var mismatches = new List<string>();
        var checkedCount = 0;

        foreach (var sample in OpenSpaceDiscTestHelper.EnumerateCompressedSnaPayloads(levelDir, "astrolabe.sna"))
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

    [Theory]
    [MemberData(nameof(RelocationFileNames))]
    public void LayerA_RelocationCompressedBlocks_RecompressMatchesOriginal(string fileName)
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            return;
        }

        var mismatches = new List<string>();

        foreach (var sample in OpenSpaceDiscTestHelper.EnumerateCompressedRelocationPayloads(levelDir, fileName))
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

    [Fact]
    public void LayerB_SnaPlaintext_MatchesExportPipeline()
    {
        if (!fixture.IsAvailable)
        {
            return;
        }

        var discPlaintext = OpenSpaceDiscTestHelper.IndexPlaintextSamples(
            OpenSpaceDiscTestHelper.EnumerateSnaPlaintextPayloads(fixture.LevelDir, "astrolabe.sna"));
        var exportPlaintext = OpenSpaceDiscTestHelper.IndexPlaintextSamples(
            OpenSpaceDiscTestHelper.EnumerateSnaPlaintextPayloads(fixture.ExportDir, "astrolabe.sna"));

        var mismatches = new List<string>();
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

        var discPlaintext = OpenSpaceDiscTestHelper.IndexPlaintextSamples(
            OpenSpaceDiscTestHelper.EnumerateRelocationPlaintextPayloads(fixture.LevelDir, fileName));
        var exportPlaintext = OpenSpaceDiscTestHelper.IndexPlaintextSamples(
            OpenSpaceDiscTestHelper.EnumerateRelocationPlaintextPayloads(fixture.ExportDir, fileName));

        var mismatches = new List<string>();
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
    public void LayerC_AstrolabeLevel_FilesMatchSourceDisc()
    {
        if (!fixture.IsAvailable)
        {
            return;
        }

        var mismatches = new List<string>();
        foreach (var sourcePath in Directory.EnumerateFiles(fixture.LevelDir).OrderBy(path => path))
        {
            var fileName = Path.GetFileName(sourcePath);
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