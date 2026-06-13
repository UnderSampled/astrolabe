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

public sealed class ElementSpritesRecord
{
    public string Schema { get; set; } = "astrolabe.element-sprites.v1";
    public uint NumSprites { get; set; }
    public int Sprites { get; set; }
    public uint Unknown08 { get; set; }
    public uint Unknown0C { get; set; }
    public int Unknown10 { get; set; }
    public float Unknown14 { get; set; }
    public float Unknown18 { get; set; }
    public float Unknown1C { get; set; }
}

public sealed class CollideSetRecord
{
    public string Schema { get; set; } = "astrolabe.collide-set.v1";
    public int ZdxList { get; set; }
    public int ZddList { get; set; }
    public int ZdeList { get; set; }
    public byte[] Unknown0C { get; set; } = [];
}

public sealed class SectorRecord
{
    public string Schema { get; set; } = "astrolabe.sector.v1";
    public int CollideObj { get; set; }
    public int EnvHead { get; set; }
    public int EnvTail { get; set; }
    public int EnvHdr { get; set; }
    public uint EnvCount { get; set; }
    public int SurfHead { get; set; }
    public int SurfTail { get; set; }
    public int SurfHdr { get; set; }
    public uint SurfCount { get; set; }
    public int PersosHead { get; set; }
    public int PersosTail { get; set; }
    public int PersosHdr { get; set; }
    public uint PersosCount { get; set; }
    public int StaticLightsHead { get; set; }
    public int StaticLightsTail { get; set; }
    public int StaticLightsHdr { get; set; }
    public uint StaticLightsCount { get; set; }
    public int DynLightsHead { get; set; }
    public int DynLightsTail { get; set; }
    public int DynLightsHdr { get; set; }
    public uint DynLightsCount { get; set; }
    public int StreamsHead { get; set; }
    public int StreamsTail { get; set; }
    public int StreamsHdr { get; set; }
    public uint StreamsCount { get; set; }
    public int GraphicSectorsHead { get; set; }
    public int GraphicSectorsTail { get; set; }
    public int GraphicSectorsHdr { get; set; }
    public uint GraphicSectorsCount { get; set; }
    public int CollisionSectorsHead { get; set; }
    public int CollisionSectorsTail { get; set; }
    public int CollisionSectorsHdr { get; set; }
    public uint CollisionSectorsCount { get; set; }
    public int ActivitySectorsHead { get; set; }
    public int ActivitySectorsTail { get; set; }
    public int ActivitySectorsHdr { get; set; }
    public uint ActivitySectorsCount { get; set; }
    public int SoundSectorsHead { get; set; }
    public int SoundSectorsTail { get; set; }
    public int SoundSectorsHdr { get; set; }
    public uint SoundSectorsCount { get; set; }
    public int PlaceholderHead { get; set; }
    public int PlaceholderTail { get; set; }
    public int PlaceholderHdr { get; set; }
    public uint PlaceholderCount { get; set; }
    public uint UnknownB4 { get; set; }
    public uint UnknownB8 { get; set; }
    public uint UnknownBC { get; set; }
    public uint IsSectorVirtual { get; set; }
    public uint ActivationFlag { get; set; }
    public int Name { get; set; }
    public uint UnknownCC { get; set; }
}
