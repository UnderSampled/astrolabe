using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;
using Astrolabe.Core.FileFormats.Semantic;
using Astrolabe.Core.Serialization;

namespace Astrolabe.Core.Rete;

/// <summary>
/// Dual-layer aggregation for non-animation domains: pull typed leaves into pool docs,
/// rewrite content.json to ordered segments + expand runs, delete type-folder soup.
/// </summary>
internal static class SemanticDomainAggregator
{
    private static readonly JsonSerializerOptions CompactJson = new()
    {
        WriteIndented = false,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    private static readonly JsonSerializerOptions IndentedJson = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    public static void AggregateAll(string packageDir, RetePackageManifest manifest)
    {
        if (!manifest.PackageRole.Equals("level", StringComparison.OrdinalIgnoreCase) &&
            !manifest.PackageRole.Equals("fix", StringComparison.OrdinalIgnoreCase))
        {
            return;
        }

        // Domain order matters only for path rewrites; each domain owns its doc + kinds.
        // See notes/semantic-dual-layer-framework.md.
        SceneTreeAggregator.Aggregate(packageDir, manifest);
        GeometryDomainAggregator.Aggregate(packageDir, manifest);
        AiDomainAggregator.Aggregate(packageDir, manifest);
        CharacterDomainAggregator.Aggregate(packageDir, manifest);
        SectorDomainAggregator.Aggregate(packageDir, manifest);
        SidecarAggregator.Aggregate(packageDir, manifest);
    }

    /// <summary>
    /// Shared dual-layer pool builder used by domain aggregators.
    /// Domain files call this with their kind set and document path.
    /// </summary>
    internal static void AggregateDomain(
        string packageDir,
        RetePackageManifest manifest,
        string domain,
        string relativeDocPath,
        string schema,
        HashSet<string> kinds,
        bool denseBuffers = false,
        bool sexprScripts = false) =>
        AggregateDomainCore(
            packageDir, manifest, domain, relativeDocPath, schema, kinds, denseBuffers, sexprScripts);

    private static void AggregateDomainCore(
        string packageDir,
        RetePackageManifest manifest,
        string domain,
        string relativeDocPath,
        string schema,
        HashSet<string> kinds,
        bool denseBuffers,
        bool sexprScripts)
    {
        var blocks = LoadBlocksMentioning(packageDir, manifest, kinds);
        if (blocks.Count == 0)
        {
            return;
        }

        var byId = new Dictionary<string, SemanticPoolNode>(StringComparer.OrdinalIgnoreCase);
        var runs = new Dictionary<string, List<string>>(StringComparer.OrdinalIgnoreCase);
        var pathToUri = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        var elementToId = new Dictionary<SnaBlockContentElement, string>();
        var counter = 0;

        foreach (var block in blocks)
        {
            foreach (var element in block.OrderedElements)
            {
                if (!kinds.Contains(element.Kind))
                {
                    continue;
                }

                // Already a dual-layer pool leaf from a prior domain pass / expand.
                if (element.DataPath.Contains('#', StringComparison.Ordinal))
                {
                    continue;
                }

                if (elementToId.ContainsKey(element))
                {
                    continue;
                }

                var id = $"{domain}_{counter:D5}";
                counter++;
                elementToId[element] = id;

                var node = BuildNode(packageDir, element, id, domain, denseBuffers, sexprScripts);
                byId[id] = node;
                var uri = DomainUri(domain, id);
                pathToUri[NormalizePath(element.DataPath)] = uri;
            }
        }

        if (byId.Count == 0)
        {
            return;
        }

        var runCounter = 0;
        foreach (var block in blocks)
        {
            RewriteBlock(packageDir, block, kinds, elementToId, domain, runs, ref runCounter);
        }

        WritePoolDocument(packageDir, relativeDocPath, schema, domain, byId, runs);
        RewritePackageWideUris(packageDir, pathToUri);
        DeleteLegacyTypeFiles(packageDir, kinds, pathToUri);
    }

    private static SemanticPoolNode BuildNode(
        string packageDir,
        SnaBlockContentElement element,
        string id,
        string domain,
        bool denseBuffers,
        bool sexprScripts)
    {
        var node = new SemanticPoolNode
        {
            Id = id,
            Kind = element.Kind,
            ProvenanceVirtualAddress = element.VirtualAddress != 0 ? element.VirtualAddress : null
        };

        // Never open fragment URIs as filesystem paths (pool leaves already aggregated).
        if (element.DataPath.Contains('#', StringComparison.Ordinal))
        {
            return node;
        }

        var fullPath = ResolvePath(packageDir, element.DataPath);
        if (!File.Exists(fullPath))
        {
            return node;
        }

        if (denseBuffers && SemanticDomainKinds.IsDenseBufferKind(element.Kind))
        {
            TryMaterializeDenseBuffer(packageDir, element, node);
            return node;
        }

        if (fullPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
        {
            using var doc = JsonDocument.Parse(File.ReadAllText(fullPath));
            var record = doc.RootElement.Clone();

            // Relocate opaque .bin payloads out of types/ so legacy trees can be deleted.
            var payloadDir = domain switch
            {
                "ai" => AiPoolDocument.PayloadDir,
                "character" => "characters/payloads",
                "sector" => "sectors/payloads",
                "geometry" => GeometryPoolDocument.BufferDir,
                _ => $"{domain}/payloads"
            };
            record = RelocateOpaquePayload(packageDir, id, record, payloadDir);

            node.Record = record;

            if (sexprScripts &&
                element.Kind.Equals("script", StringComparison.OrdinalIgnoreCase))
            {
                TryWriteSexpr(packageDir, id, node);
            }
        }
        else
        {
            // Binary leaf — store as opaque base64 record for lossless export.
            var bytes = File.ReadAllBytes(fullPath);
            var json = JsonSerializer.SerializeToElement(new
            {
                schema = $"astrolabe.{element.Kind}.v1",
                data = Convert.ToBase64String(bytes)
            });
            node.Record = json;
        }

        return node;
    }

    /// <summary>
    /// If the record is an opaque descriptor with <c>path</c> to a .bin under types/,
    /// copy the bin under <paramref name="payloadDir"/>/{id}.bin and rewrite path.
    /// </summary>
    private static JsonElement RelocateOpaquePayload(
        string packageDir,
        string id,
        JsonElement record,
        string payloadDir)
    {
        if (record.ValueKind != JsonValueKind.Object ||
            !record.TryGetProperty("path", out var pathProp) ||
            pathProp.ValueKind != JsonValueKind.String)
        {
            return record;
        }

        var rel = pathProp.GetString();
        if (string.IsNullOrWhiteSpace(rel) ||
            !rel.EndsWith(".bin", StringComparison.OrdinalIgnoreCase))
        {
            return record;
        }

        var source = ResolvePath(packageDir, rel);
        if (!File.Exists(source))
        {
            return record;
        }

        var destRel = $"{payloadDir}/{id}.bin";
        var dest = ResolvePath(packageDir, destRel);
        Directory.CreateDirectory(Path.GetDirectoryName(dest)!);
        if (!string.Equals(Path.GetFullPath(source), Path.GetFullPath(dest), StringComparison.OrdinalIgnoreCase))
        {
            File.Copy(source, dest, overwrite: true);
        }

        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream))
        {
            writer.WriteStartObject();
            foreach (var prop in record.EnumerateObject())
            {
                writer.WritePropertyName(prop.Name);
                if (prop.Name.Equals("path", StringComparison.OrdinalIgnoreCase))
                {
                    writer.WriteStringValue(destRel);
                }
                else
                {
                    prop.Value.WriteTo(writer);
                }
            }

            writer.WriteEndObject();
        }

        using var doc = JsonDocument.Parse(stream.ToArray());
        return doc.RootElement.Clone();
    }

