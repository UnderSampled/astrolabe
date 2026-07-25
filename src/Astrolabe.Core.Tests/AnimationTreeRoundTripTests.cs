using Astrolabe.Core.FileFormats.Animation;
using Astrolabe.Core.Rete;
using Astrolabe.Core.Serialization.Codecs;
using Xunit;

namespace Astrolabe.Core.Tests;

public sealed class AnimationTreeRoundTripTests
{
    [Fact]
    public void TransformCodec_RoundTrips_Type1_WithTrailingGap()
    {
        var record = new TransformRecord
        {
            Id = "t_test",
            WireBytes = [0x11, 0x00, 0xE9, 0xFF, 0x2D, 0x00, 0xB0, 0x04],
            TrailingGap = [0x00, 0x00, 0x00, 0x00]
        };

        var bytes = TransformCodec.Instance.Write(record);
        Assert.Equal(12, bytes.Length);

        var roundTrip = TransformCodec.Instance.Read(bytes, 0, bytes.Length);
        Assert.Equal(record.WireBytes, roundTrip.WireBytes);
        Assert.Equal(record.TrailingGap, roundTrip.TrailingGap);
    }

    [Theory]
    [InlineData(new byte[] { 0x01, 0x00 }, 8)]
    [InlineData(new byte[] { 0x02, 0x00 }, 10)]
    [InlineData(new byte[] { 0x03, 0x00 }, 16)]
    [InlineData(new byte[] { 0x07, 0x00 }, 18)]
    [InlineData(new byte[] { 0x0B, 0x00 }, 22)]
    [InlineData(new byte[] { 0x0F, 0x00 }, 28)]
    public void TransformCodec_ReadsExpectedWireLengths(byte[] wirePrefix, int expectedLength)
    {
        var padded = wirePrefix.Concat(new byte[32]).ToArray();
        var record = TransformCodec.Instance.Read(padded, 0, padded.Length);
        Assert.Equal(expectedLength, record.WireBytes.Length);
    }

    [Fact]
    public void AnimationDocuments_HaveExpectedPaths()
    {
        Assert.Equal("astrolabe.animation-families.v1", AnimationFamiliesDocument.SchemaValue);
        Assert.Equal("animation/families.json", AnimationFamiliesDocument.RelativePath);
        Assert.Equal("astrolabe.animation-transforms.v1", AnimationTransformsDocument.SchemaValue);
        Assert.Equal("animation/transforms.json", AnimationTransformsDocument.RelativePath);
    }

    [Fact]
    public void Linearizer_ExpandsOrderedGroupChildren()
    {
        var document = new SnaBlockContentDocument
        {
            Schema = SnaBlockContentDocument.SchemaValue,
            Segments =
            [
                new SnaBlockContentSegment { Kind = "raw", DataPath = "types/raw/a.bin" },
                new SnaBlockContentSegment
                {
                    Kind = "group",
                    Children =
                    [
                        new SnaBlockContentSegment { Kind = "transform", DataPath = "animation/transforms.json#/byId/t_00000" },
                        new SnaBlockContentSegment { Kind = "transform", DataPath = "animation/transforms.json#/byId/t_00001" }
                    ]
                },
                new SnaBlockContentSegment { Kind = "raw", DataPath = "types/raw/b.bin" }
            ]
        };

        var leaves = SnaBlockContentLinearizer.Linearize("/tmp/unused", document);
        Assert.Equal(4, leaves.Count);
        Assert.Equal("raw", leaves[0].Kind);
        Assert.Equal("transform", leaves[1].Kind);
        Assert.Equal("animation/transforms.json#/byId/t_00000", leaves[1].DataPath);
        Assert.Equal("transform", leaves[2].Kind);
        Assert.Equal("raw", leaves[3].Kind);
    }

    [Fact]
    public void Linearizer_RejectsMissingSegments()
    {
        var document = new SnaBlockContentDocument
        {
            Schema = SnaBlockContentDocument.SchemaValue,
            BlockKey = "05:01",
            Segments = []
        };

        Assert.Throws<InvalidDataException>(() =>
            SnaBlockContentLinearizer.Linearize("/tmp/unused", document));
    }

    [Fact]
    public void Linearizer_RejectsLegacyV1Schema()
    {
        var document = new SnaBlockContentDocument
        {
            Schema = "astrolabe.sna-block-content.v1",
            BlockKey = "05:01",
            Segments =
            [
                new SnaBlockContentSegment { Kind = "raw", DataPath = "a.bin" }
            ]
        };

        Assert.Throws<InvalidDataException>(() =>
            SnaBlockContentLinearizer.Linearize("/tmp/unused", document));
    }

