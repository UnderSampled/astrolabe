using System.Text.Json;
using Astrolabe.Core.FileFormats;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class UInt16ArrayCodec : IStructCodec<UInt16ArrayRecord>
{
    private UInt16ArrayCodec(string kind)
    {
        Kind = kind;
    }

    public string Kind { get; }
    public string Schema => "astrolabe.uint16-array.v1";
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];

    public static UInt16ArrayCodec ElementTypes { get; } = new("elementtypes");
    public static UInt16ArrayCodec VertexIndices { get; } = new("vertexindices");
    public static UInt16ArrayCodec UvMapping { get; } = new("uvmapping");
    public static UInt16ArrayCodec Triangles { get; } = new("triangles");

    public UInt16ArrayRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        if (length % 2 != 0)
        {
            throw new InvalidDataException($"{Kind} length {length} is not a multiple of 2.");
        }

        var slice = data.Slice(offset, length);
        var values = new ushort[length / 2];
        for (var i = 0; i < values.Length; i++)
        {
            values[i] = StructBinaryIO.ReadUInt16(slice, i * 2);
        }

        return new UInt16ArrayRecord { Type = Kind, Values = values };
    }

    public byte[] Write(UInt16ArrayRecord value)
    {
        if (value.Values == null)
        {
            throw new InvalidDataException($"{Schema} ({Kind}) requires a non-null values array.");
        }

        var bytes = new byte[value.Values.Length * 2];
        for (var i = 0; i < value.Values.Length; i++)
        {
            StructBinaryIO.WriteUInt16(bytes, i * 2, value.Values[i]);
        }

        return bytes;
    }

    public UInt16ArrayRecord FromJson(JsonElement json)
    {
        var record = JsonStructCodec.Deserialize<UInt16ArrayRecord>(json, Schema);
        JsonStructCodec.RequireValuesArray(record.Values, Schema, Kind);
        return record;
    }

    public void ToJson(UInt16ArrayRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}