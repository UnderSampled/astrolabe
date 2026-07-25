using System.Text.Json;
using Astrolabe.Core.FileFormats.Semantic;

namespace Astrolabe.Core.Rete;

/// <summary>
/// Sector / collision dual-layer pool.
/// Documented leaves → <c>sectors/sectors.json</c> byId + expand runs;
/// optional authoring Children: sector → collide geo/name, collideset → zone lists → zones
/// via pointer URIs rewritten into the pool.
/// </summary>
internal static class SectorDomainAggregator
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = false,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    public static void Aggregate(string packageDir, RetePackageManifest manifest)
    {
        SemanticDomainAggregator.AggregateDomain(
            packageDir,
            manifest,
            "sector",
            SectorPoolDocument.RelativePath,
            SectorPoolDocument.SchemaValue,
            SemanticDomainKinds.Sector);

        AttachOptionalNesting(packageDir);
    }

    /// <summary>
    /// Authoring-graph pass: claim later-in-stream pool targets referenced by pointer URIs
    /// so sector → collideset/geo/name and collideset → zone lists/zones nest in Children.
    /// Stream order (byId expand runs) is unchanged.
    /// </summary>
    private static void AttachOptionalNesting(string packageDir)
    {
        var path = Path.Combine(
            packageDir,
            SectorPoolDocument.RelativePath.Replace('/', Path.DirectorySeparatorChar));
        if (!File.Exists(path))
        {
            return;
        }

        var doc = JsonSerializer.Deserialize<SectorPoolDocument>(File.ReadAllText(path), JsonOptions);
        if (doc is null || doc.ById.Count == 0)
        {
            return;
        }

        // Insertion / id counter order is stream order from AggregateDomain.
        var orderedIds = doc.ById.Keys.ToList();
        var indexById = new Dictionary<string, int>(StringComparer.OrdinalIgnoreCase);
        for (var i = 0; i < orderedIds.Count; i++)
        {
            indexById[orderedIds[i]] = i;
        }

        var claimed = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var changed = false;

        foreach (var id in orderedIds)
        {
            if (!doc.ById.TryGetValue(id, out var parent) ||
                parent.Record is not { } record ||
                !indexById.TryGetValue(id, out var parentIndex))
            {
                continue;
            }

            foreach (var targetId in EnumerateReferencedPoolIds(record))
            {
                if (claimed.Contains(targetId) ||
                    !indexById.TryGetValue(targetId, out var childIndex) ||
                    childIndex <= parentIndex)
                {
                    continue;
                }

                // Prefer ownership edges that match sector → collideset → zones topology,
                // but still accept any later domain leaf (geo, name, verts, ptrs).
                if (!IsAllowedNestingEdge(parent.Kind, doc.ById[targetId].Kind))
                {
                    continue;
                }

                if (!parent.Children.Contains(targetId, StringComparer.OrdinalIgnoreCase))
                {
                    parent.Children.Add(targetId);
                    changed = true;
                }

                claimed.Add(targetId);
            }
        }

        if (changed)
        {
            File.WriteAllText(path, JsonSerializer.Serialize(doc, JsonOptions));
        }
    }

    /// <summary>
    /// Restrict authoring Children to the documented ownership shape:
    /// sector → name/geo/collideset; collideset → zone lists; zone list → zones;
    /// sectorcollidegeo → verts/ptrs. Other cross-links stay as pointer URIs only.
    /// </summary>
    private static bool IsAllowedNestingEdge(string parentKind, string childKind)
    {
        var p = parentKind.ToLowerInvariant();
        var c = childKind.ToLowerInvariant();

        return p switch
        {
            "sector" => c is "sectorname" or "sectorcollidegeo" or "collideset" or "collideelementptrs",
            "collideset" => c is "collidezdxlist" or "collidezddlist" or "collidezdelist",
            "collidezdxlist" => c is "collidezdxzone",
            "collidezddlist" => c is "collidezddzone",
            "collidezdelist" => c is "collidezdezone",
            "collidezdxzone" or "collidezddzone" or "collidezdezone" =>
                c is "sectorcollidegeo" or "collideelementptrs" or "sectorcollideverts",
            "sectorcollidegeo" => c is "sectorcollideverts" or "collideelementptrs",
            _ => false
        };
    }

    private static IEnumerable<string> EnumerateReferencedPoolIds(JsonElement record)
    {
        foreach (var text in EnumerateStrings(record))
        {
            var hash = text.IndexOf('#');
            if (hash < 0)
            {
                continue;
            }

            // Pointer URIs rewritten by AggregateDomain: sectors/sectors.json#/byId/{id}
            if (text.AsSpan(0, hash).IndexOf("sectors.json", StringComparison.OrdinalIgnoreCase) < 0)
            {
                continue;
            }

            if (SemanticPoolPaths.TryParseById(text[(hash + 1)..], out var id))
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
                var value = element.GetString();
                if (!string.IsNullOrEmpty(value))
                {
                    yield return value;
                }

                break;
        }
    }
}
