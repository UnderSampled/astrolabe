using Astrolabe.Core.Hub;

namespace Astrolabe.Core.FileFormats.Geometry;

public sealed class GeometricObjectRecord
{
    public string Schema { get; set; } = "astrolabe.geometric-object.v1";
    public uint NumVertices { get; set; }
    public HubReference Vertices { get; set; } = HubReference.Null;
    public HubReference Normals { get; set; } = HubReference.Null;
    public HubReference Materials { get; set; } = HubReference.Null;
    public int Unknown0 { get; set; }
    public uint NumElements { get; set; }
    public HubReference ElementTypes { get; set; } = HubReference.Null;
    public HubReference Elements { get; set; } = HubReference.Null;
    public int[] Unknowns { get; set; } = [];
    public float SphereRadius { get; set; }
    public float[] SphereCenterRaw { get; set; } = [];
}

public sealed class PhysicalObjectRecord
{
    public string Schema { get; set; } = "astrolabe.physical-object.v1";
    public HubReference VisualSet { get; set; } = HubReference.Null;
    public HubReference CollideSet { get; set; } = HubReference.Null;
    public HubReference VisualBoundingVolume { get; set; } = HubReference.Null;
    public int Unknown0 { get; set; }
}

public sealed class IpoRecord
{
    public string Schema { get; set; } = "astrolabe.ipo.v1";
    public HubReference PhysicalObject { get; set; } = HubReference.Null;
    public HubReference Radiosity { get; set; } = HubReference.Null;
}

public sealed class VisualSetRecord
{
    public string Schema { get; set; } = "astrolabe.visual-set.v1";
    public uint Unknown0 { get; set; }
    public ushort NumberOfLod { get; set; }
    public ushort VisualSetType { get; set; }
    public HubReference LodDistances { get; set; } = HubReference.Null;
    public HubReference LodDataOffsets { get; set; } = HubReference.Null;
}

public sealed class ElementTrianglesRecord
{
    public string Schema { get; set; } = "astrolabe.element-triangles.v1";
    public HubReference Material { get; set; } = HubReference.Null;
    public ushort NumTriangles { get; set; }
    public ushort NumUvs { get; set; }
    public HubReference Triangles { get; set; } = HubReference.Null;
    public HubReference MappingUvs { get; set; } = HubReference.Null;
    public HubReference Normals { get; set; } = HubReference.Null;
    public HubReference Uvs { get; set; } = HubReference.Null;
    public uint Unknown18 { get; set; }
    public HubReference VertexIndices { get; set; } = HubReference.Null;
    public ushort NumVertexIndices { get; set; }
    public ushort ParallelBox { get; set; }
    public uint Unknown24 { get; set; }
}

public sealed class RadiosityHeaderRecord
{
    public string Schema { get; set; } = "astrolabe.radiosity-header.v1";
    public uint NumLod { get; set; }
    public HubReference Lods { get; set; } = HubReference.Null;
    public uint Unknown08 { get; set; }
    public uint Unknown0C { get; set; }
}

public sealed class ElementSpritesRecord
{
    public string Schema { get; set; } = "astrolabe.element-sprites.v1";
    public uint NumSprites { get; set; }
    public HubReference Sprites { get; set; } = HubReference.Null;
    public uint Unknown08 { get; set; }
    public uint Unknown0C { get; set; }
    public HubReference Unknown10 { get; set; } = HubReference.Null;
    public float Unknown14 { get; set; }
    public float Unknown18 { get; set; }
    public float Unknown1C { get; set; }
}

public sealed class CollideSetRecord
{
    public string Schema { get; set; } = "astrolabe.collide-set.v1";
    public HubReference ZdxList { get; set; } = HubReference.Null;
    public HubReference ZddList { get; set; } = HubReference.Null;
    public HubReference ZdeList { get; set; } = HubReference.Null;
    public byte[] Unknown0C { get; set; } = [];
}

public sealed class SectorRecord
{
    public string Schema { get; set; } = "astrolabe.sector.v1";
    public HubReference CollideObj { get; set; } = HubReference.Null;
    public HubReference EnvHead { get; set; } = HubReference.Null;
    public HubReference EnvTail { get; set; } = HubReference.Null;
    public HubReference EnvHdr { get; set; } = HubReference.Null;
    public uint EnvCount { get; set; }
    public HubReference SurfHead { get; set; } = HubReference.Null;
    public HubReference SurfTail { get; set; } = HubReference.Null;
    public HubReference SurfHdr { get; set; } = HubReference.Null;
    public uint SurfCount { get; set; }
    public HubReference PersosHead { get; set; } = HubReference.Null;
    public HubReference PersosTail { get; set; } = HubReference.Null;
    public HubReference PersosHdr { get; set; } = HubReference.Null;
    public uint PersosCount { get; set; }
    public HubReference StaticLightsHead { get; set; } = HubReference.Null;
    public HubReference StaticLightsTail { get; set; } = HubReference.Null;
    public HubReference StaticLightsHdr { get; set; } = HubReference.Null;
    public uint StaticLightsCount { get; set; }
    public HubReference DynLightsHead { get; set; } = HubReference.Null;
    public HubReference DynLightsTail { get; set; } = HubReference.Null;
    public HubReference DynLightsHdr { get; set; } = HubReference.Null;
    public uint DynLightsCount { get; set; }
    public HubReference StreamsHead { get; set; } = HubReference.Null;
    public HubReference StreamsTail { get; set; } = HubReference.Null;
    public HubReference StreamsHdr { get; set; } = HubReference.Null;
    public uint StreamsCount { get; set; }
    public HubReference GraphicSectorsHead { get; set; } = HubReference.Null;
    public HubReference GraphicSectorsTail { get; set; } = HubReference.Null;
    public HubReference GraphicSectorsHdr { get; set; } = HubReference.Null;
    public uint GraphicSectorsCount { get; set; }
    public HubReference CollisionSectorsHead { get; set; } = HubReference.Null;
    public HubReference CollisionSectorsTail { get; set; } = HubReference.Null;
    public HubReference CollisionSectorsHdr { get; set; } = HubReference.Null;
    public uint CollisionSectorsCount { get; set; }
    public HubReference ActivitySectorsHead { get; set; } = HubReference.Null;
    public HubReference ActivitySectorsTail { get; set; } = HubReference.Null;
    public HubReference ActivitySectorsHdr { get; set; } = HubReference.Null;
    public uint ActivitySectorsCount { get; set; }
    public HubReference SoundSectorsHead { get; set; } = HubReference.Null;
    public HubReference SoundSectorsTail { get; set; } = HubReference.Null;
    public HubReference SoundSectorsHdr { get; set; } = HubReference.Null;
    public uint SoundSectorsCount { get; set; }
    public HubReference PlaceholderHead { get; set; } = HubReference.Null;
    public HubReference PlaceholderTail { get; set; } = HubReference.Null;
    public HubReference PlaceholderHdr { get; set; } = HubReference.Null;
    public uint PlaceholderCount { get; set; }
    public uint UnknownB4 { get; set; }
    public uint UnknownB8 { get; set; }
    public uint UnknownBC { get; set; }
    public uint IsSectorVirtual { get; set; }
    public uint ActivationFlag { get; set; }
    public HubReference Name { get; set; } = HubReference.Null;
    public uint UnknownCC { get; set; }
}