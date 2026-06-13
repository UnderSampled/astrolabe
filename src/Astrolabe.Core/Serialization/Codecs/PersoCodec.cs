using System.Text.Json;
using Astrolabe.Core.FileFormats.Perso;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class PersoCodec : IStructCodec<PersoRecord>
{
    public const int Size = 0x40;

    public static PersoCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "perso3dData", PointerTarget.BlockRelative),
        new PointerField(0x04, "stdGame", PointerTarget.BlockRelative),
        new PointerField(0x08, "dynam", PointerTarget.BlockRelative),
        new PointerField(0x10, "brain", PointerTarget.BlockRelative),
        new PointerField(0x14, "camera", PointerTarget.BlockRelative),
        new PointerField(0x18, "collSet", PointerTarget.BlockRelative),
        new PointerField(0x1C, "msWay", PointerTarget.BlockRelative),
        new PointerField(0x20, "msLight", PointerTarget.BlockRelative),
        new PointerField(0x28, "sectInfo", PointerTarget.BlockRelative),
        new PointerField(0x30, "unknown30", PointerTarget.BlockRelative)
    ];

    public string Kind => "perso";
    public string Schema => "astrolabe.perso.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public PersoRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(PersoRecord));
        return new PersoRecord
        {
            Perso3dData = StructBinaryIO.ReadInt32(slice, 0x00),
            StdGame = StructBinaryIO.ReadInt32(slice, 0x04),
            Dynam = StructBinaryIO.ReadInt32(slice, 0x08),
            Unknown0C = StructBinaryIO.ReadUInt32(slice, 0x0C),
            Brain = StructBinaryIO.ReadInt32(slice, 0x10),
            Camera = StructBinaryIO.ReadInt32(slice, 0x14),
            CollSet = StructBinaryIO.ReadInt32(slice, 0x18),
            MsWay = StructBinaryIO.ReadInt32(slice, 0x1C),
            MsLight = StructBinaryIO.ReadInt32(slice, 0x20),
            Unknown24 = StructBinaryIO.ReadUInt32(slice, 0x24),
            SectInfo = StructBinaryIO.ReadInt32(slice, 0x28),
            Unknown2C = StructBinaryIO.ReadUInt32(slice, 0x2C),
            Unknown30 = StructBinaryIO.ReadInt32(slice, 0x30),
            Unknown34 = StructBinaryIO.ReadUInt32(slice, 0x34),
            Unknown38 = StructBinaryIO.ReadUInt32(slice, 0x38),
            Unknown3C = StructBinaryIO.ReadUInt32(slice, 0x3C)
        };
    }

    public byte[] Write(PersoRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteInt32(bytes, 0x00, value.Perso3dData);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.StdGame);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.Dynam);
        StructBinaryIO.WriteUInt32(bytes, 0x0C, value.Unknown0C);
        StructBinaryIO.WriteInt32(bytes, 0x10, value.Brain);
        StructBinaryIO.WriteInt32(bytes, 0x14, value.Camera);
        StructBinaryIO.WriteInt32(bytes, 0x18, value.CollSet);
        StructBinaryIO.WriteInt32(bytes, 0x1C, value.MsWay);
        StructBinaryIO.WriteInt32(bytes, 0x20, value.MsLight);
        StructBinaryIO.WriteUInt32(bytes, 0x24, value.Unknown24);
        StructBinaryIO.WriteInt32(bytes, 0x28, value.SectInfo);
        StructBinaryIO.WriteUInt32(bytes, 0x2C, value.Unknown2C);
        StructBinaryIO.WriteInt32(bytes, 0x30, value.Unknown30);
        StructBinaryIO.WriteUInt32(bytes, 0x34, value.Unknown34);
        StructBinaryIO.WriteUInt32(bytes, 0x38, value.Unknown38);
        StructBinaryIO.WriteUInt32(bytes, 0x3C, value.Unknown3C);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(PersoRecord));
    }

    public PersoRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<PersoRecord>(json, Schema);

    public void ToJson(PersoRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}