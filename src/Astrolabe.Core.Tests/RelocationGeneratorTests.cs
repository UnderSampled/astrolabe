using System.Buffers.Binary;
using System.Reflection;
using System.Text.Json;
using Astrolabe.Core.FileFormats;
using Astrolabe.Core.Rete;
using Astrolabe.Core.Rete.OpenSpace;
using Astrolabe.Core.Serialization;
using Astrolabe.Core.Serialization.Codecs;
using Xunit;

namespace Astrolabe.Core.Tests;

public sealed class RelocationGeneratorTests
{
    [Fact]
    public void BehaviorArrayCodec_MisalignedLength_ReturnsNoPointerFields()
    {
        var fields = BehaviorArrayCodec.BehaviorsNormal.GetPointerFieldsForLength(0x11);
        Assert.Empty(fields);
    }

    [Fact]
    public void BehaviorArrayCodec_TruncatedAlignedPrefix_StillEmitsPointerFields()
    {
        IPointerArrayCodec codec = BehaviorArrayCodec.BehaviorsNormal;
        var fields = codec.EnumeratePointerFields(new byte[0x20]);
        Assert.Equal(4, fields.Count);
    }

    [Fact]
    public void AnimFramesCodec_UsesFrameStride()
    {
        Assert.Equal(0x10, AnimFramesCodec.Instance.PointerEntryStride);
    }

    [Fact]
    public void AnimFramesCodec_MisalignedLength_ReturnsNoPointerFields()
    {
        var fields = AnimFramesCodec.Instance.GetPointerFieldsForLength(0x14);
        Assert.Empty(fields);
    }

    [Fact]
    public void RawBlobCodec_OnlyEnumeratesVmLikePointers()
    {
        var data = new byte[8];
        BinaryPrimitives.WriteInt32LittleEndian(data.AsSpan(0), 0x1234);
        BinaryPrimitives.WriteInt32LittleEndian(data.AsSpan(4), 0x0900_0000);

        var fields = RawBlobCodec.Instance.EnumeratePointerFields(data);
        Assert.Single(fields);
        Assert.Equal(4, fields[0].Offset);
        Assert.True(fields[0].RequiresDecompressedTarget);
    }

    [Fact]
    public void VmPointerScanning_RejectsNonVmValues()
    {
        Assert.False(VmPointerScanning.IsLikelyVirtualAddress(0));
        Assert.False(VmPointerScanning.IsLikelyVirtualAddress(0x1234));
        Assert.True(VmPointerScanning.IsLikelyVirtualAddress(0x0900_0000));
    }

    [Fact]
    public void OpenSpaceLzo_CompressRoundTripsThroughLzoNet()
    {
        var pointerData = new byte[0x4000];
        for (var index = 0; index < pointerData.Length / 8; index++)
        {
            var offset = index * 8;
            BinaryPrimitives.WriteUInt32LittleEndian(pointerData.AsSpan(offset), (uint)(0x0900_0000 + index * 4));
            pointerData[offset + 4] = 0x05;
            pointerData[offset + 5] = 0x01;
        }

        var compressed = OpenSpaceLzo.Compress(pointerData);
        var roundTrip = OpenSpaceLzo.Decompress(compressed, pointerData.Length);

        Assert.Equal(pointerData, roundTrip);
        Assert.True(compressed.Length < pointerData.Length);
    }

