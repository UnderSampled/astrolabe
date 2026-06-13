using System.Text.Json;
using Astrolabe.Core.FileFormats.Geometry;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class IpoCodec : IStructCodec<IpoRecord>
{
    public const int Size = 8;

    public static IpoCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "physicalObject", PointerTarget.BlockRelative),
        new PointerField(0x04, "radiosity", PointerTarget.BlockRelative)
    ];

    public string Kind => "ipo";
    public string Schema => "astrolabe.ipo.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public IpoRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        return new IpoRecord
        {
            PhysicalObject = StructBinaryIO.ReadInt32(slice, 0x00),
            Radiosity = StructBinaryIO.ReadInt32(slice, 0x04)
        };
    }

    public byte[] Write(IpoRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteInt32(bytes, 0x00, value.PhysicalObject);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.Radiosity);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(IpoRecord));
    }

    public IpoRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<IpoRecord>(json, Schema);

    public void ToJson(IpoRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}