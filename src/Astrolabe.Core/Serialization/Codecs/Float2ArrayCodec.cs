using System.Text.Json;
using Astrolabe.Core.FileFormats;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class Float2ArrayCodec : IStructCodec<Float2ArrayRecord>
{
    private Float2ArrayCodec(string kind)
    {
        Kind = kind;
    }

    public string Kind { get; }
    public string Schema => "astrolabe.float2-array.v1";
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];

    public static Float2ArrayCodec Uvs { get; } = new("uvs");

    public Float2ArrayRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        if (length % 8 != 0)
        {
            throw new InvalidDataException($"{Kind} length {length} is not a multiple of 8.");
        }

        var slice = data.Slice(offset, length);
        var values = new float[length / 8][];
        for (var i = 0; i < values.Length; i++)
        {
            values[i] =
            [
                StructBinaryIO.ReadSingle(slice, i * 8),
                StructBinaryIO.ReadSingle(slice, i * 8 + 4)
            ];
        }

        return new Float2ArrayRecord { Type = Kind, Values = values };
    }

    public byte[] Write(Float2ArrayRecord value)
    {
        if (value.Values == null)
        {
            throw new InvalidDataException($"{Schema} ({Kind}) requires a non-null values array.");
        }

        var bytes = new byte[value.Values.Length * 8];
        for (var i = 0; i < value.Values.Length; i++)
        {
            if (value.Values[i].Length != 2)
            {
                throw new InvalidDataException($"{Kind}[{i}] must contain exactly 2 values.");
            }

            StructBinaryIO.WriteSingle(bytes, i * 8, value.Values[i][0]);
            StructBinaryIO.WriteSingle(bytes, i * 8 + 4, value.Values[i][1]);
        }

        return bytes;
    }

    public Float2ArrayRecord FromJson(JsonElement json)
    {
        var record = JsonStructCodec.Deserialize<Float2ArrayRecord>(json, Schema);
        JsonStructCodec.RequireValuesArray(record.Values, Schema, Kind);

        for (var i = 0; i < record.Values.Length; i++)
        {
            if (record.Values[i] == null || record.Values[i].Length != 2)
            {
                throw new InvalidDataException($"{Kind}[{i}] must contain exactly 2 values.");
            }
        }

        return record;
    }

    public void ToJson(Float2ArrayRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}