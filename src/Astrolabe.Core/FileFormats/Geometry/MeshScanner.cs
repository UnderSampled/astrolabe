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

        // Scan for potential GeometricObject structures
        // Montreal format:
        // uint32 num_vertices
        // ptr off_vertices
        // ptr off_normals
        // ptr off_materials
        // int32 unknown
        // uint32 num_elements
        // ptr off_element_types
        // ptr off_elements
        // ...

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

            // Validate this looks like a GeometricObject by checking pointer locations
            int vertPtrAddr = memAddr + 4;
            int normPtrAddr = memAddr + 8;

            // Check if these are known pointer locations (strong validation)
            var vertPtr = _memory.GetPointerAt(vertPtrAddr);
            var normPtr = _memory.GetPointerAt(normPtrAddr);

            // For pointer-validated meshes
            if (vertPtr != null && normPtr != null)
            {
                var mesh = TryReadMesh(vertPtr.RawValue, numVertices, normPtr.RawValue, numElements, block, offset);
                if (mesh != null)
                {
                    meshes.Add(mesh);
                    offset += 60; // Skip past this structure
                }
                continue;
            }

            // Fallback: check if pointers point within any valid block
            var vertBlock = FindBlockContaining(offVerts);
            var normBlock = FindBlockContaining(offNormals);

            if (vertBlock != null && normBlock != null)
            {
                var mesh = TryReadMesh(offVerts, numVertices, offNormals, numElements, block, offset);
                if (mesh != null)
                {
                    meshes.Add(mesh);
                    offset += 60;
                }
            }
        }

        return meshes;
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

    private MeshData? TryReadMesh(int vertAddr, uint numVertices, int normAddr, uint numElements, SnaBlock sourceBlock, int sourceOffset)
    {
        // Read vertices
        var vertReader = _memory.GetReaderAt(vertAddr);
        if (vertReader == null) return null;

        var vertices = new Vector3[numVertices];
        float minX = float.MaxValue, maxX = float.MinValue;
        float minY = float.MaxValue, maxY = float.MinValue;
        float minZ = float.MaxValue, maxZ = float.MinValue;

        try
        {
            for (int i = 0; i < numVertices; i++)
            {
                float x = vertReader.ReadSingle();
                float z = vertReader.ReadSingle();
                float y = vertReader.ReadSingle();

                if (float.IsNaN(x) || float.IsNaN(y) || float.IsNaN(z) ||
                    float.IsInfinity(x) || float.IsInfinity(y) || float.IsInfinity(z) ||
                    Math.Abs(x) > 100000 || Math.Abs(y) > 100000 || Math.Abs(z) > 100000)
                {
                    return null;
                }

                vertices[i] = new Vector3(x, y, z);
                minX = Math.Min(minX, x); maxX = Math.Max(maxX, x);
                minY = Math.Min(minY, y); maxY = Math.Max(maxY, y);
                minZ = Math.Min(minZ, z); maxZ = Math.Max(maxZ, z);
            }
        }
        catch
        {
            return null;
        }

        // Check for meaningful size (not all zeros or trivial)
        float sizeX = maxX - minX;
        float sizeY = maxY - minY;
        float sizeZ = maxZ - minZ;

        if (sizeX < 0.01f && sizeY < 0.01f && sizeZ < 0.01f)
            return null;

        var mesh = new MeshData
        {
            Name = $"Mesh_{sourceBlock.Module:X2}_{sourceBlock.Id:X2}_{sourceOffset:X}",
            Vertices = vertices,
            SourceBlock = sourceBlock,
            SourceOffset = sourceOffset,
            NumVertices = numVertices,
            NumElements = numElements
        };

        // Try to read normals
        var normReader = _memory.GetReaderAt(normAddr);
        if (normReader != null)
        {
            try
            {
                var normals = new Vector3[numVertices];
                bool normValid = true;

                for (int i = 0; i < numVertices && normValid; i++)
                {
                    float x = normReader.ReadSingle();
                    float z = normReader.ReadSingle();
                    float y = normReader.ReadSingle();

                    if (float.IsNaN(x) || float.IsNaN(y) || float.IsNaN(z))
                    {
                        normValid = false;
                        break;
                    }

                    normals[i] = new Vector3(x, y, z);
                }

                if (normValid)
                    mesh.Normals = normals;
            }
            catch { /* Ignore normal read failures */ }
        }

        return mesh;
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
