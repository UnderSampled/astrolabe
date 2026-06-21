using Astrolabe.Core.Hub;

namespace Astrolabe.Core.FileFormats.Animation;

public sealed class AnimChannelRecord
{
    public string Schema { get; set; } = "astrolabe.anim-channel.v1";

    /// <summary>
    /// Montreal <c>AnimChannelMontreal</c> first <c>uint32</c>: sentinel <c>0</c> (none), <c>1</c> (identity),
    /// or a virtual address to compressed matrix data. JSON field name is <c>isIdentity</c>.
    /// </summary>
    public HubReference IsIdentity { get; set; } = HubReference.Null;

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
    public HubReference Unknown10 { get; set; } = HubReference.Null;
}

public sealed class AnimationMontrealRecord
{
    public string Schema { get; set; } = "astrolabe.animation-montreal.v1";
    public HubReference OffFrames { get; set; } = HubReference.Null;
    public byte NumFrames { get; set; }
    public byte Speed { get; set; }
    public byte NumChannels { get; set; }
    public byte UnkByte { get; set; }
    public HubReference OffUnk { get; set; } = HubReference.Null;
    public uint Unk0C { get; set; }
    public uint Unk10 { get; set; }
    public float[] SpeedMatrix { get; set; } = [];
    public uint[] Tail { get; set; } = [];
}

public sealed class AnimFrameRecord
{
    public HubReference Channels { get; set; } = HubReference.Null;
    public HubReference Mat { get; set; } = HubReference.Null;
    public HubReference Vec { get; set; } = HubReference.Null;
    public HubReference Hierarchies { get; set; } = HubReference.Null;
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
    public HubReference OffHierarchies { get; set; } = HubReference.Null;
}