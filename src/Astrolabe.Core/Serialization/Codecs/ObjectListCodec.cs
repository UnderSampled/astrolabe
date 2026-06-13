using System.Text.Json;
using Astrolabe.Core.FileFormats.Perso;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class ObjectListCodec : IStructCodec<ObjectListRecord>
{
    public const int Size = 0x14;

    public static ObjectListCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "next", PointerTarget.BlockRelative),
        new PointerField(0x04, "prev", PointerTarget.BlockRelative),
        new PointerField(0x08, "hdr", PointerTarget.BlockRelative),
        new PointerField(0x0C, "entries", PointerTarget.BlockRelative)
    ];

    public string Kind => "objectlist";
    public string Schema => "astrolabe.object-list.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public ObjectListRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(ObjectListRecord));
        return new ObjectListRecord
        {
            Next = StructBinaryIO.ReadInt32(slice, 0x00),
            Prev = StructBinaryIO.ReadInt32(slice, 0x04),
            Hdr = StructBinaryIO.ReadInt32(slice, 0x08),
            Entries = StructBinaryIO.ReadInt32(slice, 0x0C),
            NumEntries = StructBinaryIO.ReadUInt32(slice, 0x10)
        };
    }

    public byte[] Write(ObjectListRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteInt32(bytes, 0x00, value.Next);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.Prev);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.Hdr);
        StructBinaryIO.WriteInt32(bytes, 0x0C, value.Entries);
        StructBinaryIO.WriteUInt32(bytes, 0x10, value.NumEntries);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(ObjectListRecord));
    }

    public ObjectListRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<ObjectListRecord>(json, Schema);

    public void ToJson(ObjectListRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}