using System.Text.Json;
using Astrolabe.Core.FileFormats.Perso;
using Astrolabe.Core.Hub;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class ObjectTypeEntryCodec : IStructCodec<ObjectTypeEntryRecord>
{
    public const int Size = 0x14;

    public static ObjectTypeEntryCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "next", PointerTarget.BlockRelative),
        new PointerField(0x04, "prev", PointerTarget.BlockRelative),
        new PointerField(0x08, "hdr", PointerTarget.BlockRelative),
        new PointerField(0x0C, "name", PointerTarget.BlockRelative)
    ];

    public string Kind => "objecttypeentry";
    public string Schema => "astrolabe.object-type-entry.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public ObjectTypeEntryRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(ObjectTypeEntryRecord));
        return new ObjectTypeEntryRecord
        {
            Next = HubReferenceIO.Read(slice, 0x00),
            Prev = HubReferenceIO.Read(slice, 0x04),
            Hdr = HubReferenceIO.Read(slice, 0x08),
            Name = HubReferenceIO.Read(slice, 0x0C),
            Marker = StructBinaryIO.ReadUInt32(slice, 0x10)
        };
    }

    public byte[] Write(ObjectTypeEntryRecord value)
    {
        var bytes = new byte[Size];
        HubReferenceIO.Write(bytes, 0x00, value.Next);
        HubReferenceIO.Write(bytes, 0x04, value.Prev);
        HubReferenceIO.Write(bytes, 0x08, value.Hdr);
        HubReferenceIO.Write(bytes, 0x0C, value.Name);
        StructBinaryIO.WriteUInt32(bytes, 0x10, value.Marker);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(ObjectTypeEntryRecord));
    }

    public ObjectTypeEntryRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<ObjectTypeEntryRecord>(json, Schema);

    public void ToJson(ObjectTypeEntryRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}
