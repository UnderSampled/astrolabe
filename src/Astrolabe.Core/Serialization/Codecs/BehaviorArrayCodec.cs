using Astrolabe.Core.Hub;
using System.Text.Json;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class BehaviorArrayCodec : IStructCodec<OpaqueBinaryRecord>, IPointerArrayCodec
{
    public const int EntrySize = 0x10;

    private BehaviorArrayCodec(string kind, string schema)
    {
        Kind = kind;
        Schema = schema;
    }

    public string Kind { get; }
    public string Schema { get; }
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];
    public string PointerArrayPropertyName => "data";
    public int PointerEntryStride => EntrySize;

    public static BehaviorArrayCodec BehaviorsNormal { get; } = new(
        "behaviors_normal",
        "astrolabe.behaviors-normal.v1");

    public static BehaviorArrayCodec BehaviorsReflex { get; } = new(
        "behaviors_reflex",
        "astrolabe.behaviors-reflex.v1");

    public IReadOnlyList<PointerField> GetPointerFieldsForLength(int byteLength) =>
        BuildPointerFields(byteLength);

    private static IReadOnlyList<PointerField> BuildPointerFields(int byteLength)
    {
        if (byteLength == 0 || byteLength % EntrySize != 0)
        {
            return [];
        }

        var entryCount = byteLength / EntrySize;
        var fields = new PointerField[entryCount * 2];
        for (var entryIndex = 0; entryIndex < entryCount; entryIndex++)
        {
            var entryOffset = entryIndex * EntrySize;
            fields[entryIndex * 2 + 0] = new PointerField(entryOffset + 0x00, "scripts", PointerTarget.BlockRelative);
            fields[entryIndex * 2 + 1] = new PointerField(entryOffset + 0x04, "scheduleScript", PointerTarget.BlockRelative);
        }

        return fields;
    }

    public OpaqueBinaryRecord Read(ReadOnlySpan<byte> data, int offset, int length) =>
        OpaqueBinaryRecord.FromSlice(Schema, data, offset, length);

    public byte[] Write(OpaqueBinaryRecord value) => value.Data;

    public OpaqueBinaryRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<OpaqueBinaryRecord>(json, Schema);

    public void ToJson(OpaqueBinaryRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}