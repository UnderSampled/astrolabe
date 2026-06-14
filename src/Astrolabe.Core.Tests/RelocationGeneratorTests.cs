using System.Buffers.Binary;
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
            var overlayPath = Path.Combine(packageDir, "types/raw/source.reloc.json");
            RelocationPointerOverlay.Merge(
                overlayPath,
                new Dictionary<string, string?> { ["0x9"] = "types/raw/target.json" });

            Assert.Throws<InvalidDataException>(() =>
                OpenSpaceExporter.PreviewStructuredElementBytes(packageDir, "raw", "types/raw/source.json"));
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void PreviewStructuredElementBytes_NullUriOverlay_PreservesSerializedBytes()
    {
        var packageDir = CreateTempDir();
        try
        {
            CreateOpaquePackageFixture(packageDir, []);
            var overlayPath = Path.Combine(packageDir, "types/raw/source.reloc.json");
            RelocationPointerOverlay.Merge(
                overlayPath,
                new Dictionary<string, string?> { ["0x0"] = null },
                new Dictionary<string, RelocationOverlayTarget> { ["0x0"] = new(0x06, 0x01) });

            var bytes = OpenSpaceExporter.PreviewStructuredElementBytes(packageDir, "raw", "types/raw/source.json");

            Assert.Equal(0x1234_5678, BinaryPrimitives.ReadInt32LittleEndian(bytes.AsSpan(0, 4)));
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void PreviewStructuredElementBytes_OpaqueEmptyPointers_SidecarResolvesUri()
    {
        var packageDir = CreateTempDir();
        try
        {
            CreateOpaquePackageFixture(packageDir, []);
            RelocationPointerOverlay.Merge(
                Path.Combine(packageDir, "types/raw/source.reloc.json"),
                new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" });

            var bytes = OpenSpaceExporter.PreviewStructuredElementBytes(packageDir, "raw", "types/raw/source.json");

            Assert.Equal(0x0900_0000, BinaryPrimitives.ReadInt32LittleEndian(bytes.AsSpan(0, 4)));
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void GenerateImportedRtb_UsesTargetUriOverride()
    {
        var packageDir = CreateTempDir();
        try
        {
            CreateOpaquePackageFixture(
                packageDir,
                new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" },
                module: 0x04,
                id: 0x05);

            var jsonOptions = new JsonSerializerOptions
            {
                PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
                PropertyNameCaseInsensitive = true
            };
            var manifestPath = Path.Combine(packageDir, "manifest.json");
            var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(manifestPath),
                jsonOptions)!;
            manifest.Semantic = new SemanticManifest
            {
                RtbSitesPath = "semantic/rtb-sites.json"
            };
            WriteJson(manifestPath, manifest, jsonOptions);
            WriteJson(
                Path.Combine(packageDir, "semantic/rtb-sites.json"),
                new RtbSitesDocument
                {
                    PackageName = "test",
                    Sites =
                    [
                        new RtbSiteEntry
                        {
                            SourceModule = 0x04,
                            SourceId = 0x05,
                            OffsetInMemory = 0x0900_0004,
                            TargetModule = 0xFF,
                            TargetId = 0xFF,
                            TargetUri = "types/raw/target.json"
                        }
                    ]
                },
                jsonOptions);

            var table = OpenSpaceExporter.GenerateRtb(packageDir, "test.rtb", []);
            var block = Assert.Single(table.Blocks);
            var pointer = Assert.Single(block.Pointers);

            Assert.Equal(0x04, pointer.TargetModule);
            Assert.Equal(0x05, pointer.TargetId);
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void GeneratePointerFileTable_UsesFileNameSpecificSitesDocument()
    {
        var packageDir = CreateTempDir();
        try
        {
            var rtpSitesPath = Path.Combine(packageDir, "semantic", "level.rtp-sites.json");
            var rttSitesPath = Path.Combine(packageDir, "semantic", "level.rtt-sites.json");
            Directory.CreateDirectory(Path.Combine(packageDir, "semantic"));
            WriteJson(
                Path.Combine(packageDir, "manifest.json"),
                new RetePackageManifest
                {
                    PackageRole = "level",
                    LevelName = "test",
                    SourceDirectoryName = "test"
                },
                new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });
            WriteJson(rtpSitesPath, new PointerFileSitesDocument
            {
                FileName = "level.rtp",
                SourceModule = 0x05,
                SourceId = 0x01,
                Sites =
                [
                    new PointerFileSiteEntry
                    {
                        OffsetInMemory = 0x0100,
                        TargetModule = 0x06,
                        TargetId = 0x02
                    }
                ]
            }, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });
            WriteJson(rttSitesPath, new PointerFileSitesDocument
            {
                FileName = "level.rtt",
                SourceModule = 0x05,
                SourceId = 0x01,
                Sites =
                [
                    new PointerFileSiteEntry
                    {
                        OffsetInMemory = 0x0200,
                        TargetModule = 0x06,
                        TargetId = 0x03
                    },
                    new PointerFileSiteEntry
                    {
                        OffsetInMemory = 0x0300,
                        TargetModule = 0x06,
                        TargetId = 0x03
                    }
                ]
            }, new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase });

            var rtpTable = RelocationGenerator.GeneratePointerFileTable(
                packageDir,
                "level.rtp",
                Path.Combine(packageDir, "files", "level.gpt"),
                []);
            var rttTable = RelocationGenerator.GeneratePointerFileTable(
                packageDir,
                "level.rtt",
                Path.Combine(packageDir, "files", "level.ptx"),
                []);

            Assert.Single(rtpTable.Blocks);
            Assert.Single(rtpTable.Blocks[0].Pointers);
            Assert.Equal(0x0100u, rtpTable.Blocks[0].Pointers[0].OffsetInMemory);
            Assert.Equal(2, rttTable.Blocks[0].Pointers.Count);
            Assert.Equal(0x0200u, rttTable.Blocks[0].Pointers[0].OffsetInMemory);
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
    public void ExportLevel_PrunedPackage_EmitsMatchingRtbBytes()
    {
        var packageDir = CreateTempDir();
        try
        {
            CreatePrunedRtbPackageFixture(packageDir, out var expectedTable);
            var outputDir = Path.Combine(packageDir, "export");
            OpenSpaceExporter.ExportLevel(packageDir, outputDir);

            var exportedPath = Path.Combine(outputDir, "test.rtb");
            Assert.True(File.Exists(exportedPath));

            var reader = new RelocationTableReader(exportedPath);
            var exportedBlock = Assert.Single(reader.PointerBlocks);
            var exportedPointer = Assert.Single(exportedBlock.Pointers);
            var expectedBlock = Assert.Single(expectedTable.Blocks);
            var expectedPointer = Assert.Single(expectedBlock.Pointers);

            Assert.Equal(expectedBlock.Module, exportedBlock.Module);
            Assert.Equal(expectedBlock.Id, exportedBlock.Id);
            Assert.Equal(expectedPointer.OffsetInMemory, exportedPointer.OffsetInMemory);
            Assert.Equal(expectedPointer.TargetModule, exportedPointer.TargetModule);
            Assert.Equal(expectedPointer.TargetId, exportedPointer.TargetId);
            Assert.Equal(expectedPointer.Byte6, exportedPointer.Byte6);
            Assert.Equal(expectedPointer.Byte7, exportedPointer.Byte7);
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
                        new RelocationTableFileManifest
                        {
                            FileName = "test.rtv",
                            EncodingPath = "semantic/test.rtv-encoding.json"
                        }
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
            Assert.Contains("Pass-through", result.Note, StringComparison.Ordinal);
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact]
    public void PrunedPackage_ReferenceTableMatchesGeneratedRtb()
    {
        var packageDir = CreateTempDir();
        try
        {
            CreateOpaquePackageFixture(
                packageDir,
                new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" },
                module: 0x04,
                id: 0x05);
            var jsonOptions = new JsonSerializerOptions
            {
                PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
                PropertyNameCaseInsensitive = true
            };
            var manifestPath = Path.Combine(packageDir, "manifest.json");
            var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(manifestPath),
                jsonOptions)!;
            manifest.RelocationTables =
            [
                new RelocationTableFileManifest
                {
                    FileName = "test.rtb",
                    EncodingPath = "semantic/test.rtb-encoding.json"
                }
            ];
            manifest.Semantic = new SemanticManifest
            {
                RtbSitesPath = "semantic/rtb-sites.json"
            };
            WriteJson(
                Path.Combine(packageDir, "semantic/rtb-sites.json"),
                new RtbSitesDocument
                {
                    PackageName = "test",
                    Sites =
                    [
                        new RtbSiteEntry
                        {
                            SourceModule = 0x04,
                            SourceId = 0x05,
                            OffsetInMemory = 0x0900_0004,
                            TargetModule = 0x04,
                            TargetId = 0x05,
                            TargetUri = "types/raw/target.json"
                        }
                    ]
                },
                jsonOptions);
            WriteJson(
                Path.Combine(packageDir, "semantic/test.rtb-encoding.json"),
                new RelocationEncodingDocument
                {
                    FileName = "test.rtb",
                    Blocks =
                    [
                        new RelocationEncodingBlockManifest
                        {
                            Order = 0,
                            Key = "04:05",
                            Module = 0x04,
                            Id = 0x05,
                            EntrySize = 8,
                            PointerCount = 1
                        }
                    ]
                },
                jsonOptions);
            WriteJson(manifestPath, manifest, jsonOptions);

            var results = OpenSpaceExporter.CompareGeneratedRelocations(packageDir);
            var result = Assert.Single(results);

            Assert.Equal(1, result.MatchingPointerCount);
            Assert.Equal(0, result.MissingPointerCount);
            Assert.Equal(0, result.ExtraPointerCount);
        }
        finally
        {
            Directory.Delete(packageDir, true);
        }
    }

    [Fact(Skip = "Diagnostic only")]
    public void Diagnostic_AnalyzeAstrolabeRtbMissingPointers()
    {
        var packageDir = Path.GetFullPath(Path.Combine(
            AppContext.BaseDirectory,
            "..", "..", "..", "..", "..",
            "output", "test-rete", "astrolabe"));
        var fixDir = Path.Combine(Path.GetDirectoryName(packageDir)!, "fix");
        var sitesPath = Path.Combine(packageDir, "semantic", "rtb-sites.json");
        if (!File.Exists(sitesPath))
        {
            return;
        }

        var sites = JsonSerializer.Deserialize<RtbSitesDocument>(
            File.ReadAllText(sitesPath),
            new JsonSerializerOptions { PropertyNameCaseInsensitive = true })!;
        var preservedCount = sites.Sites.Count;
        var generated = OpenSpaceExporter.GenerateRtb(packageDir, "astrolabe.rtb", [fixDir]);
        var generatedSet = new HashSet<string>();
        foreach (var block in generated.Blocks)
        {
            foreach (var pointer in block.Pointers)
            {
                generatedSet.Add($"{block.Module:X2}:{block.Id:X2}:{pointer.OffsetInMemory:X8}");
            }
        }

        var layout = RelocationGenerator.PackageLayout.Load(packageDir);
        var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
            File.ReadAllText(Path.Combine(packageDir, OpenSpacePackageCodec.ManifestFileName)),
            new JsonSerializerOptions { PropertyNameCaseInsensitive = true })!;
        var elementKinds = new Dictionary<(byte Module, byte Id, uint Offset), string>();
        foreach (var snaFile in manifest.SnaFiles)
        {
            foreach (var block in snaFile.Blocks)
            {
                if (block.ContentPath == null)
                {
                    continue;
                }

                var content = JsonSerializer.Deserialize<SnaBlockContentDocument>(
                    File.ReadAllText(Path.Combine(packageDir, block.ContentPath.Replace('/', Path.DirectorySeparatorChar))),
                    new JsonSerializerOptions { PropertyNameCaseInsensitive = true })!;
                foreach (var element in content.Elements)
                {
                    elementKinds[(block.Module, block.Id, (uint)element.VirtualAddress)] = element.Kind;
                }
            }
        }

        var missing = 0;
        var zeroValue = 0;
        var nonzeroValue = 0;
        var ffTarget = 0;
        var kindCounts = new Dictionary<string, int>(StringComparer.OrdinalIgnoreCase);
        using var details = new StreamWriter("/tmp/astrolabe-rtb-missing-details.txt");
        foreach (var site in sites.Sites)
        {
            var key = $"{site.SourceModule:X2}:{site.SourceId:X2}:{site.OffsetInMemory:X8}";
            if (generatedSet.Contains(key))
            {
                continue;
            }

            missing++;
            if (site.TargetModule == 0xFF && site.TargetId == 0xFF)
            {
                ffTarget++;
            }

            var kind = "unknown";
            foreach (var ((module, id, start), elementKind) in elementKinds)
            {
                if (module == site.SourceModule &&
                    id == site.SourceId &&
                    site.OffsetInMemory >= start)
                {
                    kind = elementKind;
                }
            }

            kindCounts[kind] = kindCounts.GetValueOrDefault(kind) + 1;
            if (layout.TryReadInt32(checked((int)site.OffsetInMemory), out var value))
            {
                if (value == 0)
                {
                    zeroValue++;
                }
                else
                {
                    nonzeroValue++;
                    details.WriteLine(
                        $"nonzero missing {key} kind={kind} value=0x{value:X8} target={site.TargetModule:X2}:{site.TargetId:X2}");
                }
            }
        }

        var summary = $"missing={missing} zero={zeroValue} nonzero={nonzeroValue} ffTarget={ffTarget}";
        File.WriteAllText(
            "/tmp/astrolabe-rtb-missing.txt",
            summary + Environment.NewLine + string.Join(Environment.NewLine, kindCounts.OrderByDescending(p => p.Value).Select(p => $"{p.Key}:{p.Value}")));
        Assert.True(missing > 0, summary);
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

    private static void CreatePrunedRtbPackageFixture(string packageDir, out RelocationTableDocument expectedTable)
    {
        CreateOpaquePackageFixture(
            packageDir,
            new Dictionary<string, string?> { ["0x0"] = "types/raw/target.json" },
            module: 0x04,
            id: 0x05);
        var jsonOptions = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            PropertyNameCaseInsensitive = true
        };
        expectedTable = OpenSpaceExporter.GenerateRtb(packageDir, "test.rtb", []);
        var manifestPath = Path.Combine(packageDir, "manifest.json");
        var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
            File.ReadAllText(manifestPath),
            jsonOptions)!;
        manifest.RelocationTables =
        [
            new RelocationTableFileManifest
            {
                FileName = "test.rtb",
                EncodingPath = "semantic/test.rtb-encoding.json"
            }
        ];
        manifest.Semantic = new SemanticManifest
        {
            RtbSitesPath = "semantic/rtb-sites.json"
        };
        WriteJson(
            Path.Combine(packageDir, "semantic/rtb-sites.json"),
            new RtbSitesDocument
            {
                PackageName = "test",
                Sites =
                [
                    new RtbSiteEntry
                    {
                        SourceModule = 0x04,
                        SourceId = 0x05,
                        OffsetInMemory = 0x0900_0004,
                        TargetModule = 0x04,
                        TargetId = 0x05,
                        TargetUri = "types/raw/target.json"
                    }
                ]
            },
            jsonOptions);
        WriteJson(
            Path.Combine(packageDir, "semantic/test.rtb-encoding.json"),
            new RelocationEncodingDocument
            {
                FileName = "test.rtb",
                Blocks =
                [
                    new RelocationEncodingBlockManifest
                    {
                        Order = 0,
                        Key = "04:05",
                        Module = 0x04,
                        Id = 0x05,
                        EntrySize = 8,
                        PointerCount = 1,
                        PointerDataSha256 = expectedTable.Blocks[0].PointerDataSha256
                    }
                ]
            },
            jsonOptions);
        WriteJson(manifestPath, manifest, jsonOptions);
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

        var sourceJsonPath = Path.Combine(packageDir, "types/raw/source.json");
        codec.WriteJson(packageDir, sourceJsonPath, new OpaqueBinaryRecord
        {
            Schema = schema,
            Data = sourceData,
            Pointers = pointers
        });

        if (pointers.Count > 0)
        {
            RelocationPointerOverlay.Merge(
                RelocationPointerOverlay.GetOverlayPath(sourceJsonPath),
                pointers);
        }
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
