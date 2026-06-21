using Astrolabe.Core.Hub;
using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class BrainCodec : IStructCodec<BrainRecord>
{
    public const int Size = 0x0C;

    public static BrainCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "mind", PointerTarget.BlockRelative)
    ];

    public string Kind => "brain";
    public string Schema => "astrolabe.brain.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public BrainRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(BrainRecord));
        return new BrainRecord
        {
            Mind = HubReferenceIO.Read(slice, 0x00),
            Unknown04 = StructBinaryIO.ReadInt32(slice, 0x04),
            Unknown08 = StructBinaryIO.ReadInt32(slice, 0x08)
        };
    }

    public byte[] Write(BrainRecord value)
    {
        var bytes = new byte[Size];
        HubReferenceIO.Write(bytes, 0x00, value.Mind);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.Unknown04);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.Unknown08);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(BrainRecord));
    }

    public BrainRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<BrainRecord>(json, Schema);

    public void ToJson(BrainRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}