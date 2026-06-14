using System.Text.Json;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class AlwaysSuperObjectsCodec : IStructCodec<OpaqueBinaryRecord>, IPointerArrayCodec
{
    public const int EntrySize = SuperObjectCodec.Size;

    public static AlwaysSuperObjectsCodec Instance { get; } = new();

    public string Kind => "alwayssuperobjects";
    public string Schema => "astrolabe.always-super-objects.v1";
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];
    public string PointerArrayPropertyName => "data";
    public int PointerEntryStride => EntrySize;

    public IReadOnlyList<PointerField> GetPointerFieldsForLength(int byteLength) =>
        BuildPointerFields(byteLength);

    private static IReadOnlyList<PointerField> BuildPointerFields(int byteLength)
    {
        if (byteLength == 0 || byteLength % EntrySize != 0)
        {
            return [];
        }

        var entryCount = byteLength / EntrySize;
        var template = SuperObjectCodec.Instance.PointerFields;
        var fields = new PointerField[entryCount * template.Count];
        for (var entryIndex = 0; entryIndex < entryCount; entryIndex++)
        {
            var entryOffset = entryIndex * EntrySize;
            for (var fieldIndex = 0; fieldIndex < template.Count; fieldIndex++)
            {
                var field = template[fieldIndex];
                fields[entryIndex * template.Count + fieldIndex] = field with
                {
                    Offset = entryOffset + field.Offset,
                    Name = $"{field.Name}_{entryIndex}"
                };
            }
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