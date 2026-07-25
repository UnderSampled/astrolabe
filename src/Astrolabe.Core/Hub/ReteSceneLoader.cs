using System.Numerics;
using System.Text.Json;
using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Semantic;
using Astrolabe.Core.Rete;
using Astrolabe.Core.Serialization;
using Astrolabe.Core.Serialization.Codecs;

namespace Astrolabe.Core.Hub;

internal static class ReteSceneLoader
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    public static SceneGraph Load(string packageDir, HubCatalog catalog)
    {
        var treePath = Path.Combine(packageDir, SceneTreeDocument.RelativePath.Replace('/', Path.DirectorySeparatorChar));
        if (File.Exists(treePath))
        {
            return LoadFromTreeDocument(packageDir, catalog, treePath);
        }

        // Legacy folder forest (pre-semantic collapse).
        var graph = new SceneGraph();
        graph.ActualWorld = LoadRootFolder(packageDir, catalog, graph, "scene/actual_world");
        graph.DynamicWorld = LoadRootFolder(packageDir, catalog, graph, "scene/dynamic_world");
        graph.FatherSector = LoadRootFolder(packageDir, catalog, graph, "scene/father_sector");
        return graph;
    }

    private static SceneGraph LoadFromTreeDocument(string packageDir, HubCatalog catalog, string treePath)
    {
        var doc = JsonSerializer.Deserialize<SceneTreeDocument>(File.ReadAllText(treePath), JsonOptions)
                  ?? throw new InvalidDataException($"Could not read {treePath}");

        var graph = new SceneGraph();
        graph.ActualWorld = LoadTreeRoot(packageDir, catalog, graph, doc, "actual_world");
        graph.DynamicWorld = LoadTreeRoot(packageDir, catalog, graph, doc, "dynamic_world");
        graph.FatherSector = LoadTreeRoot(packageDir, catalog, graph, doc, "father_sector");
        return graph;
    }

    private static SceneNode? LoadTreeRoot(
        string packageDir,
        HubCatalog catalog,
        SceneGraph graph,
        SceneTreeDocument doc,
        string rootName)
    {
        if (!doc.Roots.TryGetValue(rootName, out var rootId) || string.IsNullOrWhiteSpace(rootId))
        {
            return null;
        }

        return LoadTreeNode(packageDir, catalog, graph, doc, rootId!, null);
    }

    private static SceneNode? LoadTreeNode(
        string packageDir,
        HubCatalog catalog,
        SceneGraph graph,
        SceneTreeDocument doc,
        string id,
        SceneNode? parent)
    {
        if (!doc.ById.TryGetValue(id, out var poolNode) || poolNode.Record is not { } json)
        {
            return null;
        }

        var record = SuperObjectCodec.Instance.FromJson(json);
        var node = new SceneNode
        {
            Address = poolNode.ProvenanceVirtualAddress ?? 0,
            Parent = parent,
            TypeCode = record.TypeCode,
            Type = ParseSuperObjectType(record.TypeCode),
            Name = json.TryGetProperty("name", out var nameElement) ? nameElement.GetString() : null,
            OffData = HubReferenceIO.Materialize(record.OffData),
            OffMatrix = HubReferenceIO.Materialize(record.Matrix),
            OffStaticMatrix = HubReferenceIO.Materialize(record.StaticMatrix),
            OffBoundingVolume = HubReferenceIO.Materialize(record.BoundingVolume),
            DrawFlags = record.DrawFlags,
            Flags = record.Flags,
            GeometricObjectAddress = json.TryGetProperty("geometricObjectAddress", out var geoElement) &&
                                     geoElement.TryGetInt32(out var geoAddress)
                ? geoAddress
                : catalog.ResolveVirtualAddress(record.OffData)
        };

        if (poolNode.Matrix is { } matrixJson)
        {
            node.Transform = TryLoadMatrixFromJson(matrixJson);
        }

        graph.AllNodes.Add(node);

        foreach (var childId in EnumerateChildIds(poolNode, json))
        {
            var child = LoadTreeNode(packageDir, catalog, graph, doc, childId, node);
            if (child != null)
            {
                node.Children.Add(child);
            }
        }

        return node;
    }

    /// <summary>
    /// Prefer SemanticPoolNode.Children (bare ids); fall back to record.children URI refs
    /// (<c>scene/tree.json#/byId/{id}</c>) written by SceneTreeAggregator.
    /// </summary>
    private static IEnumerable<string> EnumerateChildIds(SemanticPoolNode poolNode, JsonElement record)
    {
        if (poolNode.Children.Count > 0)
        {
            return poolNode.Children;
        }

        if (!record.TryGetProperty("children", out var children) ||
            children.ValueKind != JsonValueKind.Array)
        {
            return [];
        }

        var ids = new List<string>();
        foreach (var child in children.EnumerateArray())
        {
            if (child.ValueKind != JsonValueKind.String)
            {
                continue;
            }

            var uri = child.GetString();
            if (string.IsNullOrWhiteSpace(uri))
            {
                continue;
            }

            // Bare id or URI with #/byId/{id}
            var hash = uri.IndexOf('#');
            var pointer = hash >= 0 ? uri[(hash + 1)..] : uri;
            if (SemanticPoolPaths.TryParseByIdField(pointer, out var id, out _) ||
                (!uri.Contains('/') && !uri.Contains('#')))
            {
                if (hash < 0 && !uri.Contains('/'))
                {
                    ids.Add(uri);
                }
                else if (id.Length > 0)
                {
                    ids.Add(id);
                }
            }
        }

        return ids;
    }

    private static SceneNode? LoadRootFolder(
        string packageDir,
        HubCatalog catalog,
        SceneGraph graph,
        string rootDir)
    {
        var rootPath = Path.Combine(packageDir, rootDir);
        if (!Directory.Exists(rootPath))
        {
            return null;
        }

        var nodeJson = Directory.EnumerateFiles(rootPath, "node.json", SearchOption.AllDirectories)
            .OrderBy(path => path.Count(c => c == Path.DirectorySeparatorChar))
            .FirstOrDefault();
        if (nodeJson == null)
        {
            return null;
        }

        return LoadNodeFolder(packageDir, catalog, graph, nodeJson, null);
    }

    private static SceneNode? LoadNodeFolder(
        string packageDir,
        HubCatalog catalog,
        SceneGraph graph,
        string nodeJsonPath,
        SceneNode? parent)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(nodeJsonPath));
        var json = document.RootElement;
        var record = SuperObjectCodec.Instance.FromJson(json);

        var node = new SceneNode
        {
            Address = ParseAddressFromPath(nodeJsonPath),
            Parent = parent,
            TypeCode = record.TypeCode,
            Type = ParseSuperObjectType(record.TypeCode),
            Name = json.TryGetProperty("name", out var nameElement) ? nameElement.GetString() : null,
            OffData = HubReferenceIO.Materialize(record.OffData),
            OffMatrix = HubReferenceIO.Materialize(record.Matrix),
            OffStaticMatrix = HubReferenceIO.Materialize(record.StaticMatrix),
            OffBoundingVolume = HubReferenceIO.Materialize(record.BoundingVolume),
            DrawFlags = record.DrawFlags,
            Flags = record.Flags,
            GeometricObjectAddress = json.TryGetProperty("geometricObjectAddress", out var geoElement) &&
                                     geoElement.TryGetInt32(out var geoAddress)
                ? geoAddress
                : catalog.ResolveVirtualAddress(record.OffData)
        };

        if (json.TryGetProperty("matrixPath", out var matrixPathElement) &&
            matrixPathElement.ValueKind == JsonValueKind.String)
        {
            node.Transform = TryLoadMatrix(packageDir, matrixPathElement.GetString());
        }

        graph.AllNodes.Add(node);

        if (json.TryGetProperty("children", out var childrenElement) &&
            childrenElement.ValueKind == JsonValueKind.Array)
        {
            foreach (var childPathElement in childrenElement.EnumerateArray())
            {
                if (childPathElement.ValueKind != JsonValueKind.String)
                {
                    continue;
                }

                var childPath = childPathElement.GetString();
                if (string.IsNullOrWhiteSpace(childPath))
                {
                    continue;
                }

                var childFullPath = Path.Combine(packageDir, childPath.Replace('/', Path.DirectorySeparatorChar));
                if (!File.Exists(childFullPath))
                {
                    continue;
                }

                var child = LoadNodeFolder(packageDir, catalog, graph, childFullPath, node);
                if (child != null)
                {
                    node.Children.Add(child);
                }
            }
        }

        return node;
    }

    private static Matrix4x4? TryLoadMatrixFromJson(JsonElement json)
    {
        var matrix = MatrixCodec.Instance.FromJson(json);
        if (matrix.BasisX.Length < 3 || matrix.BasisY.Length < 3 || matrix.BasisZ.Length < 3 ||
            matrix.Translation.Length < 3)
        {
            return null;
        }

        return new Matrix4x4(
            matrix.BasisX[0], matrix.BasisX[1], matrix.BasisX[2], 0,
            matrix.BasisY[0], matrix.BasisY[1], matrix.BasisY[2], 0,
            matrix.BasisZ[0], matrix.BasisZ[1], matrix.BasisZ[2], 0,
            matrix.Translation[0], matrix.Translation[1], matrix.Translation[2], 1);
    }

    private static Matrix4x4? TryLoadMatrix(string packageDir, string? relativePath)
    {
        if (string.IsNullOrWhiteSpace(relativePath))
        {
            return null;
        }

        var matrixPath = Path.Combine(packageDir, relativePath.Replace('/', Path.DirectorySeparatorChar));
        if (!File.Exists(matrixPath))
        {
            return null;
        }

        using var document = JsonDocument.Parse(File.ReadAllText(matrixPath));
        return TryLoadMatrixFromJson(document.RootElement);
    }

    private static int ParseAddressFromPath(string nodeJsonPath)
    {
        var fileName = Path.GetFileName(Path.GetDirectoryName(nodeJsonPath)) ?? "";
        var underscore = fileName.LastIndexOf('_');
        if (underscore < 0 || underscore == fileName.Length - 1)
        {
            return 0;
        }

        var hex = fileName[(underscore + 1)..];
        if (uint.TryParse(
                hex,
                System.Globalization.NumberStyles.HexNumber,
                System.Globalization.CultureInfo.InvariantCulture,
                out var uaddr))
        {
            return unchecked((int)uaddr);
        }

        return 0;
    }

    private static SuperObjectType ParseSuperObjectType(uint typeCode) =>
        Enum.TryParse<SuperObjectType>(TrackingSuperObjectReader.GetSuperObjectType(typeCode).ToString(), out var parsed)
            ? parsed
            : SuperObjectType.Unknown;
}
