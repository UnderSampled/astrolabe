namespace Astrolabe.Core.FileFormats.Materials;

/// <summary>
/// Collision flags for CollideMaterial.
/// </summary>
[Flags]
public enum CollisionFlags : ushort
{
    None = 0,
    Slide = 1 << 0,
    Trampoline = 1 << 1,
    GrabbableLedge = 1 << 2,
    Wall = 1 << 3,
    Unknown1 = 1 << 4,
    HangableCeiling = 1 << 5,
    ClimbableWall = 1 << 6,
    Electric = 1 << 7,
    LavaDeathWarp = 1 << 8,
    FallTrigger = 1 << 9,
    HurtTrigger = 1 << 10,
    DeathWarp = 1 << 11,
    Unknown2 = 1 << 12,
    Unknown3 = 1 << 13,
    Water = 1 << 14,
    NoCollision = 1 << 15,
    All = 0xFFFF
}

/// <summary>
/// Collide Material - controls physics/collision behavior.
/// </summary>
public class CollideMaterial
{
    public int Address { get; set; }

    public ushort Type { get; set; }
    public CollisionFlags Identifier { get; set; }
    public uint TypeForAI { get; set; }

    // Flag helpers
    public bool Slide => Identifier.HasFlag(CollisionFlags.Slide);
    public bool Trampoline => Identifier.HasFlag(CollisionFlags.Trampoline);
    public bool GrabbableLedge => Identifier.HasFlag(CollisionFlags.GrabbableLedge);
    public bool Wall => Identifier.HasFlag(CollisionFlags.Wall);
    public bool HangableCeiling => Identifier.HasFlag(CollisionFlags.HangableCeiling);
    public bool ClimbableWall => Identifier.HasFlag(CollisionFlags.ClimbableWall);
    public bool Electric => Identifier.HasFlag(CollisionFlags.Electric);
    public bool LavaDeathWarp => Identifier.HasFlag(CollisionFlags.LavaDeathWarp);
    public bool FallTrigger => Identifier.HasFlag(CollisionFlags.FallTrigger);
    public bool HurtTrigger => Identifier.HasFlag(CollisionFlags.HurtTrigger);
    public bool DeathWarp => Identifier.HasFlag(CollisionFlags.DeathWarp);
    public bool Water => Identifier.HasFlag(CollisionFlags.Water);
    public bool NoCollision => Identifier.HasFlag(CollisionFlags.NoCollision);
}

/// <summary>
/// Reads CollideMaterial structures from memory.
/// </summary>
public class CollideMaterialReader
{
    private readonly MemoryContext _memory;
    private readonly Dictionary<int, CollideMaterial> _cache = new();

    public CollideMaterialReader(MemoryContext memory)
    {
        _memory = memory;
    }

    public CollideMaterial? Read(int address)
    {
        if (address == 0) return null;

        if (_cache.TryGetValue(address, out var cached))
            return cached;

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        var mat = new CollideMaterial { Address = address };

        try
        {
            // R2/Montreal CollideMaterial structure
            mat.Type = reader.ReadUInt16();                          // 0x00
            mat.Identifier = (CollisionFlags)reader.ReadUInt16();    // 0x02
            mat.TypeForAI = reader.ReadUInt32();                     // 0x04

            _cache[address] = mat;
            return mat;
        }
        catch
        {
            return null;
        }
    }
}
