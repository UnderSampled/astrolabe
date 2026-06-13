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