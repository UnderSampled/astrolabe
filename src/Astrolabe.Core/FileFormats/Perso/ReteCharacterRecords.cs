using Astrolabe.Core.Hub;

namespace Astrolabe.Core.FileFormats.Perso;

public sealed class ObjectTypeEntryRecord
{
    public string Schema { get; set; } = "astrolabe.object-type-entry.v1";
    public HubReference Next { get; set; } = HubReference.Null;
    public HubReference Prev { get; set; } = HubReference.Null;
    public HubReference Hdr { get; set; } = HubReference.Null;
    public HubReference Name { get; set; } = HubReference.Null;
    public uint Marker { get; set; }
}

public sealed class TextNameRecord
{
    public string Schema { get; set; } = "astrolabe.text-name.v1";
    public string Type { get; set; } = "";
    /// <summary>Decoded text without the trailing NUL when the payload is a C string.</summary>
    public string Text { get; set; } = "";
    /// <summary>Full original payload (includes NUL and any trailing padding).</summary>
    public byte[] Bytes { get; set; } = [];
}

public sealed class AlwaysSuperObjectsRecord
{
    public string Schema { get; set; } = "astrolabe.always-super-objects.v1";
    public SuperObjectRecord[] Entries { get; set; } = [];
}

public sealed class DynamRecord
{
    public string Schema { get; set; } = "astrolabe.dynam.v1";
    public uint Unknown00 { get; set; }
    public uint Unknown04 { get; set; }
    public uint Unknown08 { get; set; }
    public uint Unknown0C { get; set; }
    public HubReference Field10 { get; set; } = HubReference.Null;
    public HubReference Field14 { get; set; } = HubReference.Null;
    public byte[] Unknown18 { get; set; } = [];
}
