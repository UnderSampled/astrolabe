using System.Buffers.Binary;
using System.Text.Json;
using Astrolabe.Core.Rete;
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

    private static void CreateOpaquePackageFixture(string packageDir, Dictionary<string, string?> pointers)
    {
        var jsonOptions = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase
        };

        Directory.CreateDirectory(packageDir);
        Assert.True(StructCodecRegistry.TryGet("raw", out var codec));

        var manifest = new RetePackageManifest
        {
            LevelName = "test",
            SourceDirectoryName = "test",
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
                            Key = "00:01",
                            Module = 0x00,
                            Id = 0x01,
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
            BlockKey = "00:01",
            Module = 0x00,
            Id = 0x01,
            BaseInMemory = 0x0900_0000,
            BaseInMemoryHex = "0x09000000",
            OriginalDataSha256 = "test",
            Elements =
            [
                new SnaBlockContentElement
                {
                    Order = 0,
                    Kind = "raw",
                    DataPath = "types/raw/target.json",
                    OffsetInBlock = 0,
                    Length = 4,
                    VirtualAddress = 0x0900_0000,
                    VirtualAddressHex = "0x09000000",
                    Sha256 = "target"
                },
                new SnaBlockContentElement
                {
                    Order = 1,
                    Kind = "raw",
                    DataPath = "types/raw/source.json",
                    OffsetInBlock = 4,
                    Length = 8,
                    VirtualAddress = 0x0900_0004,
                    VirtualAddressHex = "0x09000004",
                    Sha256 = "source"
                }
            ]
        };

        WriteJson(Path.Combine(packageDir, "manifest.json"), manifest, jsonOptions);
        WriteJson(Path.Combine(packageDir, "sna/test/blocks/0000/content.json"), content, jsonOptions);

        codec.WriteJson(packageDir, Path.Combine(packageDir, "types/raw/target.json"), new OpaqueBinaryRecord
        {
            Schema = RawBlobCodec.Instance.Schema,
            Data = [0xAA, 0xBB, 0xCC, 0xDD]
        });

        codec.WriteJson(packageDir, Path.Combine(packageDir, "types/raw/source.json"), new OpaqueBinaryRecord
        {
            Schema = RawBlobCodec.Instance.Schema,
            Data = [0x78, 0x56, 0x34, 0x12, 0, 0, 0, 0],
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
}
