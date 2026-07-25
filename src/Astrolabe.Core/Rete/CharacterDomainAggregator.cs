using System.Text.Json;
using Astrolabe.Core.FileFormats.Semantic;

namespace Astrolabe.Core.Rete;

/// <summary>
/// Perso / object-list / standard-game dual-layer pool (non-animation character package).
/// Animation families/states are handled by <see cref="AnimationTreeImporter"/>; this domain
/// only aggregates non-animation character kinds into <c>characters/persos.json</c>.
/// </summary>
internal static class CharacterDomainAggregator
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = false,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    /// <summary>
    /// Kinds treated as authoring forest roots when unclaimed by another character node.
    /// Other unclaimed leaves are still roots (orphans) but listed after preferred kinds.
    /// </summary>
    private static readonly string[] PreferredRootKinds =
    [
        "perso",
        "spawnableentry",
        "objectlist",
        "objecttypeentry",
        "alwayssuperobjects"
    ];

    public static void Aggregate(string packageDir, RetePackageManifest manifest)
    {
        SemanticDomainAggregator.AggregateDomain(
            packageDir,
            manifest,
            "character",
            CharacterPoolDocument.RelativePath,
            CharacterPoolDocument.SchemaValue,
            SemanticDomainKinds.Character);

        // Optional authoring pass: build Roots + Children ownership from pointer URIs.
        LinkOwnership(packageDir);
    }

    /// <summary>
    /// Walk each pool node's codec record for in-domain <c>characters/persos.json#/byId/…</c>
    /// URI edges and materialize them as <see cref="SemanticPoolNode.Children"/>. Unclaimed
    /// nodes become <see cref="CharacterPoolDocument.Roots"/>.
    /// </summary>
    internal static void LinkOwnership(string packageDir)
    {
        var full = Path.Combine(
            packageDir,
            CharacterPoolDocument.RelativePath.Replace('/', Path.DirectorySeparatorChar));
        if (!File.Exists(full))
        {
            return;
        }

        var doc = JsonSerializer.Deserialize<CharacterPoolDocument>(File.ReadAllText(full), JsonOptions);
        if (doc == null || doc.ById.Count == 0)
        {
            return;
        }

        // Stable stream-ish order: character_00000, character_00001, …
        var orderedIds = doc.ById.Keys
            .OrderBy(id => id, StringComparer.OrdinalIgnoreCase)
            .ToList();

        var claimed = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var parentId in orderedIds)
        {
            if (!doc.ById.TryGetValue(parentId, out var parent))
            {
                continue;
            }

            parent.Children = [];
            if (parent.Record is not { } record)
            {
                continue;
            }

            foreach (var childId in EnumerateCharacterChildIds(record))
            {
                if (childId.Equals(parentId, StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                if (!doc.ById.ContainsKey(childId) || claimed.Contains(childId))
                {
                    continue;
                }

                parent.Children.Add(childId);
                claimed.Add(childId);
            }
        }

        doc.Roots = orderedIds
            .Where(id => !claimed.Contains(id))
            .OrderBy(id => RootKindRank(doc.ById[id].Kind))
            .ThenBy(id => id, StringComparer.OrdinalIgnoreCase)
            .ToList();

        File.WriteAllText(full, JsonSerializer.Serialize(doc, JsonOptions));
    }

    private static int RootKindRank(string kind)
    {
        for (var i = 0; i < PreferredRootKinds.Length; i++)
        {
            if (PreferredRootKinds[i].Equals(kind, StringComparison.OrdinalIgnoreCase))
            {
                return i;
            }
        }

        return PreferredRootKinds.Length;
    }

    private static IEnumerable<string> EnumerateCharacterChildIds(JsonElement record)
    {
        foreach (var text in EnumerateStrings(record))
        {
            if (TryParseCharacterById(text, out var id))
            {
                yield return id;
            }
        }
    }

    internal static bool TryParseCharacterById(string? uri, out string id)
    {
        id = "";
        if (string.IsNullOrWhiteSpace(uri))
        {
            return false;
        }

        var hash = uri.IndexOf('#');
        if (hash < 0)
        {
            return false;
        }

        var docPath = uri[..hash].Replace('\\', '/');
        if (docPath.Length > 0 &&
            !docPath.EndsWith(CharacterPoolDocument.RelativePath, StringComparison.OrdinalIgnoreCase) &&
            !docPath.Equals(CharacterPoolDocument.RelativePath, StringComparison.OrdinalIgnoreCase))
        {
            // Also accept bare fragment docs that still carry the path prefix somewhere.
            if (docPath.IndexOf("persos.json", StringComparison.OrdinalIgnoreCase) < 0)
            {
                return false;
            }
        }

        var pointer = uri[(hash + 1)..].TrimStart('/');
        if (!pointer.StartsWith("byId/", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        var rest = pointer["byId/".Length..];
        var slash = rest.IndexOf('/');
        id = slash >= 0 ? rest[..slash] : rest;
        return id.Length > 0;
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
}
