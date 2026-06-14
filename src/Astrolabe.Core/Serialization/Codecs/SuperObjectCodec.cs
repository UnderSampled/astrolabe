using System.Text.Json;
using Astrolabe.Core.FileFormats;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class SuperObjectCodec : IStructCodec<SuperObjectRecord>
{
    public const int Size = 0x38;

    public static SuperObjectCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "typeCode", PointerTarget.BlockRelative, RequiresVmRange: true),
        new PointerField(0x04, "offData", PointerTarget.BlockRelative),
        new PointerField(0x08, "childrenHead", PointerTarget.BlockRelative),
        new PointerField(0x0C, "childrenTail", PointerTarget.BlockRelative),
        new PointerField(0x10, "childrenCount", PointerTarget.BlockRelative, RequiresVmRange: true),
        new PointerField(0x14, "brotherNext", PointerTarget.BlockRelative),
        new PointerField(0x18, "brotherPrev", PointerTarget.BlockRelative),
        new PointerField(0x1C, "parent", PointerTarget.BlockRelative),
        new PointerField(0x20, "matrix", PointerTarget.BlockRelative),
        new PointerField(0x24, "staticMatrix", PointerTarget.BlockRelative),
        new PointerField(0x28, "globalMatrix", PointerTarget.BlockRelative),
        new PointerField(0x34, "boundingVolume", PointerTarget.BlockRelative)
    ];

    public string Kind => "superObject";
    public string Schema => "astrolabe.super-object.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public SuperObjectRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        var typeCode = StructBinaryIO.ReadUInt32(slice, 0x00);

        return new SuperObjectRecord
        {
            TypeCode = typeCode,
            Type = TrackingSuperObjectReader.GetSuperObjectType(typeCode).ToString(),
            OffData = StructBinaryIO.ReadInt32(slice, 0x04),
            ChildrenHead = StructBinaryIO.ReadInt32(slice, 0x08),
            ChildrenTail = StructBinaryIO.ReadInt32(slice, 0x0C),
            ChildrenCount = StructBinaryIO.ReadUInt32(slice, 0x10),
            BrotherNext = StructBinaryIO.ReadInt32(slice, 0x14),
            BrotherPrev = StructBinaryIO.ReadInt32(slice, 0x18),
            Parent = StructBinaryIO.ReadInt32(slice, 0x1C),
            Matrix = StructBinaryIO.ReadInt32(slice, 0x20),
            StaticMatrix = StructBinaryIO.ReadInt32(slice, 0x24),
            GlobalMatrix = StructBinaryIO.ReadInt32(slice, 0x28),
            DrawFlags = StructBinaryIO.ReadUInt32(slice, 0x2C),
            Flags = StructBinaryIO.ReadUInt32(slice, 0x30),
            BoundingVolume = StructBinaryIO.ReadInt32(slice, 0x34)
        };
    }

    public byte[] Write(SuperObjectRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteUInt32(bytes, 0x00, value.TypeCode);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.OffData);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.ChildrenHead);
        StructBinaryIO.WriteInt32(bytes, 0x0C, value.ChildrenTail);
        StructBinaryIO.WriteUInt32(bytes, 0x10, value.ChildrenCount);
        StructBinaryIO.WriteInt32(bytes, 0x14, value.BrotherNext);
        StructBinaryIO.WriteInt32(bytes, 0x18, value.BrotherPrev);
        StructBinaryIO.WriteInt32(bytes, 0x1C, value.Parent);
        StructBinaryIO.WriteInt32(bytes, 0x20, value.Matrix);
        StructBinaryIO.WriteInt32(bytes, 0x24, value.StaticMatrix);
        StructBinaryIO.WriteInt32(bytes, 0x28, value.GlobalMatrix);
        StructBinaryIO.WriteUInt32(bytes, 0x2C, value.DrawFlags);
        StructBinaryIO.WriteUInt32(bytes, 0x30, value.Flags);
        StructBinaryIO.WriteInt32(bytes, 0x34, value.BoundingVolume);
        return bytes;
    }

    public SuperObjectRecord FromJson(JsonElement json)
    {
        var value = json.Deserialize<SuperObjectRecord>(JsonStructCodec.Options)
            ?? throw new InvalidDataException($"Could not deserialize {Schema} JSON.");

        if (value.Schema != Schema && value.Schema != "astrolabe.scene-node.v1")
        {
            throw new InvalidDataException($"Unsupported super object schema: {value.Schema}");
        }

        return value;
    }

    public void ToJson(SuperObjectRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}
