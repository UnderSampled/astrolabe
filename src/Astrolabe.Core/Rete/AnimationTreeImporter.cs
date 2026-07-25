using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;
using Astrolabe.Core.Serialization;

namespace Astrolabe.Core.Rete;

/// <summary>
/// Builds semantic animation families + transform pool, then rewrites block content
/// to ordered segments with expand targets. Optimized for tens of thousands of leaves:
/// O(n) passes, expand groups instead of nested per-leaf segments, compact JSON.
/// </summary>
internal static class AnimationTreeImporter
{
    private static readonly HashSet<string> AnimationKinds = new(StringComparer.OrdinalIgnoreCase)
    {
        "state",
        "animationmontreal",
        "animframes",
        "animchannel",
        "animchannelptrs",
        "animhierarchiesheader",
        "animhierarchies",
        "transition"
    };

    private static readonly HashSet<string> TransformKinds = new(StringComparer.OrdinalIgnoreCase)
    {
        "compressedmatrix",
        "transform"
    };

    /// <summary>Compact JSON — indented output is multi-second on large pools.</summary>
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = false,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    private static readonly JsonSerializerOptions IndentedOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    public static void AggregateLevelPackage(string packageDir, RetePackageManifest manifest)
    {
        if (!manifest.PackageRole.Equals("level", StringComparison.OrdinalIgnoreCase))
        {
            return;
        }

        // Only deserialize content.json files that mention animation/transform kinds.
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
                if (!File.Exists(contentPath) || !ContentFileMentionsAnimation(contentPath))
                {
                    continue;
                }

                blocks.Add(LoadBlockContext(packageDir, block));
            }
        }

        if (blocks.Count == 0)
        {
            return;
        }

        var pathToTransformUri = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        var elementToTransformId = new Dictionary<SnaBlockContentElement, string>();
        var transforms = BuildTransformPool(
            packageDir, blocks, pathToTransformUri, elementToTransformId);

        var elementToAnimId = new Dictionary<SnaBlockContentElement, string>();
        var pathToAnimUri = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        var families = BuildFamilyForest(
            packageDir, blocks, pathToTransformUri, elementToAnimId, pathToAnimUri);

        if (families.ById.Count == 0 && transforms.ById.Count == 0)
        {
            return;
        }

        foreach (var context in blocks)
        {
            RewriteBlockContent(packageDir, context, families, transforms, elementToTransformId, elementToAnimId);
        }

        WriteDocuments(packageDir, families, transforms);

        // Rewrite remaining package JSON that still points at pre-aggregate type paths
        // (e.g. perso3ddata → state) so export address resolution finds animation URIs.
        RewritePackageWideTypeUris(packageDir, pathToTransformUri, pathToAnimUri);

        DeleteLegacyAnimationFiles(packageDir);
    }

    private sealed class BlockContext
    {
        public required string BlockKey { get; init; }
        public required string ContentPath { get; init; }
        public required SnaBlockContentDocument Document { get; init; }
        public required List<SnaBlockContentElement> OrderedElements { get; init; }
    }

    /// <summary>
    /// Cheap ASCII scan before full JSON deserialize of multi-MB content.json files.
    /// </summary>
    private static bool ContentFileMentionsAnimation(string contentPath)
    {
        // Read a limited prefix+sample; kinds appear throughout the file so scan whole as bytes
        // without UTF-16 expansion. content.json is ASCII.
        var bytes = File.ReadAllBytes(contentPath);
        // Prefer distinctive kind tokens; avoid bare "state" (too many false positives).
        return ContainsAscii(bytes, "animchannel") ||
               ContainsAscii(bytes, "compressedmatrix") ||
               ContainsAscii(bytes, "animationmontreal") ||
               ContainsAscii(bytes, "animframes") ||
               ContainsAscii(bytes, "animhierarchies");
    }

    private static bool ContainsAscii(byte[] haystack, string needle)
    {
        if (needle.Length == 0 || haystack.Length < needle.Length)
        {
            return false;
        }

        // Simple search; content files are a few MB at most.
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
        var document = ReadJson<SnaBlockContentDocument>(contentPath);
        // Import always writes v2 leaf segments; linearize for expand-aware blocks.
        var ordered = SnaBlockContentLinearizer.Linearize(packageDir, document)
            .Select((leaf, index) => new SnaBlockContentElement
            {
                Order = index,
                Kind = leaf.Kind,
                DataPath = leaf.DataPath
            })
            .ToList();

        // Prefer provenance VA from original leaf segments when not yet expanded.
        if (!document.Segments.Any(s =>
                s.Kind.Equals(SnaBlockContentSegment.ExpandKind, StringComparison.OrdinalIgnoreCase)))
        {
            for (var i = 0; i < ordered.Count && i < document.Segments.Count; i++)
            {
                var seg = document.Segments[i];
                if (seg.ProvenanceVirtualAddress is { } va)
                {
                    ordered[i].VirtualAddress = va;
                    ordered[i].Length = seg.Length ?? 0;
                }
            }
        }

        return new BlockContext
        {
            BlockKey = block.Key,
            ContentPath = block.ContentPath!,
            Document = document,
            OrderedElements = ordered
        };
    }

    private static AnimationTransformsDocument BuildTransformPool(
        string packageDir,
        List<BlockContext> blocks,
        Dictionary<string, string> pathToTransformUri,
        Dictionary<SnaBlockContentElement, string> elementToTransformId)
    {
        var doc = new AnimationTransformsDocument();
        var counter = 0;

        foreach (var block in blocks)
        {
            var ordered = block.OrderedElements;
            for (var i = 0; i < ordered.Count; i++)
            {
                var element = ordered[i];
                if (!TransformKinds.Contains(element.Kind))
                {
                    continue;
                }

                var id = $"t_{counter:D5}";
                counter++;

                var bytes = ReadBinaryPayloadFast(packageDir, element);
                var wireLength = TransformWire.GetPayloadLength(bytes);
                if (wireLength > bytes.Length)
                {
                    wireLength = bytes.Length;
                }

                var trailingGap = Array.Empty<byte>();
                if (i + 1 < ordered.Count)
                {
                    var next = ordered[i + 1];
                    if (next.Kind.Equals("raw", StringComparison.OrdinalIgnoreCase) &&
                        next.Length is 4 or 6)
                    {
                        trailingGap = ReadBinaryPayloadFast(packageDir, next);
                    }
                }

                var wire = new byte[wireLength];
                Buffer.BlockCopy(bytes, 0, wire, 0, wireLength);

                doc.ById[id] = new TransformRecord
                {
                    Id = id,
                    WireBytes = wire,
                    TrailingGap = trailingGap,
                    ProvenanceVirtualAddress = element.VirtualAddress != 0
                        ? element.VirtualAddress
                        : null
                };
                doc.Stream.Add(id);
                elementToTransformId[element] = id;

                var uri = AnimationPaths.TransformUri(id);
                RegisterPathKeys(pathToTransformUri, packageDir, element.DataPath, uri);
            }
        }

        return doc;
    }

    private static AnimationFamiliesDocument BuildFamilyForest(
        string packageDir,
        List<BlockContext> blocks,
        Dictionary<string, string> pathToTransformUri,
        Dictionary<SnaBlockContentElement, string> elementToAnimId,
        Dictionary<string, string> pathToAnimUri)
    {
        var families = new AnimationFamiliesDocument();
        var pathToId = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        var counter = 0;

        // Pass 1: assign ids + path maps only (no JSON rewrite yet).
        foreach (var block in blocks)
        {
            foreach (var element in block.OrderedElements)
            {
                if (!AnimationKinds.Contains(element.Kind))
                {
                    continue;
                }

                var id = $"{NormalizeKind(element.Kind)}_{counter:D5}";
                counter++;
                elementToAnimId[element] = id;
                RegisterPathKeys(pathToId, packageDir, element.DataPath, id);
            }
        }

        foreach (var (path, id) in pathToId)
        {
            pathToAnimUri[path] = AnimationPaths.FamilyNodeUri(id);
        }

        // Pass 2: load records + rewrite pointers once.
        foreach (var block in blocks)
        {
            foreach (var element in block.OrderedElements)
            {
                if (!elementToAnimId.TryGetValue(element, out var id))
                {
                    continue;
                }

                var record = ReadElementJson(packageDir, element);
                // Relocate opaque .bin payloads under animation/payloads so types/* can be deleted.
                record = RelocateOpaquePayload(packageDir, id, record);
                record = RewriteJsonStrings(record, value =>
                    RewriteStringRefFast(value, pathToTransformUri, pathToAnimUri));

                families.ById[id] = new AnimationNode
                {
                    Id = id,
                    Kind = NormalizeKind(element.Kind),
                    Record = record,
                    ProvenanceVirtualAddress = element.VirtualAddress != 0
                        ? element.VirtualAddress
                        : null
                };
            }
        }

        // Pass 3: semantic ownership Children (URI edges) for authoring graph.
        var allAnimIds = families.ById.Keys.ToList();
        var indexById = new Dictionary<string, int>(allAnimIds.Count, StringComparer.OrdinalIgnoreCase);
        // Use a global order index from first block stream walk for "later in stream" claims.
        var globalOrder = 0;
        foreach (var block in blocks)
        {
            foreach (var element in block.OrderedElements)
            {
                if (elementToAnimId.TryGetValue(element, out var id))
                {
                    indexById[id] = globalOrder++;
                }
            }
        }

        var ownershipClaimed = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var (parentId, parent) in families.ById)
        {
            if (parent.Record is not { } record ||
                !indexById.TryGetValue(parentId, out var parentIndex))
            {
                continue;
            }

            foreach (var targetId in EnumerateReferencedAnimIds(record))
            {
                if (ownershipClaimed.Contains(targetId) ||
                    !indexById.TryGetValue(targetId, out var childIndex) ||
                    childIndex <= parentIndex)
                {
                    continue;
                }

                parent.Children.Add(targetId);
                ownershipClaimed.Add(targetId);
            }
        }

        // Pass 4: nest Family → State authoring tree from state linked lists + hdr groups.
        BuildNestedFamilyTree(families, pathToId);

        // Layout roots: family ids for documentation / optional forest expand.
        if (families.Families.Count > 0)
        {
            families.LayoutRoots["semantic"] = families.Families.Keys.ToList();
        }

        return families;
    }

    /// <summary>
    /// Build authoring tree: group states by family provenance (hdr / linked-list chains).
    /// </summary>
    private static void BuildNestedFamilyTree(
        AnimationFamiliesDocument families,
        Dictionary<string, string> pathToId)
    {
        var states = families.ById
            .Where(pair => pair.Value.Kind.Equals("state", StringComparison.OrdinalIgnoreCase))
            .Select(pair => pair.Value)
            .ToList();

        if (states.Count == 0)
        {
            families.OrphanLeafIds = families.ById.Keys.ToList();
            return;
        }

        // Map state id → next state id (when next points at a state byId URI).
        var nextById = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        var prevSet = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var hdrKeyByState = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);

        foreach (var state in states)
        {
            if (state.Record is not { } record)
            {
                continue;
            }

            if (TryReadStringProp(record, "next", out var nextUri) &&
                TryFamilyNodeIdFromUri(nextUri, out var nextId) &&
                families.ById.ContainsKey(nextId))
            {
                nextById[state.Id] = nextId;
                prevSet.Add(nextId);
            }

            if (TryReadStringProp(record, "hdr", out var hdr) && !string.IsNullOrEmpty(hdr))
            {
                // Group by family blob file (strip fragment) — states under one family share hdr region.
                hdrKeyByState[state.Id] = hdr.Split('#')[0];
            }
            else
            {
                hdrKeyByState[state.Id] = $"orphan:{state.Id}";
            }
        }

        // Chains: start at states with no prev in nextById graph.
        var heads = states.Select(s => s.Id).Where(id => !prevSet.Contains(id)).ToList();
        var orderedByFamily = new Dictionary<string, List<string>>(StringComparer.OrdinalIgnoreCase);

        foreach (var head in heads)
        {
            var chain = new List<string>();
            var cur = head;
            var guard = 0;
            while (!string.IsNullOrEmpty(cur) && guard++ < 10_000)
            {
                if (chain.Contains(cur, StringComparer.OrdinalIgnoreCase))
                {
                    break;
                }

                chain.Add(cur);
                if (!nextById.TryGetValue(cur, out cur!))
                {
                    break;
                }
            }

            var famKey = hdrKeyByState.GetValueOrDefault(head, $"orphan:{head}");
            if (!orderedByFamily.TryGetValue(famKey, out var list))
            {
                list = [];
                orderedByFamily[famKey] = list;
            }

            // Prefer longer chain when multiple heads share hdr.
            if (list.Count < chain.Count)
            {
                // Merge unique states from this chain.
                foreach (var id in chain)
                {
                    if (!list.Contains(id, StringComparer.OrdinalIgnoreCase))
                    {
                        list.Add(id);
                    }
                }
            }
            else
            {
                foreach (var id in chain)
                {
                    if (!list.Contains(id, StringComparer.OrdinalIgnoreCase))
                    {
                        list.Add(id);
                    }
                }
            }
        }

        // Any states not placed.
        var placed = new HashSet<string>(
            orderedByFamily.Values.SelectMany(v => v),
            StringComparer.OrdinalIgnoreCase);
        foreach (var state in states)
        {
            if (placed.Contains(state.Id))
            {
                continue;
            }

            var famKey = hdrKeyByState.GetValueOrDefault(state.Id, $"orphan:{state.Id}");
            if (!orderedByFamily.TryGetValue(famKey, out var list))
            {
                list = [];
                orderedByFamily[famKey] = list;
            }

            list.Add(state.Id);
            placed.Add(state.Id);
        }

        var familyIndex = 0;
        foreach (var (provenance, stateIds) in orderedByFamily.OrderBy(pair => pair.Key, StringComparer.OrdinalIgnoreCase))
        {
            var famId = $"family_{familyIndex:D3}";
            familyIndex++;

            uint? index = null;
            // Try family_index at +0x0C of opaque family when provenance is a raw blob URI.
            // (Optional; name still Family_N.)

            var entry = new AnimationFamilyEntry
            {
                Id = famId,
                Name = $"Family_{familyIndex - 1}",
                FamilyIndex = index,
                ProvenanceRef = provenance.StartsWith("orphan:", StringComparison.Ordinal)
                    ? null
                    : provenance
            };

            foreach (var stateId in stateIds)
            {
                if (!families.ById.TryGetValue(stateId, out var stateNode) ||
                    stateNode.Record is not { } stateRecord)
                {
                    continue;
                }

                var stateEntry = new AnimationStateEntry
                {
                    Id = stateId,
                    Name = null
                };

                if (TryReadStringProp(stateRecord, "animRef", out var animUri) &&
                    TryFamilyNodeIdFromUri(animUri, out var animId) &&
                    families.ById.ContainsKey(animId))
                {
                    stateEntry.AnimationId = animId;
                }

                // Transitions: children of kind transition under this state.
                foreach (var childId in stateNode.Children)
                {
                    if (families.ById.TryGetValue(childId, out var child) &&
                        child.Kind.Equals("transition", StringComparison.OrdinalIgnoreCase))
                    {
                        stateEntry.TransitionIds.Add(childId);
                    }
                }

                entry.States.Add(stateEntry);
            }

            if (entry.States.Count > 0)
            {
                families.Families[famId] = entry;
            }
        }

        // Orphans: anim leaves not reachable from any state ownership.
        var underFamily = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var fam in families.Families.Values)
        {
            foreach (var st in fam.States)
            {
                underFamily.Add(st.Id);
                if (!string.IsNullOrEmpty(st.AnimationId))
                {
                    CollectDescendants(families, st.AnimationId!, underFamily);
                }

                foreach (var t in st.TransitionIds)
                {
                    underFamily.Add(t);
                    CollectDescendants(families, t, underFamily);
                }
            }
        }

        foreach (var id in families.ById.Keys)
        {
            if (!underFamily.Contains(id))
            {
                families.OrphanLeafIds.Add(id);
            }
        }

        _ = pathToId;
    }

    private static void CollectDescendants(
        AnimationFamiliesDocument families,
        string rootId,
        HashSet<string> into)
    {
        if (!into.Add(rootId) || !families.ById.TryGetValue(rootId, out var node))
        {
            return;
        }

        foreach (var child in node.Children)
        {
            CollectDescendants(families, child, into);
        }
    }

    private static bool TryReadStringProp(JsonElement record, string name, out string value)
    {
        value = "";
        if (record.ValueKind != JsonValueKind.Object)
        {
            return false;
        }

        foreach (var prop in record.EnumerateObject())
        {
            if (!prop.Name.Equals(name, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            if (prop.Value.ValueKind == JsonValueKind.String)
            {
                value = prop.Value.GetString() ?? "";
                return value.Length > 0;
            }

            return false;
        }

        return false;
    }

    private static bool TryFamilyNodeIdFromUri(string uri, out string id)
    {
        id = "";
        var hash = uri.IndexOf('#');
        if (hash < 0)
        {
            return false;
        }

        return AnimationPaths.TryParseFamilyById(uri[(hash + 1)..], out id);
    }

    private static void RegisterPathKeys(
        Dictionary<string, string> map,
        string packageDir,
        string dataPath,
        string value)
    {
        var norm = NormalizeDataPath(dataPath);
        map[norm] = value;
        // Basename for cheap relative matches
        var fileName = Path.GetFileName(norm);
        if (fileName.Length > 0)
        {
            map.TryAdd(fileName, value);
        }

        // Absolute resolved path (only when needed — skip if already absolute-looking)
        if (!Path.IsPathRooted(dataPath))
        {
            try
            {
                map[ReferenceUri.Resolve(packageDir, dataPath).FilePath] = value;
            }
            catch
            {
                // ignore
            }
        }
    }

    private static string RewriteStringRefFast(
        string value,
        Dictionary<string, string> pathToTransformUri,
        Dictionary<string, string> pathToAnimUri)
    {
        if (string.IsNullOrEmpty(value) ||
            value.StartsWith("animation/", StringComparison.Ordinal) ||
            value.StartsWith("fix:/", StringComparison.Ordinal) ||
            value.StartsWith("level:/", StringComparison.Ordinal))
        {
            return value;
        }

        // Only rewrite filesystem-style type paths.
        if (!value.Contains("types/", StringComparison.Ordinal) &&
            !value.Contains(".bin", StringComparison.OrdinalIgnoreCase) &&
            !value.Contains(".json", StringComparison.OrdinalIgnoreCase))
        {
            return value;
        }

        // Preserve #byteOffset=N when the path rewrites to a byId URI.
        var hash = value.IndexOf('#');
        var pathPart = hash >= 0 ? value[..hash] : value;
        var fragment = hash >= 0 ? value[hash..] : "";
        var norm = NormalizeDataPath(pathPart);

        string? rewritten = null;
        if (pathToTransformUri.TryGetValue(norm, out var transformUri))
        {
            rewritten = transformUri;
        }
        else if (pathToAnimUri.TryGetValue(norm, out var animUri))
        {
            rewritten = animUri;
        }
        else
        {
            var fileName = Path.GetFileName(norm);
            if (fileName.Length > 0)
            {
                if (pathToTransformUri.TryGetValue(fileName, out transformUri))
                {
                    rewritten = transformUri;
                }
                else if (pathToAnimUri.TryGetValue(fileName, out animUri))
                {
                    rewritten = animUri;
                }
            }
        }

        if (rewritten == null)
        {
            return value;
        }

        if (fragment.StartsWith("#byteOffset=", StringComparison.OrdinalIgnoreCase))
        {
            // byId URI already uses '#'; encode byte offset as ";byteOffset=N".
            return rewritten + fragment.Replace("#byteOffset=", ";byteOffset=", StringComparison.OrdinalIgnoreCase);
        }

        if (fragment.Length > 0 && !rewritten.Contains('#', StringComparison.Ordinal))
        {
            return rewritten + fragment;
        }

        return rewritten;
    }

    private static IEnumerable<string> EnumerateReferencedAnimIds(JsonElement record)
    {
        foreach (var text in EnumerateStrings(record))
        {
            var hash = text.IndexOf('#');
            if (hash < 0)
            {
                continue;
            }

            if (text.AsSpan(0, hash).IndexOf("families.json", StringComparison.OrdinalIgnoreCase) < 0)
            {
                continue;
            }

            if (AnimationPaths.TryParseFamilyById(text[(hash + 1)..], out var id))
            {
                yield return id;
            }
        }
    }

    private static IEnumerable<string> EnumerateStrings(JsonElement element)
    {
        switch (element.ValueKind)
        {
            case JsonValueKind.Object:
                foreach (var prop in element.EnumerateObject())
                {
                    foreach (var s in EnumerateStrings(prop.Value))
                    {
                        yield return s;
                    }
                }

                break;
            case JsonValueKind.Array:
                foreach (var item in element.EnumerateArray())
                {
                    foreach (var s in EnumerateStrings(item))
                    {
                        yield return s;
                    }
                }

                break;
            case JsonValueKind.String:
                var t = element.GetString();
                if (!string.IsNullOrEmpty(t))
                {
                    yield return t;
                }

                break;
        }
    }

    private static void RewriteBlockContent(
        string packageDir,
        BlockContext context,
        AnimationFamiliesDocument families,
        AnimationTransformsDocument transforms,
        Dictionary<SnaBlockContentElement, string> elementToTransformId,
        Dictionary<SnaBlockContentElement, string> elementToAnimId)
    {
        var segments = new List<SnaBlockContentSegment>();
        var ordered = context.OrderedElements;
        var i = 0;
        var runCounter = 0;

        while (i < ordered.Count)
        {
            var element = ordered[i];

            if (TransformKinds.Contains(element.Kind))
            {
                var transformIds = new List<string>();
                while (i < ordered.Count && TransformKinds.Contains(ordered[i].Kind))
                {
                    if (elementToTransformId.TryGetValue(ordered[i], out var id))
                    {
                        transformIds.Add(id);
                    }

                    i++;
                    // Absorb adjacent 4/6-byte raw gaps into transform records already.
                    if (i < ordered.Count &&
                        ordered[i].Kind.Equals("raw", StringComparison.OrdinalIgnoreCase) &&
                        ordered[i].Length is 4 or 6)
                    {
                        i++;
                    }
                }

                if (transformIds.Count == 0)
                {
                    continue;
                }

                if (transformIds.Count == 1)
                {
                    segments.Add(new SnaBlockContentSegment
                    {
                        Kind = "transform",
                        DataPath = AnimationPaths.TransformUri(transformIds[0])
                    });
                }
                else
                {
                    // One expand segment → ordered id list on the transforms doc (not 20k nested children).
                    var runId = $"run_{context.BlockKey.Replace(':', '_')}_{runCounter:D3}";
                    runCounter++;
                    transforms.Runs[runId] = transformIds;
                    segments.Add(new SnaBlockContentSegment
                    {
                        Kind = SnaBlockContentSegment.ExpandKind,
                        DataPath = $"{AnimationTransformsDocument.RelativePath}#/runs/{runId}"
                    });
                }

                continue;
            }

            if (AnimationKinds.Contains(element.Kind))
            {
                var runIds = new List<string>();
                while (i < ordered.Count && AnimationKinds.Contains(ordered[i].Kind))
                {
                    if (elementToAnimId.TryGetValue(ordered[i], out var id))
                    {
                        runIds.Add(id);
                    }

                    i++;
                }

                if (runIds.Count == 0)
                {
                    continue;
                }

                if (runIds.Count == 1)
                {
                    var id = runIds[0];
                    segments.Add(new SnaBlockContentSegment
                    {
                        Kind = families.ById[id].Kind,
                        DataPath = AnimationPaths.FamilyNodeUri(id)
                    });
                }
                else
                {
                    // Stream-order run: expand emits each leaf once (no semantic DFS).
                    var runId = $"anim_run_{context.BlockKey.Replace(':', '_')}_{runCounter:D3}";
                    runCounter++;
                    families.Runs[runId] = runIds;
                    segments.Add(new SnaBlockContentSegment
                    {
                        Kind = SnaBlockContentSegment.ExpandKind,
                        DataPath = AnimationPaths.FamilyRunUri(runId)
                    });
                }

                continue;
            }

            segments.Add(new SnaBlockContentSegment
            {
                Kind = element.Kind,
                DataPath = element.DataPath,
                ProvenanceVirtualAddress = element.VirtualAddress != 0 ? element.VirtualAddress : null,
                Length = element.Length > 0 ? element.Length : null
            });
            i++;
        }

        context.Document.Schema = SnaBlockContentDocument.SchemaValue;
        context.Document.Segments = segments;
        // content.json stays compact
        WriteJson(ResolvePath(packageDir, context.ContentPath), context.Document, compact: true);
    }

    private static void WriteDocuments(
        string packageDir,
        AnimationFamiliesDocument families,
        AnimationTransformsDocument transforms)
    {
        var animDir = Path.Combine(packageDir, "animation");
        Directory.CreateDirectory(animDir);

        File.WriteAllText(
            Path.Combine(packageDir, AnimationFamiliesDocument.RelativePath),
            JsonSerializer.Serialize(families, JsonOptions));
        File.WriteAllText(
            Path.Combine(packageDir, AnimationTransformsDocument.RelativePath),
            JsonSerializer.Serialize(transforms, JsonOptions));
    }

    /// <summary>
    /// Fast payload read: prefer .bin; only lightly parse JSON descriptors for path.
    /// Avoids full struct codec re-encode for binary leaves.
    /// </summary>
    private static byte[] ReadBinaryPayloadFast(string packageDir, SnaBlockContentElement element)
    {
        var path = ResolvePath(packageDir, element.DataPath);
        if (!File.Exists(path))
        {
            throw new FileNotFoundException($"Missing element payload: {path}");
        }

        if (!path.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
        {
            return File.ReadAllBytes(path);
        }

        // Opaque descriptor: {"path":"....bin", ...}
        var text = File.ReadAllText(path);
        if (TryExtractJsonStringProperty(text, "path", out var rel) &&
            !string.IsNullOrWhiteSpace(rel))
        {
            var bin = ResolvePath(packageDir, rel);
            if (File.Exists(bin))
            {
                return File.ReadAllBytes(bin);
            }
        }

        if (StructCodecRegistry.TryGet(element.Kind, out var codec))
        {
            return codec.WriteFromJsonPath(packageDir, path);
        }

        return File.ReadAllBytes(path);
    }

    private static bool TryExtractJsonStringProperty(string json, string name, out string value)
    {
        value = "";
        var key = $"\"{name}\"";
        var idx = json.IndexOf(key, StringComparison.Ordinal);
        if (idx < 0)
        {
            return false;
        }

        var colon = json.IndexOf(':', idx + key.Length);
        if (colon < 0)
        {
            return false;
        }

        var start = json.IndexOf('"', colon + 1);
        if (start < 0)
        {
            return false;
        }

        var end = json.IndexOf('"', start + 1);
        if (end < 0)
        {
            return false;
        }

        value = json[(start + 1)..end];
        return true;
    }

    private static byte[] ReadElementBytes(string packageDir, SnaBlockContentElement element) =>
        ReadBinaryPayloadFast(packageDir, element);

    private static JsonElement ReadElementJson(string packageDir, SnaBlockContentElement element)
    {
        var path = ResolvePath(packageDir, element.DataPath);
        if (path.EndsWith(".json", StringComparison.OrdinalIgnoreCase) && File.Exists(path))
        {
            using var document = JsonDocument.Parse(File.ReadAllText(path));
            return document.RootElement.Clone();
        }

        var bytes = ReadBinaryPayloadFast(packageDir, element);
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream))
        {
            writer.WriteStartObject();
            writer.WriteString("schema", $"astrolabe.{element.Kind.ToLowerInvariant()}.v1");
            writer.WriteBase64String("data", bytes);
            writer.WriteEndObject();
        }

        using var wrapped = JsonDocument.Parse(stream.ToArray());
        return wrapped.RootElement.Clone();
    }

    private static JsonElement RewriteJsonStrings(JsonElement value, Func<string, string> rewrite)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream))
        {
            WriteRewrite(value, writer, rewrite);
        }

        using var document = JsonDocument.Parse(stream.ToArray());
        return document.RootElement.Clone();
    }

    private static void WriteRewrite(JsonElement value, Utf8JsonWriter writer, Func<string, string> rewrite)
    {
        switch (value.ValueKind)
        {
            case JsonValueKind.Object:
                writer.WriteStartObject();
                foreach (var prop in value.EnumerateObject())
                {
                    writer.WritePropertyName(prop.Name);
                    WriteRewrite(prop.Value, writer, rewrite);
                }

                writer.WriteEndObject();
                break;
            case JsonValueKind.Array:
                writer.WriteStartArray();
                foreach (var item in value.EnumerateArray())
                {
                    WriteRewrite(item, writer, rewrite);
                }

                writer.WriteEndArray();
                break;
            case JsonValueKind.String:
                writer.WriteStringValue(rewrite(value.GetString() ?? ""));
                break;
            default:
                value.WriteTo(writer);
                break;
        }
    }

    /// <summary>
    /// If the record is an opaque descriptor with <c>path</c> to a .bin under types/,
    /// copy the bin to <c>animation/payloads/{id}.bin</c> and rewrite path.
    /// </summary>
    private static JsonElement RelocateOpaquePayload(string packageDir, string id, JsonElement record)
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

        var destRel = $"animation/payloads/{id}.bin";
        var dest = ResolvePath(packageDir, destRel);
        Directory.CreateDirectory(Path.GetDirectoryName(dest)!);
        if (!File.Exists(dest))
        {
            File.Copy(source, dest, overwrite: false);
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

        using var document = JsonDocument.Parse(stream.ToArray());
        return document.RootElement.Clone();
    }

    private static string NormalizeKind(string kind) =>
        kind.Equals("compressedmatrix", StringComparison.OrdinalIgnoreCase) ? "transform" : kind;

    private static string NormalizeDataPath(string dataPath) =>
        dataPath.Replace('\\', '/');

    /// <summary>
    /// Rewrite types/* and scene/* JSON that still reference pre-aggregate animation paths.
    /// </summary>
    private static void RewritePackageWideTypeUris(
        string packageDir,
        Dictionary<string, string> pathToTransformUri,
        Dictionary<string, string> pathToAnimUri)
    {
        foreach (var sub in new[] { "types", "scene", "slots" })
        {
            var root = Path.Combine(packageDir, sub);
            if (!Directory.Exists(root))
            {
                continue;
            }

            foreach (var jsonPath in Directory.EnumerateFiles(root, "*.json", SearchOption.AllDirectories))
            {
                // Skip if under kinds we already rewrote into families byId (optional).
                var text = File.ReadAllText(jsonPath);
                if (!text.Contains("types/anim", StringComparison.Ordinal) &&
                    !text.Contains("types/state", StringComparison.Ordinal) &&
                    !text.Contains("types/transition", StringComparison.Ordinal) &&
                    !text.Contains("compressedmatrix", StringComparison.Ordinal) &&
                    !text.Contains("types/animchannel", StringComparison.Ordinal))
                {
                    continue;
                }

                using var document = JsonDocument.Parse(text);
                var rewritten = RewriteJsonStrings(document.RootElement, value =>
                    RewriteStringRefFast(value, pathToTransformUri, pathToAnimUri));
                var newText = rewritten.GetRawText();
                if (!string.Equals(text, newText, StringComparison.Ordinal))
                {
                    File.WriteAllText(jsonPath, newText);
                }
            }
        }
    }

    private static void DeleteLegacyAnimationFiles(string packageDir)
    {
        // Only remove compressed-matrix micro-files: transform pool is authoritative and
        // channel/perso records are rewritten to pool URIs. Keep state/animchannel/etc.
        // types/* leaves for now so non-stream hub records (perso3ddata, …) still resolve.
        foreach (var kind in new[] { "compressedmatrix" })
        {
            var dir = Path.Combine(packageDir, "types", kind);
            if (Directory.Exists(dir))
            {
                Directory.Delete(dir, recursive: true);
            }
        }

#pragma warning disable CS0618
        var legacy = Path.Combine(packageDir, AnimationTreeDocument.RelativePath);
#pragma warning restore CS0618
        if (File.Exists(legacy))
        {
            File.Delete(legacy);
        }
    }

    private static string ResolvePath(string rootDir, string relativePath) =>
        Path.Combine(relativePath.Split('/').Prepend(rootDir).ToArray());

    private static T ReadJson<T>(string path) where T : class =>
        JsonSerializer.Deserialize<T>(File.ReadAllText(path), JsonOptions)
        ?? throw new InvalidDataException($"Could not read {path}");

    private static void WriteJson(string path, object value, bool compact = false)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllText(
            path,
            JsonSerializer.Serialize(value, compact ? JsonOptions : IndentedOptions));
    }
}
