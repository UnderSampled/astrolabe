using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;
using Astrolabe.Core.FileFormats.Perso;
using Astrolabe.Core.FileFormats.Semantic;
using Astrolabe.Core.Hub;
using Astrolabe.Core.Rete;
using Astrolabe.Core.Serialization.Codecs;
using Xunit;

namespace Astrolabe.Core.Tests;

public sealed class SemanticTreeRoundTripTests
{

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };



    [Fact]
    public void SemanticDocuments_HaveExpectedPaths()
    {
        Assert.Equal("scene/tree.json", SceneTreeDocument.RelativePath);
        Assert.Equal("geometry/meshes.json", GeometryPoolDocument.RelativePath);
        Assert.Equal("ai/models.json", AiPoolDocument.RelativePath);
        Assert.Equal("characters/persos.json", CharacterPoolDocument.RelativePath);
        Assert.Equal("sectors/sectors.json", SectorPoolDocument.RelativePath);
        Assert.Equal("sidecars/level.json", SidecarDocument.RelativePath);
    }



    [Fact]
    public void AiDomainAggregator_CreatesPoolAndSexpr()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-ai-agg-" + Guid.NewGuid().ToString("N"));
        try
        {
            // Real ScriptNode stream: (if) end — keyword if + end marker
            // node0: param=0 indent=1 type=KeyWord
            // node1: param=0 indent=0 type=end
            var scriptWire = new byte[]
            {
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, // indent=1 type=0 KeyWord If
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08  // indent=0 end
            };

            Directory.CreateDirectory(Path.Combine(root, "types", "brain"));
            Directory.CreateDirectory(Path.Combine(root, "types", "mind"));
            Directory.CreateDirectory(Path.Combine(root, "types", "script"));

            File.WriteAllText(
                Path.Combine(root, "types", "brain", "b0.json"),
                """{"schema":"astrolabe.brain.v1","mind":"types/mind/m0.json","unknown04":0,"unknown08":0}""");
            File.WriteAllText(
                Path.Combine(root, "types", "mind", "m0.json"),
                """{"schema":"astrolabe.mind.v1","aiModel":null,"intelligenceNormal":null,"intelligenceReflex":null,"dsgMem":null,"unknown10":0,"unknown14":0}""");
            File.WriteAllBytes(Path.Combine(root, "types", "script", "s0.bin"), scriptWire);
            File.WriteAllText(
                Path.Combine(root, "types", "script", "s0.json"),
                """{"schema":"astrolabe.script.v1","path":"types/script/s0.bin","sha256":"","pointers":{}}""");

            var blockDir = Path.Combine(root, "sna", "lvl", "blocks", "0001_05_01");
            Directory.CreateDirectory(blockDir);
            var content = new SnaBlockContentDocument
            {
                Schema = SnaBlockContentDocument.SchemaValue,
                BlockKey = "05:01",
                Segments =
                [
                    new SnaBlockContentSegment
                    {
Kind = "brain",
                        DataPath = "types/brain/b0.json"
                    },
                    new SnaBlockContentSegment
                    {
Kind = "mind",
                        DataPath = "types/mind/m0.json"
                    },
                    new SnaBlockContentSegment
                    {
Kind = "script",
                        DataPath = "types/script/s0.json"
                    }
                ]
            };
            File.WriteAllText(
                Path.Combine(blockDir, "content.json"),
                JsonSerializer.Serialize(content));

            var manifest = new RetePackageManifest
            {
                PackageRole = "level",
                LevelName = "test",
                SnaFiles =
                [
                    new SnaFileManifest
                    {
                        FileName = "test.sna",
                        Blocks =
                        [
                            new SnaBlockManifest
                            {
                                Key = "05:01",
                                ContentPath = "sna/lvl/blocks/0001_05_01/content.json"
                            }
                        ]
                    }
                ]
            };
            File.WriteAllText(
                Path.Combine(root, "manifest.json"),
                JsonSerializer.Serialize(manifest));

            AiDomainAggregator.Aggregate(root, manifest);

            var modelsPath = Path.Combine(root, "ai", "models.json");
            Assert.True(File.Exists(modelsPath));

            var pool = JsonSerializer.Deserialize<AiPoolDocument>(
                File.ReadAllText(modelsPath), JsonOptions);
            Assert.NotNull(pool);
            Assert.Equal(3, pool!.ById.Count);

            // Contiguous AI leaves → expand run
            var rewritten = JsonSerializer.Deserialize<SnaBlockContentDocument>(
                File.ReadAllText(Path.Combine(blockDir, "content.json")), JsonOptions);
            Assert.NotNull(rewritten);
            Assert.Equal(SnaBlockContentDocument.SchemaValue, rewritten!.Schema);
            Assert.Single(rewritten.Segments);
            Assert.Equal(SnaBlockContentSegment.ExpandKind, rewritten.Segments[0].Kind);
            Assert.Contains("ai/models.json#/runs/", rewritten.Segments[0].DataPath);

            // Linearize expands back to three leaves
            var leaves = SnaBlockContentLinearizer.Linearize(root, rewritten);
            Assert.Equal(3, leaves.Count);
            Assert.All(leaves, leaf =>
                Assert.Contains("ai/models.json#/byId/", leaf.DataPath, StringComparison.OrdinalIgnoreCase));

            // Script got sexpr authoring source
            var scriptNode = pool.ById.Values.Single(n =>
                n.Kind.Equals("script", StringComparison.OrdinalIgnoreCase));
            Assert.False(string.IsNullOrEmpty(scriptNode.SexprPath));
            var sexprFull = Path.Combine(root, scriptNode.SexprPath!.Replace('/', Path.DirectorySeparatorChar));
            Assert.True(File.Exists(sexprFull));
            var sexprText = File.ReadAllText(sexprFull);
            Assert.Contains("if", sexprText, StringComparison.OrdinalIgnoreCase);

            // Wire preserved under ai/payloads (not only sexpr)
            Assert.NotNull(scriptNode.Record);
            Assert.True(scriptNode.Record!.Value.TryGetProperty("path", out var pathProp));
            var payloadRel = pathProp.GetString()!;
            Assert.StartsWith("ai/payloads/", payloadRel, StringComparison.OrdinalIgnoreCase);
            var payloadBytes = File.ReadAllBytes(
                Path.Combine(root, payloadRel.Replace('/', Path.DirectorySeparatorChar)));
            Assert.Equal(scriptWire, payloadBytes);

            // Authoring nest: brain → mind
            var brain = pool.ById.Values.Single(n =>
                n.Kind.Equals("brain", StringComparison.OrdinalIgnoreCase));
            var mind = pool.ById.Values.Single(n =>
                n.Kind.Equals("mind", StringComparison.OrdinalIgnoreCase));
            Assert.Contains(mind.Id, brain.Children);
            Assert.Contains(brain.Id, pool.Roots);

            // Legacy type files removed
            Assert.False(File.Exists(Path.Combine(root, "types", "brain", "b0.json")));
            Assert.False(File.Exists(Path.Combine(root, "types", "script", "s0.bin")));

            // Export wire matches original script bytes
            File.WriteAllText(
                Path.Combine(root, "manifest.json"),
                """{"schema":"astrolabe.rete.v1","packageRole":"level","levelName":"t","snaFiles":[],"looseFiles":[]}""");
            var resolver = new ReferenceAddressResolver(root);
            Assert.True(SemanticPoolExport.TryWriteElementBytes(
                root,
                SemanticPoolPaths.AiNodeUri(scriptNode.Id),
                resolver,
                out var exported));
            Assert.Equal(scriptWire, exported);
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void CharacterDomainAggregator_BuildsRootsAndChildren()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-char-agg-" + Guid.NewGuid().ToString("N"));
        try
        {
            // Micro-file soup: perso → perso3ddata → objectlist (paths rewritten into pool URIs).
            var persoDir = Path.Combine(root, "types", "perso");
            var p3dDir = Path.Combine(root, "types", "perso3ddata");
            var olDir = Path.Combine(root, "types", "objectlist");
            var stdDir = Path.Combine(root, "types", "standardgame");
            Directory.CreateDirectory(persoDir);
            Directory.CreateDirectory(p3dDir);
            Directory.CreateDirectory(olDir);
            Directory.CreateDirectory(stdDir);

            File.WriteAllText(
                Path.Combine(olDir, "ol.json"),
                JsonSerializer.Serialize(new
                {
                    schema = "astrolabe.object-list.v1",
                    next = (string?)null,
                    prev = (string?)null,
                    hdr = (string?)null,
                    entries = (string?)null,
                    numEntries = 0
                }, JsonOptions));
            File.WriteAllText(
                Path.Combine(p3dDir, "p3d.json"),
                JsonSerializer.Serialize(new
                {
                    schema = "astrolabe.perso-3d-data.v1",
                    stateInitial = (string?)null,
                    stateCurrent = (string?)null,
                    state2 = (string?)null,
                    objectList = "types/objectlist/ol.json",
                    objectListInitial = (string?)null,
                    family = (string?)null,
                    unknown18 = 0,
                    unknown1C = 0
                }, JsonOptions));
            File.WriteAllText(
                Path.Combine(stdDir, "std.json"),
                JsonSerializer.Serialize(new
                {
                    schema = "astrolabe.standard-game.v1",
                    objectType0 = 1,
                    objectType1 = 2,
                    objectType2 = 3,
                    superObject = (string?)null,
                    unknown10 = Convert.ToBase64String(new byte[0x20])
                }, JsonOptions));
            File.WriteAllText(
                Path.Combine(persoDir, "p.json"),
                JsonSerializer.Serialize(new
                {
                    schema = "astrolabe.perso.v1",
                    perso3dData = "types/perso3ddata/p3d.json",
                    stdGame = "types/standardgame/std.json",
                    dynam = (string?)null,
                    unknown0C = 0,
                    brain = (string?)null,
                    camera = (string?)null,
                    collSet = (string?)null,
                    msWay = (string?)null,
                    msLight = (string?)null,
                    unknown24 = 0,
                    sectInfo = (string?)null,
                    unknown2C = 0,
                    unknown30 = (string?)null,
                    unknown34 = 0,
                    unknown38 = 0,
                    unknown3C = 0
                }, JsonOptions));

            var blockDir = Path.Combine(root, "sna", "lvl", "blocks", "0001_05_01");
            Directory.CreateDirectory(blockDir);
            var content = new SnaBlockContentDocument
            {
                Schema = SnaBlockContentDocument.SchemaValue,
                BlockKey = "05:01",
                Segments =
                [
                    new SnaBlockContentSegment
                    {
Kind = "perso",
                        DataPath = "types/perso/p.json"
                    },
                    new SnaBlockContentSegment
                    {
Kind = "perso3ddata",
                        DataPath = "types/perso3ddata/p3d.json"
                    },
                    new SnaBlockContentSegment
                    {
Kind = "standardgame",
                        DataPath = "types/standardgame/std.json"
                    },
                    new SnaBlockContentSegment
                    {
Kind = "objectlist",
                        DataPath = "types/objectlist/ol.json"
                    },
                    new SnaBlockContentSegment
                    {
Kind = "raw",
                        DataPath = "types/raw/gap.bin"
                    }
                ]
            };
            File.WriteAllText(
                Path.Combine(blockDir, "content.json"),
                JsonSerializer.Serialize(content, JsonOptions));

            var manifest = new RetePackageManifest
            {
                PackageRole = "level",
                LevelName = "test",
                SnaFiles =
                [
                    new SnaFileManifest
                    {
                        FileName = "test.sna",
                        Blocks =
                        [
                            new SnaBlockManifest
                            {
                                Key = "05:01",
                                ContentPath = "sna/lvl/blocks/0001_05_01/content.json"
                            }
                        ]
                    }
                ]
            };
            File.WriteAllText(
                Path.Combine(root, "manifest.json"),
                JsonSerializer.Serialize(manifest, JsonOptions));

            CharacterDomainAggregator.Aggregate(root, manifest);

            var poolPath = Path.Combine(root, "characters", "persos.json");
            Assert.True(File.Exists(poolPath));
            Assert.False(File.Exists(Path.Combine(persoDir, "p.json")));

            var pool = JsonSerializer.Deserialize<CharacterPoolDocument>(
                File.ReadAllText(poolPath), JsonOptions);
            Assert.NotNull(pool);
            Assert.Equal(CharacterPoolDocument.SchemaValue, pool!.Schema);
            Assert.Equal(4, pool.ById.Count);

            // perso is the authoring root; dependents claimed via pointer URIs.
            Assert.Single(pool.Roots);
            var rootId = pool.Roots[0];
            Assert.Equal("perso", pool.ById[rootId].Kind);
            Assert.Contains(pool.ById[rootId].Children, id => pool.ById[id].Kind == "perso3ddata");
            Assert.Contains(pool.ById[rootId].Children, id => pool.ById[id].Kind == "standardgame");

            var p3dId = pool.ById[rootId].Children.First(id => pool.ById[id].Kind == "perso3ddata");
            Assert.Contains(pool.ById[p3dId].Children, id => pool.ById[id].Kind == "objectlist");

            var rewritten = JsonSerializer.Deserialize<SnaBlockContentDocument>(
                File.ReadAllText(Path.Combine(blockDir, "content.json")), JsonOptions);
            Assert.NotNull(rewritten);
            Assert.Equal(SnaBlockContentDocument.SchemaValue, rewritten!.Schema);
            Assert.Contains(rewritten.Segments, s =>
                s.Kind.Equals(SnaBlockContentSegment.ExpandKind, StringComparison.OrdinalIgnoreCase) ||
                s.DataPath.Contains("characters/persos.json", StringComparison.OrdinalIgnoreCase));
            Assert.Contains(rewritten.Segments, s =>
                s.Kind.Equals("raw", StringComparison.OrdinalIgnoreCase));

            // Linearize expand run preserves stream order of the four character leaves.
            var leaves = SnaBlockContentLinearizer.Linearize(root, rewritten);
            Assert.Equal(5, leaves.Count);
            Assert.Equal("perso", leaves[0].Kind);
            Assert.Equal("perso3ddata", leaves[1].Kind);
            Assert.Equal("standardgame", leaves[2].Kind);
            Assert.Equal("objectlist", leaves[3].Kind);
            Assert.Equal("raw", leaves[4].Kind);
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void CharacterDomainKinds_CoverPromotedNonAnimationSet()
    {
        var expected = new[]
        {
            "perso",
            "perso3ddata",
            "standardgame",
            "objectlist",
            "spawnableentry",
            "dynam",
            "persosectorinfo",
            "objecttypeentry",
            "objecttypename",
            "alwayssuperobjects"
        };
        foreach (var kind in expected)
        {
            Assert.Contains(kind, SemanticDomainKinds.Character);
        }

        // Animation kinds must stay out of this pool.
        Assert.DoesNotContain("state", SemanticDomainKinds.Character);
        Assert.DoesNotContain("animationmontreal", SemanticDomainKinds.Character);
        Assert.DoesNotContain("family", SemanticDomainKinds.Character);
        Assert.DoesNotContain("transform", SemanticDomainKinds.Character);
    }



    [Fact]
    public void CharacterPool_Export_WritesPersoCodecBytes()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-char-exp-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(Path.Combine(root, "characters"));

            // Null pointers only — export address layout is out of scope for this unit test.
            var record = new PersoRecord
            {
                Perso3dData = HubReference.Null,
                StdGame = HubReference.Null,
                Dynam = HubReference.Null,
                Unknown0C = 0x11,
                Brain = HubReference.Null,
                Camera = HubReference.Null,
                CollSet = HubReference.Null,
                MsWay = HubReference.Null,
                MsLight = HubReference.Null,
                Unknown24 = 0x22,
                SectInfo = HubReference.Null,
                Unknown2C = 0x33,
                Unknown30 = HubReference.Null,
                Unknown34 = 0x44,
                Unknown38 = 0x55,
                Unknown3C = 0x66
            };
            var expected = PersoCodec.Instance.Write(record);

            using var stream = new MemoryStream();
            using (var writer = new Utf8JsonWriter(stream))
            {
                PersoCodec.Instance.ToJson(record, writer);
            }

            using var jsonDoc = JsonDocument.Parse(stream.ToArray());

            var pool = new CharacterPoolDocument
            {
                Roots = ["character_00000"],
                ById =
                {
                    ["character_00000"] = new SemanticPoolNode
                    {
                        Id = "character_00000",
                        Kind = "perso",
                        Record = jsonDoc.RootElement.Clone()
                    }
                }
            };
            File.WriteAllText(
                Path.Combine(root, CharacterPoolDocument.RelativePath),
                JsonSerializer.Serialize(pool, JsonOptions));
            File.WriteAllText(
                Path.Combine(root, "manifest.json"),
                """{"schema":"astrolabe.rete.v1","packageRole":"level","levelName":"t","snaFiles":[],"looseFiles":[]}""");

            var resolver = new ReferenceAddressResolver(root);
            Assert.True(SemanticPoolExport.TryWriteElementBytes(
                root,
                SemanticPoolPaths.CharacterNodeUri("character_00000"),
                resolver,
                out var bytes));
            Assert.Equal(PersoCodec.Size, bytes.Length);
            Assert.Equal(expected, bytes);
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void DenseBuffer_Export_ReadsBinPayload()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-buf-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(Path.Combine(root, "geometry", "buffers"));
            var wire = new byte[] { 0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x40, 0x40 };
            File.WriteAllBytes(Path.Combine(root, "geometry", "buffers", "v.bin"), wire);

            var pool = new GeometryPoolDocument
            {
                ById =
                {
                    ["geometry_00000"] = new SemanticPoolNode
                    {
                        Id = "geometry_00000",
                        Kind = "vertices",
                        BufferPath = "geometry/buffers/v.bin",
                        Record = JsonSerializer.SerializeToElement(new
                        {
                            schema = "astrolabe.dense-buffer.v1",
                            type = "vertices",
                            path = "geometry/buffers/v.bin",
                            byteLength = wire.Length
                        })
                    }
                }
            };
            Directory.CreateDirectory(Path.Combine(root, "geometry"));
            File.WriteAllText(
                Path.Combine(root, GeometryPoolDocument.RelativePath),
                JsonSerializer.Serialize(pool));

            // Minimal manifest so address index load succeeds if resolver is constructed.
            File.WriteAllText(
                Path.Combine(root, "manifest.json"),
                """{"schema":"astrolabe.rete.v1","packageRole":"level","levelName":"t","snaFiles":[],"looseFiles":[]}""");
            var resolver = new ReferenceAddressResolver(root);
            Assert.True(SemanticPoolExport.TryWriteElementBytes(
                root,
                "geometry/meshes.json#/byId/geometry_00000",
                resolver,
                out var bytes));
            Assert.Equal(wire, bytes);
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void GeometryDomainAggregator_MaterializesCompanionBinAndExpandRuns()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-geoagg-" + Guid.NewGuid().ToString("N"));
        try
        {
            // One geometricobject + one vertices dense buffer with companion .bin
            // matching codec.Write (parity-preferred path).
            var wire = new byte[]
            {
                0x00, 0x00, 0x80, 0x3F, // 1.0f
                0x00, 0x00, 0x00, 0x40, // 2.0f
                0x00, 0x00, 0x40, 0x40  // 3.0f
            };

            Directory.CreateDirectory(Path.Combine(root, "types", "vertices"));
            Directory.CreateDirectory(Path.Combine(root, "types", "geometricobject"));

            File.WriteAllBytes(Path.Combine(root, "types", "vertices", "0000.bin"), wire);
            // Values JSON present but materializer must prefer companion .bin
            File.WriteAllText(
                Path.Combine(root, "types", "vertices", "0000.json"),
                """
                {
                  "schema": "astrolabe.float3-array.v1",
                  "type": "vertices",
                  "values": [[9.9, 9.9, 9.9]]
                }
                """);

            File.WriteAllText(
                Path.Combine(root, "types", "geometricobject", "0000.json"),
                """
                {
                  "schema": "astrolabe.geometric-object.v1",
                  "numVertices": 1,
                  "vertices": "types/vertices/0000.json",
                  "normals": null,
                  "materials": null,
                  "unknown0": 0,
                  "numElements": 0,
                  "elementTypes": null,
                  "elements": null,
                  "unknowns": [0, 0, 0, 0],
                  "sphereRadius": 1.0,
                  "sphereCenterRaw": [0, 0, 0]
                }
                """);

            var blockDir = Path.Combine(root, "sna", "lvl", "blocks", "0001_05_01");
            Directory.CreateDirectory(blockDir);
            var content = new SnaBlockContentDocument
            {
                Schema = SnaBlockContentDocument.SchemaValue,
                BlockKey = "05:01",
                Segments =
                [
                    new SnaBlockContentSegment
                    {
Kind = "geometricobject",
                        DataPath = "types/geometricobject/0000.json"
                    },
                    new SnaBlockContentSegment
                    {
Kind = "vertices",
                        DataPath = "types/vertices/0000.json"
                    }
                ]
            };
            File.WriteAllText(
                Path.Combine(blockDir, "content.json"),
                JsonSerializer.Serialize(content));

            var manifest = new RetePackageManifest
            {
                PackageRole = "level",
                LevelName = "test",
                SnaFiles =
                [
                    new SnaFileManifest
                    {
                        FileName = "test.sna",
                        Blocks =
                        [
                            new SnaBlockManifest
                            {
                                Key = "05:01",
                                ContentPath = "sna/lvl/blocks/0001_05_01/content.json"
                            }
                        ]
                    }
                ]
            };
            File.WriteAllText(
                Path.Combine(root, "manifest.json"),
                JsonSerializer.Serialize(manifest));

            GeometryDomainAggregator.Aggregate(root, manifest);

            Assert.True(File.Exists(Path.Combine(root, "geometry", "meshes.json")));
            Assert.False(File.Exists(Path.Combine(root, "types", "vertices", "0000.json")));
            Assert.False(File.Exists(Path.Combine(root, "types", "vertices", "0000.bin")));

            var pool = JsonSerializer.Deserialize<GeometryPoolDocument>(
                File.ReadAllText(Path.Combine(root, "geometry", "meshes.json")), JsonOptions);
            Assert.NotNull(pool);
            Assert.Equal(2, pool!.ById.Count);

            var verticesNode = pool.ById.Values.Single(n =>
                n.Kind.Equals("vertices", StringComparison.OrdinalIgnoreCase));
            Assert.False(string.IsNullOrEmpty(verticesNode.BufferPath));
            var binFull = Path.Combine(
                root,
                verticesNode.BufferPath!.Replace('/', Path.DirectorySeparatorChar));
            Assert.True(File.Exists(binFull));
            Assert.Equal(wire, File.ReadAllBytes(binFull));

            // Descriptor must not carry large values arrays.
            Assert.NotNull(verticesNode.Record);
            Assert.False(verticesNode.Record!.Value.TryGetProperty("values", out _));

            // content.json rewritten to pool URIs / expand
            var rewritten = JsonSerializer.Deserialize<SnaBlockContentDocument>(
                File.ReadAllText(Path.Combine(blockDir, "content.json")), JsonOptions);
            Assert.NotNull(rewritten);
            Assert.Equal(SnaBlockContentDocument.SchemaValue, rewritten!.Schema);
            Assert.NotEmpty(rewritten.Segments);
            Assert.All(rewritten.Segments, s =>
                Assert.Contains("geometry/meshes.json", s.DataPath, StringComparison.OrdinalIgnoreCase));

            // Linearize recovers stream order of both leaves.
            var leaves = SnaBlockContentLinearizer.Linearize(root, rewritten);
            Assert.Equal(2, leaves.Count);
            Assert.Equal("geometricobject", leaves[0].Kind);
            Assert.Equal("vertices", leaves[1].Kind);

            // Export reads .bin payload byte-identically (companion path, not float JSON).
            var resolver = new ReferenceAddressResolver(root);
            Assert.True(SemanticPoolExport.TryWriteElementBytes(
                root, leaves[1].DataPath, resolver, out var exported));
            Assert.Equal(wire, exported);

            // geometricobject children should include vertices when URI rewritten.
            var geoNode = pool.ById.Values.Single(n =>
                n.Kind.Equals("geometricobject", StringComparison.OrdinalIgnoreCase));
            Assert.Contains(verticesNode.Id, geoNode.Children);
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void GeometryDomainAggregator_MaterializesFromValuesJsonWhenNoCompanion()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-geoval-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(Path.Combine(root, "types", "loddistances"));
            // 1.0f, 2.0f
            var expected = new byte[]
            {
                0x00, 0x00, 0x80, 0x3F,
                0x00, 0x00, 0x00, 0x40
            };
            File.WriteAllText(
                Path.Combine(root, "types", "loddistances", "0000.json"),
                """
                {
                  "schema": "astrolabe.float-array.v1",
                  "type": "loddistances",
                  "values": [1.0, 2.0]
                }
                """);

            var blockDir = Path.Combine(root, "sna", "lvl", "blocks", "0001_05_02");
            Directory.CreateDirectory(blockDir);
            File.WriteAllText(
                Path.Combine(blockDir, "content.json"),
                JsonSerializer.Serialize(new SnaBlockContentDocument
                {
                    Schema = SnaBlockContentDocument.SchemaValue,
                    Segments =
                    [
                        new SnaBlockContentSegment
                        {
Kind = "loddistances",
                            DataPath = "types/loddistances/0000.json"
                        }
                    ]
                }));

            var manifest = new RetePackageManifest
            {
                PackageRole = "level",
                LevelName = "test",
                SnaFiles =
                [
                    new SnaFileManifest
                    {
                        FileName = "test.sna",
                        Blocks =
                        [
                            new SnaBlockManifest
                            {
                                Key = "05:02",
                                ContentPath = "sna/lvl/blocks/0001_05_02/content.json"
                            }
                        ]
                    }
                ]
            };

            GeometryDomainAggregator.Aggregate(root, manifest);

            var pool = JsonSerializer.Deserialize<GeometryPoolDocument>(
                File.ReadAllText(Path.Combine(root, "geometry", "meshes.json")), JsonOptions)!;
            var node = pool.ById.Values.Single();
            Assert.Equal("loddistances", node.Kind);
            Assert.False(string.IsNullOrEmpty(node.BufferPath));

            var resolver = new ReferenceAddressResolver(root);
            Assert.True(SemanticPoolExport.TryWriteElementBytes(
                root,
                SemanticPoolPaths.GeometryNodeUri(node.Id),
                resolver,
                out var bytes));
            Assert.Equal(expected, bytes);
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void GeometryKinds_IncludePromotedDenseAndStructTypes()
    {
        Assert.Contains("geometricobject", SemanticDomainKinds.Geometry);
        Assert.Contains("visualmaterial", SemanticDomainKinds.Geometry);
        Assert.Contains("gamematerial", SemanticDomainKinds.Geometry);
        Assert.Contains("elementtriangles", SemanticDomainKinds.Geometry);
        Assert.Contains("loddistances", SemanticDomainKinds.Geometry);
        Assert.Contains("loddataoffsets", SemanticDomainKinds.Geometry);

        Assert.True(SemanticDomainKinds.IsDenseBufferKind("vertices"));
        Assert.True(SemanticDomainKinds.IsDenseBufferKind("loddistances"));
        Assert.True(SemanticDomainKinds.IsDenseBufferKind("elementtypes"));
        // Pointer arrays stay JSON (URI rewrite), not dense .bin payloads.
        Assert.False(SemanticDomainKinds.IsDenseBufferKind("elementptrs"));
        Assert.False(SemanticDomainKinds.IsDenseBufferKind("loddataoffsets"));
    }



    [Fact]
    public void Linearizer_ExpandsAiRuns()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-ai-lin-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(Path.Combine(root, "ai"));
            var pool = new AiPoolDocument
            {
                Runs = { ["ai_run_a"] = ["ai_00000", "ai_00001"] },
                ById =
                {
                    ["ai_00000"] = new SemanticPoolNode
                    {
                        Id = "ai_00000",
                        Kind = "brain",
                        Record = JsonSerializer.SerializeToElement(new
                        {
                            schema = "astrolabe.brain.v1",
                            mind = "ai/models.json#/byId/ai_00001"
                        })
                    },
                    ["ai_00001"] = new SemanticPoolNode
                    {
                        Id = "ai_00001",
                        Kind = "mind",
                        Record = JsonSerializer.SerializeToElement(new
                        {
                            schema = "astrolabe.mind.v1",
                            aiModel = (string?)null
                        })
                    }
                }
            };
            File.WriteAllText(
                Path.Combine(root, AiPoolDocument.RelativePath),
                JsonSerializer.Serialize(pool));

            var document = new SnaBlockContentDocument
            {
                Schema = SnaBlockContentDocument.SchemaValue,
                Segments =
                [
                    new SnaBlockContentSegment
                    {
                        Kind = SnaBlockContentSegment.ExpandKind,
                        DataPath = SemanticPoolPaths.AiRunUri("ai_run_a")
                    }
                ]
            };

            var leaves = SnaBlockContentLinearizer.Linearize(root, document);
            Assert.Equal(2, leaves.Count);
            Assert.Equal("brain", leaves[0].Kind);
            Assert.Equal("ai/models.json#/byId/ai_00000", leaves[0].DataPath);
            Assert.Equal("mind", leaves[1].Kind);
            Assert.Equal("ai/models.json#/byId/ai_00001", leaves[1].DataPath);
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void Linearizer_ExpandsCharacterRuns()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-char-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(Path.Combine(root, "characters"));
            var pool = new CharacterPoolDocument
            {
                Roots = ["character_00000"],
                Runs = { ["char_run"] = ["character_00000", "character_00001", "character_00002"] },
                ById =
                {
                    ["character_00000"] = new SemanticPoolNode
                    {
                        Id = "character_00000",
                        Kind = "perso",
                        Children = ["character_00001", "character_00002"],
                        Record = JsonSerializer.SerializeToElement(new
                        {
                            schema = "astrolabe.perso.v1",
                            perso3dData = "characters/persos.json#/byId/character_00001",
                            stdGame = "characters/persos.json#/byId/character_00002"
                        })
                    },
                    ["character_00001"] = new SemanticPoolNode
                    {
                        Id = "character_00001",
                        Kind = "perso3ddata",
                        Record = JsonSerializer.SerializeToElement(new { schema = "astrolabe.perso-3d-data.v1" })
                    },
                    ["character_00002"] = new SemanticPoolNode
                    {
                        Id = "character_00002",
                        Kind = "standardgame",
                        Record = JsonSerializer.SerializeToElement(new { schema = "astrolabe.standard-game.v1" })
                    }
                }
            };
            File.WriteAllText(
                Path.Combine(root, CharacterPoolDocument.RelativePath),
                JsonSerializer.Serialize(pool, JsonOptions));

            var document = new SnaBlockContentDocument
            {
                Schema = SnaBlockContentDocument.SchemaValue,
                Segments =
                [
                    new SnaBlockContentSegment
                    {
                        Kind = SnaBlockContentSegment.ExpandKind,
                        DataPath = SemanticPoolPaths.CharacterRunUri("char_run")
                    }
                ]
            };

            var leaves = SnaBlockContentLinearizer.Linearize(root, document);
            Assert.Equal(3, leaves.Count);
            Assert.Equal("perso", leaves[0].Kind);
            Assert.Equal("characters/persos.json#/byId/character_00000", leaves[0].DataPath);
            Assert.Equal("perso3ddata", leaves[1].Kind);
            Assert.Equal("standardgame", leaves[2].Kind);
            // Ownership children must not reorder stream expand (runs are explicit).
            Assert.DoesNotContain(leaves, leaf => leaf.DataPath.Contains("/children", StringComparison.Ordinal));
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void Linearizer_ExpandsGeometryRuns()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-geo-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(Path.Combine(root, "geometry"));
            var pool = new GeometryPoolDocument
            {
                Runs = { ["run_a"] = ["geometry_00000", "geometry_00001"] },
                ById =
                {
                    ["geometry_00000"] = new SemanticPoolNode
                    {
                        Id = "geometry_00000",
                        Kind = "geometricobject",
                        Record = JsonSerializer.SerializeToElement(new { schema = "astrolabe.geometric-object.v1" })
                    },
                    ["geometry_00001"] = new SemanticPoolNode
                    {
                        Id = "geometry_00001",
                        Kind = "vertices",
                        BufferPath = "geometry/buffers/geometry_00001.bin"
                    }
                }
            };
            File.WriteAllText(
                Path.Combine(root, GeometryPoolDocument.RelativePath),
                JsonSerializer.Serialize(pool));

            var document = new SnaBlockContentDocument
            {
                Schema = SnaBlockContentDocument.SchemaValue,
                Segments =
                [
                    new SnaBlockContentSegment
                    {
                        Kind = SnaBlockContentSegment.ExpandKind,
                        DataPath = "geometry/meshes.json#/runs/run_a"
                    }
                ]
            };

            var leaves = SnaBlockContentLinearizer.Linearize(root, document);
            Assert.Equal(2, leaves.Count);
            Assert.Equal("geometricobject", leaves[0].Kind);
            Assert.Equal("geometry/meshes.json#/byId/geometry_00000", leaves[0].DataPath);
            Assert.Equal("vertices", leaves[1].Kind);
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void Linearizer_ExpandsSceneRun_WithMatrixFieldKeys()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-scene-mat-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(Path.Combine(root, "scene"));
            var matrixRecord = JsonSerializer.SerializeToElement(new
            {
                schema = "astrolabe.matrix.v1",
                type = 0u,
                translation = new[] { 0f, 0f, 0f },
                basisX = new[] { 1f, 0f, 0f },
                basisY = new[] { 0f, 1f, 0f },
                basisZ = new[] { 0f, 0f, 1f },
                extraBase64 = Convert.ToBase64String(new byte[MatrixCodec.Size - 0x34])
            });
            var tree = new SceneTreeDocument
            {
                Roots = { ["actual_world"] = "scene_00000" },
                Runs = { ["scene_run"] = ["scene_00000", "scene_00000/matrix"] },
                ById =
                {
                    ["scene_00000"] = new SemanticPoolNode
                    {
                        Id = "scene_00000",
                        Kind = "superObject",
                        Record = JsonSerializer.SerializeToElement(new
                        {
                            schema = "astrolabe.super-object.v1",
                            typeCode = 4u,
                            type = "World",
                            childrenCount = 0u,
                            drawFlags = 0u,
                            flags = 0u
                        }),
                        Matrix = matrixRecord
                    }
                }
            };
            File.WriteAllText(
                Path.Combine(root, SceneTreeDocument.RelativePath),
                JsonSerializer.Serialize(tree));
            File.WriteAllText(
                Path.Combine(root, "manifest.json"),
                """{"schema":"astrolabe.rete.v1","packageRole":"level","levelName":"t","snaFiles":[],"looseFiles":[]}""");

            var document = new SnaBlockContentDocument
            {
                Schema = SnaBlockContentDocument.SchemaValue,
                Segments =
                [
                    new SnaBlockContentSegment
                    {
                        Kind = SnaBlockContentSegment.ExpandKind,
                        DataPath = SemanticPoolPaths.SceneRunUri("scene_run")
                    }
                ]
            };

            var leaves = SnaBlockContentLinearizer.Linearize(root, document);
            Assert.Equal(2, leaves.Count);
            Assert.Equal("superObject", leaves[0].Kind, ignoreCase: true);
            Assert.Equal("scene/tree.json#/byId/scene_00000", leaves[0].DataPath);
            Assert.Equal("matrix", leaves[1].Kind, ignoreCase: true);
            Assert.Equal("scene/tree.json#/byId/scene_00000/matrix", leaves[1].DataPath);

            var resolver = new ReferenceAddressResolver(root);
            Assert.True(SemanticPoolExport.TryWriteElementBytes(
                root, leaves[0].DataPath, resolver, out var so));
            Assert.Equal(SuperObjectCodec.Size, so.Length);
            Assert.True(SemanticPoolExport.TryWriteElementBytes(
                root, leaves[1].DataPath, resolver, out var mx));
            Assert.Equal(MatrixCodec.Size, mx.Length);
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void Linearizer_ExpandsSceneRuns()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-scene-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(Path.Combine(root, "scene"));
            var tree = new SceneTreeDocument
            {
                Roots = { ["actual_world"] = "scene_00000" },
                Runs = { ["scene_run"] = ["scene_00000", "scene_00001"] },
                ById =
                {
                    ["scene_00000"] = new SemanticPoolNode
                    {
                        Id = "scene_00000",
                        Kind = "superObject",
                        Children = ["scene_00001"]
                    },
                    ["scene_00001"] = new SemanticPoolNode
                    {
                        Id = "scene_00001",
                        Kind = "superObject"
                    }
                }
            };
            File.WriteAllText(
                Path.Combine(root, SceneTreeDocument.RelativePath),
                JsonSerializer.Serialize(tree));

            var document = new SnaBlockContentDocument
            {
                Schema = SnaBlockContentDocument.SchemaValue,
                Segments =
                [
                    new SnaBlockContentSegment
                    {
                        Kind = SnaBlockContentSegment.ExpandKind,
                        DataPath = SemanticPoolPaths.SceneRunUri("scene_run")
                    }
                ]
            };

            var leaves = SnaBlockContentLinearizer.Linearize(root, document);
            Assert.Equal(2, leaves.Count);
            Assert.All(leaves, leaf => Assert.Equal("superObject", leaf.Kind));
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void Linearizer_ExpandsSectorRuns()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-sector-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(Path.Combine(root, "sectors"));
            var pool = new SectorPoolDocument
            {
                Runs = { ["sector_run"] = ["sector_00000", "sector_00001", "sector_00002"] },
                ById =
                {
                    ["sector_00000"] = new SemanticPoolNode
                    {
                        Id = "sector_00000",
                        Kind = "sector",
                        Children = ["sector_00001"],
                        Record = JsonSerializer.SerializeToElement(new
                        {
                            schema = "astrolabe.sector.v1",
                            collideObj = "sectors/sectors.json#/byId/sector_00001",
                            name = "sectors/sectors.json#/byId/sector_00002"
                        })
                    },
                    ["sector_00001"] = new SemanticPoolNode
                    {
                        Id = "sector_00001",
                        Kind = "sectorcollidegeo",
                        Record = JsonSerializer.SerializeToElement(new
                        {
                            schema = "astrolabe.sector-collide-geo.v1",
                            data = Convert.ToBase64String(new byte[0x30])
                        })
                    },
                    ["sector_00002"] = new SemanticPoolNode
                    {
                        Id = "sector_00002",
                        Kind = "sectorname",
                        Record = JsonSerializer.SerializeToElement(new
                        {
                            schema = "astrolabe.sectorname.v1",
                            data = Convert.ToBase64String("Sector_A\0"u8.ToArray())
                        })
                    }
                }
            };
            File.WriteAllText(
                Path.Combine(root, SectorPoolDocument.RelativePath),
                JsonSerializer.Serialize(pool));

            var document = new SnaBlockContentDocument
            {
                Schema = SnaBlockContentDocument.SchemaValue,
                Segments =
                [
                    new SnaBlockContentSegment
                    {
                        Kind = SnaBlockContentSegment.ExpandKind,
                        DataPath = SemanticPoolPaths.SectorRunUri("sector_run")
                    }
                ]
            };

            var leaves = SnaBlockContentLinearizer.Linearize(root, document);
            Assert.Equal(3, leaves.Count);
            Assert.Equal("sector", leaves[0].Kind);
            Assert.Equal("sectors/sectors.json#/byId/sector_00000", leaves[0].DataPath);
            Assert.Equal("sectorcollidegeo", leaves[1].Kind);
            Assert.Equal("sectorname", leaves[2].Kind);
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void ReferenceUri_ResolvesTextureAndSoundSchemes()
    {
        var workspace = Path.Combine(Path.GetTempPath(), "astrolabe-texuri-" + Guid.NewGuid().ToString("N"));
        var levelDir = Path.Combine(workspace, "astrolabe");
        try
        {
            Directory.CreateDirectory(levelDir);
            File.WriteAllText(
                Path.Combine(levelDir, OpenSpacePackageCodec.ManifestFileName),
                """
                {
                  "schema": "astrolabe.rete.v1",
                  "packageRole": "level",
                  "levelName": "astrolabe",
                  "looseFiles": []
                }
                """);

            var png = Path.Combine(workspace, "Gamedata", "Textures", "torch.png");
            Directory.CreateDirectory(Path.GetDirectoryName(png)!);
            File.WriteAllBytes(png, [1, 2, 3]);
            var wav = Path.Combine(workspace, "Gamedata", "World", "Sound", "Bnk_0", "0000_hit.wav");
            Directory.CreateDirectory(Path.GetDirectoryName(wav)!);
            File.WriteAllBytes(wav, [4, 5, 6]);

            Assert.True(ReferenceUri.TryResolve(
                levelDir,
                "texture:/Gamedata/Textures/torch.png",
                out var texPath,
                out _));
            Assert.Equal(Path.GetFullPath(png), texPath);

            Assert.True(ReferenceUri.TryResolve(
                levelDir,
                "sound:/Gamedata/World/Sound/Bnk_0/0000_hit.wav",
                out var soundPath,
                out _));
            Assert.Equal(Path.GetFullPath(wav), soundPath);

            var madeTex = ReferenceUri.MakeReference(levelDir, png);
            Assert.Equal("texture:/Gamedata/Textures/torch.png", madeTex);
            var madeSound = ReferenceUri.MakeReference(levelDir, wav);
            Assert.Equal("sound:/Gamedata/World/Sound/Bnk_0/0000_hit.wav", madeSound);
        }
        finally
        {
            if (Directory.Exists(workspace))
            {
                Directory.Delete(workspace, recursive: true);
            }
        }
    }



    [Fact]
    public void SceneTreeAggregator_CollapsesFolderForest()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-fold-" + Guid.NewGuid().ToString("N"));
        try
        {
            var nodeDir = Path.Combine(root, "scene", "actual_world", "World_AABBCCDD");
            Directory.CreateDirectory(nodeDir);
            var node = new
            {
                schema = "astrolabe.scene-node.v1",
                id = "World_AABBCCDD",
                typeCode = 4u,
                type = "World",
                children = Array.Empty<string>(),
                name = "World"
            };
            File.WriteAllText(
                Path.Combine(nodeDir, "node.json"),
                JsonSerializer.Serialize(node));
            File.WriteAllText(
                Path.Combine(nodeDir, "matrix.json"),
                JsonSerializer.Serialize(new
                {
                    schema = "astrolabe.matrix.v1",
                    type = 0u,
                    translation = new[] { 1f, 2f, 3f },
                    basisX = new[] { 1f, 0f, 0f },
                    basisY = new[] { 0f, 1f, 0f },
                    basisZ = new[] { 0f, 0f, 1f },
                    extraBase64 = ""
                }));

            // Minimal content.json referencing the scene node
            var blockDir = Path.Combine(root, "sna", "lvl", "blocks", "0001_05_01");
            Directory.CreateDirectory(blockDir);
            var content = new SnaBlockContentDocument
            {
                Schema = SnaBlockContentDocument.SchemaValue,
                BlockKey = "05:01",
                Segments =
                [
                    new SnaBlockContentSegment
                    {
Kind = "superObject",
                        DataPath = "scene/actual_world/World_AABBCCDD/node.json"
                    },
                    new SnaBlockContentSegment
                    {
Kind = "matrix",
                        DataPath = "scene/actual_world/World_AABBCCDD/matrix.json"
                    }
                ]
            };
            File.WriteAllText(
                Path.Combine(blockDir, "content.json"),
                JsonSerializer.Serialize(content));

            var manifest = new RetePackageManifest
            {
                PackageRole = "level",
                LevelName = "test",
                SnaFiles =
                [
                    new SnaFileManifest
                    {
                        FileName = "test.sna",
                        Blocks =
                        [
                            new SnaBlockManifest
                            {
                                Key = "05:01",
                                ContentPath = "sna/lvl/blocks/0001_05_01/content.json"
                            }
                        ]
                    }
                ]
            };
            File.WriteAllText(
                Path.Combine(root, "manifest.json"),
                JsonSerializer.Serialize(manifest));

            SceneTreeAggregator.Aggregate(root, manifest);

            Assert.True(File.Exists(Path.Combine(root, "scene", "tree.json")));
            Assert.False(Directory.Exists(Path.Combine(root, "scene", "actual_world")));

            var tree = JsonSerializer.Deserialize<SceneTreeDocument>(
                File.ReadAllText(Path.Combine(root, "scene", "tree.json")), JsonOptions);
            Assert.NotNull(tree);
            Assert.True(tree!.Roots.ContainsKey("actual_world"));
            Assert.False(string.IsNullOrEmpty(tree.Roots["actual_world"]));
            Assert.NotEmpty(tree.ById);

            var rewritten = JsonSerializer.Deserialize<SnaBlockContentDocument>(
                File.ReadAllText(Path.Combine(blockDir, "content.json")), JsonOptions);
            Assert.NotNull(rewritten);
            Assert.Equal(SnaBlockContentDocument.SchemaValue, rewritten!.Schema);
            Assert.NotEmpty(rewritten.Segments);
            Assert.All(rewritten.Segments, s =>
                Assert.Contains("scene/tree.json", s.DataPath, StringComparison.OrdinalIgnoreCase));
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void SceneTreeAggregator_NestedChildren_UriLinksAndExportRoundTrip()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-scene-nest-" + Guid.NewGuid().ToString("N"));
        try
        {
            var parentDir = Path.Combine(root, "scene", "actual_world", "World_10000001");
            var childDir = Path.Combine(parentDir, "Sector_10000002");
            Directory.CreateDirectory(childDir);

            var childRel = "scene/actual_world/World_10000001/Sector_10000002/node.json";
            File.WriteAllText(
                Path.Combine(childDir, "node.json"),
                JsonSerializer.Serialize(new
                {
                    schema = "astrolabe.super-object.v1",
                    typeCode = 2u,
                    type = "Sector",
                    children = Array.Empty<string>(),
                    name = "Sector",
                    offData = (int?)null,
                    childrenHead = (int?)null,
                    childrenTail = (int?)null,
                    childrenCount = 0u,
                    brotherNext = (int?)null,
                    brotherPrev = (int?)null,
                    parent = (int?)null,
                    matrix = (int?)null,
                    staticMatrix = (int?)null,
                    globalMatrix = (int?)null,
                    drawFlags = 0u,
                    flags = 0u,
                    boundingVolume = (int?)null
                }));

            var matrixJson = new
            {
                schema = "astrolabe.matrix.v1",
                type = 0u,
                translation = new[] { 10f, 20f, 30f },
                basisX = new[] { 1f, 0f, 0f },
                basisY = new[] { 0f, 1f, 0f },
                basisZ = new[] { 0f, 0f, 1f },
                extraBase64 = Convert.ToBase64String(new byte[MatrixCodec.Size - 0x34])
            };
            File.WriteAllText(
                Path.Combine(childDir, "matrix.json"),
                JsonSerializer.Serialize(matrixJson));

            File.WriteAllText(
                Path.Combine(parentDir, "node.json"),
                JsonSerializer.Serialize(new
                {
                    schema = "astrolabe.super-object.v1",
                    typeCode = 4u,
                    type = "World",
                    children = new[] { childRel },
                    name = "World",
                    offData = (int?)null,
                    childrenHead = (int?)null,
                    childrenTail = (int?)null,
                    childrenCount = 1u,
                    brotherNext = (int?)null,
                    brotherPrev = (int?)null,
                    parent = (int?)null,
                    matrix = (int?)null,
                    staticMatrix = (int?)null,
                    globalMatrix = (int?)null,
                    drawFlags = 0u,
                    flags = 0u,
                    boundingVolume = (int?)null
                }));

            var blockDir = Path.Combine(root, "sna", "lvl", "blocks", "0001_05_01");
            Directory.CreateDirectory(blockDir);
            var content = new SnaBlockContentDocument
            {
                Schema = SnaBlockContentDocument.SchemaValue,
                BlockKey = "05:01",
                Segments =
                [
                    new SnaBlockContentSegment
                    {
Kind = "superObject",
                        DataPath = "scene/actual_world/World_10000001/node.json"
                    },
                    new SnaBlockContentSegment
                    {
Kind = "superObject",
                        DataPath = childRel
                    },
                    new SnaBlockContentSegment
                    {
Kind = "matrix",
                        DataPath = "scene/actual_world/World_10000001/Sector_10000002/matrix.json"
                    }
                ]
            };
            File.WriteAllText(
                Path.Combine(blockDir, "content.json"),
                JsonSerializer.Serialize(content));

            var manifest = new RetePackageManifest
            {
                PackageRole = "level",
                LevelName = "test",
                SnaFiles =
                [
                    new SnaFileManifest
                    {
                        FileName = "test.sna",
                        Blocks =
                        [
                            new SnaBlockManifest
                            {
                                Key = "05:01",
                                ContentPath = "sna/lvl/blocks/0001_05_01/content.json"
                            }
                        ]
                    }
                ]
            };
            File.WriteAllText(Path.Combine(root, "manifest.json"), JsonSerializer.Serialize(manifest));

            SceneTreeAggregator.Aggregate(root, manifest);

            Assert.False(Directory.Exists(Path.Combine(root, "scene", "actual_world")));
            var tree = JsonSerializer.Deserialize<SceneTreeDocument>(
                File.ReadAllText(Path.Combine(root, "scene", "tree.json")), JsonOptions)!;
            Assert.Equal(2, tree.ById.Count);

            var parentId = tree.Roots["actual_world"]!;
            var parent = tree.ById[parentId];
            Assert.Single(parent.Children);
            var childId = parent.Children[0];
            Assert.True(tree.ById.ContainsKey(childId));

            // Record children are URI links into byId.
            Assert.True(parent.Record.HasValue);
            var parentRecord = parent.Record!.Value;
            Assert.True(parentRecord.TryGetProperty("children", out var kids));
            Assert.Equal(SemanticPoolPaths.SceneNodeUri(childId), kids[0].GetString());

            var rewritten = JsonSerializer.Deserialize<SnaBlockContentDocument>(
                File.ReadAllText(Path.Combine(blockDir, "content.json")), JsonOptions)!;
            var leaves = SnaBlockContentLinearizer.Linearize(root, rewritten);
            Assert.Equal(3, leaves.Count);
            Assert.Equal("superObject", leaves[0].Kind, ignoreCase: true);
            Assert.Equal("superObject", leaves[1].Kind, ignoreCase: true);
            Assert.Equal("matrix", leaves[2].Kind, ignoreCase: true);

            var resolver = new ReferenceAddressResolver(root);
            Assert.True(SemanticPoolExport.TryWriteElementBytes(
                root, leaves[0].DataPath, resolver, out var superBytes));
            Assert.Equal(SuperObjectCodec.Size, superBytes.Length);

            Assert.True(SemanticPoolExport.TryWriteElementBytes(
                root, leaves[2].DataPath, resolver, out var matrixBytes));
            Assert.Equal(MatrixCodec.Size, matrixBytes.Length);

            // Matrix translation floats at offset 0x04 match authoring values.
            Assert.Equal(10f, BitConverter.ToSingle(matrixBytes, 0x04));
            Assert.Equal(20f, BitConverter.ToSingle(matrixBytes, 0x08));
            Assert.Equal(30f, BitConverter.ToSingle(matrixBytes, 0x0C));
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void Script_LooksLikeNodeStream_RejectsPointerShell()
    {
        // 8-byte off_script-style shell (non-zero padding when misread as ScriptNode)
        var shell = new byte[] { 0x58, 0x0D, 0x00, 0x09, 0xA1, 0x06, 0x00, 0x00 };
        Assert.False(Script.LooksLikeNodeStream(shell));

        // Valid empty script: sole end marker
        var empty = new byte[] { 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 };
        Assert.True(Script.LooksLikeNodeStream(empty));

        // Valid one-statement script
        var one = new byte[]
        {
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08
        };
        Assert.True(Script.LooksLikeNodeStream(one));
        Assert.True(Script.TryRead(one, AITypes.Hype, out var script));
        Assert.Equal(2, script.Nodes.Count);
        var sexpr = new SExpressionConverter(AITypes.Hype).Convert(script);
        Assert.Contains("if", sexpr, StringComparison.OrdinalIgnoreCase);
    }



    [Fact]
    public void SectorDomainAggregator_BuildsPoolRunsAndNesting()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-sector-agg-" + Guid.NewGuid().ToString("N"));
        try
        {
            var blockDir = Path.Combine(root, "sna", "lvl", "blocks", "0001_05_01");
            Directory.CreateDirectory(Path.Combine(blockDir));
            Directory.CreateDirectory(Path.Combine(root, "types", "sector"));
            Directory.CreateDirectory(Path.Combine(root, "types", "collideset"));
            Directory.CreateDirectory(Path.Combine(root, "types", "collidezdxlist"));
            Directory.CreateDirectory(Path.Combine(root, "types", "collidezdxzone"));

            // Codec / opaque JSON with pointer path strings rewritten into pool URIs.
            File.WriteAllText(
                Path.Combine(root, "types", "sector", "b_0000.json"),
                """{"schema":"astrolabe.sector.v1","collideObj":null,"name":null}""");
            File.WriteAllText(
                Path.Combine(root, "types", "collideset", "b_0001.json"),
                """{"schema":"astrolabe.collide-set.v1","zdxList":"types/collidezdxlist/b_0002.json","zddList":null,"zdeList":null,"unknown0C":"AAAAAAAAAAA="}""");
            // Zone list head → zone (authoring pointer for optional nesting).
            File.WriteAllText(
                Path.Combine(root, "types", "collidezdxlist", "b_0002.json"),
                """{"schema":"astrolabe.collidezdxlist.v1","head":"types/collidezdxzone/b_0003.json","tail":null,"count":1}""");
            File.WriteAllText(
                Path.Combine(root, "types", "collidezdxzone", "b_0003.json"),
                """{"schema":"astrolabe.collidezdxzone.v1","data":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="}""");

            var content = new SnaBlockContentDocument
            {
                Schema = SnaBlockContentDocument.SchemaValue,
                BlockKey = "05:01",
                Segments =
                [
                    new SnaBlockContentSegment
                    {
Kind = "sector",
                        DataPath = "types/sector/b_0000.json"
                    },
                    new SnaBlockContentSegment
                    {
Kind = "collideset",
                        DataPath = "types/collideset/b_0001.json"
                    },
                    new SnaBlockContentSegment
                    {
Kind = "collidezdxlist",
                        DataPath = "types/collidezdxlist/b_0002.json"
                    },
                    new SnaBlockContentSegment
                    {
Kind = "collidezdxzone",
                        DataPath = "types/collidezdxzone/b_0003.json"
                    }
                ]
            };
            File.WriteAllText(
                Path.Combine(blockDir, "content.json"),
                JsonSerializer.Serialize(content));

            var manifest = new RetePackageManifest
            {
                PackageRole = "level",
                LevelName = "test",
                SnaFiles =
                [
                    new SnaFileManifest
                    {
                        FileName = "test.sna",
                        Blocks =
                        [
                            new SnaBlockManifest
                            {
                                Key = "05:01",
                                ContentPath = "sna/lvl/blocks/0001_05_01/content.json"
                            }
                        ]
                    }
                ]
            };
            File.WriteAllText(Path.Combine(root, "manifest.json"), JsonSerializer.Serialize(manifest));

            SectorDomainAggregator.Aggregate(root, manifest);

            Assert.True(File.Exists(Path.Combine(root, "sectors", "sectors.json")));
            Assert.False(File.Exists(Path.Combine(root, "types", "sector", "b_0000.json")));

            var pool = JsonSerializer.Deserialize<SectorPoolDocument>(
                File.ReadAllText(Path.Combine(root, "sectors", "sectors.json")), JsonOptions);
            Assert.NotNull(pool);
            Assert.Equal(4, pool!.ById.Count);
            Assert.NotEmpty(pool.Runs);

            // collideset → zdx list; zdx list → zone via rewritten pointer URIs
            var collideset = pool.ById.Values.First(n => n.Kind.Equals("collideset", StringComparison.OrdinalIgnoreCase));
            Assert.Contains(collideset.Children, id =>
                pool.ById[id].Kind.Equals("collidezdxlist", StringComparison.OrdinalIgnoreCase));
            var zdxList = pool.ById.Values.First(n => n.Kind.Equals("collidezdxlist", StringComparison.OrdinalIgnoreCase));
            Assert.Contains(zdxList.Children, id =>
                pool.ById[id].Kind.Equals("collidezdxzone", StringComparison.OrdinalIgnoreCase));

            var rewritten = JsonSerializer.Deserialize<SnaBlockContentDocument>(
                File.ReadAllText(Path.Combine(blockDir, "content.json")), JsonOptions);
            Assert.NotNull(rewritten);
            Assert.Equal(SnaBlockContentDocument.SchemaValue, rewritten!.Schema);
            Assert.NotEmpty(rewritten.Segments);
            Assert.Contains(rewritten.Segments, s =>
                s.DataPath.Contains("sectors/sectors.json", StringComparison.OrdinalIgnoreCase));

            // Linearize the rewritten expand run(s)
            var leaves = SnaBlockContentLinearizer.Linearize(root, rewritten);
            Assert.Equal(4, leaves.Count);
            Assert.Equal("sector", leaves[0].Kind);
            Assert.Equal("collideset", leaves[1].Kind);
            Assert.Equal("collidezdxlist", leaves[2].Kind);
            Assert.Equal("collidezdxzone", leaves[3].Kind);
        }
        finally
        {
            if (Directory.Exists(root))
            {
                Directory.Delete(root, recursive: true);
            }
        }
    }



    [Fact]
    public void SectorDomainKinds_MatchPromotedCodecs()
    {
        // collidez* must include zdx/zdd/zde list + zone for both registry and pool.
        Assert.Contains("collidezdxzone", SemanticDomainKinds.Sector);
        Assert.Contains("collidezddzone", SemanticDomainKinds.Sector);
        Assert.Contains("collidezdezone", SemanticDomainKinds.Sector);
        Assert.Contains("collidezdxlist", SemanticDomainKinds.Sector);
        Assert.Contains("collidezddlist", SemanticDomainKinds.Sector);
        Assert.Contains("collidezdelist", SemanticDomainKinds.Sector);
        Assert.Contains("sector", SemanticDomainKinds.Sector);
        Assert.Contains("collideset", SemanticDomainKinds.Sector);
        Assert.Contains("collideelementptrs", SemanticDomainKinds.Sector);
        Assert.Contains("sectorcollidegeo", SemanticDomainKinds.Sector);
        Assert.Contains("sectorcollideverts", SemanticDomainKinds.Sector);
        Assert.Contains("sectorname", SemanticDomainKinds.Sector);

        // Residual opaques stay out of the pool kind set.
        Assert.DoesNotContain("collideobject", SemanticDomainKinds.Sector);
        Assert.DoesNotContain("sectorcollideelemtypes", SemanticDomainKinds.Sector);
        Assert.DoesNotContain("sectorcollideelemptrs", SemanticDomainKinds.Sector);
        Assert.DoesNotContain("collidetriangles", SemanticDomainKinds.Sector);
    }



    [Fact]
    public void SidecarAggregator_RoundTripsWireBytes()
    {
        var root = Path.Combine(Path.GetTempPath(), "astrolabe-side-" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(Path.Combine(root, "files"));
            var gpt = new byte[] { 0x01, 0x02, 0x03, 0x04, 0x10, 0x20, 0x30, 0x40 };
            File.WriteAllBytes(Path.Combine(root, "files", "astrolabe.gpt"), gpt);
            File.WriteAllBytes(Path.Combine(root, "files", "astrolabe.ptx"), new byte[] { 0xAA, 0xBB, 0xCC, 0xDD });

            var manifest = new RetePackageManifest
            {
                PackageRole = "level",
                LevelName = "astrolabe",
                LooseFiles =
                [
                    new LooseFileManifest
                    {
                        FileName = "astrolabe.gpt",
                        Path = "files/astrolabe.gpt",
                        Size = gpt.Length,

                    },
                    new LooseFileManifest
                    {
                        FileName = "astrolabe.ptx",
                        Path = "files/astrolabe.ptx",
                        Size = 4,

                    }
                ]
            };

            SidecarAggregator.Aggregate(root, manifest);

            Assert.True(File.Exists(Path.Combine(root, "sidecars", "level.json")));
            Assert.False(File.Exists(Path.Combine(root, "files", "astrolabe.gpt")));
            Assert.True(SidecarAggregator.TryWriteLooseFile(root, "astrolabe.gpt", out var roundTrip));
            Assert.Equal(gpt, roundTrip);
            Assert.Empty(manifest.LooseFiles.Where(f =>
                f.FileName.EndsWith(".gpt", StringComparison.OrdinalIgnoreCase) ||
                f.FileName.EndsWith(".ptx", StringComparison.OrdinalIgnoreCase)));
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
