using System.Text.Json;
using Astrolabe.Core.FileFormats;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class Float3ArrayCodec : IStructCodec<Float3ArrayRecord>
{
    private Float3ArrayCodec(string kind)
    {
        Kind = kind;
    }

    public string Kind { get; }
    public string Schema => "astrolabe.float3-array.v1";
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];

    public static Float3ArrayCodec Vertices { get; } = new("vertices");
    public static Float3ArrayCodec Normals { get; } = new("normals");
    public static Float3ArrayCodec TriangleNormals { get; } = new("trianglenormals");

    public Float3ArrayRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        if (length % 12 != 0)
        {
            throw new InvalidDataException($"{Kind} length {length} is not a multiple of 12.");
        }

        var slice = data.Slice(offset, length);
        var values = new float[length / 12][];
        for (var i = 0; i < values.Length; i++)
        {
            var entry = new float[3];
            JsonStructCodec.ReadFloat3(slice, i * 12, entry);
            values[i] = entry;
        }

        return new Float3ArrayRecord { Type = Kind, Values = values };
    }

    public byte[] Write(Float3ArrayRecord value)
    {
        var bytes = new byte[value.Values.Length * 12];
        for (var i = 0; i < value.Values.Length; i++)
        {
            JsonStructCodec.WriteFloat3(bytes, i * 12, value.Values[i], $"{Kind}[{i}]");
        }

        return bytes;
    }

    public Float3ArrayRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<Float3ArrayRecord>(json, Schema);

    public void ToJson(Float3ArrayRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}