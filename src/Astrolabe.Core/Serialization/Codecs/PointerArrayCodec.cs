using System.Text.Json;
using Astrolabe.Core.FileFormats;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class PointerArrayCodec : IStructCodec<PointerArrayRecord>, IPointerArrayCodec
{
    private PointerArrayCodec(string kind)
    {
        Kind = kind;
    }

    public string Kind { get; }
    public string Schema => "astrolabe.pointer-array.v1";
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];

    public static PointerArrayCodec ElementPtrs { get; } = new("elementptrs");
    public static PointerArrayCodec LodDataOffsets { get; } = new("loddataoffsets");
    public static PointerArrayCodec AnimChannelPtrs { get; } = new("animchannelptrs");
    public static PointerArrayCodec ScriptPtrs { get; } = new("scriptptrs");
    public static PointerArrayCodec DsgVarPtrIndirect { get; } = new("dsgvarptrindirect");
    public static PointerArrayCodec CollideElementPtrs { get; } = new("collideelementptrs");

    public string PointerArrayPropertyName => "values";

    public IReadOnlyList<PointerField> GetPointerFieldsForLength(int byteLength)
    {
        if (byteLength == 0)
        {
            return [];
        }

        if (byteLength % 4 != 0)
        {
            throw new InvalidDataException($"{Kind} serialized length {byteLength} is not a multiple of 4.");
        }

        var count = byteLength / 4;
        var fields = new PointerField[count];
        for (var i = 0; i < count; i++)
        {
            fields[i] = new PointerField(i * 4, PointerArrayPropertyName, PointerTarget.BlockRelative);
        }

        return fields;
    }

    public PointerArrayRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        if (length % 4 != 0)
        {
            throw new InvalidDataException($"{Kind} length {length} is not a multiple of 4.");
        }

        var slice = data.Slice(offset, length);
        var values = new int[length / 4];
        for (var i = 0; i < values.Length; i++)
        {
            values[i] = StructBinaryIO.ReadInt32(slice, i * 4);
        }

        return new PointerArrayRecord { Type = Kind, Values = values };
    }

    public byte[] Write(PointerArrayRecord value)
    {
        if (value.Values == null)
        {
            throw new InvalidDataException($"{Schema} ({Kind}) requires a non-null values array.");
        }

        var bytes = new byte[value.Values.Length * 4];
        for (var i = 0; i < value.Values.Length; i++)
        {
            StructBinaryIO.WriteInt32(bytes, i * 4, value.Values[i]);
        }

        return bytes;
    }

    public PointerArrayRecord FromJson(JsonElement json)
    {
        var record = JsonStructCodec.Deserialize<PointerArrayRecord>(json, Schema);
        JsonStructCodec.RequireValuesArray(record.Values, Schema, Kind);
        return record;
    }

    public void ToJson(PointerArrayRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}