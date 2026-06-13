using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class IntelligenceCodec : IStructCodec<IntelligenceRecord>
{
    public const int Size = 0x18;

    public static IntelligenceCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "aiModel", PointerTarget.BlockRelative),
        new PointerField(0x04, "actionTree", PointerTarget.BlockRelative),
        new PointerField(0x08, "comport", PointerTarget.BlockRelative),
        new PointerField(0x0C, "lastComport", PointerTarget.BlockRelative),
        new PointerField(0x10, "actionTable", PointerTarget.BlockRelative),
        new PointerField(0x14, "defaultComport", PointerTarget.BlockRelative)
    ];

    public string Kind => "intelligence";
    public string Schema => "astrolabe.intelligence.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public IntelligenceRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(IntelligenceRecord));
        return new IntelligenceRecord
        {
            AiModel = StructBinaryIO.ReadInt32(slice, 0x00),
            ActionTree = StructBinaryIO.ReadInt32(slice, 0x04),
            Comport = StructBinaryIO.ReadInt32(slice, 0x08),
            LastComport = StructBinaryIO.ReadInt32(slice, 0x0C),
            ActionTable = StructBinaryIO.ReadInt32(slice, 0x10),
            DefaultComport = StructBinaryIO.ReadInt32(slice, 0x14)
        };
    }

    public byte[] Write(IntelligenceRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteInt32(bytes, 0x00, value.AiModel);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.ActionTree);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.Comport);
        StructBinaryIO.WriteInt32(bytes, 0x0C, value.LastComport);
        StructBinaryIO.WriteInt32(bytes, 0x10, value.ActionTable);
        StructBinaryIO.WriteInt32(bytes, 0x14, value.DefaultComport);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(IntelligenceRecord));
    }

    public IntelligenceRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<IntelligenceRecord>(json, Schema);

    public void ToJson(IntelligenceRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}