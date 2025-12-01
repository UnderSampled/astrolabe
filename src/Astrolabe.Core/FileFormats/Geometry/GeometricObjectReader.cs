using System.Numerics;

namespace Astrolabe.Core.FileFormats.Geometry;

/// <summary>
/// Reads GeometricObject mesh data from Montreal engine format.
/// </summary>
public class GeometricObjectReader
{
    public uint NumVertices { get; private set; }
    public uint NumElements { get; private set; }
    public int OffVertices { get; private set; }
    public int OffNormals { get; private set; }
    public int OffMaterials { get; private set; }
    public int OffElementTypes { get; private set; }
    public int OffElements { get; private set; }
    public float SphereRadius { get; private set; }
    public Vector3 SphereCenter { get; private set; }

    public Vector3[]? Vertices { get; private set; }
    public Vector3[]? Normals { get; private set; }
    public ushort[]? ElementTypes { get; private set; }
    public int[]? ElementOffsets { get; private set; }

    /// <summary>
    /// Reads a GeometricObject at the specified offset in the data.
    /// </summary>
    public static GeometricObjectReader? Read(byte[] data, int offset, int baseAddress = 0)
    {
        if (offset < 0 || offset + 60 > data.Length)
            return null;

        var geo = new GeometricObjectReader();

        using var ms = new MemoryStream(data);
        ms.Position = offset;
        using var reader = new BinaryReader(ms);

        // Montreal format:
        // uint32 num_vertices
        // int32 off_vertices (pointer)
        // int32 off_normals (pointer)
        // int32 off_materials (pointer)
        // int32 unknown
        // uint32 num_elements
        // int32 off_element_types (pointer)
        // int32 off_elements (pointer)
        // int32 unknown x4
        // float sphereRadius
        // float sphereX, sphereZ, sphereY

        geo.NumVertices = reader.ReadUInt32();
        geo.OffVertices = reader.ReadInt32();
        geo.OffNormals = reader.ReadInt32();
        geo.OffMaterials = reader.ReadInt32();
        reader.ReadInt32(); // unknown
        geo.NumElements = reader.ReadUInt32();
        geo.OffElementTypes = reader.ReadInt32();
        geo.OffElements = reader.ReadInt32();

        reader.ReadInt32(); // unknown
        reader.ReadInt32(); // unknown
        reader.ReadInt32(); // unknown
        reader.ReadInt32(); // unknown

        geo.SphereRadius = reader.ReadSingle();
        float sphereX = reader.ReadSingle();
        float sphereZ = reader.ReadSingle();
        float sphereY = reader.ReadSingle();
        geo.SphereCenter = new Vector3(sphereX, sphereY, sphereZ);

        // Validate basic sanity
        if (geo.NumVertices == 0 || geo.NumVertices > 100000 ||
            geo.NumElements == 0 || geo.NumElements > 10000)
        {
            return null;
        }

        // Convert pointers to offsets
        // Pointers in the file are memory addresses - we need to convert to file offsets
        // For now, we'll try direct offsets if they look valid

        return geo;
    }

    /// <summary>
    /// Reads vertex data from the data array at the vertex offset.
    /// </summary>
    public bool ReadVertices(byte[] data, int vertexOffset)
    {
        if (vertexOffset < 0 || vertexOffset + NumVertices * 12 > data.Length)
            return false;

        using var ms = new MemoryStream(data);
        ms.Position = vertexOffset;
        using var reader = new BinaryReader(ms);

        Vertices = new Vector3[NumVertices];
        for (int i = 0; i < NumVertices; i++)
        {
            float x = reader.ReadSingle();
            float z = reader.ReadSingle();
            float y = reader.ReadSingle();
            Vertices[i] = new Vector3(x, y, z); // Note: Y and Z swapped for OpenSpace
        }

        return true;
    }

    /// <summary>
    /// Reads normal data from the data array at the normal offset.
    /// </summary>
    public bool ReadNormals(byte[] data, int normalOffset)
    {
        if (normalOffset < 0 || normalOffset + NumVertices * 12 > data.Length)
            return false;

        using var ms = new MemoryStream(data);
        ms.Position = normalOffset;
        using var reader = new BinaryReader(ms);

        Normals = new Vector3[NumVertices];
        for (int i = 0; i < NumVertices; i++)
        {
            float x = reader.ReadSingle();
            float z = reader.ReadSingle();
            float y = reader.ReadSingle();
            Normals[i] = new Vector3(x, y, z); // Note: Y and Z swapped for OpenSpace
        }

        return true;
    }

    /// <summary>
    /// Reads element types from the data array.
    /// </summary>
    public bool ReadElementTypes(byte[] data, int elementTypesOffset)
    {
        if (elementTypesOffset < 0 || elementTypesOffset + NumElements * 2 > data.Length)
            return false;

        using var ms = new MemoryStream(data);
        ms.Position = elementTypesOffset;
        using var reader = new BinaryReader(ms);

        ElementTypes = new ushort[NumElements];
        for (int i = 0; i < NumElements; i++)
        {
            ElementTypes[i] = reader.ReadUInt16();
        }

        return true;
    }
}
