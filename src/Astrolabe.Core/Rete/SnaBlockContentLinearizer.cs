using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;
using Astrolabe.Core.Serialization;

namespace Astrolabe.Core.Rete;

/// <summary>
/// Linearizes a whole SNA block from ordered segments (or legacy elements).
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

    public readonly record struct Leaf(string Kind, string DataPath);

    public static IReadOnlyList<Leaf> Linearize(string packageRoot, SnaBlockContentDocument document)
    {
        EnsureDocCache(packageRoot);

        if (document.Segments.Count > 0)
        {
            var leaves = new List<Leaf>();
            var emitted = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            foreach (var segment in document.Segments)
            {
                ExpandSegment(packageRoot, segment, leaves, emitted);
            }

            return leaves;
        }

        // Legacy v1: array order preferred; fall back to Order rank when mixed.
        return document.Elements
            .Select((element, index) => (element, index))
            .OrderBy(pair => pair.element.Order != 0 ? pair.element.Order : pair.index)
            .ThenBy(pair => pair.index)
            .Select(pair => new Leaf(pair.element.Kind, pair.element.DataPath))
            .ToList();
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

        leaves.Add(new Leaf(segment.Kind, segment.DataPath));
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

        throw new InvalidDataException($"Unsupported expand target: {dataPath}");
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
        if (string.Equals(_cacheRoot, full, StringComparison.OrdinalIgnoreCase) &&
            (_cacheFamilies != null || _cacheTransforms != null ||
             !File.Exists(Path.Combine(full, AnimationFamiliesDocument.RelativePath))))
        {
            return;
        }

        _cacheRoot = full;
        _cacheFamilies = null;
        _cacheTransforms = null;

        var familiesPath = Path.Combine(full, AnimationFamiliesDocument.RelativePath);
        if (File.Exists(familiesPath))
        {
            _cacheFamilies = JsonSerializer.Deserialize<AnimationFamiliesDocument>(
                                 File.ReadAllText(familiesPath), JsonOptions)
                             ?? throw new InvalidDataException($"Could not read {familiesPath}");
        }

        var transformsPath = Path.Combine(full, AnimationTransformsDocument.RelativePath);
        if (File.Exists(transformsPath))
        {
            _cacheTransforms = JsonSerializer.Deserialize<AnimationTransformsDocument>(
                                   File.ReadAllText(transformsPath), JsonOptions)
                               ?? throw new InvalidDataException($"Could not read {transformsPath}");
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
