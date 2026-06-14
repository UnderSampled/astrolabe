using System.Buffers.Binary;
using System.Text.Json;
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
    public void PreviewStructuredElementBytes_OpaqueSidecar_ResolvesPointerOverlay()
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
    public void PreviewStructuredElementBytes_OpaqueSidecar_NullPointerWritesZero()
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
    public void PreviewStructuredElementBytes_OpaqueSidecar_InvalidOffsetThrows()
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
    public void GenerateFixLevelRtb_UsesLevelSchemeReferences()
    {
        var workspaceDir = CreateTempDir();
        try
        {
            var levelDir = Path.Combine(workspaceDir, "astrolabe");
            var fixDir = Path.Combine(workspaceDir, "fix");

            CreateOpaquePackageFixture(
                levelDir,
                [],
                module: 0x04,
                id: 0x05);
            CreateOpaquePackageFixture(
                fixDir,
                new Dictionary<string, string?> { ["0x0"] = "level:/types/raw/target.json" },
                sourceData: [0x00, 0x00, 0x00, 0x09, 0, 0, 0, 0],
                packageRole: "fix",
                levelName: "Fix",
                module: 0x02,
                id: 0x03);
            var jsonOptions = new JsonSerializerOptions
            {
                PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
                PropertyNameCaseInsensitive = true
            };
            var levelManifestPath = Path.Combine(levelDir, "manifest.json");
            var levelManifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(levelManifestPath),
                jsonOptions)!;
            levelManifest.Semantic = new SemanticManifest
            {
                FixLevelSitesPath = "semantic/fix-level-sites.json"
            };
            WriteJson(levelManifestPath, levelManifest, jsonOptions);
            WriteJson(
                Path.Combine(levelDir, "semantic/fix-level-sites.json"),
                new FixLevelSitesDocument
                {
                    LevelName = "test",
                    Blocks =
                    [
                        new FixLevelSiteBlock
                        {
                            Order = 0,
                            SourceModule = 0x02,
                            SourceId = 0x03
                        }
                    ],
                    Sites =
                    [
                        new FixLevelSiteEntry
                        {
                            SourceModule = 0x02,
                            SourceId = 0x03,
                            OffsetInMemory = 0x0900_0004,
                            TargetModule = 0x04,
                            TargetId = 0x05,
                            TargetUri = "level:/types/raw/target.json"
                        }
                    ]
                },
                jsonOptions);

            var table = RelocationGenerator.GenerateFixLevelRtb(
                fixDir,
                levelDir,
                "fixlvl.rtb");
            var block = Assert.Single(table.Blocks);
            var pointer = Assert.Single(block.Pointers);

            Assert.Equal((uint)0x0900_0004, pointer.OffsetInMemory);
            Assert.Equal(0x04, pointer.TargetModule);
            Assert.Equal(0x05, pointer.TargetId);
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
        byte id = 0x01)
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
                            BaseInMemory = 0x0900_0000,
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
            BaseInMemory = 0x0900_0000,
            BaseInMemoryHex = "0x09000000",
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
                    VirtualAddress = 0x0900_0000,
                    VirtualAddressHex = "0x09000000",
                    Sha256 = "target"
                },
                new SnaBlockContentElement
                {
                    Order = 1,
                    Kind = kind,
                    DataPath = "types/raw/source.json",
                    OffsetInBlock = sourceOffset,
                    Length = sourceData.Length,
                    VirtualAddress = 0x0900_0000 + sourceOffset,
                    VirtualAddressHex = $"0x{0x0900_0000 + sourceOffset:X8}",
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
}
