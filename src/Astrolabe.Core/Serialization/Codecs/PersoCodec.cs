using Astrolabe.Core.Hub;
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
            Perso3dData = HubReferenceIO.Read(slice, 0x00),
            StdGame = HubReferenceIO.Read(slice, 0x04),
            Dynam = HubReferenceIO.Read(slice, 0x08),
            Unknown0C = StructBinaryIO.ReadUInt32(slice, 0x0C),
            Brain = HubReferenceIO.Read(slice, 0x10),
            Camera = HubReferenceIO.Read(slice, 0x14),
            CollSet = HubReferenceIO.Read(slice, 0x18),
            MsWay = HubReferenceIO.Read(slice, 0x1C),
            MsLight = HubReferenceIO.Read(slice, 0x20),
            Unknown24 = StructBinaryIO.ReadUInt32(slice, 0x24),
            SectInfo = HubReferenceIO.Read(slice, 0x28),
            Unknown2C = StructBinaryIO.ReadUInt32(slice, 0x2C),
            Unknown30 = HubReferenceIO.Read(slice, 0x30),
            Unknown34 = StructBinaryIO.ReadUInt32(slice, 0x34),
            Unknown38 = StructBinaryIO.ReadUInt32(slice, 0x38),
            Unknown3C = StructBinaryIO.ReadUInt32(slice, 0x3C)
        };
    }

    public byte[] Write(PersoRecord value)
    {
        var bytes = new byte[Size];
        HubReferenceIO.Write(bytes, 0x00, value.Perso3dData);
        HubReferenceIO.Write(bytes, 0x04, value.StdGame);
        HubReferenceIO.Write(bytes, 0x08, value.Dynam);
        StructBinaryIO.WriteUInt32(bytes, 0x0C, value.Unknown0C);
        HubReferenceIO.Write(bytes, 0x10, value.Brain);
        HubReferenceIO.Write(bytes, 0x14, value.Camera);
        HubReferenceIO.Write(bytes, 0x18, value.CollSet);
        HubReferenceIO.Write(bytes, 0x1C, value.MsWay);
        HubReferenceIO.Write(bytes, 0x20, value.MsLight);
        StructBinaryIO.WriteUInt32(bytes, 0x24, value.Unknown24);
        HubReferenceIO.Write(bytes, 0x28, value.SectInfo);
        StructBinaryIO.WriteUInt32(bytes, 0x2C, value.Unknown2C);
        HubReferenceIO.Write(bytes, 0x30, value.Unknown30);
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