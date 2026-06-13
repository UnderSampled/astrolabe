using System.Text.Json;
using Astrolabe.Core.FileFormats.Geometry;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class ElementSpritesCodec : IStructCodec<ElementSpritesRecord>
{
    public const int Size = 0x20;

    public static ElementSpritesCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x04, "sprites", PointerTarget.BlockRelative),
        new PointerField(0x10, "unknown10", PointerTarget.BlockRelative)
    ];

    public string Kind => "elementsprites";
    public string Schema => "astrolabe.element-sprites.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public ElementSpritesRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        return new ElementSpritesRecord
        {
            NumSprites = StructBinaryIO.ReadUInt32(slice, 0x00),
            Sprites = StructBinaryIO.ReadInt32(slice, 0x04),
            Unknown08 = StructBinaryIO.ReadUInt32(slice, 0x08),
            Unknown0C = StructBinaryIO.ReadUInt32(slice, 0x0C),
            Unknown10 = StructBinaryIO.ReadInt32(slice, 0x10),
            Unknown14 = StructBinaryIO.ReadSingle(slice, 0x14),
            Unknown18 = StructBinaryIO.ReadSingle(slice, 0x18),
            Unknown1C = StructBinaryIO.ReadSingle(slice, 0x1C)
        };
    }

    public byte[] Write(ElementSpritesRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteUInt32(bytes, 0x00, value.NumSprites);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.Sprites);
        StructBinaryIO.WriteUInt32(bytes, 0x08, value.Unknown08);
        StructBinaryIO.WriteUInt32(bytes, 0x0C, value.Unknown0C);
        StructBinaryIO.WriteInt32(bytes, 0x10, value.Unknown10);
        StructBinaryIO.WriteSingle(bytes, 0x14, value.Unknown14);
        StructBinaryIO.WriteSingle(bytes, 0x18, value.Unknown18);
        StructBinaryIO.WriteSingle(bytes, 0x1C, value.Unknown1C);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(ElementSpritesRecord));
    }

    public ElementSpritesRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<ElementSpritesRecord>(json, Schema);

    public void ToJson(ElementSpritesRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}