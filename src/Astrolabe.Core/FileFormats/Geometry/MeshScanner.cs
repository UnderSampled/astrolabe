using System.Numerics;
using Astrolabe.Core.FileFormats.Materials;

namespace Astrolabe.Core.FileFormats.Geometry;

/// <summary>
/// Scans SNA data blocks for mesh structures using pointer resolution.
/// Based on raymap's GeometricObject reading approach.
/// </summary>
public class MeshScanner
{
    private readonly MemoryContext _memory;
    private readonly TextureTable? _textureTable;
    private readonly GameMaterialReader _gameMaterialReader;

    public MeshScanner(LevelLoader level, TextureTable? textureTable = null)
    {
        _memory = new MemoryContext(level.Sna, level.Rtb);
        _textureTable = textureTable;
        _gameMaterialReader = new GameMaterialReader(_memory);
    }

    /// <summary>
    /// Scans all data blocks for potential mesh data using pointer-guided detection.
    /// </summary>
    public List<MeshData> ScanForMeshes()
    {
        var meshes = new List<MeshData>();

        foreach (var block in _memory.Sna.Blocks.Where(b => b.Data != null && b.Data.Length > 100))
        {
            var blockMeshes = ScanBlock(block);
            meshes.AddRange(blockMeshes);
        }

        return meshes;
    }

    private List<MeshData> ScanBlock(SnaBlock block)
    {
        var meshes = new List<MeshData>();
        var data = block.Data!;
        int baseAddr = block.BaseInMemory;

        // Montreal GeometricObject structure (64 bytes):
        // uint32 num_vertices        +0
        // ptr off_vertices           +4
        // ptr off_normals            +8
        // ptr off_materials          +12
        // int32 unknown              +16
        // uint32 num_elements        +20
        // ptr off_element_types      +24
        // ptr off_elements           +28
        // int32 unknown              +32
        // int32 unknown              +36
        // int32 unknown              +40
        // int32 unknown              +44
        // float sphereRadius         +48
        // float sphereX              +52
        // float sphereZ              +56
        // float sphereY              +60

        for (int offset = 0; offset < data.Length - 64; offset += 4)
        {
            int memAddr = baseAddr + offset;

            using var ms = new MemoryStream(data);
            ms.Position = offset;
            using var reader = new BinaryReader(ms);

            uint numVertices = reader.ReadUInt32();
            if (numVertices < 3 || numVertices > 10000)
                continue;

            int offVerts = reader.ReadInt32();
            int offNormals = reader.ReadInt32();
            int offMaterials = reader.ReadInt32();
            reader.ReadInt32(); // skip
            uint numElements = reader.ReadUInt32();

            if (numElements == 0 || numElements > 1000)
                continue;

            int offElementTypes = reader.ReadInt32();
            int offElements = reader.ReadInt32();

            // Validate pointers - check if they point to valid memory ranges
            // We relax RTB validation since not all pointers are in relocation tables
            var vertBlock = FindBlockContaining(offVerts);
            var elemTypesBlock = FindBlockContaining(offElementTypes);
            var elemsBlock = FindBlockContaining(offElements);

            if (vertBlock == null || elemTypesBlock == null || elemsBlock == null)
                continue;

            // Read vertices
            var vertices = ReadVertices(offVerts, numVertices);
            if (vertices == null)
                continue;

            // Check for meaningful vertex data
            if (!HasMeaningfulSize(vertices))
                continue;

            // Read normals
            var normals = ReadNormals(offNormals, numVertices);

            // Read element types
            var elementTypes = ReadElementTypes(offElementTypes, numElements);
            if (elementTypes == null)
                continue;

            // Read elements and extract triangles/UVs per submesh
            var subMeshes = new List<SubMeshData>();
            var allTriangles = new List<int>();
            var allUVs = new List<Vector2>();
            var allUVIndices = new List<int>();
            string? textureName = null;

            for (int i = 0; i < numElements; i++)
            {
                if (elementTypes[i] == 1) // Material/Triangles element
                {
                    // Read element pointer
                    int elemPtrOffset = offElements + (i * 4);
                    var elemReader = _memory.GetReaderAt(elemPtrOffset);
                    if (elemReader == null) continue;

                    int elemAddr = elemReader.ReadInt32();
                    var element = ReadElementTriangles(elemAddr, numVertices);
                    if (element != null)
                    {
                        // Create submesh for this element
                        var subMesh = new SubMeshData
                        {
                            Triangles = element.Triangles,
                            TextureName = element.TextureName,
                            MaterialFlags = element.MaterialFlags,
                            IsLight = element.IsLight,
                            GameMaterial = element.GameMaterial,
                            VisualMaterial = element.GameMaterial?.VisualMaterial
                        };

                        if (element.UVs != null && element.UVMapping != null)
                        {
                            subMesh.UVs = element.UVs;
                            subMesh.UVIndices = element.UVMapping;
                        }

                        subMeshes.Add(subMesh);

                        // Also accumulate for legacy single-mesh support
                        allTriangles.AddRange(element.Triangles);

                        if (element.UVs != null && element.UVMapping != null)
                        {
                            int uvOffset = allUVs.Count;
                            allUVs.AddRange(element.UVs);

                            foreach (var uvIdx in element.UVMapping)
                            {
                                allUVIndices.Add(uvIdx + uvOffset);
                            }
                        }

                        if (textureName == null && element.TextureName != null)
                        {
                            textureName = element.TextureName;
                        }
                    }
                }
            }

            var mesh = new MeshData
            {
                Name = $"Mesh_{block.Module:X2}_{block.Id:X2}_{offset:X}",
                Vertices = vertices,
                Normals = normals,
                SubMeshes = subMeshes,
                Indices = allTriangles.Count > 0 ? allTriangles.ToArray() : null,
                UVs = allUVs.Count > 0 ? allUVs.ToArray() : null,
                UVIndices = allUVIndices.Count > 0 ? allUVIndices.ToArray() : null,
                TextureName = textureName,
                SourceBlock = block,
                SourceOffset = offset,
                NumVertices = numVertices,
                NumElements = numElements
            };

            meshes.Add(mesh);
            offset += 60; // Skip past this structure
        }

        return meshes;
    }

