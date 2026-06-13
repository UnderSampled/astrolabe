namespace Astrolabe.Core.FileFormats.Geometry;

public sealed class GeometricObjectRecord
{
    public string Schema { get; set; } = "astrolabe.geometric-object.v1";
    public uint NumVertices { get; set; }
    public int Vertices { get; set; }
    public int Normals { get; set; }
    public int Materials { get; set; }
    public int Unknown0 { get; set; }
    public uint NumElements { get; set; }
    public int ElementTypes { get; set; }
    public int Elements { get; set; }
    public int[] Unknowns { get; set; } = [];
    public float SphereRadius { get; set; }
    public float[] SphereCenterRaw { get; set; } = [];
}

public sealed class PhysicalObjectRecord
{
    public string Schema { get; set; } = "astrolabe.physical-object.v1";
    public int VisualSet { get; set; }
    public int CollideSet { get; set; }
    public int VisualBoundingVolume { get; set; }
    public int Unknown0 { get; set; }
}

public sealed class IpoRecord
{
    public string Schema { get; set; } = "astrolabe.ipo.v1";
    public int PhysicalObject { get; set; }
    public int Radiosity { get; set; }
}

public sealed class VisualSetRecord
{
    public string Schema { get; set; } = "astrolabe.visual-set.v1";
    public uint Unknown0 { get; set; }
    public ushort NumberOfLod { get; set; }
    public ushort VisualSetType { get; set; }
    public int LodDistances { get; set; }
    public int LodDataOffsets { get; set; }
}

public sealed class ElementTrianglesRecord
{
    public string Schema { get; set; } = "astrolabe.element-triangles.v1";
    public int Material { get; set; }
    public ushort NumTriangles { get; set; }
    public ushort NumUvs { get; set; }
    public int Triangles { get; set; }
    public int MappingUvs { get; set; }
    public int Normals { get; set; }
    public int Uvs { get; set; }
    public uint Unknown18 { get; set; }
    public int VertexIndices { get; set; }
    public ushort NumVertexIndices { get; set; }
    public ushort ParallelBox { get; set; }
    public uint Unknown24 { get; set; }
}

public sealed class RadiosityHeaderRecord
{
    public string Schema { get; set; } = "astrolabe.radiosity-header.v1";
    public uint NumLod { get; set; }
    public int Lods { get; set; }
    public uint Unknown08 { get; set; }
    public uint Unknown0C { get; set; }
}