    [Fact]
    public void Linearizer_ExpandsTransformRuns()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-lin-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(Path.Combine(root, "animation"));
            var transforms = new AnimationTransformsDocument
            {
                Stream = ["t_00000", "t_00001", "t_00002"],
                Runs = { ["run_a"] = ["t_00000", "t_00001", "t_00002"] },
                ById =
                {
                    ["t_00000"] = new TransformRecord { Id = "t_00000", WireBytes = [1, 0] },
                    ["t_00001"] = new TransformRecord { Id = "t_00001", WireBytes = [1, 0] },
                    ["t_00002"] = new TransformRecord { Id = "t_00002", WireBytes = [1, 0] }
                }
            };
            File.WriteAllText(
                Path.Combine(root, AnimationTransformsDocument.RelativePath),
                System.Text.Json.JsonSerializer.Serialize(transforms));

            var document = new SnaBlockContentDocument
            {
                Schema = SnaBlockContentDocument.SchemaValue,
                Segments =
                [
                    new SnaBlockContentSegment
                    {
                        Kind = SnaBlockContentSegment.ExpandKind,
                        DataPath = "animation/transforms.json#/runs/run_a"
                    }
                ]
            };

            var leaves = SnaBlockContentLinearizer.Linearize(root, document);
            Assert.Equal(3, leaves.Count);
            Assert.All(leaves, leaf => Assert.Equal("transform", leaf.Kind));
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }
}

/// <summary>
/// Fast default gate: one <c>cp -a</c> + one aggregate (budget 60s), then shape checks.
/// Full export parity is opt-in — multi-minute export kills the edit loop.
/// </summary>
public sealed class AnimationTreeAggregatePerfTests : IClassFixture<AnimationTreeAggregateFixture>
{
    /// <summary>Default developer budget for aggregate (not full export).</summary>
    public static readonly TimeSpan AggregateBudget = TimeSpan.FromMinutes(1);

    private readonly AnimationTreeAggregateFixture _fixture;

    public AnimationTreeAggregatePerfTests(AnimationTreeAggregateFixture fixture) =>
        _fixture = fixture;

    [Fact]
    public void AggregateLevelPackage_CompletesUnderOneMinute_OnExistingAstrolabePackage()
    {
        Assert.True(
            _fixture.AggregateElapsed < AggregateBudget,
            $"Aggregate took {_fixture.AggregateElapsed.TotalSeconds:F1}s (budget {AggregateBudget.TotalSeconds:F0}s).");
        Assert.True(File.Exists(Path.Combine(_fixture.WorkDir, AnimationTransformsDocument.RelativePath)));
        Assert.True(File.Exists(Path.Combine(_fixture.WorkDir, AnimationFamiliesDocument.RelativePath)));
    }

    [Fact]
    public void AggregateLevelPackage_ProducesNestedFamilyStateTree_AndPoolUris()
    {
        var familiesJson = File.ReadAllText(
            Path.Combine(_fixture.WorkDir, AnimationFamiliesDocument.RelativePath));
        var families = System.Text.Json.JsonSerializer.Deserialize<AnimationFamiliesDocument>(
            familiesJson,
            new System.Text.Json.JsonSerializerOptions
            {
                PropertyNameCaseInsensitive = true,
                PropertyNamingPolicy = System.Text.Json.JsonNamingPolicy.CamelCase
            })!;

        Assert.True(families.Families.Count > 0, "Expected nested Families map.");
        Assert.Contains(families.Families.Values, fam => fam.States.Count > 0);
        Assert.Contains(
            families.Families.Values.SelectMany(f => f.States),
            st => !string.IsNullOrEmpty(st.AnimationId));

        var channel = families.ById.Values.First(n =>
            n.Kind.Equals("animchannel", StringComparison.OrdinalIgnoreCase) &&
            n.Record is { } rec &&
            rec.ValueKind == System.Text.Json.JsonValueKind.Object);
        var channelText = channel.Record!.Value.GetRawText();
        Assert.Contains("animation/transforms.json#/byId/", channelText, StringComparison.Ordinal);

        var contentPath = Directory.GetFiles(_fixture.WorkDir, "content.json", SearchOption.AllDirectories)
            .First(p => p.Contains("05_01", StringComparison.Ordinal) ||
                        p.Contains("05:01", StringComparison.Ordinal));
        var content = File.ReadAllText(contentPath);
        Assert.Contains("astrolabe.sna-block-content.v2", content, StringComparison.Ordinal);
        Assert.Contains("\"segments\"", content, StringComparison.Ordinal);
        Assert.Contains("#/runs/", content, StringComparison.Ordinal);
    }