    private Vector3[]? ReadVertices(int address, uint count)
    {
        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var vertices = new Vector3[count];
            for (int i = 0; i < count; i++)
            {
                float x = reader.ReadSingle();
                float z = reader.ReadSingle();
                float y = reader.ReadSingle();

                if (float.IsNaN(x) || float.IsNaN(y) || float.IsNaN(z) ||
                    float.IsInfinity(x) || float.IsInfinity(y) || float.IsInfinity(z) ||
                    Math.Abs(x) > 100000 || Math.Abs(y) > 100000 || Math.Abs(z) > 100000)
                {
                    return null;
                }

                vertices[i] = new Vector3(x, y, z);
            }
            return vertices;
        }
        catch
        {
            return null;
        }
    }

    private Vector3[]? ReadNormals(int address, uint count)
    {
        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var normals = new Vector3[count];
            for (int i = 0; i < count; i++)
            {
                float x = reader.ReadSingle();
                float z = reader.ReadSingle();
                float y = reader.ReadSingle();

                if (float.IsNaN(x) || float.IsNaN(y) || float.IsNaN(z))
                    return null;

                normals[i] = new Vector3(x, y, z);
            }
            return normals;
        }
        catch
        {
            return null;
        }
    }

    private ushort[]? ReadElementTypes(int address, uint count)
    {
        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var types = new ushort[count];
            for (int i = 0; i < count; i++)
            {
                types[i] = reader.ReadUInt16();
            }
            return types;
        }
        catch
        {
            return null;
        }
    }

    private ElementData? ReadElementTriangles(int address, uint numVertices)
    {
        // Montreal GeometricObjectElementTriangles structure:
        // ptr off_material           +0
        // uint16 num_triangles       +4
        // uint16 num_uvs             +6
        // ptr off_triangles          +8
        // ptr off_mapping_uvs        +12
        // ptr off_normals            +16
        // ptr off_uvs                +20
        // uint32 (skip)              +24
        // ptr off_vertex_indices     +28
        // uint16 num_vertex_indices  +32
        // uint16 parallelBox         +34
        // uint32 (skip)              +36

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            int offMaterial = reader.ReadInt32();
            ushort numTriangles = reader.ReadUInt16();
            ushort numUvs = reader.ReadUInt16();
            int offTriangles = reader.ReadInt32();
            int offMappingUvs = reader.ReadInt32();
            int offNormals = reader.ReadInt32();
            int offUvs = reader.ReadInt32();

            if (numTriangles == 0 || numTriangles > 10000)
                return null;

            // Read triangle indices
            var triReader = _memory.GetReaderAt(offTriangles);
            if (triReader == null) return null;

            var triangles = new int[numTriangles * 3];
            for (int i = 0; i < numTriangles * 3; i++)
            {
                short index = triReader.ReadInt16();
                if (index < 0 || index >= numVertices)
                    return null; // Invalid index
                triangles[i] = index;
            }

            var element = new ElementData { Triangles = triangles };

            // Read UVs
            if (numUvs > 0 && offUvs != 0)
            {
                var uvReader = _memory.GetReaderAt(offUvs);
                if (uvReader != null)
                {
                    element.UVs = new Vector2[numUvs];
                    for (int i = 0; i < numUvs; i++)
                    {
                        float u = uvReader.ReadSingle();
                        float v = uvReader.ReadSingle();
                        // Flip V coordinate because we flip textures vertically on export
                        // (game stores textures upside-down for GPU, we export right-side up)
                        element.UVs[i] = new Vector2(u, 1.0f - v);
                    }
                }
            }

            // Read UV mapping (maps each triangle vertex to a UV index)
            if (offMappingUvs != 0 && numTriangles > 0)
            {
                var uvMapReader = _memory.GetReaderAt(offMappingUvs);
                if (uvMapReader != null)
                {
                    element.UVMapping = new int[numTriangles * 3];
                    for (int i = 0; i < numTriangles * 3; i++)
                    {
                        element.UVMapping[i] = uvMapReader.ReadInt16();
                    }
                }
            }

            // Read material using the full material reader
            var gameMaterial = _gameMaterialReader.Read(offMaterial);
            element.GameMaterial = gameMaterial;

            if (gameMaterial?.VisualMaterial != null)
            {
                element.MaterialFlags = gameMaterial.VisualMaterial.Flags;

                // Try to get texture entry from texture table (includes name and flags)
                if (_textureTable != null && gameMaterial.VisualMaterial.OffTexture != 0)
                {
                    var textureEntry = _textureTable.GetTextureEntry(gameMaterial.VisualMaterial.OffTexture);
                    if (textureEntry != null)
                    {
                        element.TextureName = textureEntry.Name;
                        element.IsLight = textureEntry.IsLight;
                    }
                }
            }

            return element;
        }
        catch
        {
            return null;
        }
    }

    private SnaBlock? FindBlockContaining(int address)
    {
        foreach (var block in _memory.Sna.Blocks)
        {
            if (block.Data == null) continue;
            int endAddr = block.BaseInMemory + block.Data.Length;
            if (address >= block.BaseInMemory && address < endAddr)
                return block;
        }
        return null;
    }

    private static bool HasMeaningfulSize(Vector3[] vertices)
    {
        if (vertices.Length < 3) return false;

        float minX = float.MaxValue, maxX = float.MinValue;
        float minY = float.MaxValue, maxY = float.MinValue;
        float minZ = float.MaxValue, maxZ = float.MinValue;

        foreach (var v in vertices)
        {
            minX = Math.Min(minX, v.X); maxX = Math.Max(maxX, v.X);
            minY = Math.Min(minY, v.Y); maxY = Math.Max(maxY, v.Y);
            minZ = Math.Min(minZ, v.Z); maxZ = Math.Max(maxZ, v.Z);
        }

        float sizeX = maxX - minX;
        float sizeY = maxY - minY;
        float sizeZ = maxZ - minZ;

        // At least some dimension should be meaningful
        return sizeX > 0.01f || sizeY > 0.01f || sizeZ > 0.01f;
    }
}

