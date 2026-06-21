using Astrolabe.Core.Hub;
using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class MindCodec : IStructCodec<MindRecord>
{
    public const int Size = 0x18;

    public static MindCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "aiModel", PointerTarget.BlockRelative),
        new PointerField(0x04, "intelligenceNormal", PointerTarget.BlockRelative),
        new PointerField(0x08, "intelligenceReflex", PointerTarget.BlockRelative),
        new PointerField(0x0C, "dsgMem", PointerTarget.BlockRelative)
    ];

    public string Kind => "mind";
    public string Schema => "astrolabe.mind.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public MindRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(MindRecord));
        return new MindRecord
        {
            AiModel = HubReferenceIO.Read(slice, 0x00),
            IntelligenceNormal = HubReferenceIO.Read(slice, 0x04),
            IntelligenceReflex = HubReferenceIO.Read(slice, 0x08),
            DsgMem = HubReferenceIO.Read(slice, 0x0C),
            Unknown10 = StructBinaryIO.ReadUInt32(slice, 0x10),
            Unknown14 = StructBinaryIO.ReadUInt32(slice, 0x14)
        };
    }

    public byte[] Write(MindRecord value)
    {
        var bytes = new byte[Size];
        HubReferenceIO.Write(bytes, 0x00, value.AiModel);
        HubReferenceIO.Write(bytes, 0x04, value.IntelligenceNormal);
        HubReferenceIO.Write(bytes, 0x08, value.IntelligenceReflex);
        HubReferenceIO.Write(bytes, 0x0C, value.DsgMem);
        StructBinaryIO.WriteUInt32(bytes, 0x10, value.Unknown10);
        StructBinaryIO.WriteUInt32(bytes, 0x14, value.Unknown14);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(MindRecord));
    }

    public MindRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<MindRecord>(json, Schema);

    public void ToJson(MindRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}