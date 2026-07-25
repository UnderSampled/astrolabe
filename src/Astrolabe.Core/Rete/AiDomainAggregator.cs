using System.Text.Json;
using Astrolabe.Core.FileFormats.Semantic;

namespace Astrolabe.Core.Rete;

/// <summary>
/// AI dual-layer pool: models metadata in ai/models.json; script AST as ai/scripts/*.sexpr.
/// Wire bytes for opaque leaves live under ai/payloads/*.bin (Record.path) for lossless export.
/// Authoring graph nests brain → mind → intelligence/aimodel → scripts where URI edges allow.
/// </summary>
internal static class AiDomainAggregator
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = false,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    /// <summary>
    /// Structured ownership fields (parent kind → JSON property names that own children).
    /// Only targets that resolve to byId pool nodes become Children edges.
    /// </summary>
    private static readonly Dictionary<string, string[]> StructuredOwnership =
        new(StringComparer.OrdinalIgnoreCase)
        {
            ["brain"] = ["mind"],
            ["mind"] =
            [
                "aiModel",
                "intelligenceNormal",
                "intelligenceReflex",
                "dsgMem"
            ],
            ["aimodel"] = ["behaviorsNormal", "behaviorsReflex", "dsgVar"],
            // Intelligence.comport often points at raw/behavior bytes; claim only when pool-local.
            ["intelligence"] = ["comport", "lastComport", "defaultComport"]
        };

    public static void Aggregate(string packageDir, RetePackageManifest manifest)
    {
        SemanticDomainAggregator.AggregateDomain(
            packageDir,
            manifest,
            "ai",
            AiPoolDocument.RelativePath,
            AiPoolDocument.SchemaValue,
            SemanticDomainKinds.Ai,
            sexprScripts: true);

        NestAuthoringGraph(packageDir);
    }

    /// <summary>
    /// Build optional Children / Roots from pointer URI edges already rewritten into records.
    /// Stream order remains in <see cref="AiPoolDocument.Runs"/>; this is authoring only.
    /// </summary>
    internal static void NestAuthoringGraph(string packageDir)
    {
        var path = Path.Combine(
            packageDir,
            AiPoolDocument.RelativePath.Replace('/', Path.DirectorySeparatorChar));
        if (!File.Exists(path))
        {
            return;
        }

        var doc = JsonSerializer.Deserialize<AiPoolDocument>(File.ReadAllText(path), JsonOptions);
        if (doc == null || doc.ById.Count == 0)
        {
            return;
        }

        // uri / bare-id → node id
        var resolve = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (var (id, node) in doc.ById)
        {
            resolve[id] = id;
            resolve[SemanticPoolPaths.AiNodeUri(id)] = id;
            // Fragment-only forms after partial rewrites
            resolve[$"#/byId/{id}"] = id;
            resolve[$"/byId/{id}"] = id;
        }

        var claimed = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        // Prefer structural owners first (brain → mind → …) so scripts nest under behaviors.
        var ordered = doc.ById.Values
            .OrderBy(n => KindNestPriority(n.Kind))
            .ThenBy(n => n.Id, StringComparer.OrdinalIgnoreCase)
            .ToList();

        foreach (var parent in ordered)
        {
            parent.Children ??= [];
            foreach (var targetId in EnumerateOwnedTargets(parent, resolve))
            {
                if (string.Equals(targetId, parent.Id, StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                if (!doc.ById.ContainsKey(targetId) || claimed.Contains(targetId))
                {
                    continue;
                }

                parent.Children.Add(targetId);
                claimed.Add(targetId);
            }
        }

        doc.Roots = doc.ById.Values
            .Where(n => n.Kind.Equals("brain", StringComparison.OrdinalIgnoreCase))
            .Select(n => n.Id)
            .OrderBy(id => id, StringComparer.OrdinalIgnoreCase)
            .ToList();

        File.WriteAllText(path, JsonSerializer.Serialize(doc, JsonOptions));
    }

    private static int KindNestPriority(string kind) =>
        kind.ToLowerInvariant() switch
        {
            "brain" => 0,
            "mind" => 1,
            "aimodel" => 2,
            "intelligence" => 3,
            "behaviorlist_normal" or "behaviorlist_reflex" => 4,
            "behaviors_normal" or "behaviors_reflex" => 5,
            "dsgvar" or "dsgmem" or "dsgvarptrindirect" => 6,
            "scriptptrs" => 7,
            "script" => 8,
            _ => 9
        };

    private static IEnumerable<string> EnumerateOwnedTargets(
        SemanticPoolNode parent,
        Dictionary<string, string> resolve)
    {
        if (parent.Record is not { } record || record.ValueKind != JsonValueKind.Object)
        {
            yield break;
        }

        if (StructuredOwnership.TryGetValue(parent.Kind, out var fields))
        {
            foreach (var field in fields)
            {
                if (!TryGetPropertyIgnoreCase(record, field, out var prop))
                {
                    continue;
                }

                if (prop.ValueKind == JsonValueKind.String &&
                    TryResolveId(prop.GetString(), resolve, out var id))
                {
                    yield return id;
                }
            }
        }

        // Opaque leaves: Pointers map offsets → URI (behavior arrays → scripts, etc.).
        if (TryGetPropertyIgnoreCase(record, "pointers", out var pointers) &&
            pointers.ValueKind == JsonValueKind.Object)
        {
            foreach (var ptr in pointers.EnumerateObject())
            {
                if (ptr.Value.ValueKind == JsonValueKind.String &&
                    TryResolveId(ptr.Value.GetString(), resolve, out var id))
                {
                    yield return id;
                }
            }
        }
    }

    private static bool TryGetPropertyIgnoreCase(JsonElement obj, string name, out JsonElement value)
    {
        foreach (var prop in obj.EnumerateObject())
        {
            if (prop.Name.Equals(name, StringComparison.OrdinalIgnoreCase))
            {
                value = prop.Value;
                return true;
            }
        }

        value = default;
        return false;
    }

    private static bool TryResolveId(
        string? uriOrId,
        Dictionary<string, string> resolve,
        out string id)
    {
        id = "";
        if (string.IsNullOrWhiteSpace(uriOrId))
        {
            return false;
        }

        var key = uriOrId.Trim();
        // Strip JSON pointer query-like suffixes (#byteOffset=…) from non-pool paths.
        if (resolve.TryGetValue(key, out id!))
        {
            return true;
        }

        // ai/models.json#/byId/ai_00001 or #/byId/ai_00001
        var hash = key.IndexOf('#');
        if (hash >= 0)
        {
            var doc = key[..hash];
            var fragment = key[(hash + 1)..];
            // Ignore non-byId fragments (byteOffset on raw leaves).
            if (fragment.StartsWith("/byId/", StringComparison.OrdinalIgnoreCase) ||
                fragment.StartsWith("byId/", StringComparison.OrdinalIgnoreCase))
            {
                if (resolve.TryGetValue(key, out id!))
                {
                    return true;
                }

                var pointer = fragment.TrimStart('/');
                if (SemanticPoolPaths.TryParseById(pointer, out var parsed) &&
                    resolve.TryGetValue(parsed, out id!))
                {
                    return true;
                }

                // Accept any doc path as long as byId id is in the AI pool.
                if (SemanticPoolPaths.TryParseById(pointer, out parsed) &&
                    resolve.ContainsKey(parsed))
                {
                    id = parsed;
                    return true;
                }
            }

            _ = doc; // silence unused when fragment is not byId
        }

        return false;
    }
}
