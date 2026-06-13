using System.Text.Json;
using Astrolabe.Core.FileFormats.Perso;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class Perso3dDataCodec : IStructCodec<Perso3dDataRecord>
{
    public const int Size = 0x20;

    public static Perso3dDataCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "stateInitial", PointerTarget.BlockRelative),
        new PointerField(0x04, "stateCurrent", PointerTarget.BlockRelative),
        new PointerField(0x08, "state2", PointerTarget.BlockRelative),
        new PointerField(0x0C, "objectList", PointerTarget.BlockRelative),
        new PointerField(0x10, "objectListInitial", PointerTarget.BlockRelative),
        new PointerField(0x14, "family", PointerTarget.BlockRelative)
    ];

    public string Kind => "perso3ddata";
    public string Schema => "astrolabe.perso-3d-data.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public Perso3dDataRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(Perso3dDataRecord));
        return new Perso3dDataRecord
        {
            StateInitial = StructBinaryIO.ReadInt32(slice, 0x00),
            StateCurrent = StructBinaryIO.ReadInt32(slice, 0x04),
            State2 = StructBinaryIO.ReadInt32(slice, 0x08),
            ObjectList = StructBinaryIO.ReadInt32(slice, 0x0C),
            ObjectListInitial = StructBinaryIO.ReadInt32(slice, 0x10),
            Family = StructBinaryIO.ReadInt32(slice, 0x14),
            Unknown18 = StructBinaryIO.ReadInt32(slice, 0x18),
            Unknown1C = StructBinaryIO.ReadInt32(slice, 0x1C)
        };
    }

    public byte[] Write(Perso3dDataRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteInt32(bytes, 0x00, value.StateInitial);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.StateCurrent);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.State2);
        StructBinaryIO.WriteInt32(bytes, 0x0C, value.ObjectList);
        StructBinaryIO.WriteInt32(bytes, 0x10, value.ObjectListInitial);
        StructBinaryIO.WriteInt32(bytes, 0x14, value.Family);
        StructBinaryIO.WriteInt32(bytes, 0x18, value.Unknown18);
        StructBinaryIO.WriteInt32(bytes, 0x1C, value.Unknown1C);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(Perso3dDataRecord));
    }

    public Perso3dDataRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<Perso3dDataRecord>(json, Schema);

    public void ToJson(Perso3dDataRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}