using System.Reflection;
using System.Text.Json;
using Astrolabe.Core.Rete;
using Astrolabe.Core.Rete.OpenSpace;
using Xunit;

namespace Astrolabe.Core.Tests;

/// <summary>
/// Astrolabe disc parity tests that require a fresh Rete import via <see cref="AstrolabeDiscFixture"/>.
/// </summary>
[Trait("Category", "Disc")]
[Trait("Category", "Slow")]
[Collection("AstrolabeDisc")]
public sealed class RelocationGeneratorDiscTests(AstrolabeDiscFixture fixture)
{
    [Fact]
    public void AstrolabeRtp_GeneratedPointerData_MatchesDisc()
    {
        if (!fixture.IsAvailable)
        {
            return;
        }

        var results = OpenSpaceExporter.CompareGeneratedRelocations(fixture.PackageDir);
        var rtp = results.Single(result => result.FileName.Equals("astrolabe.rtp", StringComparison.OrdinalIgnoreCase));

        Assert.Equal(86, rtp.MatchingPointerCount);
        Assert.True(rtp.PointerDataMatches, DescribeRtpPointerDataMismatch(fixture.PackageDir));
    }

    [Fact]
    public void AstrolabeFixlvl_GeneratedPointerData_MatchesDisc()
    {
        if (!fixture.IsAvailable)
        {
            return;
        }

        var manifestPath = Path.Combine(fixture.PackageDir, "manifest.json");
        var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
            File.ReadAllText(manifestPath),
            new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });
        Assert.NotNull(manifest);
        Assert.NotEmpty(manifest.FixlvlBlockKeys);

        var results = OpenSpaceExporter.CompareGeneratedRelocations(fixture.PackageDir);
        var fixlvl = results.Single(result => result.FileName.Equals("fixlvl.rtb", StringComparison.OrdinalIgnoreCase));

        Assert.Equal(1117, fixlvl.MatchingPointerCount);
        Assert.Equal(0, fixlvl.MissingPointerCount);
        Assert.Equal(0, fixlvl.ExtraPointerCount);
        Assert.True(fixlvl.PointerDataMatches, DescribeFixlvlPointerDataMismatch(fixture.PackageDir));
    }

    private static string DescribeRtpPointerDataMismatch(string packageDir)
    {
        var readDisc = typeof(OpenSpacePackageCodec).GetMethod(
            "ReadRelocationTableFromDisc",
            BindingFlags.Static | BindingFlags.NonPublic)!;
        var generate = typeof(RelocationGenerator).GetMethod(
            "GeneratePointerFileTable",
            BindingFlags.Public | BindingFlags.Static)!;
        var fixDir = Path.Combine(Path.GetDirectoryName(packageDir)!, "fix");
        var pointerPath = Path.Combine(packageDir, "files", "astrolabe.gpt");
        var preserved = (RelocationTableDocument)readDisc.Invoke(
            null,
            [Path.Combine(OpenSpaceDiscTestHelper.GetRepositoryRoot(), "disc", "Gamedata", "World", "Levels", "astrolabe", "astrolabe.rtp")])!;
        var generated = (RelocationTableDocument)generate.Invoke(
            null,
            [packageDir, "astrolabe.rtp", pointerPath, new[] { fixDir }, null])!;
        var preservedBlock = preserved.Blocks.Single();
        var generatedBlock = generated.Blocks.Single();
        if (preservedBlock.Module != generatedBlock.Module || preservedBlock.Id != generatedBlock.Id)
        {
            return
                $"block mismatch preserved={preservedBlock.Key} generated={generatedBlock.Key}";
        }

        var preservedBytes = BuildOrderedRtpPointerBytes(preservedBlock);
        var generatedBytes = BuildOrderedRtpPointerBytes(generatedBlock);
        if (preservedBytes.AsSpan().SequenceEqual(generatedBytes))
        {
            return "ordered bytes match but PointerDataMatches was false";
        }

        var preservedOffsets = preservedBlock.Pointers.Select(p => p.OffsetInMemory).OrderBy(v => v).ToList();
        var generatedOffsets = generatedBlock.Pointers.Select(p => p.OffsetInMemory).OrderBy(v => v).ToList();
        var missingOffsets = preservedOffsets.Except(generatedOffsets).Take(5).Select(v => $"0x{v:X8}");
        var extraOffsets = generatedOffsets.Except(preservedOffsets).Take(5).Select(v => $"0x{v:X8}");
        if (missingOffsets.Any() || extraOffsets.Any())
        {
            return
                $"offset set diff missing=[{string.Join(", ", missingOffsets)}] extra=[{string.Join(", ", extraOffsets)}]";
        }

        var length = Math.Min(preservedBytes.Length, generatedBytes.Length);
        for (var index = 0; index < length; index++)
        {
            if (preservedBytes[index] == generatedBytes[index])
            {
                continue;
            }

            var entryIndex = index / 8;
            var preservedPointer = preservedBlock.Pointers
                .OrderBy(p => p.OffsetInMemory)
                .ThenBy(p => p.TargetModule)
                .ThenBy(p => p.TargetId)
                .ElementAt(entryIndex);
            var generatedPointer = generatedBlock.Pointers
                .OrderBy(p => p.OffsetInMemory)
                .ThenBy(p => p.TargetModule)
                .ThenBy(p => p.TargetId)
                .ElementAt(entryIndex);

            return
                $"entry {entryIndex} diff at byte 0x{index:X3}: " +
                $"preserved=0x{preservedPointer.OffsetInMemory:X8}->{preservedPointer.TargetModule:X2}:{preservedPointer.TargetId:X2} " +
                $"generated=0x{generatedPointer.OffsetInMemory:X8}->{generatedPointer.TargetModule:X2}:{generatedPointer.TargetId:X2}";
        }

        return $"length diff preserved={preservedBytes.Length} generated={generatedBytes.Length}";
    }

    private static string DescribeFixlvlPointerDataMismatch(string packageDir)
    {
        var readDisc = typeof(OpenSpacePackageCodec).GetMethod(
            "ReadRelocationTableFromDisc",
            BindingFlags.Static | BindingFlags.NonPublic)!;
        var fixDir = Path.Combine(Path.GetDirectoryName(packageDir)!, "fix");
        var preserved = (RelocationTableDocument)readDisc.Invoke(
            null,
            [Path.Combine(OpenSpaceDiscTestHelper.GetRepositoryRoot(), "disc", "Gamedata", "World", "Levels", "astrolabe", "fixlvl.rtb")])!;
        var generated = OpenSpaceExporter.GenerateFixLevelRtb(fixDir, packageDir, "fixlvl.rtb");

        foreach (var preservedBlock in preserved.Blocks.OrderBy(b => b.Module).ThenBy(b => b.Id))
        {
            var generatedBlock = generated.Blocks.SingleOrDefault(
                b => b.Module == preservedBlock.Module && b.Id == preservedBlock.Id);
            if (generatedBlock == null)
            {
                return $"missing block {preservedBlock.Key}";
            }

            if (preservedBlock.Pointers.Count != generatedBlock.Pointers.Count)
            {
                return
                    $"block {preservedBlock.Key} pointer count diff preserved={preservedBlock.Pointers.Count} generated={generatedBlock.Pointers.Count}";
            }

            var preservedBytes = BuildOrderedRtpPointerBytes(preservedBlock);
            var generatedBytes = BuildOrderedRtpPointerBytes(generatedBlock);
            if (!preservedBytes.AsSpan().SequenceEqual(generatedBytes))
            {
                var preservedOrdered = preservedBlock.Pointers
                    .OrderBy(p => p.OffsetInMemory)
                    .ThenBy(p => p.TargetModule)
                    .ThenBy(p => p.TargetId)
                    .ThenBy(p => p.Byte6)
                    .ThenBy(p => p.Byte7)
                    .ToList();
                var generatedOrdered = generatedBlock.Pointers
                    .OrderBy(p => p.OffsetInMemory)
                    .ThenBy(p => p.TargetModule)
                    .ThenBy(p => p.TargetId)
                    .ThenBy(p => p.Byte6)
                    .ThenBy(p => p.Byte7)
                    .ToList();
                for (var index = 0; index < preservedOrdered.Count; index++)
                {
                    var preservedPointer = preservedOrdered[index];
                    var generatedPointer = generatedOrdered[index];
                    if (preservedPointer.OffsetInMemory == generatedPointer.OffsetInMemory &&
                        preservedPointer.TargetModule == generatedPointer.TargetModule &&
                        preservedPointer.TargetId == generatedPointer.TargetId &&
                        preservedPointer.Byte6 == generatedPointer.Byte6 &&
                        preservedPointer.Byte7 == generatedPointer.Byte7)
                    {
                        continue;
                    }

                    return
                        $"block {preservedBlock.Key} entry {index}: " +
                        $"preserved=0x{preservedPointer.OffsetInMemory:X8}->{preservedPointer.TargetModule:X2}:{preservedPointer.TargetId:X2} " +
                        $"generated=0x{generatedPointer.OffsetInMemory:X8}->{generatedPointer.TargetModule:X2}:{generatedPointer.TargetId:X2}";
                }

                return $"block {preservedBlock.Key} ordered bytes differ with matching pointer tuples";
            }
        }

        return "pointer tuples match but PointerDataMatches was false";
    }

    private static byte[] BuildOrderedRtpPointerBytes(RelocationPointerBlockManifest block)
    {
        using var stream = new MemoryStream();
        using var writer = new BinaryWriter(stream);
        var entrySize = block.EntrySize >= 8 ? 8 : 6;
        foreach (var pointer in block.Pointers
                     .OrderBy(p => p.OffsetInMemory)
                     .ThenBy(p => p.TargetModule)
                     .ThenBy(p => p.TargetId)
                     .ThenBy(p => p.Byte6)
                     .ThenBy(p => p.Byte7))
        {
            writer.Write(pointer.OffsetInMemory);
            writer.Write(pointer.TargetModule);
            writer.Write(pointer.TargetId);
            if (entrySize >= 8)
            {
                writer.Write(pointer.Byte6);
                writer.Write(pointer.Byte7);
            }
        }

        writer.Flush();
        return stream.ToArray();
    }
}