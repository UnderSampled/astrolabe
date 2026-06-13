using System.Text.Json;
using Astrolabe.Core.FileFormats;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class FloatArrayCodec : IStructCodec<FloatArrayRecord>
{
    private FloatArrayCodec(string kind)
    {
        Kind = kind;
    }

    public string Kind { get; }
    public string Schema => "astrolabe.float-array.v1";
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];

    public static FloatArrayCodec LodDistances { get; } = new("loddistances");

    public FloatArrayRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        if (length % 4 != 0)
        {
            throw new InvalidDataException($"{Kind} length {length} is not a multiple of 4.");
        }

        var slice = data.Slice(offset, length);
        var values = new float[length / 4];
        for (var i = 0; i < values.Length; i++)
        {
            values[i] = StructBinaryIO.ReadSingle(slice, i * 4);
        }

        return new FloatArrayRecord { Type = Kind, Values = values };
    }

    public byte[] Write(FloatArrayRecord value)
    {
        if (value.Values == null)
        {
            throw new InvalidDataException($"{Schema} ({Kind}) requires a non-null values array.");
        }

        var bytes = new byte[value.Values.Length * 4];
        for (var i = 0; i < value.Values.Length; i++)
        {
            StructBinaryIO.WriteSingle(bytes, i * 4, value.Values[i]);
        }

        return bytes;
    }

    public FloatArrayRecord FromJson(JsonElement json)
    {
        var record = JsonStructCodec.Deserialize<FloatArrayRecord>(json, Schema);
        JsonStructCodec.RequireValuesArray(record.Values, Schema, Kind);
        return record;
    }

    public void ToJson(FloatArrayRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}