    /// <summary>
    /// Decompressed SNA parity for animation blocks. Uses SNA-only export
    /// (<c>ASTROLABE_EXPORT_SNA_ONLY=1</c>) so the gate stays under ~1 minute;
    /// full RT* generation remains a separate slow path.
    /// </summary>
    [Fact]
    public void AggregateThenExport_DecompressedParity_Blocks_05_01_And_06_02()
    {
        if (!OpenSpaceDiscTestHelper.TryGetAstrolabeLevelDir(out var levelDir))
        {
            Assert.Fail("Astrolabe disc fixture is required for parity test.");
        }

        var exportDir = Path.Combine(Path.GetTempPath(), "astrolabe-anim-export-" + Guid.NewGuid().ToString("N"));
        var previous = Environment.GetEnvironmentVariable("ASTROLABE_EXPORT_SNA_ONLY");
        try
        {
            Environment.SetEnvironmentVariable("ASTROLABE_EXPORT_SNA_ONLY", "1");
            var sw = System.Diagnostics.Stopwatch.StartNew();
            OpenSpaceExporter.ExportLevel(_fixture.WorkDir, exportDir);
            sw.Stop();
            Assert.True(
                sw.Elapsed < TimeSpan.FromMinutes(1),
                $"SNA-only export took {sw.Elapsed.TotalSeconds:F1}s (budget 60s).");

            foreach (var (fileName, blockKey, sourcePlaintext) in
                     OpenSpaceDiscTestHelper.EnumerateDecompressedDiscContent(levelDir)
                         .Where(sample => sample.FileName == "astrolabe.sna" &&
                                          sample.BlockKey is "05:01" or "06:02"))
            {
                Assert.True(
                    OpenSpaceDiscTestHelper.TryGetExportDecompressedBlock(
                        exportDir, fileName, blockKey, out var exported),
                    $"Missing exported block {blockKey}.");
                Assert.True(
                    sourcePlaintext.AsSpan().SequenceEqual(exported),
                    $"Decompressed mismatch for {fileName} block {blockKey}: source={sourcePlaintext.Length} export={exported.Length}");
            }
        }
        finally
        {
            Environment.SetEnvironmentVariable("ASTROLABE_EXPORT_SNA_ONLY", previous);
            if (Directory.Exists(exportDir))
            {
                Directory.Delete(exportDir, recursive: true);
            }
        }
    }
}

/// <summary>
/// One fast <c>cp -a</c> + one aggregate shared by shape/budget tests.
/// </summary>
public sealed class AnimationTreeAggregateFixture : IDisposable
{
    public string WorkDir { get; }
    public TimeSpan AggregateElapsed { get; }

    public AnimationTreeAggregateFixture()
    {
        var source = ResolvePrebuiltAstrolabePackage();
        if (!Directory.Exists(Path.Combine(source, "types", "compressedmatrix")) &&
            !Directory.Exists(Path.Combine(source, "types", "animchannel")))
        {
            throw new InvalidOperationException(
                "Package already aggregated (no types/compressedmatrix); re-import test-rete first.");
        }

        WorkDir = Path.Combine(Path.GetTempPath(), "astrolabe-agg-fx-" + Guid.NewGuid().ToString("N"));
        FastCopyDirectory(source, WorkDir);

        var manifest = System.Text.Json.JsonSerializer.Deserialize<RetePackageManifest>(
            File.ReadAllText(Path.Combine(WorkDir, "manifest.json")),
            new System.Text.Json.JsonSerializerOptions
            {
                PropertyNameCaseInsensitive = true,
                PropertyNamingPolicy = System.Text.Json.JsonNamingPolicy.CamelCase
            })!;

        var sw = System.Diagnostics.Stopwatch.StartNew();
        AnimationTreeImporter.AggregateLevelPackage(WorkDir, manifest);
        sw.Stop();
        AggregateElapsed = sw.Elapsed;
    }

    public void Dispose()
    {
        if (Directory.Exists(WorkDir))
        {
            Directory.Delete(WorkDir, recursive: true);
        }
    }

    private static string ResolvePrebuiltAstrolabePackage()
    {
        var source = Path.GetFullPath("output/test-rete/astrolabe");
        if (!File.Exists(Path.Combine(source, "manifest.json")))
        {
            source = Path.GetFullPath(Path.Combine(
                AppContext.BaseDirectory, "..", "..", "..", "..", "..", "output", "test-rete", "astrolabe"));
        }

        if (!File.Exists(Path.Combine(source, "manifest.json")))
        {
            throw new InvalidOperationException($"Prebuilt package not found: {source}");
        }

        return source;
    }