    [Fact]
    public void CompileRelocationTable_LargePointerBlock_WritesLzoCompressedPayload()
    {
        var packageDir = CreateTempDir();
        var outputPath = Path.Combine(packageDir, "test.rtb");
        try
        {
            var pointers = Enumerable.Range(0, 0x400)
                .Select(index => new RelocationPointerManifest
                {
                    OffsetInMemory = (uint)(0x0900_1000 + index * 8),
                    TargetModule = 0x05,
                    TargetId = 0x01
                })
                .ToList();

            var document = new RelocationTableDocument
            {
                FileName = "test.rtb",
                Blocks =
                [
                    new RelocationPointerBlockManifest
                    {
                        Order = 0,
                        Key = "05:01",
                        Module = 0x05,
                        Id = 0x01,
                        EntrySize = 8,
                        Pointers = pointers
                    }
                ]
            };

            InvokeCompileRelocationTable(packageDir, document, outputPath);

            var compiled = new RelocationTableReader(outputPath);
            var compiledBlock = Assert.Single(compiled.PointerBlocks);
            Assert.True(compiledBlock.IsCompressed);
            Assert.Equal((uint)pointers.Count, compiledBlock.Count);
            Assert.Equal(pointers[0].OffsetInMemory, compiledBlock.Pointers[0].OffsetInMemory);
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void OpaqueCodec_WriteJson_UsesSidecarBinFile()
    {
        var packageDir = CreateTempDir();
        try
        {
            Assert.True(StructCodecRegistry.TryGet("raw", out var codec));

            var jsonPath = Path.Combine(packageDir, "types/raw/source.json");
            codec.WriteJson(packageDir, jsonPath, new OpaqueBinaryRecord
            {
                Schema = RawBlobCodec.Instance.Schema,
                Data = [0x01, 0x02, 0x03, 0x04]
            });

            var json = File.ReadAllText(jsonPath);
            Assert.DoesNotContain("\"data\"", json, StringComparison.Ordinal);
            Assert.Contains("\"path\":\"types/raw/source.bin\"", json, StringComparison.Ordinal);
            Assert.Equal(new byte[] { 0x01, 0x02, 0x03, 0x04 }, File.ReadAllBytes(Path.Combine(packageDir, "types/raw/source.bin")));
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void PreviewStructuredElementBytes_OpaqueInlinePointers_ResolvesUri()
    {
        var packageDir = CreateTempDir();
        try
        {
            CreateOpaquePackageFixture(packageDir, new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" });

            var bytes = OpenSpaceExporter.PreviewStructuredElementBytes(packageDir, "raw", "types/raw/source.json");

            Assert.Equal(0x0900_0000, BinaryPrimitives.ReadInt32LittleEndian(bytes.AsSpan(0, 4)));
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void PreviewStructuredElementBytes_OpaqueInlinePointers_NullPointerWritesZero()
    {
        var packageDir = CreateTempDir();
        try
        {
            CreateOpaquePackageFixture(packageDir, new Dictionary<string, string?> { ["0x0"] = null });

            var bytes = OpenSpaceExporter.PreviewStructuredElementBytes(packageDir, "raw", "types/raw/source.json");

            Assert.Equal(0, BinaryPrimitives.ReadInt32LittleEndian(bytes.AsSpan(0, 4)));
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void PreviewStructuredElementBytes_OpaqueInlinePointers_InvalidOffsetThrows()
    {
        var packageDir = CreateTempDir();
        try
        {
            CreateOpaquePackageFixture(packageDir, new Dictionary<string, string?> { ["0x9"] = "types/raw/target.json" });

            Assert.Throws<InvalidDataException>(() =>
                OpenSpaceExporter.PreviewStructuredElementBytes(packageDir, "raw", "types/raw/source.json"));
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void GenerateRtb_OpaquePointerMap_UsesExplicitOffsetsOnly()
    {
        var packageDir = CreateTempDir();
        try
        {
            var sourceData = new byte[8];
            BinaryPrimitives.WriteInt32LittleEndian(sourceData.AsSpan(0, 4), 0x1234_5678);
            BinaryPrimitives.WriteInt32LittleEndian(sourceData.AsSpan(4, 4), 0x0900_0000);
            CreateOpaquePackageFixture(
                packageDir,
                new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" },
                sourceData: sourceData);

            var table = OpenSpaceExporter.GenerateRtb(packageDir, "test.rtb", []);
            var block = Assert.Single(table.Blocks);
            var pointer = Assert.Single(block.Pointers);

            Assert.Equal((uint)0x0900_0004, pointer.OffsetInMemory);
            Assert.Equal(0x00, pointer.TargetModule);
            Assert.Equal(0x01, pointer.TargetId);
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void GenerateRtb_OpaquePointerMap_DoesNotRequireCodecPointerMetadata()
    {
        var packageDir = CreateTempDir();
        try
        {
            StructCodecRegistry.Register(TestOpaqueNoMetadataCodec.Instance);
            CreateOpaquePackageFixture(
                packageDir,
                new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" },
                kind: TestOpaqueNoMetadataCodec.Instance.Kind,
                schema: TestOpaqueNoMetadataCodec.Instance.Schema);

            var table = OpenSpaceExporter.GenerateRtb(packageDir, "test.rtb", []);
            var block = Assert.Single(table.Blocks);
            var pointer = Assert.Single(block.Pointers);

            Assert.Equal((uint)0x0900_0004, pointer.OffsetInMemory);
            Assert.Equal(0x00, pointer.TargetModule);
            Assert.Equal(0x01, pointer.TargetId);
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void GenerateRtb_OpaquePointerMap_PrefersExplicitUriTargetPackage()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var levelDir = Path.Combine(workspaceDir, "astrolabe");
            var fixDir = Path.Combine(workspaceDir, "fix");

            CreateOpaquePackageFixture(
                levelDir,
                new Dictionary<string, string?> { ["0x0"] = "fix:/types/raw/target.json" },
                sourceData: [0x00, 0x00, 0x00, 0x09, 0, 0, 0, 0],
                module: 0x00,
                id: 0x01);
            CreateOpaquePackageFixture(
                fixDir,
                [],
                packageRole: "fix",
                levelName: "Fix",
                module: 0x02,
                id: 0x03);

            var table = OpenSpaceExporter.GenerateRtb(levelDir, "test.rtb", [fixDir]);
            var block = Assert.Single(table.Blocks);
            var pointer = Assert.Single(block.Pointers);

            Assert.Equal((uint)0x0900_0004, pointer.OffsetInMemory);
            Assert.Equal(0x02, pointer.TargetModule);
            Assert.Equal(0x03, pointer.TargetId);
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void ExportLevel_GeneratesRtbFromInlinePointers()
    {
        var packageDir = CreateTempDir();
        try
        {
            CreateOpaquePackageFixture(
                packageDir,
                new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" },
                module: 0x04,
                id: 0x05);
            var manifestPath = Path.Combine(packageDir, "manifest.json");
            var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(manifestPath),
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })!;
            manifest.RelocationTables =
            [
                new RelocationTableFileManifest { FileName = "test.rtb" }
            ];
            WriteJson(manifestPath, manifest, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });

            var outputDir = Path.Combine(packageDir, "export");
            OpenSpaceExporter.ExportLevel(packageDir, outputDir);

            var exportedPath = Path.Combine(outputDir, "test.rtb");
            Assert.True(File.Exists(exportedPath));

            var reader = new RelocationTableReader(exportedPath);
            var exportedBlock = Assert.Single(reader.PointerBlocks);
            var exportedPointer = Assert.Single(exportedBlock.Pointers);

            Assert.Equal(0x04, exportedBlock.Module);
            Assert.Equal(0x05, exportedBlock.Id);
            Assert.Equal(0x0900_0004u, exportedPointer.OffsetInMemory);
            Assert.Equal(0x04, exportedPointer.TargetModule);
            Assert.Equal(0x05, exportedPointer.TargetId);
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void ExportLevel_DoesNotReuseSnaEncodedCache()
    {
        var packageDir = CreateTempDir();
        try
        {
            CreateOpaquePackageFixture(
                packageDir,
                new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" },
                module: 0x04,
                id: 0x05);
            var manifestPath = Path.Combine(packageDir, "manifest.json");
            var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(manifestPath),
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })!;
            var block = manifest.SnaFiles.Single().Blocks.Single();
            var encodedBytes = new byte[] { 0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04 };
            var encodedPath = "sna/test/blocks/0000/0000_04_05.encoded.bin";
            Directory.CreateDirectory(Path.Combine(packageDir, Path.GetDirectoryName(encodedPath)!));
            File.WriteAllBytes(Path.Combine(packageDir, encodedPath), encodedBytes);
#pragma warning disable CS0618
            block.OriginalStorage = new SnaStorageManifest
            {
                IsCompressed = true,
                CompressedSize = (uint)encodedBytes.Length,
                CompressedChecksum = 0x12345678,
                DecompressedSize = 12,
                DecompressedChecksum = 0x87654321,
                EncodedPath = encodedPath,
                EncodedSha256 = Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(encodedBytes))
                    .ToLowerInvariant()
            };
#pragma warning restore CS0618
            WriteJson(manifestPath, manifest, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });

            var outputDir = Path.Combine(packageDir, "export");
            OpenSpaceExporter.ExportLevel(packageDir, outputDir);

            var exportedSna = File.ReadAllBytes(Path.Combine(outputDir, "test.sna"));
            Assert.DoesNotContain(encodedBytes, exportedSna);
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void CompareGeneratedRelocations_UnsupportedTable_ValidatesLooseFileSha256()
    {
        var packageDir = CreateTempDir();
        try
        {
            var looseData = new byte[] { 0x01, 0x02, 0x03, 0x04 };
            var loosePath = Path.Combine(packageDir, "files", "test.rtv");
            Directory.CreateDirectory(Path.GetDirectoryName(loosePath)!);
            File.WriteAllBytes(loosePath, looseData);
            var jsonOptions = new JsonSerializerOptions
            {
                PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
                PropertyNameCaseInsensitive = true
            };
            WriteJson(
                Path.Combine(packageDir, "manifest.json"),
                new RetePackageManifest
                {
                    PackageRole = "level",
                    LevelName = "test",
                    SourceDirectoryName = "test",
                    RelocationTables =
                    [
                        new RelocationTableFileManifest { FileName = "test.rtv" }
                    ],
                    LooseFiles =
                    [
                        new LooseFileManifest
                        {
                            FileName = "test.rtv",
                            Path = "files/test.rtv",
                            Size = looseData.Length,
                            Sha256 = Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(looseData))
                                .ToLowerInvariant()
                        }
                    ]
                },
                jsonOptions);

            var results = OpenSpaceExporter.CompareGeneratedRelocations(packageDir);
            var result = Assert.Single(results);

            Assert.True(result.Supported);
            Assert.True(result.PointerDataMatches);
            Assert.Contains("package fidelity", result.Note, StringComparison.OrdinalIgnoreCase);
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void CompareGeneratedRelocations_MatchesSourceDiscRtb()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var sourceDir = Path.Combine(workspaceDir, "disc", "Gamedata", "World", "Levels", "test");
            var packageDir = Path.Combine(workspaceDir, "rete", "test");
            Directory.CreateDirectory(sourceDir);

            CreateOpaquePackageFixture(
                packageDir,
                new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" },
                module: 0x04,
                id: 0x05);
            var manifestPath = Path.Combine(packageDir, "manifest.json");
            var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(manifestPath),
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })!;
            manifest.RelocationTables =
            [
                new RelocationTableFileManifest { FileName = "test.rtb" }
            ];
            WriteJson(manifestPath, manifest, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });

            var generated = OpenSpaceExporter.GenerateRtb(packageDir, "test.rtb", []);
            var sourceRtbPath = Path.Combine(sourceDir, "test.rtb");
            WriteSourceRtb(sourceRtbPath, generated);

            var results = OpenSpaceExporter.CompareGeneratedRelocations(packageDir);
            var result = Assert.Single(results);

            Assert.Equal(1, result.PreservedPointerCount);
            Assert.Equal(1, result.GeneratedPointerCount);
            Assert.Equal(1, result.MatchingPointerCount);
            Assert.Equal(0, result.MissingPointerCount);
            Assert.Equal(0, result.ExtraPointerCount);
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void CompareGeneratedRelocations_SourceNotFound_ReportsUnsupportedWithNote()
    {
        var packageDir = CreateTempDir();
        try
        {
            CreateOpaquePackageFixture(
                packageDir,
                new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" });
            var manifestPath = Path.Combine(packageDir, "manifest.json");
            var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(manifestPath),
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })!;
            manifest.RelocationTables =
            [
                new RelocationTableFileManifest { FileName = "test.rtb" }
            ];
            WriteJson(manifestPath, manifest, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });

            var results = OpenSpaceExporter.CompareGeneratedRelocations(packageDir);
            var result = Assert.Single(results);

            Assert.False(result.Supported);
            Assert.Contains("Source RT*", result.Note, StringComparison.Ordinal);
            Assert.Contains("ASTROLABE_SOURCE_DIR", result.Note, StringComparison.Ordinal);
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void CompareGeneratedRelocations_ResolvesSourceViaAstrolabeSourceDirParent()
    {
        var workspaceDir = CreateTempDir();
        var previousSourceDir = Environment.GetEnvironmentVariable("ASTROLABE_SOURCE_DIR");
        try
        {
            var sourceDir = Path.Combine(workspaceDir, "custom-levels", "test");
            var packageDir = Path.Combine(workspaceDir, "deep", "nested", "rete", "test");
            Directory.CreateDirectory(sourceDir);
            Directory.CreateDirectory(packageDir);
            Environment.SetEnvironmentVariable(
                "ASTROLABE_SOURCE_DIR",
                Path.Combine(workspaceDir, "custom-levels"));

            CreateOpaquePackageFixture(
                packageDir,
                new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" },
                module: 0x04,
                id: 0x05);
            var manifestPath = Path.Combine(packageDir, "manifest.json");
            var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(manifestPath),
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })!;
            manifest.RelocationTables =
            [
                new RelocationTableFileManifest { FileName = "test.rtb" }
            ];
            WriteJson(manifestPath, manifest, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });

            var generated = OpenSpaceExporter.GenerateRtb(packageDir, "test.rtb", []);
            WriteSourceRtb(Path.Combine(sourceDir, "test.rtb"), generated);

            var result = Assert.Single(OpenSpaceExporter.CompareGeneratedRelocations(packageDir));
            Assert.True(result.Supported);
            Assert.Equal(1, result.MatchingPointerCount);
        }
        finally
        {
            Environment.SetEnvironmentVariable("ASTROLABE_SOURCE_DIR", previousSourceDir);
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void CompareGeneratedRelocations_ResolvesSharedParentRelocationTable()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var levelsDir = Path.Combine(workspaceDir, "disc", "Gamedata", "World", "Levels");
            var sourceDir = Path.Combine(levelsDir, "test");
            var packageDir = Path.Combine(workspaceDir, "rete", "test");
            Directory.CreateDirectory(sourceDir);
            Directory.CreateDirectory(packageDir);

            CreateOpaquePackageFixture(
                packageDir,
                [],
                levelName: "test");
            var manifestPath = Path.Combine(packageDir, "manifest.json");
            var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(manifestPath),
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })!;
            manifest.RelocationTables =
            [
                new RelocationTableFileManifest { FileName = "Fix.rtb" }
            ];
            WriteJson(manifestPath, manifest, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });

            var fixRtb = new RelocationTableDocument
            {
                FileName = "Fix.rtb",
                Blocks =
                [
                    new RelocationPointerBlockManifest
                    {
                        Order = 0,
                        Key = "02:03",
                        Module = 0x02,
                        Id = 0x03,
                        EntrySize = 8,
                        Pointers =
                        [
                            new RelocationPointerManifest
                            {
                                OffsetInMemory = 0x0900_0004,
                                TargetModule = 0x02,
                                TargetId = 0x03
                            }
                        ]
                    }
                ]
            };
            WriteSourceRtb(Path.Combine(levelsDir, "Fix.rtb"), fixRtb);

            var result = Assert.Single(OpenSpaceExporter.CompareGeneratedRelocations(packageDir));
            Assert.True(result.Supported);
            Assert.Equal(1, result.PreservedPointerCount);
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void ExportLevel_FixLevelRtb_GeneratesFromUriLut()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var levelDir = Path.Combine(workspaceDir, "level");
            var fixDir = Path.Combine(workspaceDir, "fix");
            var fixSourceData = new byte[8];
            BinaryPrimitives.WriteInt32LittleEndian(fixSourceData.AsSpan(0, 4), 0x0900_0000);
            CreateOpaquePackageFixture(
                levelDir,
                [],
                module: 0x06,
                id: 0x02,
                baseInMemory: 0x0900_0000);
            CreateOpaquePackageFixture(
                fixDir,
                new Dictionary<string, string?> { ["0x0"] = "level:/slots/0x0200_0004.json" },
                sourceData: fixSourceData,
                packageRole: "fix",
                levelName: "Fix",
                module: 0x05,
                id: 0x00,
                baseInMemory: 0x0200_0000);

            var slotPath = Path.Combine(levelDir, "slots", "0x0200_0004.json");
            Directory.CreateDirectory(Path.GetDirectoryName(slotPath)!);
            File.Copy(
                Path.Combine(levelDir, "types/raw/target.json"),
                slotPath,
                overwrite: true);

            var levelManifestPath = Path.Combine(levelDir, "manifest.json");
            var levelManifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(levelManifestPath),
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })!;
            levelManifest.RelocationTables =
            [
                new RelocationTableFileManifest { FileName = "fixlvl.rtb" }
            ];
            WriteJson(levelManifestPath, levelManifest, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });

            var outputDir = Path.Combine(workspaceDir, "export");
            OpenSpaceExporter.ExportLevel(levelDir, outputDir);

            Assert.True(File.Exists(Path.Combine(outputDir, "fixlvl.rtb")));
            Assert.True(File.Exists(Path.Combine(outputDir, "test.sna")));
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void GenerateFixLevelRtb_LevelUri_ResolvesMappedTarget()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var levelDir = Path.Combine(workspaceDir, "level");
            var fixDir = Path.Combine(workspaceDir, "fix");
            var fixSourceData = new byte[8];
            BinaryPrimitives.WriteInt32LittleEndian(fixSourceData.AsSpan(0, 4), 0x0900_0000);
            CreateOpaquePackageFixture(
                levelDir,
                [],
                module: 0x06,
                id: 0x02,
                baseInMemory: 0x0900_0000);
            CreateOpaquePackageFixture(
                fixDir,
                new Dictionary<string, string?> { ["0x0"] = "level:/slots/0x0200_0004.json" },
                sourceData: fixSourceData,
                packageRole: "fix",
                levelName: "Fix",
                module: 0x05,
                id: 0x00,
                baseInMemory: 0x0200_0000);

            var slotPath = Path.Combine(levelDir, "slots", "0x0200_0004.json");
            Directory.CreateDirectory(Path.GetDirectoryName(slotPath)!);
            File.Copy(
                Path.Combine(levelDir, "types/raw/target.json"),
                slotPath,
                overwrite: true);

            var table = OpenSpaceExporter.GenerateFixLevelRtb(fixDir, levelDir, "fixlvl.rtb");
            var block = Assert.Single(table.Blocks);
            var pointer = Assert.Single(block.Pointers);

            Assert.Equal((uint)0x0200_0004, pointer.OffsetInMemory);
            Assert.Equal(0x06, pointer.TargetModule);
            Assert.Equal(0x02, pointer.TargetId);
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void GenerateFixLevelRtb_EscapingPointerWithoutUri_ResolvesLevelTargetByValue()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var levelDir = Path.Combine(workspaceDir, "level");
            var fixDir = Path.Combine(workspaceDir, "fix");
            var fixSourceData = new byte[8];
            BinaryPrimitives.WriteInt32LittleEndian(fixSourceData.AsSpan(0, 4), 0x0900_0000);
            CreateOpaquePackageFixture(
                levelDir,
                [],
                module: 0x06,
                id: 0x02,
                baseInMemory: 0x0900_0000);
            CreateOpaquePackageFixture(
                fixDir,
                new Dictionary<string, string?> { ["0x0"] = null },
                sourceData: fixSourceData,
                packageRole: "fix",
                levelName: "Fix",
                module: 0x05,
                id: 0x00,
                baseInMemory: 0x0200_0000);
            var table = OpenSpaceExporter.GenerateFixLevelRtb(fixDir, levelDir, "fixlvl.rtb");
            var block = Assert.Single(table.Blocks);
            var pointer = Assert.Single(block.Pointers);

            Assert.Equal((uint)0x0200_0004, pointer.OffsetInMemory);
            Assert.Equal(0x06, pointer.TargetModule);
            Assert.Equal(0x02, pointer.TargetId);
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void GenerateFixLevelRtb_EscapingPointerWithoutUri_EmitsSentinel()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var levelDir = Path.Combine(workspaceDir, "level");
            var fixDir = Path.Combine(workspaceDir, "fix");
            var fixSourceData = new byte[8];
            BinaryPrimitives.WriteInt32LittleEndian(fixSourceData.AsSpan(0, 4), 0x1234_5678);
            CreateOpaquePackageFixture(
                levelDir,
                [],
                module: 0x06,
                id: 0x02,
                baseInMemory: 0x0900_0000);
            CreateOpaquePackageFixture(
                fixDir,
                new Dictionary<string, string?> { ["0x0"] = null },
                sourceData: fixSourceData,
                packageRole: "fix",
                levelName: "Fix",
                module: 0x05,
                id: 0x00,
                baseInMemory: 0x0200_0000);
            var table = OpenSpaceExporter.GenerateFixLevelRtb(fixDir, levelDir, "fixlvl.rtb");
            var block = Assert.Single(table.Blocks);
            var pointer = Assert.Single(block.Pointers);

            Assert.Equal((uint)0x0200_0004, pointer.OffsetInMemory);
            Assert.Equal(0xFF, pointer.TargetModule);
            Assert.Equal(0xFF, pointer.TargetId);
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void GenerateRtb_StructOverlay_NonVmSentinel_EmitsFfFf()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            StructCodecRegistry.Register(TestStructWithExtraPointerCodec.Instance);

            var packageDir = Path.Combine(workspaceDir, "rete");
            var sourceData = new byte[8];
            BinaryPrimitives.WriteInt32LittleEndian(sourceData.AsSpan(0, 4), 0x0900_0000);
            BinaryPrimitives.WriteInt32LittleEndian(sourceData.AsSpan(4, 4), 0x1234_5678);
            Assert.True(StructCodecRegistry.TryGet(
                TestStructWithExtraPointerCodec.Instance.Kind,
                out var structCodec));
            CreateStructPackageFixture(
                packageDir,
                sourceData,
                structCodec,
                module: 0x04,
                id: 0x05);

            var sourceJsonPath = Path.Combine(packageDir, "types/teststruct_extra_ptr/source.json");
            ReferenceJson.ApplyStructPointerLut(
                sourceJsonPath,
                new Dictionary<string, string?> { ["0x4"] = null });

            var table = OpenSpaceExporter.GenerateRtb(packageDir, "test.rtb", []);
            var sentinel = table.Blocks.Single().Pointers.Single(
                pointer => pointer.TargetModule == 0xFF && pointer.TargetId == 0xFF);

            Assert.Equal((uint)0x0900_000C, sentinel.OffsetInMemory);
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void GenerateFixLevelRtb_InVmDiscSentinel_EmitsFfFf()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var levelDir = Path.Combine(workspaceDir, "level");
            var fixDir = Path.Combine(workspaceDir, "fix");
            var fixSourceData = new byte[8];
            BinaryPrimitives.WriteInt32LittleEndian(fixSourceData.AsSpan(0, 4), 0x0200_0100);
            CreateOpaquePackageFixture(
                levelDir,
                [],
                module: 0x06,
                id: 0x02,
                baseInMemory: 0x0900_0000);
            CreateOpaquePackageFixture(
                fixDir,
                new Dictionary<string, string?> { ["0x0"] = null },
                sourceData: fixSourceData,
                packageRole: "fix",
                levelName: "Fix",
                module: 0x05,
                id: 0x00,
                baseInMemory: 0x0200_0000);

            var table = OpenSpaceExporter.GenerateFixLevelRtb(fixDir, levelDir, "fixlvl.rtb");
            var pointer = Assert.Single(table.Blocks.Single().Pointers);

            Assert.Equal((uint)0x0200_0004, pointer.OffsetInMemory);
            Assert.Equal(0xFF, pointer.TargetModule);
            Assert.Equal(0xFF, pointer.TargetId);
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void GenerateFixLevelRtb_StructPointerFieldsWithoutLut_DoesNotOverScan()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            StructCodecRegistry.Register(TestStructWithExtraPointerCodec.Instance);

            var levelDir = Path.Combine(workspaceDir, "level");
            var fixDir = Path.Combine(workspaceDir, "fix");
            var sourceData = new byte[8];
            BinaryPrimitives.WriteInt32LittleEndian(sourceData.AsSpan(0, 4), 0x0900_0000);
            BinaryPrimitives.WriteInt32LittleEndian(sourceData.AsSpan(4, 4), 0x1234_5678);
            Assert.True(StructCodecRegistry.TryGet(
                TestStructWithExtraPointerCodec.Instance.Kind,
                out var structCodec));
            CreateOpaquePackageFixture(levelDir, [], module: 0x06, id: 0x02, baseInMemory: 0x0900_0000);
            CreateStructPackageFixture(
                fixDir,
                sourceData,
                structCodec,
                packageRole: "fix",
                levelName: "Fix",
                module: 0x05,
                id: 0x00,
                baseInMemory: 0x0200_0000);

            var table = OpenSpaceExporter.GenerateFixLevelRtb(fixDir, levelDir, "fixlvl.rtb");

            Assert.Empty(table.Blocks);
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void AnnotateFixLevelRelocations_MappedRow_WritesSlotFileAndLevelUri()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var levelDir = Path.Combine(workspaceDir, "level");
            var fixDir = Path.Combine(workspaceDir, "fix");
            var levelSourceDir = Path.Combine(workspaceDir, "source-level");
            Directory.CreateDirectory(levelSourceDir);

            var fixSourceData = new byte[8];
            BinaryPrimitives.WriteInt32LittleEndian(fixSourceData.AsSpan(0, 4), 0x0900_0000);
            CreateOpaquePackageFixture(
                levelDir,
                [],
                module: 0x06,
                id: 0x02,
                baseInMemory: 0x0900_0000);
            CreateOpaquePackageFixture(
                fixDir,
                [],
                sourceData: fixSourceData,
                packageRole: "fix",
                levelName: "Fix",
                module: 0x05,
                id: 0x00,
                baseInMemory: 0x0200_0000);

            WriteSourceRtb(
                Path.Combine(levelSourceDir, "fixlvl.rtb"),
                new RelocationTableDocument
                {
                    FileName = "fixlvl.rtb",
                    Blocks =
                    [
                        new RelocationPointerBlockManifest
                        {
                            Order = 0,
                            Key = "05:00",
                            Module = 0x05,
                            Id = 0x00,
                            EntrySize = 8,
                            Pointers =
                            [
                                new RelocationPointerManifest
                                {
                                    OffsetInMemory = 0x0200_0004,
                                    TargetModule = 0x06,
                                    TargetId = 0x02
                                }
                            ]
                        }
                    ]
                });

            var levelManifestPath = Path.Combine(levelDir, "manifest.json");
            var levelManifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(levelManifestPath),
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })!;
            levelManifest.RelocationTables =
            [
                new RelocationTableFileManifest { FileName = "fixlvl.rtb" }
            ];
            WriteJson(levelManifestPath, levelManifest, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });

