using System.Numerics;

namespace Astrolabe.Core.FileFormats.Animation;

/// <summary>
/// Animation data for the Montreal engine (Hype: The Time Quest).
/// </summary>
public class AnimationMontreal
{
    public int Address { get; set; }
    public int OffFrames { get; set; }
    public byte NumFrames { get; set; }
    public byte Speed { get; set; }
    public byte NumChannels { get; set; }
    public byte UnkByte { get; set; }
    public int OffUnk { get; set; }
    public Matrix4x4 SpeedMatrix { get; set; }

    public AnimFrameMontreal[] Frames { get; set; } = [];
}

/// <summary>
/// A single frame of animation with per-channel transforms.
/// </summary>
public class AnimFrameMontreal
{
    public int Address { get; set; }
    public int OffChannels { get; set; }
    public int OffMat { get; set; }
    public int OffVec { get; set; }
    public int OffHierarchies { get; set; }

    public AnimChannelMontreal[] Channels { get; set; } = [];
    public AnimHierarchy[] Hierarchies { get; set; } = [];
}

/// <summary>
/// Per-channel transform data for a single frame.
/// </summary>
public class AnimChannelMontreal
{
    public int Address { get; set; }
    public int OffMatrix { get; set; }
    public uint IsIdentity { get; set; }
    public sbyte ObjectIndex { get; set; }
    public byte Unk1 { get; set; }
    public short Unk2 { get; set; }
    public short Unk3 { get; set; }
    public byte UnkByte1 { get; set; }
    public byte UnkByte2 { get; set; }
    public uint UnkUint { get; set; }

    /// <summary>
    /// Decoded transform matrix (position, rotation, scale).
    /// </summary>
    public CompressedMatrix? Matrix { get; set; }
}

/// <summary>
/// Parent-child relationship between channels.
/// </summary>
public class AnimHierarchy
{
    public short ChildChannelId { get; set; }
    public short ParentChannelId { get; set; }
}

/// <summary>
/// Decoded compressed matrix with position, rotation, and scale.
/// </summary>
public class CompressedMatrix
{
    public ushort Type { get; set; }
    public Vector3 Position { get; set; }
    public Quaternion Rotation { get; set; }
    public Vector3 Scale { get; set; } = Vector3.One;

    /// <summary>
    /// Creates a transformation matrix from the components.
    /// </summary>
    public Matrix4x4 ToMatrix4x4()
    {
        return Matrix4x4.CreateScale(Scale) *
               Matrix4x4.CreateFromQuaternion(Rotation) *
               Matrix4x4.CreateTranslation(Position);
    }
}

/// <summary>
/// A character state (animation action like "Idle", "Walk").
/// </summary>
public class State
{
    public int Address { get; set; }
    public string? Name { get; set; }
    public int Index { get; set; }
    public int OffAnimRef { get; set; }
    public int OffNextState { get; set; }
    public byte Speed { get; set; }

    public AnimationMontreal? Animation { get; set; }
}

/// <summary>
/// A character family (shared graphics template).
/// </summary>
public class Family
{
    public int Address { get; set; }
    public uint FamilyIndex { get; set; }
    /// <summary>
    /// ObjectType index from stdGame (used to lookup name in object types table).
    /// </summary>
    public uint ObjectTypeIndex { get; set; }
    public string? Name { get; set; }
    public byte AnimBank { get; set; }
    public byte Properties { get; set; }
    public int OffBoundingVolume { get; set; }

    public List<State> States { get; set; } = [];
    public List<ObjectList> ObjectLists { get; set; } = [];
}

/// <summary>
/// A collection of mesh parts for a character.
/// </summary>
public class ObjectList
{
    public int Address { get; set; }
    public List<ObjectListEntry> Entries { get; set; } = [];
}

/// <summary>
/// An entry in an ObjectList pointing to a PhysicalObject.
/// </summary>
public class ObjectListEntry
{
    public int OffScale { get; set; }
    public int OffPhysicalObject { get; set; }
    public Vector3? Scale { get; set; }
    public int PhysicalObjectAddress { get; set; }
    public int GeometricObjectAddress { get; set; }
}

/// <summary>
/// A Perso (character instance) in the scene.
/// </summary>
public class Perso
{
    public int Address { get; set; }
    public string? NameFamily { get; set; }
    public string? NameModel { get; set; }
    public string? NamePerso { get; set; }

    /// <summary>
    /// ObjectType index for Family from stdGame (for name lookup).
    /// </summary>
    public uint FamilyTypeIndex { get; set; }

    public int Off3dData { get; set; }
    public int OffStdGame { get; set; }
    public int OffBrain { get; set; }
    public int OffCollSet { get; set; }

    public Perso3dData? P3dData { get; set; }
    public Family? Family { get; set; }
}

/// <summary>
/// 3D data for a Perso instance.
/// </summary>
public class Perso3dData
{
    public int Address { get; set; }
    public int OffFamily { get; set; }
    public int OffObjectList { get; set; }
    public int OffObjectListInitial { get; set; }
    public int OffStateCurrent { get; set; }
}
