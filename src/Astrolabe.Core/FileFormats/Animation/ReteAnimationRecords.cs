namespace Astrolabe.Core.FileFormats.Animation;

public sealed class AnimChannelRecord
{
    public string Schema { get; set; } = "astrolabe.anim-channel.v1";

    /// <summary>
    /// Montreal <c>AnimChannelMontreal</c> first <c>uint32</c>: sentinel <c>0</c> (none), <c>1</c> (identity),
    /// or a virtual address to compressed matrix data. JSON field name is <c>isIdentity</c>.
    /// </summary>
    public int IsIdentity { get; set; }

    public sbyte ObjectIndex { get; set; }
    public byte Unk1 { get; set; }
    public short Unk2 { get; set; }
    public short Unk3 { get; set; }
    public byte UnkByte1 { get; set; }
    public byte UnkByte2 { get; set; }
    public uint UnkUint { get; set; }

    /// <summary>
    /// Bytes <c>0x10–0x13</c>: either a virtual-address pointer (URI on import) or a small inline integer.
    /// </summary>
    public int Unknown10 { get; set; }
}

public sealed class AnimationMontrealRecord
{
    public string Schema { get; set; } = "astrolabe.animation-montreal.v1";
    public int OffFrames { get; set; }
    public byte NumFrames { get; set; }
    public byte Speed { get; set; }
    public byte NumChannels { get; set; }
    public byte UnkByte { get; set; }
    public int OffUnk { get; set; }
    public uint Unk0C { get; set; }
    public uint Unk10 { get; set; }
    public float[] SpeedMatrix { get; set; } = [];
    public uint[] Tail { get; set; } = [];
}

public sealed class AnimFrameRecord
{
    public int Channels { get; set; }
    public int Mat { get; set; }
    public int Vec { get; set; }
    public int Hierarchies { get; set; }
}

public sealed class AnimFramesRecord
{
    public string Schema { get; set; } = "astrolabe.anim-frames.v1";
    public AnimFrameRecord[] Frames { get; set; } = [];
}

public sealed class AnimHierarchiesHeaderRecord
{
    public string Schema { get; set; } = "astrolabe.anim-hierarchies-header.v1";
    public uint Count { get; set; }
    public int OffHierarchies { get; set; }
}