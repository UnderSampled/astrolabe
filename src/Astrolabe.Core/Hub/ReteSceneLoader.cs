using System.Numerics;
using System.Text.Json;
using Astrolabe.Core.FileFormats;
using Astrolabe.Core.Rete;
using Astrolabe.Core.Serialization;
using Astrolabe.Core.Serialization.Codecs;

namespace Astrolabe.Core.Hub;

internal static class ReteSceneLoader
{
    public static SceneGraph Load(string packageDir, HubCatalog catalog)
    {
        var graph = new SceneGraph();
        graph.ActualWorld = LoadRoot(packageDir, catalog, graph, "scene/actual_world");
        graph.DynamicWorld = LoadRoot(packageDir, catalog, graph, "scene/dynamic_world");
        graph.FatherSector = LoadRoot(packageDir, catalog, graph, "scene/father_sector");
        return graph;
    }

    private static SceneNode? LoadRoot(
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

        return LoadNode(packageDir, catalog, graph, nodeJson, null);
    }

    private static SceneNode? LoadNode(
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
                : ResolveGeometryAddress(catalog, record.OffData)
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

                var child = LoadNode(packageDir, catalog, graph, childFullPath, node);
                if (child != null)
                {
                    node.Children.Add(child);
                }
            }
        }

        return node;
    }

    private static int ResolveGeometryAddress(HubCatalog catalog, HubReference offData) =>
        catalog.ResolveVirtualAddress(offData);

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
        var matrix = MatrixCodec.Instance.FromJson(document.RootElement);
        if (matrix.BasisX.Length < 3 || matrix.BasisY.Length < 3 || matrix.BasisZ.Length < 3 || matrix.Translation.Length < 3)
        {
            return null;
        }

        return new Matrix4x4(
            matrix.BasisX[0], matrix.BasisX[1], matrix.BasisX[2], 0,
            matrix.BasisY[0], matrix.BasisY[1], matrix.BasisY[2], 0,
            matrix.BasisZ[0], matrix.BasisZ[1], matrix.BasisZ[2], 0,
            matrix.Translation[0], matrix.Translation[1], matrix.Translation[2], 1);
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
        return int.TryParse(hex, System.Globalization.NumberStyles.HexNumber, null, out var address)
            ? address
            : 0;
    }

    private static SuperObjectType ParseSuperObjectType(uint typeCode) =>
        Enum.TryParse<SuperObjectType>(TrackingSuperObjectReader.GetSuperObjectType(typeCode).ToString(), out var parsed)
            ? parsed
            : SuperObjectType.Unknown;
}