    private static void TryMaterializeDenseBuffer(
        string packageDir,
        SnaBlockContentElement element,
        SemanticPoolNode node)
    {
        var fullPath = ResolvePath(packageDir, element.DataPath);
        byte[] wire;
        JsonElement? metaRecord = null;
        string? companionToDelete = null;

        if (fullPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
        {
            using var doc = JsonDocument.Parse(File.ReadAllText(fullPath));
            metaRecord = StripValuesArray(doc.RootElement);

            // Parity-preferred sources (in order):
            // 1) companion .bin written at import next to the values JSON (codec.Write of original)
            // 2) existing path/buffer already pointing at a .bin
            // 3) re-encode via codec.WriteFromJsonPath (float JSON residual risk)
            var companionBin = Path.ChangeExtension(fullPath, ".bin");
            if (File.Exists(companionBin))
            {
                wire = File.ReadAllBytes(companionBin);
                companionToDelete = companionBin;
            }
            else if (TryReadExistingBufferPath(packageDir, doc.RootElement, out var existingWire))
            {
                wire = existingWire;
            }
            else if (StructCodecRegistry.TryGet(element.Kind, out var codec))
            {
                wire = codec.WriteFromJsonPath(packageDir, fullPath);
            }
            else
            {
                // Last resort: keep JSON text bytes (should not happen for promoted dense kinds).
                wire = File.ReadAllBytes(fullPath);
            }
        }
        else
        {
            wire = File.ReadAllBytes(fullPath);
        }

        var bufferDir = Path.Combine(packageDir, GeometryPoolDocument.BufferDir);
        Directory.CreateDirectory(bufferDir);
        var bufferName = $"{node.Id}.bin";
        var bufferRel = $"{GeometryPoolDocument.BufferDir}/{bufferName}";
        var bufferFull = Path.Combine(bufferDir, bufferName);
        File.WriteAllBytes(bufferFull, wire);

        if (companionToDelete != null &&
            !string.Equals(companionToDelete, bufferFull, StringComparison.OrdinalIgnoreCase))
        {
            try { File.Delete(companionToDelete); } catch { /* ignore */ }
        }

        node.BufferPath = bufferRel;
        node.Kind = element.Kind;

        var stride = ElementStride(element.Kind);
        var descriptor = new Dictionary<string, object?>
        {
            ["schema"] = "astrolabe.dense-buffer.v1",
            ["type"] = element.Kind,
            ["path"] = bufferRel,
            ["stride"] = stride,
            ["count"] = stride > 0 ? wire.Length / stride : wire.Length,
            ["byteLength"] = wire.Length,
            ["sha256"] = Convert.ToHexString(SHA256.HashData(wire)).ToLowerInvariant()
        };
        if (metaRecord is { } meta && meta.ValueKind == JsonValueKind.Object)
        {
            foreach (var prop in meta.EnumerateObject())
            {
                if (prop.NameEquals("values") ||
                    prop.NameEquals("path") ||
                    prop.NameEquals("data") ||
                    prop.NameEquals("count") ||
                    prop.NameEquals("byteLength") ||
                    prop.NameEquals("sha256") ||
                    prop.NameEquals("stride") ||
                    prop.NameEquals("schema") ||
                    prop.NameEquals("type"))
                {
                    continue;
                }

                descriptor[prop.Name] = prop.Value.Clone();
            }
        }

        node.Record = JsonSerializer.SerializeToElement(descriptor);
    }

    private static int EstimateCount(string kind, int byteLength)
    {
        var stride = kind.ToLowerInvariant() switch
        {
            "vertices" or "normals" or "trianglenormals" => 12,
            "uvs" => 8,
            "vertexindices" or "triangles" or "uvmapping" or "elementtypes" => 2,
            _ => 1
        };
        return stride > 0 ? byteLength / stride : byteLength;
    }

    private static JsonElement StripValuesArray(JsonElement root)
    {
        if (root.ValueKind != JsonValueKind.Object)
        {
            return root.Clone();
        }

        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream))
        {
            writer.WriteStartObject();
            foreach (var prop in root.EnumerateObject())
            {
                if (prop.NameEquals("values"))
                {
                    continue;
                }

                prop.WriteTo(writer);
            }

            writer.WriteEndObject();
        }

