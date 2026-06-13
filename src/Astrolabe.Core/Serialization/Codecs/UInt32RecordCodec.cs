using System.Text.Json;
using Astrolabe.Core.FileFormats;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class UInt32RecordCodec : IStructCodec<UInt32Record>
{
    private UInt32RecordCodec(string kind)
    {
        Kind = kind;
    }

    public string Kind { get; }
    public string Schema => "astrolabe.uint32-record.v1";
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];

    public static UInt32RecordCodec BoundingVolume { get; } = new("boundingvolume");
    public static UInt32RecordCodec CollideMaterial { get; } = new("collidematerial");

    public UInt32Record Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        if (length % 4 != 0)
        {
            throw new InvalidDataException($"{Kind} length {length} is not a multiple of 4.");
        }

        var slice = data.Slice(offset, length);
        var values = new uint[length / 4];
        for (var i = 0; i < values.Length; i++)
        {
            values[i] = StructBinaryIO.ReadUInt32(slice, i * 4);
        }

        return new UInt32Record { Type = Kind, Values = values };
    }

    public byte[] Write(UInt32Record value)
    {
        var bytes = new byte[value.Values.Length * 4];
        for (var i = 0; i < value.Values.Length; i++)
        {
            StructBinaryIO.WriteUInt32(bytes, i * 4, value.Values[i]);
        }

        return bytes;
    }

    public UInt32Record FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<UInt32Record>(json, Schema);

    public void ToJson(UInt32Record value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}