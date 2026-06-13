using System.Text.Json;
using Astrolabe.Core.FileFormats.Geometry;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class RadiosityHeaderCodec : IStructCodec<RadiosityHeaderRecord>
{
    public const int Size = 0x10;

    public static RadiosityHeaderCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x04, "lods", PointerTarget.Any)
    ];

    public string Kind => "radiosityheader";
    public string Schema => "astrolabe.radiosity-header.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public RadiosityHeaderRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        return new RadiosityHeaderRecord
        {
            NumLod = StructBinaryIO.ReadUInt32(slice, 0x00),
            Lods = StructBinaryIO.ReadInt32(slice, 0x04),
            Unknown08 = StructBinaryIO.ReadUInt32(slice, 0x08),
            Unknown0C = StructBinaryIO.ReadUInt32(slice, 0x0C)
        };
    }

    public byte[] Write(RadiosityHeaderRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteUInt32(bytes, 0x00, value.NumLod);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.Lods);
        StructBinaryIO.WriteUInt32(bytes, 0x08, value.Unknown08);
        StructBinaryIO.WriteUInt32(bytes, 0x0C, value.Unknown0C);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(RadiosityHeaderRecord));
    }

    public RadiosityHeaderRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<RadiosityHeaderRecord>(json, Schema);

    public void ToJson(RadiosityHeaderRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}