    private static void FastCopyDirectory(string source, string dest)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(dest.TrimEnd('/'))!);
        var psi = new System.Diagnostics.ProcessStartInfo
        {
            FileName = "cp",
            ArgumentList = { "-a", source, dest },
            RedirectStandardError = true,
            RedirectStandardOutput = true
        };
        using var process = System.Diagnostics.Process.Start(psi)
            ?? throw new InvalidOperationException("Failed to start cp.");
        process.WaitForExit();
        if (process.ExitCode != 0)
        {
            throw new InvalidOperationException($"cp -a failed: {process.StandardError.ReadToEnd()}");
        }
    }
}

/// <summary>
/// Disc-backed animation tree tests using the shared <see cref="AstrolabeDiscFixture"/> import.
/// </summary>
[Trait("Category", "Disc")]
[Trait("Category", "Slow")]
[Collection("AstrolabeDisc")]
public sealed class AnimationTreeDiscTests(AstrolabeDiscFixture fixture)
{
    [Fact]
    public void Import_CreatesNestedFamiliesAndTransforms_WithSegmentContent()
    {
        if (!fixture.IsAvailable)
        {
            Assert.Fail("Astrolabe disc fixture is required for animation tree import test.");
        }

        var familiesPath = Path.Combine(fixture.PackageDir, AnimationFamiliesDocument.RelativePath);
        var transformsPath = Path.Combine(fixture.PackageDir, AnimationTransformsDocument.RelativePath);
        Assert.True(File.Exists(familiesPath), "Expected animation/families.json.");
        Assert.True(File.Exists(transformsPath), "Expected animation/transforms.json.");

        var opts = new System.Text.Json.JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true,
            PropertyNamingPolicy = System.Text.Json.JsonNamingPolicy.CamelCase
        };
        var families = System.Text.Json.JsonSerializer.Deserialize<AnimationFamiliesDocument>(
            File.ReadAllText(familiesPath), opts)!;
        var transforms = System.Text.Json.JsonSerializer.Deserialize<AnimationTransformsDocument>(
            File.ReadAllText(transformsPath), opts)!;

        Assert.True(families.Families.Count > 0, "Expected nested Families.");
        Assert.Contains(families.Families.Values, f => f.States.Count > 0);
        Assert.True(transforms.Stream.Count > 1000, $"Expected many transforms, found {transforms.Stream.Count}.");
        Assert.Equal(transforms.Stream.Count, transforms.ById.Count);

        // Transform pool is authoritative: compressedmatrix micro-files are removed.
        // Other anim kinds may remain under types/* as dual refs for non-stream hub records
        // (see plan Deviations); content stream uses animation/* URIs.
        Assert.False(Directory.Exists(Path.Combine(fixture.PackageDir, "types", "compressedmatrix")));
        Assert.Contains(
            families.ById.Values,
            n => n.Kind.Equals("animchannel", StringComparison.OrdinalIgnoreCase) &&
                 n.Record is { } rec &&
                 rec.GetRawText().Contains("animation/transforms.json#/byId/", StringComparison.Ordinal));

        var contentFiles = Directory.GetFiles(
            Path.Combine(fixture.PackageDir, "sna"),
            "content.json",
            SearchOption.AllDirectories);
        Assert.Contains(contentFiles, path =>
        {
            var text = File.ReadAllText(path);
            return text.Contains("astrolabe.sna-block-content.v2", StringComparison.Ordinal) &&
                   text.Contains("segments", StringComparison.OrdinalIgnoreCase) &&
                   text.Contains("animation/", StringComparison.Ordinal);
        });
    }

    [Fact]
    public void Export_DecompressedParity_Blocks_05_01_And_06_02()
    {
        if (!fixture.IsAvailable)
        {
            Assert.Fail("Astrolabe disc fixture is required for animation tree export test.");
        }

        _ = fixture.ExportDir;

        foreach (var (fileName, blockKey, sourcePlaintext) in
                 OpenSpaceDiscTestHelper.EnumerateDecompressedDiscContent(fixture.LevelDir)
                     .Where(sample => sample.FileName == "astrolabe.sna" &&
                                      sample.BlockKey is "05:01" or "06:02"))
        {
            Assert.True(
                OpenSpaceDiscTestHelper.TryGetExportDecompressedBlock(fixture.ExportDir, fileName, blockKey, out var exported),
                $"Missing exported block {blockKey}.");
            Assert.Equal(sourcePlaintext, exported);
        }
    }
}
