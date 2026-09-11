using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;
using Astrolabe.Core.Hub;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class DsgVarCodec : IStructCodec<DsgVarRecord>
{
    public const int Size = 0x10;

    public static DsgVarCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "memBuffer", PointerTarget.BlockRelative),
        new PointerField(0x04, "infos", PointerTarget.BlockRelative)
    ];

    public string Kind => "dsgvar";
    public string Schema => "astrolabe.dsg-var.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public DsgVarRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(DsgVarRecord));
        return new DsgVarRecord
        {
            MemBuffer = HubReferenceIO.Read(slice, 0x00),
            Infos = HubReferenceIO.Read(slice, 0x04),
            DsgMemBufferLength = StructBinaryIO.ReadUInt32(slice, 0x08),
            // amountOfInfos is a byte + 3 pad on Hype/R2 (non-Revolution).
            AmountOfInfos = StructBinaryIO.ReadUInt32(slice, 0x0C)
        };
    }

    public byte[] Write(DsgVarRecord value)
    {
        var bytes = new byte[Size];
        HubReferenceIO.Write(bytes, 0x00, value.MemBuffer);
        HubReferenceIO.Write(bytes, 0x04, value.Infos);
        StructBinaryIO.WriteUInt32(bytes, 0x08, value.DsgMemBufferLength);
        StructBinaryIO.WriteUInt32(bytes, 0x0C, value.AmountOfInfos);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(DsgVarRecord));
    }

    public DsgVarRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<DsgVarRecord>(json, Schema);

    public void ToJson(DsgVarRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}
