namespace Astrolabe.Core.FileFormats.Animation;

public sealed class AnimHierarchyEntryRecord
{
    public short ChildChannelId { get; set; }
    public short ParentChannelId { get; set; }
    /// <summary>Optional trailing 4 bytes when the tracker recorded 8-byte hierarchy slots.</summary>
    public uint? Unknown04 { get; set; }
}

public sealed class AnimHierarchiesRecord
{
    public string Schema { get; set; } = "astrolabe.anim-hierarchies.v1";
    public AnimHierarchyEntryRecord[] Entries { get; set; } = [];
    /// <summary>Bytes after complete hierarchy entries (stream padding / following islands).</summary>
    public byte[] TrailingBytes { get; set; } = [];
}
