using System.Numerics;

namespace Astrolabe.Core.FileFormats.Materials;

/// <summary>
/// Visual Material - controls rendering/appearance of geometry.
/// </summary>
public class VisualMaterial
{
    public int Address { get; set; }

    // Flags
    public uint Flags { get; set; }

    // Lighting coefficients
    public Vector4 AmbientCoef { get; set; }
    public Vector4 DiffuseCoef { get; set; }
    public Vector4 SpecularCoef { get; set; }
    public Vector4 Color { get; set; }

    // Texture
    public int OffTexture { get; set; }
    public string? TextureName { get; set; }

    // UV scrolling
    public float CurrentScrollX { get; set; }
    public float CurrentScrollY { get; set; }
    public float ScrollX { get; set; }
    public float ScrollY { get; set; }
    public uint ScrollMode { get; set; }

    // Animated textures
    public int OffAnimTexturesFirst { get; set; }
    public int OffAnimTexturesCurrent { get; set; }
    public ushort NumAnimTextures { get; set; }

    // Properties
    public byte Properties { get; set; }

    // Flag helpers
    public static uint Flag_BackfaceCulling = 1 << 10;
    public static uint Flag_IsBillboard = 1 << 9;
    public static uint Flag_IsTransparent = 1 << 3;
    public static uint Flag_IsTransparent_R2 = 0x4000000;
    public static uint Flag_IsChromed = 1 << 22;

    public bool BackfaceCulling => (Flags & Flag_BackfaceCulling) != 0;
    public bool IsBillboard => (Flags & Flag_IsBillboard) != 0;
    public bool IsTransparent => (Flags & Flag_IsTransparent_R2) != 0;
    public bool IsChromed => (Flags & Flag_IsChromed) != 0;

    // Property helpers
    public static uint Property_ReceiveShadows = 2;
    public static uint Property_IsSpriteGenerator = 4;
    public static uint Property_IsAnimatedSpriteGenerator = 12;

    public bool ReceiveShadows => (Properties & Property_ReceiveShadows) != 0;
}

/// <summary>
/// Reads VisualMaterial structures from memory.
/// </summary>
public class VisualMaterialReader
{
    private readonly MemoryContext _memory;
    private readonly Dictionary<int, VisualMaterial> _cache = new();

    public VisualMaterialReader(MemoryContext memory)
    {
        _memory = memory;
    }

    public VisualMaterial? Read(int address)
    {
        if (address == 0) return null;

        if (_cache.TryGetValue(address, out var cached))
            return cached;

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        var mat = new VisualMaterial { Address = address };

        try
        {
            // R2/Montreal VisualMaterial structure (~0x78 bytes)
            mat.Flags = reader.ReadUInt32();  // 0x00

            // Lighting coefficients (4 floats each = 16 bytes)
            mat.AmbientCoef = ReadVector4(reader);   // 0x04
            mat.DiffuseCoef = ReadVector4(reader);   // 0x14
            mat.SpecularCoef = ReadVector4(reader);  // 0x24
            mat.Color = ReadVector4(reader);         // 0x34

            reader.ReadUInt32(); // 0x44 unknown

            mat.OffTexture = reader.ReadInt32();     // 0x48

            mat.CurrentScrollX = reader.ReadSingle(); // 0x4C
            mat.CurrentScrollY = reader.ReadSingle(); // 0x50
            mat.ScrollX = reader.ReadSingle();        // 0x54
            mat.ScrollY = reader.ReadSingle();        // 0x58
            mat.ScrollMode = reader.ReadUInt32();     // 0x5C

            reader.ReadInt32(); // 0x60 refresh number

            mat.OffAnimTexturesFirst = reader.ReadInt32();   // 0x64
            mat.OffAnimTexturesCurrent = reader.ReadInt32(); // 0x68
            mat.NumAnimTextures = reader.ReadUInt16();       // 0x6C
            reader.ReadUInt16(); // 0x6E padding

            reader.ReadUInt32(); // 0x70 unknown
            mat.Properties = reader.ReadByte(); // 0x74

            _cache[address] = mat;
            return mat;
        }
        catch
        {
            return null;
        }
    }

    private static Vector4 ReadVector4(BinaryReader reader)
    {
        return new Vector4(
            reader.ReadSingle(),
            reader.ReadSingle(),
            reader.ReadSingle(),
            reader.ReadSingle()
        );
    }
}