/// <summary>
/// Represents extracted mesh data with multiple submeshes.
/// </summary>
public class MeshData
{
    public string Name { get; set; } = "";
    public Vector3[] Vertices { get; set; } = [];
    public Vector3[]? Normals { get; set; }

    /// <summary>
    /// Per-vertex colors (from radiosity/pre-baked lighting).
    /// </summary>
    public Vector4[]? VertexColors { get; set; }

    /// <summary>
    /// Submeshes, each with their own triangles, UVs, and texture.
    /// </summary>
    public List<SubMeshData> SubMeshes { get; set; } = [];

    // Legacy properties for backward compatibility
    public int[]? Indices { get; set; }
    public Vector2[]? UVs { get; set; }
    public int[]? UVIndices { get; set; }
    public string? TextureName { get; set; }
    public SnaBlock? SourceBlock { get; set; }
    public int SourceOffset { get; set; }
    public uint NumVertices { get; set; }
    public uint NumElements { get; set; }
}

/// <summary>
/// A submesh with its own triangles, UVs, and texture.
/// </summary>
public class SubMeshData
{
    public int[] Triangles { get; set; } = [];
    public Vector2[] UVs { get; set; } = [];
    public int[] UVIndices { get; set; } = [];
    public string? TextureName { get; set; }
    public uint MaterialFlags { get; set; }

    /// <summary>
    /// True if this submesh should use additive/emissive blending (IsLight flag in texture).
    /// </summary>
    public bool IsLight { get; set; }

    /// <summary>
    /// Full visual material information.
    /// </summary>
    public VisualMaterial? VisualMaterial { get; set; }

    /// <summary>
    /// Full game material information.
    /// </summary>
    public GameMaterial? GameMaterial { get; set; }
}

/// <summary>
/// Data from a single element (submesh).
/// </summary>
public class ElementData
{
    public int[] Triangles { get; set; } = [];
    public Vector2[]? UVs { get; set; }
    public int[]? UVMapping { get; set; }
    public string? TextureName { get; set; }
    public uint MaterialFlags { get; set; }
    public bool IsLight { get; set; }
    public GameMaterial? GameMaterial { get; set; }
}

