using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class AiModelCodec : IStructCodec<AiModelRecord>
{
    public const int Size = 0x14;

    public static AiModelCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "behaviorsNormal", PointerTarget.BlockRelative),
        new PointerField(0x04, "behaviorsReflex", PointerTarget.BlockRelative),
        new PointerField(0x08, "dsgVar", PointerTarget.BlockRelative),
        new PointerField(0x0C, "macros", PointerTarget.BlockRelative)
    ];

    public string Kind => "aimodel";
    public string Schema => "astrolabe.ai-model.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public AiModelRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(AiModelRecord));
        return new AiModelRecord
        {
            BehaviorsNormal = StructBinaryIO.ReadInt32(slice, 0x00),
            BehaviorsReflex = StructBinaryIO.ReadInt32(slice, 0x04),
            DsgVar = StructBinaryIO.ReadInt32(slice, 0x08),
            Macros = StructBinaryIO.ReadInt32(slice, 0x0C),
            Unknown10 = StructBinaryIO.ReadUInt32(slice, 0x10)
        };
    }

    public byte[] Write(AiModelRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteInt32(bytes, 0x00, value.BehaviorsNormal);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.BehaviorsReflex);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.DsgVar);
        StructBinaryIO.WriteInt32(bytes, 0x0C, value.Macros);
        StructBinaryIO.WriteUInt32(bytes, 0x10, value.Unknown10);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(AiModelRecord));
    }

    public AiModelRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<AiModelRecord>(json, Schema);

    public void ToJson(AiModelRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}