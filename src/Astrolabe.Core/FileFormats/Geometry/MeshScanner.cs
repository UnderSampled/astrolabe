using System.Numerics;

namespace Astrolabe.Core.FileFormats.Geometry;

/// <summary>
/// Scans SNA data blocks for mesh structures using pointer resolution.
/// Based on raymap's GeometricObject reading approach.
/// </summary>
public class MeshScanner
{
    private readonly MemoryContext _memory;

    public MeshScanner(LevelLoader level)
    {
        _memory = new MemoryContext(level.Sna, level.Rtb);
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

            // Read elements and extract triangles
            var allTriangles = new List<int>();
            for (int i = 0; i < numElements; i++)
            {
                if (elementTypes[i] == 1) // Material/Triangles element
                {
                    // Read element pointer
                    int elemPtrOffset = offElements + (i * 4);
                    var elemReader = _memory.GetReaderAt(elemPtrOffset);
                    if (elemReader == null) continue;

                    int elemAddr = elemReader.ReadInt32();
                    var triangles = ReadElementTriangles(elemAddr, numVertices);
                    if (triangles != null)
                    {
                        allTriangles.AddRange(triangles);
                    }
                }
            }

            var mesh = new MeshData
            {
                Name = $"Mesh_{block.Module:X2}_{block.Id:X2}_{offset:X}",
                Vertices = vertices,
                Normals = normals,
                Indices = allTriangles.Count > 0 ? allTriangles.ToArray() : null,
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

    private int[]? ReadElementTriangles(int address, uint numVertices)
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

            return triangles;
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
/// Represents extracted mesh data.
/// </summary>
public class MeshData
{
    public string Name { get; set; } = "";
    public Vector3[] Vertices { get; set; } = [];
    public Vector3[]? Normals { get; set; }
    public int[]? Indices { get; set; }
    public SnaBlock? SourceBlock { get; set; }
    public int SourceOffset { get; set; }
    public uint NumVertices { get; set; }
    public uint NumElements { get; set; }
}
