namespace Astrolabe.Core.FileFormats.AI;

/// <summary>
/// Represents a single node in an OpenSpace AI script.
/// Binary format (PC, 8 bytes):
///   [0x00] uint param  - Parameter value (int, float bits, or pointer)
///   [0x04] ushort padding
///   [0x06] byte indent - Tree depth (0 = end of script)
///   [0x07] byte type   - Node type (indexes into type tables)
/// </summary>
public class ScriptNode
{
    /// <summary>Memory address where this node was read from.</summary>
    public int Offset { get; set; }

    /// <summary>Parameter value (integer, float bits, or pointer address).</summary>
    public uint Param { get; set; }

    /// <summary>If param is a pointer, this holds the resolved address.</summary>
    public int? ParamPointer { get; set; }

    /// <summary>Tree depth. 0 indicates end of script.</summary>
    public byte Indent { get; set; }

    /// <summary>Raw type byte (indexes into game-specific type tables).</summary>
    public byte Type { get; set; }

    /// <summary>Resolved node type category.</summary>
    public NodeType NodeType { get; set; } = NodeType.Unknown;

    /// <summary>
    /// Reads a script node from a binary reader.
    /// </summary>
    public static ScriptNode Read(BinaryReader reader, int offset, AITypes aiTypes)
    {
        var node = new ScriptNode { Offset = offset };

        node.Param = reader.ReadUInt32();
        reader.ReadUInt16(); // padding
        node.Indent = reader.ReadByte();
        node.Type = reader.ReadByte();

        node.NodeType = aiTypes.GetNodeType(node.Type);

        return node;
    }

    /// <summary>Size of a script node in bytes.</summary>
    public const int Size = 8;
}

/// <summary>
/// Script node type categories.
/// </summary>
public enum NodeType
{
    Unknown,
    KeyWord,
    Condition,
    Operator,
    Function,
    Procedure,
    MetaAction,
    BeginMacro,
    EndMacro,
    Field,
    DsgVarRef,
    Constant,
    Real,
    Button,
    ConstantVector,
    Vector,
    Mask,
    ModuleRef,
    DsgVarId,
    String,
    LipsSynchroRef,
    FamilyRef,
    PersoRef,
    ActionRef,
    SuperObjectRef,
    WayPointRef,
    TextRef,
    ComportRef,
    SoundEventRef,
    ObjectTableRef,
    GameMaterialRef,
    ParticleGenerator,
    VisualMaterial,
    ModelRef,
    DataType42,
    CustomBits,
    Caps,
    SubRoutine,
    Null,
    GraphRef,
    // Types for engine versions < R2
    ConstantRef,
    RealRef,
    SurfaceRef,
    Way,
    DsgVar,
    SectorRef,
    EnvironmentRef,
    FontRef,
    Color,
    Module,
    LightInfoRef
}
