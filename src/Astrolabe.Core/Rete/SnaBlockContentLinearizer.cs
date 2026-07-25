using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;
using Astrolabe.Core.FileFormats.Semantic;
using Astrolabe.Core.Serialization;

namespace Astrolabe.Core.Rete;

/// <summary>
/// Linearizes a whole SNA block from ordered v2 segments.
/// Expand walks trees / ordered id lists so content.json need not list every leaf.
/// </summary>
internal static class SnaBlockContentLinearizer
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    [ThreadStatic]
    private static string? _cacheRoot;

    [ThreadStatic]
    private static AnimationFamiliesDocument? _cacheFamilies;

    [ThreadStatic]
    private static AnimationTransformsDocument? _cacheTransforms;

    [ThreadStatic]
    private static Dictionary<string, SemanticPoolDocumentCache>? _semanticCaches;

    private sealed class SemanticPoolDocumentCache
    {
        public Dictionary<string, SemanticPoolNode> ById { get; init; } = new(StringComparer.OrdinalIgnoreCase);
        public Dictionary<string, List<string>> Runs { get; init; } = new(StringComparer.OrdinalIgnoreCase);
    }

    public readonly record struct Leaf(string Kind, string DataPath, int? Length = null);

    public static IReadOnlyList<Leaf> Linearize(string packageRoot, SnaBlockContentDocument document)
    {
        if (!string.Equals(document.Schema, SnaBlockContentDocument.SchemaValue, StringComparison.Ordinal) &&
            !string.IsNullOrEmpty(document.Schema))
        {
            throw new InvalidDataException(
                $"Unsupported SNA block content schema '{document.Schema}'. " +
                $"Only {SnaBlockContentDocument.SchemaValue} is supported (no v1 fallback).");
        }

        if (document.Segments.Count == 0)
        {
            throw new InvalidDataException(
                $"SNA block content for {document.BlockKey} has no segments " +
                $"(schema {SnaBlockContentDocument.SchemaValue} requires ordered segments).");
        }

        EnsureDocCache(packageRoot);

        var leaves = new List<Leaf>();
        var emitted = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var segment in document.Segments)
        {
            ExpandSegment(packageRoot, segment, leaves, emitted);
        }

        return leaves;
    }

    private static void ExpandSegment(
        string packageRoot,
        SnaBlockContentSegment segment,
        List<Leaf> leaves,
        HashSet<string> emitted)
    {
        if (segment.Children is { Count: > 0 })
        {
            foreach (var child in segment.Children)
            {
                ExpandSegment(packageRoot, child, leaves, emitted);
            }

            return;
        }

        if (segment.Kind.Equals(SnaBlockContentSegment.ExpandKind, StringComparison.OrdinalIgnoreCase))
        {
            ExpandUri(packageRoot, segment.DataPath, leaves, emitted);
            return;
        }

        if (string.IsNullOrWhiteSpace(segment.DataPath))
        {
            return;
        }

        var key = segment.DataPath;
        if (!emitted.Add(key))
        {
            return;
        }

        leaves.Add(new Leaf(segment.Kind, segment.DataPath, segment.Length));
    }

    private static void ExpandUri(
        string packageRoot,
        string dataPath,
        List<Leaf> leaves,
        HashSet<string> emitted)
    {
        var resolved = ReferenceUri.Resolve(packageRoot, dataPath);
        var relative = NormalizePackageRelative(packageRoot, resolved.FilePath);
        var pointer = (resolved.JsonPointer ?? "").TrimStart('/');

        if (relative.Equals(AnimationTransformsDocument.RelativePath, StringComparison.OrdinalIgnoreCase))
        {
            ExpandTransforms(packageRoot, pointer, leaves, emitted);
            return;
        }

        if (relative.Equals(AnimationFamiliesDocument.RelativePath, StringComparison.OrdinalIgnoreCase))
        {
            ExpandFamilies(packageRoot, pointer, leaves, emitted);
            return;
        }

        var semanticDoc = SemanticPoolPaths.MatchDocumentRelative(relative);
        if (semanticDoc != null)
        {
            ExpandSemanticPool(packageRoot, semanticDoc, pointer, leaves, emitted);
            return;
        }

        throw new InvalidDataException($"Unsupported expand target: {dataPath}");
    }

    private static void ExpandSemanticPool(
        string packageRoot,
        string relativeDoc,
        string pointer,
        List<Leaf> leaves,
        HashSet<string> emitted)
    {
        var cache = ReadSemanticPool(packageRoot, relativeDoc);
        if (pointer.StartsWith("runs/", StringComparison.OrdinalIgnoreCase))
        {
            var runId = pointer["runs/".Length..];
            if (!cache.Runs.TryGetValue(runId, out var runIds))
            {
                throw new InvalidDataException($"Unknown semantic run: {relativeDoc}#/{pointer}");
            }

            foreach (var id in runIds)
            {
                EmitSemanticLeaf(relativeDoc, cache, id, leaves, emitted);
            }

            return;
        }

        if (pointer.StartsWith("byId/", StringComparison.OrdinalIgnoreCase))
        {
            EmitSemanticLeaf(relativeDoc, cache, pointer["byId/".Length..], leaves, emitted);
            return;
        }

        throw new InvalidDataException($"Unsupported semantic expand pointer: {relativeDoc}#/{pointer}");
    }

    private static void EmitSemanticLeaf(
        string relativeDoc,
        SemanticPoolDocumentCache cache,
        string id,
        List<Leaf> leaves,
        HashSet<string> emitted)
    {
        // Run entries and expand targets may be bare ids or field fragments
        // (scene: id/matrix, id/staticMatrix). Bare byId expand emits the primary node only;
        // field fragments are separate stream leaves for wire parity.
        string? field = null;
        var fieldSlash = id.IndexOf('/');
        if (fieldSlash >= 0)
        {
            field = id[(fieldSlash + 1)..];
            id = id[..fieldSlash];
        }

        if (!cache.ById.TryGetValue(id, out var node))
        {
            throw new InvalidDataException($"Semantic node not found: {relativeDoc}#/byId/{id}");
        }

        if (!string.IsNullOrEmpty(field))
        {
            if (field.Equals("matrix", StringComparison.OrdinalIgnoreCase) ||
                field.Equals("staticMatrix", StringComparison.OrdinalIgnoreCase))
            {
                // Preserve original field casing from the node document path convention.
                var fieldName = field.Equals("staticMatrix", StringComparison.OrdinalIgnoreCase)
                    ? "staticMatrix"
                    : "matrix";
                var fieldPath = $"{relativeDoc}#/byId/{id}/{fieldName}";
                if (!emitted.Add(fieldPath))
                {
                    return;
                }

                leaves.Add(new Leaf("matrix", fieldPath));
                return;
            }

            throw new InvalidDataException(
                $"Unknown semantic field expand: {relativeDoc}#/byId/{id}/{field}");
        }

        var dataPath = $"{relativeDoc}#/byId/{id}";
        if (!emitted.Add(dataPath))
        {
            return;
        }

        leaves.Add(new Leaf(node.Kind, dataPath));
    }

    private static SemanticPoolDocumentCache ReadSemanticPool(string packageRoot, string relativeDoc)
    {
        EnsureDocCache(packageRoot);
        if (_semanticCaches != null &&
            _semanticCaches.TryGetValue(relativeDoc, out var cached))
        {
            return cached;
        }

        throw new FileNotFoundException($"Missing semantic pool document: {relativeDoc}");
    }

    private static void ExpandTransforms(
        string packageRoot,
        string pointer,
        List<Leaf> leaves,
        HashSet<string> emitted)
    {
        var doc = ReadTransforms(packageRoot);
        if (pointer.Length == 0 ||
            pointer.Equals("stream", StringComparison.OrdinalIgnoreCase))
        {
            foreach (var id in doc.Stream)
            {
                EmitTransform(id, leaves, emitted);
            }

            return;
        }

        if (pointer.StartsWith("byId/", StringComparison.OrdinalIgnoreCase))
        {
            var id = pointer["byId/".Length..];
            EmitTransform(id, leaves, emitted);
            return;
        }

        if (pointer.StartsWith("stream/", StringComparison.OrdinalIgnoreCase))
        {
            // stream/<id> single entry
            var id = pointer["stream/".Length..];
            if (doc.Stream.Contains(id, StringComparer.OrdinalIgnoreCase) ||
                doc.ById.ContainsKey(id))
            {
                EmitTransform(id, leaves, emitted);
                return;
            }
        }

        if (pointer.StartsWith("runs/", StringComparison.OrdinalIgnoreCase))
        {
            var runId = pointer["runs/".Length..];
            if (!doc.Runs.TryGetValue(runId, out var runIds))
            {
                throw new InvalidDataException($"Unknown transform run: {runId}");
            }

            foreach (var id in runIds)
            {
                EmitTransform(id, leaves, emitted);
            }

            return;
        }

        throw new InvalidDataException(
            $"Unsupported transforms expand pointer: /{pointer}");
    }

    private static void EmitTransform(string id, List<Leaf> leaves, HashSet<string> emitted)
    {
        var dataPath = AnimationPaths.TransformUri(id);
        if (!emitted.Add(dataPath))
        {
            return;
        }

        leaves.Add(new Leaf("transform", dataPath));
    }

    private static void ExpandFamilies(
        string packageRoot,
        string pointer,
        List<Leaf> leaves,
        HashSet<string> emitted)
    {
        var doc = ReadFamilies(packageRoot);
        if (pointer.StartsWith("layoutRoots/", StringComparison.OrdinalIgnoreCase))
        {
            var blockKey = pointer["layoutRoots/".Length..].Replace('_', ':');
            // Accept both "05:01" and "05_01" in the pointer
            if (!doc.LayoutRoots.TryGetValue(blockKey, out var roots))
            {
                var alt = pointer["layoutRoots/".Length..];
                if (!doc.LayoutRoots.TryGetValue(alt, out roots))
                {
                    // try match ignoring separator style
                    roots = doc.LayoutRoots
                        .FirstOrDefault(pair =>
                            pair.Key.Replace(':', '_').Equals(alt, StringComparison.OrdinalIgnoreCase))
                        .Value;
                }
            }

            if (roots == null)
            {
                throw new InvalidDataException($"Unknown layoutRoots block: {pointer}");
            }

            foreach (var rootId in roots)
            {
                ExpandFamilyNode(doc, rootId, leaves, emitted);
            }

            return;
        }

        if (pointer.StartsWith("byId/", StringComparison.OrdinalIgnoreCase))
        {
            ExpandFamilyNode(doc, pointer["byId/".Length..], leaves, emitted);
            return;
        }

        if (pointer.StartsWith("runs/", StringComparison.OrdinalIgnoreCase))
        {
            var runId = pointer["runs/".Length..];
            if (!doc.Runs.TryGetValue(runId, out var runIds))
            {
                throw new InvalidDataException($"Unknown animation run: {runId}");
            }

            foreach (var id in runIds)
            {
                EmitFamilyLeaf(doc, id, leaves, emitted);
            }

            return;
        }

        throw new InvalidDataException($"Unsupported families expand pointer: /{pointer}");
    }

    /// <summary>Emit a single byId leaf without walking semantic Children (stream-safe).</summary>
    private static void EmitFamilyLeaf(
        AnimationFamiliesDocument doc,
        string id,
        List<Leaf> leaves,
        HashSet<string> emitted)
    {
        if (!doc.ById.TryGetValue(id, out var node))
        {
            throw new InvalidDataException($"Animation node not found: {id}");
        }

        var dataPath = AnimationPaths.FamilyNodeUri(node.Id);
        if (!emitted.Add(dataPath))
        {
            return;
        }

        leaves.Add(new Leaf(node.Kind, dataPath));
    }

    private static void ExpandFamilyNode(
        AnimationFamiliesDocument doc,
        string id,
        List<Leaf> leaves,
        HashSet<string> emitted)
    {
        // layoutRoots may point at family ids (authoring) — expand all stream leaves under
        // that family's states via byId only when ids are leaf nodes.
        if (doc.Families.TryGetValue(id, out var family))
        {
            foreach (var state in family.States)
            {
                EmitFamilyLeaf(doc, state.Id, leaves, emitted);
                if (!string.IsNullOrEmpty(state.AnimationId))
                {
                    EmitOwnedSubtree(doc, state.AnimationId!, leaves, emitted);
                }

                foreach (var transitionId in state.TransitionIds)
                {
                    EmitFamilyLeaf(doc, transitionId, leaves, emitted);
                }
            }

            return;
        }

        EmitFamilyLeaf(doc, id, leaves, emitted);
        if (doc.ById.TryGetValue(id, out var node))
        {
            foreach (var childId in node.Children)
            {
                ExpandFamilyNode(doc, childId, leaves, emitted);
            }
        }
    }

    private static void EmitOwnedSubtree(
        AnimationFamiliesDocument doc,
        string rootId,
        List<Leaf> leaves,
        HashSet<string> emitted)
    {
        if (!doc.ById.TryGetValue(rootId, out var node))
        {
            return;
        }

        EmitFamilyLeaf(doc, rootId, leaves, emitted);
        foreach (var childId in node.Children)
        {
            EmitOwnedSubtree(doc, childId, leaves, emitted);
        }
    }

    private static void EnsureDocCache(string packageRoot)
    {
        var full = Path.GetFullPath(packageRoot);
        var familiesPath = Path.Combine(full, AnimationFamiliesDocument.RelativePath);
        var transformsPath = Path.Combine(full, AnimationTransformsDocument.RelativePath);

        // Reload when package root changes, or when a document file exists that we have not
        // loaded yet (import writes families.json after rewriting content expands — a stale
        // ThreadStatic cache must not keep _cacheFamilies=null after the file appears).
        var sameRoot = string.Equals(_cacheRoot, full, StringComparison.OrdinalIgnoreCase);
        var familiesReady = File.Exists(familiesPath);
        var transformsReady = File.Exists(transformsPath);
        if (sameRoot &&
            _semanticCaches != null &&
            (!familiesReady || _cacheFamilies != null) &&
            (!transformsReady || _cacheTransforms != null) &&
            SemanticPoolFilesCached(full))
        {
            return;
        }

        _cacheRoot = full;
        _cacheFamilies = null;
        _cacheTransforms = null;
        _semanticCaches = new Dictionary<string, SemanticPoolDocumentCache>(StringComparer.OrdinalIgnoreCase);

        if (familiesReady)
        {
            _cacheFamilies = JsonSerializer.Deserialize<AnimationFamiliesDocument>(
                                 File.ReadAllText(familiesPath), JsonOptions)
                             ?? throw new InvalidDataException($"Could not read {familiesPath}");
        }

        if (transformsReady)
        {
            _cacheTransforms = JsonSerializer.Deserialize<AnimationTransformsDocument>(
                                   File.ReadAllText(transformsPath), JsonOptions)
                               ?? throw new InvalidDataException($"Could not read {transformsPath}");
        }

        LoadSemanticDoc(full, SceneTreeDocument.RelativePath, text =>
        {
            var doc = JsonSerializer.Deserialize<SceneTreeDocument>(text, JsonOptions);
            return doc == null
                ? null
                : new SemanticPoolDocumentCache { ById = doc.ById, Runs = doc.Runs };
        });
        LoadSemanticDoc(full, GeometryPoolDocument.RelativePath, text =>
        {
            var doc = JsonSerializer.Deserialize<GeometryPoolDocument>(text, JsonOptions);
            return doc == null
                ? null
                : new SemanticPoolDocumentCache { ById = doc.ById, Runs = doc.Runs };
        });
        LoadSemanticDoc(full, AiPoolDocument.RelativePath, text =>
        {
            var doc = JsonSerializer.Deserialize<AiPoolDocument>(text, JsonOptions);
            return doc == null
                ? null
                : new SemanticPoolDocumentCache { ById = doc.ById, Runs = doc.Runs };
        });
        LoadSemanticDoc(full, CharacterPoolDocument.RelativePath, text =>
        {
            var doc = JsonSerializer.Deserialize<CharacterPoolDocument>(text, JsonOptions);
            return doc == null
                ? null
                : new SemanticPoolDocumentCache { ById = doc.ById, Runs = doc.Runs };
        });
        LoadSemanticDoc(full, SectorPoolDocument.RelativePath, text =>
        {
            var doc = JsonSerializer.Deserialize<SectorPoolDocument>(text, JsonOptions);
            return doc == null
                ? null
                : new SemanticPoolDocumentCache { ById = doc.ById, Runs = doc.Runs };
        });
    }

    private static bool SemanticPoolFilesCached(string packageRoot)
    {
        if (_semanticCaches == null)
        {
            return false;
        }

        foreach (var relative in new[]
                 {
                     SceneTreeDocument.RelativePath,
                     GeometryPoolDocument.RelativePath,
                     AiPoolDocument.RelativePath,
                     CharacterPoolDocument.RelativePath,
                     SectorPoolDocument.RelativePath
                 })
        {
            var path = Path.Combine(packageRoot, relative.Replace('/', Path.DirectorySeparatorChar));
            if (File.Exists(path) && !_semanticCaches.ContainsKey(relative))
            {
                return false;
            }
        }

        return true;
    }

    private static void LoadSemanticDoc(
        string packageRoot,
        string relative,
        Func<string, SemanticPoolDocumentCache?> loader)
    {
        var path = Path.Combine(packageRoot, relative.Replace('/', Path.DirectorySeparatorChar));
        if (!File.Exists(path))
        {
            return;
        }

        var cache = loader(File.ReadAllText(path));
        if (cache != null)
        {
            _semanticCaches![relative] = cache;
        }
    }

    private static AnimationTransformsDocument ReadTransforms(string packageRoot)
    {
        EnsureDocCache(packageRoot);
        return _cacheTransforms
               ?? throw new FileNotFoundException(
                   $"Missing {AnimationTransformsDocument.RelativePath}",
                   Path.Combine(packageRoot, AnimationTransformsDocument.RelativePath));
    }

    private static AnimationFamiliesDocument ReadFamilies(string packageRoot)
    {
        EnsureDocCache(packageRoot);
        return _cacheFamilies
               ?? throw new FileNotFoundException(
                   $"Missing {AnimationFamiliesDocument.RelativePath}",
                   Path.Combine(packageRoot, AnimationFamiliesDocument.RelativePath));
    }

    private static string NormalizePackageRelative(string packageRoot, string fullPath)
    {
        var root = Path.GetFullPath(packageRoot)
            .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        var full = Path.GetFullPath(fullPath);
        if (full.StartsWith(root, StringComparison.OrdinalIgnoreCase))
        {
            return full[(root.Length + 1)..].Replace('\\', '/');
        }

        return fullPath.Replace('\\', '/');
    }
}
