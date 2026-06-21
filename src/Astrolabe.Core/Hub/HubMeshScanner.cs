using System.Numerics;
using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Geometry;
using Astrolabe.Core.FileFormats.Materials;

namespace Astrolabe.Core.Hub;

public sealed class HubMeshScanner
{
    private const ushort TriangleElementType = 1;

    private readonly HubCatalog _catalog;
    private readonly TextureTable? _textureTable;

    public HubMeshScanner(HubCatalog catalog, TextureTable? textureTable = null)
    {
        _catalog = catalog;
        _textureTable = textureTable;
    }

    public List<MeshData> ScanForMeshes()
    {
        var meshes = new List<MeshData>();

        foreach (var element in _catalog.GetElementsOfKind("geometricobject"))
        {
            if (!_catalog.TryHydrate(element) ||
                element.Value is not GeometricObjectRecord geo ||
                geo.NumVertices < 3)
            {
                continue;
            }

            var mesh = TryBuildMesh(geo, element.VirtualAddress);
            if (mesh != null)
            {
                meshes.Add(mesh);
            }
        }

        return meshes;
    }

    private MeshData? TryBuildMesh(GeometricObjectRecord geo, int virtualAddress)
    {
        var vertices = LoadFloat3Array(_catalog.Resolve<Float3ArrayRecord>(geo.Vertices));
        if (vertices == null || vertices.Length < 3)
        {
            return null;
        }

        var normals = LoadFloat3Array(_catalog.Resolve<Float3ArrayRecord>(geo.Normals));
        var elementTypes = LoadUInt16Array(_catalog.Resolve<UInt16ArrayRecord>(geo.ElementTypes));
        var elementPtrs = _catalog.Resolve<PointerArrayRecord>(geo.Elements);
        if (elementTypes == null || elementPtrs == null)
        {
            return null;
        }

        var mesh = new MeshData
        {
            Name = $"geo_{virtualAddress:X8}",
            VirtualAddress = virtualAddress,
            Vertices = vertices,
            Normals = normals,
            NumVertices = geo.NumVertices,
            NumElements = geo.NumElements
        };

        var subMeshes = new List<SubMeshData>();
        var allTriangles = new List<int>();
        var allUVs = new List<Vector2>();
        var allUVIndices = new List<int>();

        for (var i = 0; i < Math.Min(elementTypes.Length, elementPtrs.Values.Length); i++)
        {
            if (elementTypes[i] != TriangleElementType)
            {
                continue;
            }

            var trianglesRecord = _catalog.Resolve<ElementTrianglesRecord>(elementPtrs.Values[i]);
            if (trianglesRecord == null)
            {
                continue;
            }

            var elementData = BuildElementData(trianglesRecord);
            if (elementData == null)
            {
                continue;
            }

            var indexOffset = allTriangles.Count;
            allTriangles.AddRange(elementData.Triangles.Select(index => index + indexOffset));
            if (elementData.UVs != null)
            {
                var uvBase = allUVs.Count;
                allUVs.AddRange(elementData.UVs);
                if (elementData.UVMapping != null)
                {
                    allUVIndices.AddRange(elementData.UVMapping.Select(uv => uv + uvBase));
                }
            }

            subMeshes.Add(new SubMeshData
            {
                Triangles = elementData.Triangles,
                UVs = elementData.UVs ?? [],
                UVIndices = elementData.UVMapping ?? [],
                TextureName = elementData.TextureName,
                MaterialFlags = elementData.MaterialFlags,
                IsLight = elementData.IsLight,
                GameMaterial = elementData.GameMaterial
            });

            mesh.TextureName ??= elementData.TextureName;
        }

        if (subMeshes.Count == 0)
        {
            return null;
        }

        mesh.SubMeshes = subMeshes;
        mesh.Indices = allTriangles.ToArray();
        mesh.UVs = allUVs.Count > 0 ? allUVs.ToArray() : null;
        mesh.UVIndices = allUVIndices.Count > 0 ? allUVIndices.ToArray() : null;
        return mesh;
    }

    private ElementData? BuildElementData(ElementTrianglesRecord record)
    {
        var triangles = LoadUInt16Array(_catalog.Resolve<UInt16ArrayRecord>(record.Triangles));
        if (triangles == null || triangles.Length < 3)
        {
            return null;
        }

        var element = new ElementData
        {
            Triangles = triangles.Select(value => (int)value).ToArray(),
            UVs = LoadFloat2Array(_catalog.Resolve<Float2ArrayRecord>(record.Uvs)),
            UVMapping = LoadUInt16Array(_catalog.Resolve<UInt16ArrayRecord>(record.MappingUvs))?.Select(value => (int)value).ToArray()
        };

        var gameMaterial = _catalog.Resolve<GameMaterialRecord>(record.Material);
        if (gameMaterial != null)
        {
            var visualMaterial = _catalog.Resolve<VisualMaterialRecord>(gameMaterial.VisualMaterial);
            if (visualMaterial != null)
            {
                element.MaterialFlags = visualMaterial.Flags;
                var textureAddress = _catalog.ResolveVirtualAddress(visualMaterial.OffTexture);
                if (_textureTable != null && textureAddress != 0)
                {
                    var textureEntry = _textureTable.GetTextureEntry(textureAddress);
                    if (textureEntry != null)
                    {
                        element.TextureName = textureEntry.Name;
                        element.IsLight = textureEntry.IsLight;
                    }
                }
            }
        }

        return element;
    }

    private static Vector3[]? LoadFloat3Array(Float3ArrayRecord? record) =>
        record?.Values?.Select(values => values.Length >= 3
            ? new Vector3(values[0], values[1], values[2])
            : Vector3.Zero).ToArray();

    private static Vector2[]? LoadFloat2Array(Float2ArrayRecord? record) =>
        record?.Values?.Select(values => values.Length >= 2
            ? new Vector2(values[0], values[1])
            : Vector2.Zero).ToArray();

    private static ushort[]? LoadUInt16Array(UInt16ArrayRecord? record) => record?.Values;
}