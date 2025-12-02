namespace Astrolabe.Core.FileFormats.Materials;

/// <summary>
/// Game Material - top-level container linking visual, physics, and collision materials.
/// </summary>
public class GameMaterial
{
    public int Address { get; set; }

    public int OffVisualMaterial { get; set; }
    public int OffMechanicsMaterial { get; set; }
    public uint SoundMaterial { get; set; }
    public int OffCollideMaterial { get; set; }

    // Resolved references
    public VisualMaterial? VisualMaterial { get; set; }
    public CollideMaterial? CollideMaterial { get; set; }
}

/// <summary>
/// Reads GameMaterial structures from memory.
/// </summary>
public class GameMaterialReader
{
    private readonly MemoryContext _memory;
    private readonly VisualMaterialReader _visualMaterialReader;
    private readonly CollideMaterialReader _collideMaterialReader;
    private readonly Dictionary<int, GameMaterial> _cache = new();

    public GameMaterialReader(MemoryContext memory)
    {
        _memory = memory;
        _visualMaterialReader = new VisualMaterialReader(memory);
        _collideMaterialReader = new CollideMaterialReader(memory);
    }

    public GameMaterial? Read(int address)
    {
        if (address == 0) return null;

        if (_cache.TryGetValue(address, out var cached))
            return cached;

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        var mat = new GameMaterial { Address = address };

        try
        {
            // R2/Montreal GameMaterial structure
            mat.OffVisualMaterial = reader.ReadInt32();     // 0x00
            mat.OffMechanicsMaterial = reader.ReadInt32();  // 0x04
            mat.SoundMaterial = reader.ReadUInt32();        // 0x08
            mat.OffCollideMaterial = reader.ReadInt32();    // 0x0C

            // Resolve references
            if (mat.OffVisualMaterial != 0)
            {
                mat.VisualMaterial = _visualMaterialReader.Read(mat.OffVisualMaterial);
            }

            if (mat.OffCollideMaterial != 0 && mat.OffCollideMaterial != -1)
            {
                mat.CollideMaterial = _collideMaterialReader.Read(mat.OffCollideMaterial);
            }

            _cache[address] = mat;
            return mat;
        }
        catch
        {
            return null;
        }
    }

    public VisualMaterialReader VisualMaterialReader => _visualMaterialReader;
    public CollideMaterialReader CollideMaterialReader => _collideMaterialReader;
}
