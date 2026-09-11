using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;
using Astrolabe.Core.Hub;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class BehaviorListCodec : IStructCodec<BehaviorListRecord>
{
    public const int Size = 0x08;

    private BehaviorListCodec(string kind, string schema, string type)
    {
        Kind = kind;
        Schema = schema;
        Type = type;
    }

    public string Kind { get; }
    public string Schema { get; }
    public string Type { get; }
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields { get; } =
    [
        new PointerField(0x00, "entries", PointerTarget.BlockRelative)
    ];

    public static BehaviorListCodec Normal { get; } = new(
        "behaviorlist_normal",
        "astrolabe.behavior-list-normal.v1",
        "behaviorlist_normal");

    public static BehaviorListCodec Reflex { get; } = new(
        "behaviorlist_reflex",
        "astrolabe.behavior-list-reflex.v1",
        "behaviorlist_reflex");

    public BehaviorListRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(BehaviorListRecord));
        return new BehaviorListRecord
        {
            Schema = Schema,
            Type = Type,
            Entries = HubReferenceIO.Read(slice, 0x00),
            Count = StructBinaryIO.ReadUInt32(slice, 0x04)
        };
    }

    public byte[] Write(BehaviorListRecord value)
    {
        var bytes = new byte[Size];
        HubReferenceIO.Write(bytes, 0x00, value.Entries);
        StructBinaryIO.WriteUInt32(bytes, 0x04, value.Count);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(BehaviorListRecord));
    }

    public BehaviorListRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<BehaviorListRecord>(json, Schema);

    public void ToJson(BehaviorListRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}
