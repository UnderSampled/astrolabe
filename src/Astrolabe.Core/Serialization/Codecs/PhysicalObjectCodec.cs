using System.Text.Json;
using Astrolabe.Core.FileFormats.Geometry;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class PhysicalObjectCodec : IStructCodec<PhysicalObjectRecord>
{
    public const int Size = 0x10;

    public static PhysicalObjectCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "visualSet", PointerTarget.BlockRelative),
        new PointerField(0x04, "collideSet", PointerTarget.BlockRelative),
        new PointerField(0x08, "visualBoundingVolume", PointerTarget.BlockRelative)
    ];

    public string Kind => "physicalobject";
    public string Schema => "astrolabe.physical-object.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public PhysicalObjectRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        return new PhysicalObjectRecord
        {
            VisualSet = StructBinaryIO.ReadInt32(slice, 0x00),
            CollideSet = StructBinaryIO.ReadInt32(slice, 0x04),
            VisualBoundingVolume = StructBinaryIO.ReadInt32(slice, 0x08),
            Unknown0 = StructBinaryIO.ReadInt32(slice, 0x0C)
        };
    }

    public byte[] Write(PhysicalObjectRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteInt32(bytes, 0x00, value.VisualSet);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.CollideSet);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.VisualBoundingVolume);
        StructBinaryIO.WriteInt32(bytes, 0x0C, value.Unknown0);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(PhysicalObjectRecord));
    }

    public PhysicalObjectRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<PhysicalObjectRecord>(json, Schema);

    public void ToJson(PhysicalObjectRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}