        using var doc = JsonDocument.Parse(stream.ToArray());
        return doc.RootElement.Clone();
    }

    private static bool TryReadExistingBufferPath(
        string packageDir,
        JsonElement root,
        out byte[] wire)
    {
        wire = [];
        if (root.ValueKind != JsonValueKind.Object)
        {
            return false;
        }

        if (!root.TryGetProperty("path", out var pathProp) ||
            pathProp.ValueKind != JsonValueKind.String)
        {
            return false;
        }

        var rel = pathProp.GetString();
        if (string.IsNullOrWhiteSpace(rel))
        {
            return false;
        }

        var full = ResolvePath(packageDir, rel);
        if (!File.Exists(full))
        {
            return false;
        }

        wire = File.ReadAllBytes(full);
        return true;
    }

    private static int ElementStride(string kind) =>
        kind.ToLowerInvariant() switch
        {
            "vertices" or "normals" or "trianglenormals" => 12,
            "uvs" => 8,
            "loddistances" => 4,
            "vertexindices" or "triangles" or "uvmapping" or "elementtypes" => 2,
            _ => 1
        };

    private static void TryWriteSexpr(string packageDir, string id, SemanticPoolNode node)
    {
        if (node.Record is not { } record)
        {
            return;
        }

        try
        {
            byte[]? scriptBytes = LoadScriptWireBytes(packageDir, record);
            if (scriptBytes == null || scriptBytes.Length == 0)
            {
                return;
            }

            var sexprDir = Path.Combine(packageDir, AiPoolDocument.SexprDir);
            Directory.CreateDirectory(sexprDir);
            var rel = $"{AiPoolDocument.SexprDir}/{id}.sexpr";
            var full = Path.Combine(packageDir, rel.Replace('/', Path.DirectorySeparatorChar));
            var sha = Convert.ToHexString(SHA256.HashData(scriptBytes)).ToLowerInvariant();

            // Wire stays in Record (opaque path/data). S-expr is optional authoring only.
            if (!Script.LooksLikeNodeStream(scriptBytes))
            {
                // off_script pointer shells or non-aligned blobs: keep a stub, not a fake AST.
                var stub =
                    $"; script header / non-node-stream blob ({scriptBytes.Length} bytes)\n" +
                    $"; sha256={sha}\n" +
                    $"; wire payload remains in ai/models.json record (lossless)\n";
                if (record.ValueKind == JsonValueKind.Object &&
                    record.TryGetProperty("pointers", out var ptrs) &&
                    ptrs.ValueKind == JsonValueKind.Object &&
                    ptrs.TryGetProperty("0x0", out var off0) &&
                    off0.ValueKind == JsonValueKind.String)
                {
                    stub += $"; offScript → {off0.GetString()}\n";
                }

                File.WriteAllText(full, stub);
                node.SexprPath = rel;
                return;
            }

            try
            {
                if (!Script.TryRead(scriptBytes, AITypes.Hype, out var script))
                {
                    File.WriteAllText(full,
                        $"; unparsed script blob ({scriptBytes.Length} bytes)\n; sha256={sha}\n");
                    node.SexprPath = rel;
                    return;
                }

                var converter = new SExpressionConverter(AITypes.Hype);
                var text = converter.Convert(script);
                // Prefix with provenance comment; body is the AST.
                var header =
                    $"; script id={id} nodes={script.Nodes.Count} bytes={scriptBytes.Length}\n" +
                    $"; sha256={sha}\n" +
                    $"; wire payload remains in ai/models.json record (lossless)\n";
                File.WriteAllText(full, header + text + "\n");
                node.SexprPath = rel;
            }
            catch
            {
                File.WriteAllText(full,
                    $"; unparsed script blob ({scriptBytes.Length} bytes)\n; sha256={sha}\n");
                node.SexprPath = rel;
            }
        }
        catch
        {
            // S-expr is authoring aid; wire payload remains in Record.
        }
    }

    private static byte[]? LoadScriptWireBytes(string packageDir, JsonElement record)
    {
        if (record.ValueKind != JsonValueKind.Object)
        {
            return null;
        }

        if (record.TryGetProperty("data", out var dataProp) &&
            dataProp.ValueKind == JsonValueKind.String)
        {
            var s = dataProp.GetString();
            if (!string.IsNullOrEmpty(s))
            {
                return Convert.FromBase64String(s);
            }
        }

        if (record.TryGetProperty("path", out var pathProp) &&
            pathProp.ValueKind == JsonValueKind.String)
        {
            var bin = ResolvePath(packageDir, pathProp.GetString()!);
            if (File.Exists(bin))
            {
                return File.ReadAllBytes(bin);
            }
        }

        return null;
    }

    private static void RewriteBlock(
        string packageDir,
        BlockContext block,
        HashSet<string> kinds,
        Dictionary<SnaBlockContentElement, string> elementToId,
        string domain,
        Dictionary<string, List<string>> runs,
        ref int runCounter)
    {
        // Path → pool id for this domain (from element scan).
        var pathToId = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (var (element, id) in elementToId)
        {
            pathToId[NormalizePath(element.DataPath)] = id;
        }

        List<SnaBlockContentSegment> segments;
        // Preserve non-domain expand segments (e.g. animation runs); only rewrite
        // domain leaf spans into pool URIs / expand runs.
        segments = RewriteV2Segments(
            block.Document.Segments,
            kinds,
            pathToId,
            domain,
            runs,
            ref runCounter,
            block.BlockKey);

        block.Document.Schema = SnaBlockContentDocument.SchemaValue;
        block.Document.Segments = segments;
        WriteJson(ResolvePath(packageDir, block.ContentPath), block.Document, compact: true);
    }

    private static List<SnaBlockContentSegment> RewriteV2Segments(
        List<SnaBlockContentSegment> input,
        HashSet<string> kinds,
        Dictionary<string, string> pathToId,
        string domain,
        Dictionary<string, List<string>> runs,
        ref int runCounter,
        string blockKey)
    {
        var output = new List<SnaBlockContentSegment>();
        var i = 0;
        while (i < input.Count)
        {
            var seg = input[i];

            // Preserve expands and groups from other domains (animation, prior domains).
            if (seg.Kind.Equals(SnaBlockContentSegment.ExpandKind, StringComparison.OrdinalIgnoreCase) ||
                seg.Children is { Count: > 0 })
            {
                output.Add(CloneSegment(seg));
                i++;
                continue;
            }

            if (!kinds.Contains(seg.Kind))
            {
                output.Add(CloneSegment(seg));
                i++;
                continue;
            }

            // Contiguous domain leaf span → expand run.
            var runIds = new List<string>();
            string lastKind = seg.Kind;
            while (i < input.Count)
            {
                var cur = input[i];
                if (cur.Kind.Equals(SnaBlockContentSegment.ExpandKind, StringComparison.OrdinalIgnoreCase) ||
                    cur.Children is { Count: > 0 } ||
                    !kinds.Contains(cur.Kind))
                {
                    break;
                }

                lastKind = cur.Kind;
                var path = NormalizePath(cur.DataPath.Split('#')[0] == cur.DataPath
                    ? cur.DataPath
                    : cur.DataPath); // keep full path for types/…; fragment URIs already pool
                if (pathToId.TryGetValue(NormalizePath(cur.DataPath), out var id) ||
                    pathToId.TryGetValue(NormalizePath(cur.DataPath.Split('#')[0]), out id))
                {
                    runIds.Add(id);
                }
                else if (cur.DataPath.Contains($"/{domain}/", StringComparison.OrdinalIgnoreCase) ||
                         cur.DataPath.Contains("#/byId/", StringComparison.OrdinalIgnoreCase))
                {
                    // Already a pool URI from a prior partial rewrite — keep leaf.
                    if (runIds.Count > 0)
                    {
                        AppendDomainRun(output, runIds, lastKind, domain, runs, ref runCounter, blockKey);
                        runIds = [];
                    }

                    output.Add(CloneSegment(cur));
                    i++;
                    continue;
                }

                i++;
            }

            AppendDomainRun(output, runIds, lastKind, domain, runs, ref runCounter, blockKey);
        }

        return output;
    }

    private static void AppendDomainRun(
        List<SnaBlockContentSegment> segments,
        List<string> runIds,
        string lastKind,
        string domain,
        Dictionary<string, List<string>> runs,
        ref int runCounter,
        string blockKey)
    {
        if (runIds.Count == 0)
        {
            return;
        }

        if (runIds.Count == 1)
        {
            segments.Add(new SnaBlockContentSegment
            {
                Kind = lastKind,
                DataPath = DomainUri(domain, runIds[0])
            });
            return;
        }

        var runId = $"{domain}_run_{blockKey.Replace(':', '_')}_{runCounter:D3}";
        runCounter++;
        runs[runId] = runIds;
        segments.Add(new SnaBlockContentSegment
        {
            Kind = SnaBlockContentSegment.ExpandKind,
            DataPath = DomainRunUri(domain, runId)
        });
    }

    private static SnaBlockContentSegment CloneSegment(SnaBlockContentSegment seg) =>
        new()
        {
            Kind = seg.Kind,
            DataPath = seg.DataPath,
            Children = seg.Children,
            ProvenanceVirtualAddress = seg.ProvenanceVirtualAddress,
            Length = seg.Length,
            Labels = seg.Labels
        };

    private static void WritePoolDocument(
        string packageDir,
        string relativePath,
        string schema,
        string domain,
        Dictionary<string, SemanticPoolNode> byId,
        Dictionary<string, List<string>> runs)
    {
        var full = ResolvePath(packageDir, relativePath);
        Directory.CreateDirectory(Path.GetDirectoryName(full)!);

        object doc = domain switch
        {
            "geometry" => new GeometryPoolDocument { Schema = schema, ById = byId, Runs = runs },
            "ai" => new AiPoolDocument { Schema = schema, ById = byId, Runs = runs },
            "character" => new CharacterPoolDocument { Schema = schema, ById = byId, Runs = runs },
            "sector" => new SectorPoolDocument { Schema = schema, ById = byId, Runs = runs },
            _ => new SemanticPoolDocument
            {
                Schema = schema,
                Domain = domain,
                ById = byId,
                Runs = runs
            }
        };

        File.WriteAllText(full, JsonSerializer.Serialize(doc, CompactJson));
    }

    private static string DomainUri(string domain, string id) => domain switch
    {
        "geometry" => SemanticPoolPaths.GeometryNodeUri(id),
        "ai" => SemanticPoolPaths.AiNodeUri(id),
        "character" => SemanticPoolPaths.CharacterNodeUri(id),
        "sector" => SemanticPoolPaths.SectorNodeUri(id),
        "scene" => SemanticPoolPaths.SceneNodeUri(id),
        _ => throw new ArgumentOutOfRangeException(nameof(domain))
    };

    private static string DomainRunUri(string domain, string runId) => domain switch
    {
        "geometry" => SemanticPoolPaths.GeometryRunUri(runId),
        "ai" => SemanticPoolPaths.AiRunUri(runId),
        "character" => SemanticPoolPaths.CharacterRunUri(runId),
        "sector" => SemanticPoolPaths.SectorRunUri(runId),
        "scene" => SemanticPoolPaths.SceneRunUri(runId),
        _ => throw new ArgumentOutOfRangeException(nameof(domain))
    };

    private static List<BlockContext> LoadBlocksMentioning(
        string packageDir,
        RetePackageManifest manifest,
        HashSet<string> kinds)
    {
        var blocks = new List<BlockContext>();
        foreach (var snaFile in manifest.SnaFiles)
        {
            foreach (var block in snaFile.Blocks)
            {
                if (block.ContentPath == null)
                {
                    continue;
                }

                var contentPath = ResolvePath(packageDir, block.ContentPath);
                if (!File.Exists(contentPath) || !ContentMentionsAnyKind(contentPath, kinds))
                {
                    continue;
                }

                blocks.Add(LoadBlockContext(packageDir, block));
            }
        }

        return blocks;
    }

    private static bool ContentMentionsAnyKind(string contentPath, HashSet<string> kinds)
    {
        var bytes = File.ReadAllBytes(contentPath);
        foreach (var kind in kinds)
        {
            if (ContainsAscii(bytes, $"\"{kind}\"" ) || ContainsAscii(bytes, kind))
            {
                // Prefer quoted kind tokens; also accept unquoted for path segments.
                if (ContainsAscii(bytes, $"\"kind\":\"{kind}\"") ||
                    ContainsAscii(bytes, $"\"kind\": \"{kind}\"") ||
                    ContainsAscii(bytes, $"/types/{kind}/") ||
                    ContainsAscii(bytes, $"\\types\\{kind}\\"))
                {
                    return true;
                }
            }
        }

        // Fallback: deserialize and scan (small blocks only).
        try
        {
            var doc = JsonSerializer.Deserialize<SnaBlockContentDocument>(
                File.ReadAllText(contentPath), CompactJson);
            if (doc == null)
            {
                return false;
            }

            if (doc.Segments.Any(s => kinds.Contains(s.Kind)))
            {
                return true;
            }
        }
        catch
        {
            // ignore
        }

        // Broad scan
        foreach (var kind in kinds)
        {
            if (ContainsAscii(bytes, kind))
            {
                return true;
            }
        }

        return false;
    }

    private static bool ContainsAscii(byte[] haystack, string needle)
    {
        if (needle.Length == 0 || haystack.Length < needle.Length)
        {
            return false;
        }

        var n0 = (byte)needle[0];
        for (var i = 0; i <= haystack.Length - needle.Length; i++)
        {
            if (haystack[i] != n0)
            {
                continue;
            }

            var match = true;
            for (var j = 1; j < needle.Length; j++)
            {
                if (haystack[i + j] != (byte)needle[j])
                {
                    match = false;
                    break;
                }
            }

            if (match)
            {
                return true;
            }
        }

        return false;
    }

    private static BlockContext LoadBlockContext(string packageDir, SnaBlockManifest block)
    {
        var contentPath = ResolvePath(packageDir, block.ContentPath!);
        var document = JsonSerializer.Deserialize<SnaBlockContentDocument>(
                           File.ReadAllText(contentPath), CompactJson)
                       ?? throw new InvalidDataException($"Could not read {contentPath}");

        var ordered = SnaBlockContentLinearizer.Linearize(packageDir, document)
            .Select((leaf, index) => new SnaBlockContentElement
            {
                Order = index,
                Kind = leaf.Kind,
                DataPath = leaf.DataPath
            })
            .ToList();

        return new BlockContext
        {
            BlockKey = block.Key,
            ContentPath = block.ContentPath!,
            Document = document,
            OrderedElements = ordered
        };
    }

    private static void RewritePackageWideUris(
        string packageDir,
        Dictionary<string, string> pathToUri)
    {
        if (pathToUri.Count == 0)
        {
            return;
        }

        // Longest-first so nested path prefixes don't partial-replace.
        var replacements = pathToUri
            .OrderByDescending(p => p.Key.Length)
            .ToArray();

        foreach (var file in Directory.EnumerateFiles(packageDir, "*.json", SearchOption.AllDirectories))
        {
            var rel = NormalizePath(Path.GetRelativePath(packageDir, file));
            // Skip content (already rewritten), binary-ish sidecars, and payloads.
            // MUST rewrite types/** so cross-domain pointer fields remap before we delete
            // this domain's leaves (e.g. perso → brain before brain types/ is removed).
            if (rel.EndsWith("/content.json", StringComparison.OrdinalIgnoreCase) ||
                rel.Equals("content.json", StringComparison.OrdinalIgnoreCase) ||
                rel.StartsWith("geometry/buffers/", StringComparison.OrdinalIgnoreCase) ||
                rel.StartsWith("ai/payloads/", StringComparison.OrdinalIgnoreCase) ||
                rel.StartsWith("ai/scripts/", StringComparison.OrdinalIgnoreCase) ||
                (rel.StartsWith("sna/", StringComparison.OrdinalIgnoreCase) &&
                 rel.Contains("/elements/", StringComparison.OrdinalIgnoreCase)))
            {
                continue;
            }

            var text = File.ReadAllText(file);
            // Cheap gate: only files that still mention types/ or scene/ need path rewrites.
            if (text.IndexOf("types/", StringComparison.OrdinalIgnoreCase) < 0 &&
                text.IndexOf("scene/", StringComparison.OrdinalIgnoreCase) < 0)
            {
                continue;
            }

            var changed = false;
            foreach (var (oldPath, newUri) in replacements)
            {
                if (text.IndexOf(oldPath, StringComparison.OrdinalIgnoreCase) >= 0)
                {
                    text = ReplaceIgnoreCase(text, oldPath, newUri);
                    changed = true;
                }
            }

            if (text.Contains("#/byId/", StringComparison.OrdinalIgnoreCase) &&
                text.Contains("#byteOffset=", StringComparison.OrdinalIgnoreCase))
            {
                var fixedText = System.Text.RegularExpressions.Regex.Replace(
                    text,
                    @"(#/byId/[^""#]+)#byteOffset=",
                    "$1;byteOffset=",
                    System.Text.RegularExpressions.RegexOptions.IgnoreCase);
                if (fixedText != text)
                {
                    text = fixedText;
                    changed = true;
                }
            }

            if (changed)
            {
                File.WriteAllText(file, text);
            }
        }
    }

    private static string ReplaceIgnoreCase(string input, string oldValue, string newValue)
    {
        var sb = new StringBuilder(input.Length);
        var i = 0;
        while (i < input.Length)
        {
            var idx = input.IndexOf(oldValue, i, StringComparison.OrdinalIgnoreCase);
            if (idx < 0)
            {
                sb.Append(input, i, input.Length - i);
                break;
            }

            sb.Append(input, i, idx - i);
            sb.Append(newValue);
            i = idx + oldValue.Length;
        }

        return sb.ToString();
    }

    private static void DeleteLegacyTypeFiles(
        string packageDir,
        HashSet<string> kinds,
        Dictionary<string, string> pathToUri)
    {
        foreach (var oldPath in pathToUri.Keys)
        {
            var full = ResolvePath(packageDir, oldPath);
            if (File.Exists(full))
            {
                try { File.Delete(full); } catch { /* ignore */ }
            }

            // Sibling opaque .bin next to the type JSON (relocated for AI/opaque leaves).
            if (oldPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
            {
                var binRel = Path.ChangeExtension(oldPath, ".bin");
                var binFull = ResolvePath(packageDir, binRel);
                if (File.Exists(binFull))
                {
                    try { File.Delete(binFull); } catch { /* ignore */ }
                }
            }
        }

        foreach (var kind in kinds)
        {
            var dir = Path.Combine(packageDir, "types", kind);
            if (!Directory.Exists(dir))
            {
                continue;
            }

            // Remove any leftover files under the kind folder (e.g. orphaned bins).
            try
            {
                foreach (var entry in Directory.EnumerateFileSystemEntries(dir))
                {
                    try
                    {
                        if (File.Exists(entry))
                        {
                            File.Delete(entry);
                        }
                        else if (Directory.Exists(entry))
                        {
                            Directory.Delete(entry, recursive: true);
                        }
                    }
                    catch
                    {
                        // ignore
                    }
                }

                if (!Directory.EnumerateFileSystemEntries(dir).Any())
                {
                    Directory.Delete(dir);
                }
            }
            catch
            {
                // ignore
            }
        }
    }

    private static void WriteJson(string path, object value, bool compact)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllText(path, JsonSerializer.Serialize(value, compact ? CompactJson : IndentedJson));
    }

    private static string ResolvePath(string packageDir, string relative) =>
        Path.Combine(packageDir, relative.Replace('/', Path.DirectorySeparatorChar));

    private static string NormalizePath(string path) =>
        path.Replace('\\', '/');

    private sealed class BlockContext
    {
        public required string BlockKey { get; init; }
        public required string ContentPath { get; init; }
        public required SnaBlockContentDocument Document { get; init; }
        public required List<SnaBlockContentElement> OrderedElements { get; init; }
    }
}
