namespace Astrolabe.Core.Rete;

/// <summary>Kind sets for dual-layer semantic domain aggregation.</summary>
internal static class SemanticDomainKinds
{
    public static readonly HashSet<string> Scene = new(StringComparer.OrdinalIgnoreCase)
    {
        "superobject",
        "matrix"
    };

    public static readonly HashSet<string> Geometry = new(StringComparer.OrdinalIgnoreCase)
    {
        "geometricobject",
        "physicalobject",
        "ipo",
        "visualset",
        "elementtriangles",
        "elementsprites",
        "elementptrs",
        "elementtypes",
        "vertices",
        "normals",
        "trianglenormals",
        "vertexindices",
        "triangles",
        "uvs",
        "uvmapping",
        "radiosityheader",
        "loddistances",
        "loddataoffsets",
        "visualmaterial",
        "gamematerial",
        "collidematerial",
        "boundingvolume"
    };

    /// <summary>Dense numeric arrays as descriptor + geometry/buffers/*.bin.</summary>
    public static readonly HashSet<string> DenseBuffer = new(StringComparer.OrdinalIgnoreCase)
    {
        "vertices",
        "normals",
        "trianglenormals",
        "vertexindices",
        "triangles",
        "uvs",
        "uvmapping",
        "elementtypes",
        "loddistances"
    };

    public static readonly HashSet<string> Ai = new(StringComparer.OrdinalIgnoreCase)
    {
        "brain",
        "mind",
        "intelligence",
        "aimodel",
        "script",
        "scriptptrs",
        "behaviorlist_normal",
        "behaviorlist_reflex",
        "behaviors_normal",
        "behaviors_reflex",
        "dsgvar",
        "dsgmem",
        "dsgvarptrindirect"
    };

    public static readonly HashSet<string> Character = new(StringComparer.OrdinalIgnoreCase)
    {
        "perso",
        "perso3ddata",
        "standardgame",
        "objectlist",
        "spawnableentry",
        "dynam",
        "persosectorinfo",
        "objecttypeentry",
        "objecttypename",
        "alwayssuperobjects"
    };

    /// <summary>
    /// Sector / collision pool kinds. Matches promoted codecs + named binary leaves under
    /// <c>types/</c> for this domain. Residual collision trackers left opaque (not pooled):
    /// see <c>notes/residual-opaque.txt</c>.
    /// </summary>
    public static readonly HashSet<string> Sector = new(StringComparer.OrdinalIgnoreCase)
    {
        // SectorCodec
        "sector",
        // named binary (no structured codec yet)
        "sectorname",
        // CollideSetCodec
        "collideset",
        // FixedBinaryStructCodec.CollideZoneList
        "collidezdxlist",
        "collidezddlist",
        "collidezdelist",
        // FixedBinaryStructCodec.CollideZone
        "collidezdxzone",
        "collidezddzone",
        "collidezdezone",
        // PointerArrayCodec.CollideElementPtrs
        "collideelementptrs",
        // FixedBinaryStructCodec.SectorCollideGeo + dense verts blob
        "sectorcollidegeo",
        "sectorcollideverts"
    };

    public static bool IsDenseBufferKind(string kind) =>
        DenseBuffer.Contains(kind);
}
