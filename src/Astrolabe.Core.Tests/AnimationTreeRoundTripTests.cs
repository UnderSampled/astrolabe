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
            VirtualAddress = 0x0944D21C,
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
    public void AnimationTreeDocument_HasExpectedSchema()
    {
        Assert.Equal("astrolabe.animation-tree.v1", AnimationTreeDocument.SchemaValue);
        Assert.Equal("animation/level.json", AnimationTreeDocument.RelativePath);
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
    public void Import_CreatesSingleAnimationTree_AndRemovesPerElementFiles()
    {
        if (!fixture.IsAvailable)
        {
            Assert.Fail("Astrolabe disc fixture is required for animation tree import test.");
        }

        var treePath = Path.Combine(fixture.PackageDir, AnimationTreeDocument.RelativePath);
        Assert.True(File.Exists(treePath), "Expected aggregate animation/level.json.");

        var tree = System.Text.Json.JsonSerializer.Deserialize<AnimationTreeDocument>(
            File.ReadAllText(treePath),
            new System.Text.Json.JsonSerializerOptions { PropertyNameCaseInsensitive = true });
        Assert.NotNull(tree);
        Assert.True(tree!.Transforms.Count > 1000, $"Expected many transforms, found {tree.Transforms.Count}.");
        Assert.True(tree.Elements.Count > 1000, $"Expected many animation elements, found {tree.Elements.Count}.");

        Assert.False(Directory.Exists(Path.Combine(fixture.PackageDir, "types", "compressedmatrix")));
        Assert.False(Directory.Exists(Path.Combine(fixture.PackageDir, "types", "animchannel")));

        var sampleChannel = tree.Elements.Values.FirstOrDefault(entry =>
            entry.Kind.Equals("animchannel", StringComparison.OrdinalIgnoreCase));
        Assert.NotNull(sampleChannel);
        Assert.Contains("transforms/", sampleChannel!.Record.GetRawText(), StringComparison.Ordinal);
    }

    [Fact]
    public void Export_ResolvesTransformPointers_FromAnimationTree()
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