            InvokeAnnotateOpaquePointersFromFixLevelRelocations(fixDir, levelDir, levelSourceDir);

            const string expectedSlotUri = "level:/slots/0x02000004.json";
            var fixSourceJsonPath = Path.Combine(fixDir, "types/raw/source.json");
            using var document = JsonDocument.Parse(File.ReadAllText(fixSourceJsonPath));
            Assert.True(document.RootElement.TryGetProperty("pointers", out var pointers));
            Assert.True(pointers.TryGetProperty("0x0", out var uri));
            Assert.Equal(expectedSlotUri, uri.GetString());

            var slotPath = Path.Combine(levelDir, "slots", "0x02000004.json");
            Assert.True(File.Exists(slotPath));
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void GenerateRtb_OpaqueLut_ReadsRawBinNotExportBytes()
    {
        var packageDir = CreateTempDir();
        try
        {
            var sourceData = new byte[8];
            BinaryPrimitives.WriteInt32LittleEndian(sourceData.AsSpan(0, 4), 0x0900_0004);
            CreateOpaquePackageFixture(
                packageDir,
                new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" },
                sourceData: sourceData,
                module: 0x04,
                id: 0x05);

            var table = OpenSpaceExporter.GenerateRtb(packageDir, "test.rtb", []);
            var pointer = table.Blocks.Single().Pointers.Single(
                p => p.OffsetInMemory == 0x0900_0004u);

            Assert.Equal(0x04, pointer.TargetModule);
            Assert.Equal(0x05, pointer.TargetId);
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void CompileRelocationTable_DoesNotReadSourceDisc()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var packageDir = Path.Combine(workspaceDir, "rete");
            var sourceDir = Path.Combine(
                workspaceDir,
                "disc",
                "Gamedata",
                "World",
                "Levels",
                "source");
            Directory.CreateDirectory(sourceDir);

            CreateOpaquePackageFixture(
                packageDir,
                new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" },
                module: 0x04,
                id: 0x05);
            var manifestPath = Path.Combine(packageDir, "manifest.json");
            var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(manifestPath),
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })!;
            manifest.SourceDirectoryName = "source";
            WriteJson(manifestPath, manifest, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });

            var generated = OpenSpaceExporter.GenerateRtb(packageDir, "test.rtb", []);
            var generatedPointer = generated.Blocks.Single().Pointers.Single(
                pointer => pointer.OffsetInMemory == 0x0900_0004u);
            generatedPointer.Byte6 = 0x12;
            generatedPointer.Byte7 = 0x34;

            var sourceRtbPath = Path.Combine(sourceDir, "test.rtb");
            WriteSourceRtb(
                sourceRtbPath,
                new RelocationTableDocument
                {
                    FileName = "test.rtb",
                    Blocks =
                    [
                        new RelocationPointerBlockManifest
                        {
                            Order = 0,
                            Key = "04:05",
                            Module = 0x04,
                            Id = 0x05,
                            EntrySize = 8,
                            Pointers =
                            [
                                new RelocationPointerManifest
                                {
                                    OffsetInMemory = generatedPointer.OffsetInMemory,
                                    TargetModule = generatedPointer.TargetModule,
                                    TargetId = generatedPointer.TargetId,
                                    Byte6 = 0x99,
                                    Byte7 = 0xAA
                                }
                            ]
                        }
                    ]
                });

            var outputPath = Path.Combine(packageDir, "compiled.rtb");
            InvokeCompileRelocationTable(packageDir, generated, outputPath);

            Assert.True(File.Exists(outputPath));
            Assert.NotEqual(File.ReadAllBytes(sourceRtbPath), File.ReadAllBytes(outputPath));
            Assert.Equal(0x12, generatedPointer.Byte6);
            Assert.Equal(0x34, generatedPointer.Byte7);
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void AnnotateSourceRelocations_TransientRtb_MergesStructPointerOverlay()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            StructCodecRegistry.Register(TestStructWithExtraPointerCodec.Instance);

            var sourceDir = Path.Combine(workspaceDir, "source");
            var packageDir = Path.Combine(workspaceDir, "rete");
            Directory.CreateDirectory(sourceDir);

            var sourceData = new byte[8];
            BinaryPrimitives.WriteInt32LittleEndian(sourceData.AsSpan(0, 4), 0x0900_0000);
            BinaryPrimitives.WriteInt32LittleEndian(sourceData.AsSpan(4, 4), 0x0900_0004);
            StructCodecRegistry.Register(TestStructWithExtraPointerCodec.Instance);
            Assert.True(StructCodecRegistry.TryGet(
                TestStructWithExtraPointerCodec.Instance.Kind,
                out var structCodec));
            CreateStructPackageFixture(
                packageDir,
                sourceData,
                structCodec,
                module: 0x04,
                id: 0x05);

            WriteSourceRtb(
                Path.Combine(sourceDir, "test.rtb"),
                new RelocationTableDocument
                {
                    FileName = "test.rtb",
                    Blocks =
                    [
                        new RelocationPointerBlockManifest
                        {
                            Order = 0,
                            Key = "04:05",
                            Module = 0x04,
                            Id = 0x05,
                            EntrySize = 8,
                            Pointers =
                            [
                                new RelocationPointerManifest
                                {
                                    OffsetInMemory = 0x0900_000C,
                                    TargetModule = 0x04,
                                    TargetId = 0x05
                                }
                            ]
                        }
                    ]
                });

            var manifestPath = Path.Combine(packageDir, "manifest.json");
            var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(manifestPath),
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })!;
            manifest.RelocationTables =
            [
                new RelocationTableFileManifest { FileName = "test.rtb" }
            ];
            WriteJson(manifestPath, manifest, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });

            InvokeAnnotateOpaquePointersFromSourceRelocations(packageDir, null, sourceDir);

            var sourceJsonPath = Path.Combine(packageDir, "types/teststruct_extra_ptr/source.json");
            using var document = JsonDocument.Parse(File.ReadAllText(sourceJsonPath));
            Assert.True(document.RootElement.TryGetProperty("pointers", out var pointers));
            Assert.True(pointers.TryGetProperty("0x4", out _));

            var table = OpenSpaceExporter.GenerateRtb(packageDir, "test.rtb", []);
            var offsets = table.Blocks.Single().Pointers.Select(p => p.OffsetInMemory).ToHashSet();
            Assert.Contains(0x0900_0008u, offsets);
            Assert.Contains(0x0900_000Cu, offsets);
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void AnnotateSourceRelocations_TransientRtb_MergesIntoInlineLut()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var sourceDir = Path.Combine(workspaceDir, "source");
            var packageDir = Path.Combine(workspaceDir, "rete");
            Directory.CreateDirectory(sourceDir);

            var sourceData = new byte[8];
            BinaryPrimitives.WriteInt32LittleEndian(sourceData.AsSpan(0, 4), 0x0900_0000);
            CreateOpaquePackageFixture(
                packageDir,
                [],
                sourceData: sourceData,
                module: 0x04,
                id: 0x05);

            WriteSourceRtb(
                Path.Combine(sourceDir, "test.rtb"),
                new RelocationTableDocument
                {
                    FileName = "test.rtb",
                    Blocks =
                    [
                        new RelocationPointerBlockManifest
                        {
                            Order = 0,
                            Key = "04:05",
                            Module = 0x04,
                            Id = 0x05,
                            EntrySize = 8,
                            Pointers =
                            [
                                new RelocationPointerManifest
                                {
                                    OffsetInMemory = 0x0900_0004,
                                    TargetModule = 0x04,
                                    TargetId = 0x05
                                }
                            ]
                        }
                    ]
                });

            var manifestPath = Path.Combine(packageDir, "manifest.json");
            var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(manifestPath),
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })!;
            manifest.RelocationTables =
            [
                new RelocationTableFileManifest { FileName = "test.rtb" }
            ];
            WriteJson(manifestPath, manifest, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });

            InvokeAnnotateOpaquePointersFromSourceRelocations(packageDir, null, sourceDir);

            var sourceJsonPath = Path.Combine(packageDir, "types/raw/source.json");
            using var document = JsonDocument.Parse(File.ReadAllText(sourceJsonPath));
            Assert.True(document.RootElement.TryGetProperty("pointers", out var pointers));
            Assert.True(pointers.TryGetProperty("0x0", out var uri));
            Assert.Equal("types/raw/target.json", uri.GetString());
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void ExportLevel_LegacyRelocationArtifacts_Throws()
    {
        var packageDir = CreateTempDir();
        try
        {
            CreateOpaquePackageFixture(packageDir, new Dictionary<string, string?> { ["0x0"] = null });
            var relocPath = Path.Combine(packageDir, "types/raw/source.reloc.json");
            Directory.CreateDirectory(Path.GetDirectoryName(relocPath)!);
            File.WriteAllText(relocPath, """{"schema":"astrolabe.relocation-overlay.v1","pointers":{}}""");

            var ex = Assert.Throws<InvalidDataException>(() =>
                OpenSpaceExporter.ExportLevel(packageDir, Path.Combine(packageDir, "export")));
            Assert.Contains("legacy relocation bridge artifacts", ex.Message, StringComparison.OrdinalIgnoreCase);
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void ExportLevel_LegacyFixPackageSites_Throws()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var levelDir = Path.Combine(workspaceDir, "level");
            var fixDir = Path.Combine(workspaceDir, "fix");
            CreateOpaquePackageFixture(levelDir, new Dictionary<string, string?> { ["0x0"] = null });
            CreateOpaquePackageFixture(
                fixDir,
                new Dictionary<string, string?> { ["0x0"] = null },
                packageRole: "fix",
                levelName: "Fix");

            var sitesPath = Path.Combine(fixDir, "semantic", "fixlvl-sites.json");
            Directory.CreateDirectory(Path.GetDirectoryName(sitesPath)!);
            File.WriteAllText(sitesPath, """{"schema":"astrolabe.fixlvl-sites.v1","sites":{}}""");

            var ex = Assert.Throws<InvalidDataException>(() =>
                OpenSpaceExporter.ExportLevel(levelDir, Path.Combine(workspaceDir, "export")));
            Assert.Contains("legacy relocation bridge artifacts", ex.Message, StringComparison.OrdinalIgnoreCase);
            Assert.Contains("fixlvl-sites.json", ex.Message, StringComparison.OrdinalIgnoreCase);
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void LegacyManifest_ExtraRelocationFields_DeserializeAndIgnore()
    {
        var json = """
            {
              "schema": "astrolabe.rete.v1",
              "packageRole": "level",
              "levelName": "test",
              "sourceDirectoryName": "test",
              "relocationTables": [
                {
                  "fileName": "test.rtb",
                  "jsonPath": "relocations/test.rtb.json",
                  "encodingPath": "semantic/test.rtb-encoding.json"
                }
              ],
              "semantic": {
                "rtbSitesPath": "semantic/rtb-sites.json",
                "fixLevelSitesPath": "semantic/fix-level-sites.json",
                "pointerFileSitesPaths": { "test.rtp": "semantic/test.rtp-sites.json" }
              }
            }
            """;
        var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
            json,
            new JsonSerializerOptions { PropertyNameCaseInsensitive = true })!;

        Assert.Equal("test.rtb", Assert.Single(manifest.RelocationTables).FileName);
        Assert.NotNull(manifest.Semantic);
    }

    [Fact]
    public void AnnotateFixLevelRelocations_MissingSource_RecordsSemanticError()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var levelDir = Path.Combine(workspaceDir, "level");
            var fixDir = Path.Combine(workspaceDir, "fix");
            var levelSourceDir = Path.Combine(workspaceDir, "source-level");
            Directory.CreateDirectory(levelSourceDir);

            CreateOpaquePackageFixture(levelDir, [], module: 0x04, id: 0x05);
            CreateOpaquePackageFixture(
                fixDir,
                [],
                packageRole: "fix",
                levelName: "Fix",
                module: 0x02,
                id: 0x03);

            var levelManifestPath = Path.Combine(levelDir, "manifest.json");
            var levelManifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(levelManifestPath),
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })!;
            levelManifest.RelocationTables =
            [
                new RelocationTableFileManifest { FileName = "fixlvl.rtb" }
            ];
            WriteJson(levelManifestPath, levelManifest, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });

            InvokeAnnotateOpaquePointersFromFixLevelRelocations(fixDir, levelDir, levelSourceDir);

            levelManifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(levelManifestPath),
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })!;
            Assert.Contains(
                levelManifest.Semantic!.Errors,
                error => error.Contains("fixlvl.rtb", StringComparison.OrdinalIgnoreCase));
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void GenerateRtb_PartialOpaqueLut_FallsBackToCodecPointerFields()
    {
        var packageDir = CreateTempDir();
        try
        {
            StructCodecRegistry.Register(TestOpaqueWithPointerMetadataCodec.Instance);

            // LUT only covers offset 0x0 (explicit null); export materializes zero there while the
            // VM pointer remains at 0x4 in raw data. EmitOpaqueInlinePointers emits nothing and
            // returns false; static codec pointer metadata finds the pointer via PointerFields.
            var sourceData = new byte[8];
            BinaryPrimitives.WriteInt32LittleEndian(sourceData.AsSpan(4, 4), 0x0900_0000);
            CreateOpaquePackageFixture(
                packageDir,
                new Dictionary<string, string?> { ["0x0"] = null },
                sourceData: sourceData,
                kind: TestOpaqueWithPointerMetadataCodec.Instance.Kind,
                schema: TestOpaqueWithPointerMetadataCodec.Instance.Schema);

            var table = OpenSpaceExporter.GenerateRtb(packageDir, "test.rtb", []);
            var block = Assert.Single(table.Blocks);
            var pointer = Assert.Single(block.Pointers);

            Assert.Equal((uint)0x0900_0008, pointer.OffsetInMemory);
            Assert.Equal(0x00, pointer.TargetModule);
            Assert.Equal(0x01, pointer.TargetId);
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void CompareGeneratedRelocations_LegacyArtifacts_AppendsWarning()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var sourceDir = Path.Combine(workspaceDir, "disc", "Gamedata", "World", "Levels", "test");
            var packageDir = Path.Combine(workspaceDir, "rete", "test");
            Directory.CreateDirectory(sourceDir);

            CreateOpaquePackageFixture(
                packageDir,
                new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" },
                module: 0x04,
                id: 0x05);
            var manifestPath = Path.Combine(packageDir, "manifest.json");
            var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(manifestPath),
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase })!;
            manifest.RelocationTables =
            [
                new RelocationTableFileManifest { FileName = "test.rtb" }
            ];
            WriteJson(manifestPath, manifest, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });

            var generated = OpenSpaceExporter.GenerateRtb(packageDir, "test.rtb", []);
            WriteSourceRtb(Path.Combine(sourceDir, "test.rtb"), generated);

            var relocPath = Path.Combine(packageDir, "types/raw/source.reloc.json");
            Directory.CreateDirectory(Path.GetDirectoryName(relocPath)!);
            File.WriteAllText(relocPath, """{"schema":"astrolabe.relocation-overlay.v1","pointers":{}}""");

            var result = Assert.Single(OpenSpaceExporter.CompareGeneratedRelocations(packageDir));

            Assert.True(result.Supported);
            Assert.Equal(1, result.MatchingPointerCount);
            Assert.Contains("legacy relocation bridge artifacts", result.Note, StringComparison.OrdinalIgnoreCase);
            Assert.Contains("types/raw/source.reloc.json", result.Note, StringComparison.OrdinalIgnoreCase);
        }
        finally
        {
            Directory.Delete(workspaceDir, true);
        }
    }

    [Fact]
    public void ReferenceAddressResolver_InteriorElementAddress_UsesByteOffsetFragment()
    {
        var packageDir = CreateTempDir();
        try
        {
            CreateOpaquePackageFixture(
                packageDir,
                [],
                targetData: [0xAA, 0xBB, 0xCC, 0xDD, 0x11, 0x22, 0x33, 0x44]);

            var resolver = new ReferenceAddressResolver(packageDir);

            Assert.True(resolver.TryGetReferenceUri(0x0900_0000, packageDir, out var exactUri));
            Assert.True(resolver.TryGetReferenceUri(0x0900_0004, packageDir, out var interiorUri));
            Assert.Equal("types/raw/target.json", exactUri);
            Assert.Equal("types/raw/target.json#byteOffset=4", interiorUri);
            Assert.Equal(0x0900_0004, resolver.ResolveAddress(packageDir, interiorUri));
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    private static void WriteSourceRtb(string outputPath, RelocationTableDocument document)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(outputPath)!);
        using var writer = new BinaryWriter(File.Create(outputPath));
        writer.Write((byte)document.Blocks.Count);
        foreach (var block in document.Blocks.OrderBy(b => b.Order))
        {
            writer.Write(block.Module);
            writer.Write(block.Id);
            writer.Write((uint)block.Pointers.Count);
            if (block.Pointers.Count == 0)
            {
                continue;
            }

            using var stream = new MemoryStream();
            using (var pointerWriter = new BinaryWriter(stream))
            {
                foreach (var pointer in block.Pointers)
                {
                    pointerWriter.Write(pointer.OffsetInMemory);
                    pointerWriter.Write(pointer.TargetModule);
                    pointerWriter.Write(pointer.TargetId);
                    pointerWriter.Write(pointer.Byte6);
                    pointerWriter.Write(pointer.Byte7);
                }
            }

            var pointerData = stream.ToArray();
            var checksum = OpenSpaceChecksum.Calculate(pointerData);
            writer.Write(0u);
            writer.Write((uint)pointerData.Length);
            writer.Write(checksum);
            writer.Write((uint)pointerData.Length);
            writer.Write(checksum);
            writer.Write(pointerData);
        }
    }

    private static void CreateStructPackageFixture(
        string packageDir,
        byte[] sourceData,
        IStructCodecBinding codec,
        byte module = 0x00,
        byte id = 0x01,
        int baseInMemory = 0x0900_0000,
        string packageRole = "level",
        string levelName = "test")
    {
        var targetData = new byte[8];
        BinaryPrimitives.WriteInt32LittleEndian(targetData.AsSpan(0), 0x0900_0000);
        var sourceOffset = targetData.Length;
        var jsonOptions = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase
        };

        Directory.CreateDirectory(packageDir);

        var manifest = new RetePackageManifest
        {
            PackageRole = packageRole,
            LevelName = levelName,
            SourceDirectoryName = levelName,
            SnaFiles =
            [
                new SnaFileManifest
                {
                    FileName = "test.sna",
                    Blocks =
                    [
                        new SnaBlockManifest
                        {
                            Order = 0,
                            Key = $"{module:X2}:{id:X2}",
                            Module = module,
                            Id = id,
                            BaseInMemory = baseInMemory,
                            HasPayload = true,
                            ContentPath = "sna/test/blocks/0000/content.json"
                        }
                    ]
                }
            ]
        };

        var content = new SnaBlockContentDocument
        {
            FileName = "test.sna",
            BlockOrder = 0,
            BlockKey = $"{module:X2}:{id:X2}",
            Module = module,
            Id = id,
            BaseInMemory = baseInMemory,
            BaseInMemoryHex = $"0x{baseInMemory:X8}",
            OriginalDataSha256 = "test",
            Elements =
            [
                new SnaBlockContentElement
                {
                    Order = 0,
                    Kind = codec.Kind,
                    DataPath = "types/teststruct_extra_ptr/target.json",
                    OffsetInBlock = 0,
                    Length = targetData.Length,
                    VirtualAddress = baseInMemory,
                    VirtualAddressHex = $"0x{baseInMemory:X8}",
                    Sha256 = "target"
                },
                new SnaBlockContentElement
                {
                    Order = 1,
                    Kind = codec.Kind,
                    DataPath = "types/teststruct_extra_ptr/source.json",
                    OffsetInBlock = sourceOffset,
                    Length = sourceData.Length,
                    VirtualAddress = baseInMemory + sourceOffset,
                    VirtualAddressHex = $"0x{baseInMemory + sourceOffset:X8}",
                    Sha256 = "source"
                }
            ]
        };

        WriteJson(Path.Combine(packageDir, "manifest.json"), manifest, jsonOptions);
        WriteJson(Path.Combine(packageDir, "sna/test/blocks/0000/content.json"), content, jsonOptions);

        var targetRecord = codec.ReadFromBytes(targetData, 0, targetData.Length);
        codec.WriteJson(packageDir, Path.Combine(packageDir, "types/teststruct_extra_ptr/target.json"), targetRecord);

        var sourceRecord = codec.ReadFromBytes(sourceData, 0, sourceData.Length);
        codec.WriteJson(packageDir, Path.Combine(packageDir, "types/teststruct_extra_ptr/source.json"), sourceRecord);
    }

    private static void CreateOpaquePackageFixture(
        string packageDir,
        Dictionary<string, string?> pointers,
        string kind = "raw",
        string? schema = null,
        byte[]? sourceData = null,
        byte[]? targetData = null,
        string packageRole = "level",
        string levelName = "test",
        byte module = 0x00,
        byte id = 0x01,
        int baseInMemory = 0x0900_0000)
    {
        schema ??= RawBlobCodec.Instance.Schema;
        targetData ??= [0xAA, 0xBB, 0xCC, 0xDD];
        sourceData ??= [0x78, 0x56, 0x34, 0x12, 0, 0, 0, 0];
        var sourceOffset = targetData.Length;
        var jsonOptions = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase
        };

        Directory.CreateDirectory(packageDir);
        Assert.True(StructCodecRegistry.TryGet(kind, out var codec));

        var manifest = new RetePackageManifest
        {
            PackageRole = packageRole,
            LevelName = levelName,
            SourceDirectoryName = levelName,
            SnaFiles =
            [
                new SnaFileManifest
                {
                    FileName = "test.sna",
                    Blocks =
                    [
                        new SnaBlockManifest
                        {
                            Order = 0,
                            Key = $"{module:X2}:{id:X2}",
                            Module = module,
                            Id = id,
                            BaseInMemory = baseInMemory,
                            HasPayload = true,
                            ContentPath = "sna/test/blocks/0000/content.json"
                        }
                    ]
                }
            ]
        };

        var content = new SnaBlockContentDocument
        {
            FileName = "test.sna",
            BlockOrder = 0,
            BlockKey = $"{module:X2}:{id:X2}",
            Module = module,
            Id = id,
            BaseInMemory = baseInMemory,
            BaseInMemoryHex = $"0x{baseInMemory:X8}",
            OriginalDataSha256 = "test",
            Elements =
            [
                new SnaBlockContentElement
                {
                    Order = 0,
                    Kind = kind,
                    DataPath = "types/raw/target.json",
                    OffsetInBlock = 0,
                    Length = targetData.Length,
                    VirtualAddress = baseInMemory,
                    VirtualAddressHex = $"0x{baseInMemory:X8}",
                    Sha256 = "target"
                },
                new SnaBlockContentElement
                {
                    Order = 1,
                    Kind = kind,
                    DataPath = "types/raw/source.json",
                    OffsetInBlock = sourceOffset,
                    Length = sourceData.Length,
                    VirtualAddress = baseInMemory + sourceOffset,
                    VirtualAddressHex = $"0x{baseInMemory + sourceOffset:X8}",
                    Sha256 = "source"
                }
            ]
        };

        WriteJson(Path.Combine(packageDir, "manifest.json"), manifest, jsonOptions);
        WriteJson(Path.Combine(packageDir, "sna/test/blocks/0000/content.json"), content, jsonOptions);

        codec.WriteJson(packageDir, Path.Combine(packageDir, "types/raw/target.json"), new OpaqueBinaryRecord
        {
            Schema = schema,
            Data = targetData
        });

        codec.WriteJson(packageDir, Path.Combine(packageDir, "types/raw/source.json"), new OpaqueBinaryRecord
        {
            Schema = schema,
            Data = sourceData,
            Pointers = pointers
        });
    }

    private static void InvokeCompileRelocationTable(
        string packageDir,
        RelocationTableDocument document,
        string outputPath)
    {
        var method = typeof(OpenSpacePackageCodec).GetMethod(
            "CompileRelocationTable",
            BindingFlags.Static | BindingFlags.NonPublic);
        Assert.NotNull(method);
        method!.Invoke(null, [packageDir, document, outputPath]);
    }

    private static void InvokeAnnotateOpaquePointersFromFixLevelRelocations(
        string fixPackageDir,
        string levelPackageDir,
        string levelSourceDir)
    {
        var method = typeof(OpenSpacePackageCodec).GetMethod(
            "AnnotateOpaquePointersFromFixLevelRelocations",
            BindingFlags.Static | BindingFlags.NonPublic);
        Assert.NotNull(method);
        method!.Invoke(null, [fixPackageDir, levelPackageDir, levelSourceDir]);
    }

    private static void InvokeAnnotateOpaquePointersFromSourceRelocations(
        string packageDir,
        string? extraPackageDir,
        string? sourceDir)
    {
        var method = typeof(OpenSpacePackageCodec).GetMethod(
            "AnnotateOpaquePointersFromSourceRelocations",
            BindingFlags.Static | BindingFlags.NonPublic);
        Assert.NotNull(method);
        method!.Invoke(null, [packageDir, extraPackageDir, sourceDir]);
    }

    private static void WriteJson<T>(string path, T value, JsonSerializerOptions options)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllText(path, JsonSerializer.Serialize(value, options));
    }

    private static string CreateTempDir()
    {
        var path = Path.Combine(Path.GetTempPath(), "astrolabe-tests", Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(path);
        return path;
    }

    private sealed class TestOpaqueNoMetadataCodec : IStructCodec<OpaqueBinaryRecord>
    {
        public static TestOpaqueNoMetadataCodec Instance { get; } = new();

        public string Kind => "testopaque_nometa";
        public string Schema => "astrolabe.test-opaque-no-metadata.v1";
        public int? FixedSize => null;
        public IReadOnlyList<PointerField> PointerFields { get; } = [];

        public OpaqueBinaryRecord Read(ReadOnlySpan<byte> data, int offset, int length) =>
            OpaqueBinaryRecord.FromSlice(Schema, data, offset, length);

        public byte[] Write(OpaqueBinaryRecord value) => value.Data;

        public OpaqueBinaryRecord FromJson(JsonElement json) =>
            JsonStructCodec.Deserialize<OpaqueBinaryRecord>(json, Schema);

        public void ToJson(OpaqueBinaryRecord value, Utf8JsonWriter writer) =>
            JsonStructCodec.Serialize(writer, value);
    }

    private sealed class TestStructExtraPointerRecord
    {
        public string Schema { get; set; } = "astrolabe.test-struct-extra-pointer.v1";
        public int Primary { get; set; }
        public byte[] Tail { get; set; } = [];
    }

    private sealed class TestStructWithExtraPointerCodec : IStructCodec<TestStructExtraPointerRecord>
    {
        public static TestStructWithExtraPointerCodec Instance { get; } = new();

        public string Kind => "teststruct_extra_ptr";
        public string Schema => "astrolabe.test-struct-extra-pointer.v1";
        public int? FixedSize => 8;
        public IReadOnlyList<PointerField> PointerFields { get; } =
        [
            new PointerField(0, "primary", PointerTarget.BlockRelative)
        ];

        public TestStructExtraPointerRecord Read(ReadOnlySpan<byte> data, int offset, int length)
        {
            var slice = data.Slice(offset, length);
            return new TestStructExtraPointerRecord
            {
                Primary = BinaryPrimitives.ReadInt32LittleEndian(slice),
                Tail = slice.Slice(4, 4).ToArray()
            };
        }

        public byte[] Write(TestStructExtraPointerRecord value)
        {
            var bytes = new byte[8];
            BinaryPrimitives.WriteInt32LittleEndian(bytes.AsSpan(0), value.Primary);
            if (value.Tail.Length == 4)
            {
                value.Tail.CopyTo(bytes.AsSpan(4));
            }

            return bytes;
        }

        public TestStructExtraPointerRecord FromJson(JsonElement json) =>
            JsonStructCodec.Deserialize<TestStructExtraPointerRecord>(json, Schema);

        public void ToJson(TestStructExtraPointerRecord value, Utf8JsonWriter writer) =>
            JsonStructCodec.Serialize(writer, value);
    }

    private sealed class TestOpaqueWithPointerMetadataCodec : IStructCodec<OpaqueBinaryRecord>
    {
        public static TestOpaqueWithPointerMetadataCodec Instance { get; } = new();

        public string Kind => "testopaque_meta";
        public string Schema => "astrolabe.test-opaque-with-metadata.v1";
        public int? FixedSize => null;
        public IReadOnlyList<PointerField> PointerFields { get; } =
        [
            new PointerField(4, "ptr_4", PointerTarget.BlockRelative, RequiresVmRange: true)
        ];

        public OpaqueBinaryRecord Read(ReadOnlySpan<byte> data, int offset, int length) =>
            OpaqueBinaryRecord.FromSlice(Schema, data, offset, length);

        public byte[] Write(OpaqueBinaryRecord value) => value.Data;

        public OpaqueBinaryRecord FromJson(JsonElement json) =>
            JsonStructCodec.Deserialize<OpaqueBinaryRecord>(json, Schema);

        public void ToJson(OpaqueBinaryRecord value, Utf8JsonWriter writer) =>
            JsonStructCodec.Serialize(writer, value);
    }
}