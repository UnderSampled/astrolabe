using System.Text.Json;
using Astrolabe.Core.FileFormats.Semantic;

namespace Astrolabe.Core.Rete;

/// <summary>
/// Geometry/materials dual-layer pool: meshes + materials in geometry/meshes.json;
/// dense arrays as geometry/buffers/*.bin descriptors.
/// </summary>
internal static class GeometryDomainAggregator
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
            "geometry",
            GeometryPoolDocument.RelativePath,
            GeometryPoolDocument.SchemaValue,
            SemanticDomainKinds.Geometry,
            denseBuffers: true);

        // Optional authoring graph: geometricobject → elements / buffers via rewritten URIs.
        LinkMeshOwnership(packageDir);
    }

    /// <summary>
    /// When pointer fields in records already point at <c>geometry/meshes.json#/byId/…</c>,
    /// populate <see cref="SemanticPoolNode.Children"/> for nested mesh ownership.
    /// Stream order remains in <c>runs</c> / content expand — Children are authoring only.
    /// </summary>
    private static void LinkMeshOwnership(string packageDir)
    {
        var poolPath = Path.Combine(
            packageDir,
            GeometryPoolDocument.RelativePath.Replace('/', Path.DirectorySeparatorChar));
        if (!File.Exists(poolPath))
        {
            return;
        }

        GeometryPoolDocument? pool;
        try
        {
            pool = JsonSerializer.Deserialize<GeometryPoolDocument>(
                File.ReadAllText(poolPath), JsonOptions);
        }
        catch
        {
            return;
        }

        if (pool?.ById is not { Count: > 0 })
        {
            return;
        }

        var claimed = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var changed = false;

        // Prefer owners that form a mesh tree: geometricobject first, then element* headers.
        foreach (var ownerKind in new[]
                 {
                     "geometricobject",
                     "elementtriangles",
                     "elementsprites",
                     "visualset",
                     "physicalobject",
                     "ipo",
                     "gamematerial",
                     "visualmaterial"
                 })
        {
            foreach (var (id, node) in pool.ById)
            {
                if (!node.Kind.Equals(ownerKind, StringComparison.OrdinalIgnoreCase) ||
                    node.Record is not { } record)
                {
                    continue;
                }

                foreach (var targetId in EnumerateGeometryByIdRefs(record))
                {
                    if (claimed.Contains(targetId) ||
                        !pool.ById.ContainsKey(targetId) ||
                        string.Equals(targetId, id, StringComparison.OrdinalIgnoreCase))
                    {
                        continue;
                    }

                    node.Children.Add(targetId);
                    claimed.Add(targetId);
                    changed = true;
                }
            }
        }

        if (changed)
        {
            File.WriteAllText(poolPath, JsonSerializer.Serialize(pool, JsonOptions));
        }
    }

    private static IEnumerable<string> EnumerateGeometryByIdRefs(JsonElement record)
    {
        if (record.ValueKind != JsonValueKind.Object && record.ValueKind != JsonValueKind.Array)
        {
            yield break;
        }

        var stack = new Stack<JsonElement>();
        stack.Push(record);
        while (stack.Count > 0)
        {
            var cur = stack.Pop();
            switch (cur.ValueKind)
            {
                case JsonValueKind.Object:
                    foreach (var prop in cur.EnumerateObject())
                    {
                        stack.Push(prop.Value);
                    }

                    break;
                case JsonValueKind.Array:
                    foreach (var item in cur.EnumerateArray())
                    {
                        stack.Push(item);
                    }

                    break;
                case JsonValueKind.String:
                    var s = cur.GetString();
                    if (string.IsNullOrEmpty(s))
                    {
                        break;
                    }

                    // geometry/meshes.json#/byId/{id} or bare fragment after rewrite.
                    const string marker = "geometry/meshes.json#/byId/";
                    var idx = s.IndexOf(marker, StringComparison.OrdinalIgnoreCase);
                    if (idx >= 0)
                    {
                        var id = s[(idx + marker.Length)..];
                        var slash = id.IndexOf('/');
                        if (slash >= 0)
                        {
                            id = id[..slash];
                        }

                        if (id.Length > 0)
                        {
                            yield return id;
                        }
                    }

                    break;
            }
        }
    }
}
