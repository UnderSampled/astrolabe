using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;
using Astrolabe.Core.Hub;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class DsgMemCodec : IStructCodec<DsgMemRecord>
{
    public const int Size = 0x0C;

    public static DsgMemCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "dsgVarPtr", PointerTarget.BlockRelative),
        new PointerField(0x04, "memBufferInitial", PointerTarget.BlockRelative),
        new PointerField(0x08, "memBuffer", PointerTarget.BlockRelative)
    ];

    public string Kind => "dsgmem";
    public string Schema => "astrolabe.dsg-mem.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public DsgMemRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(DsgMemRecord));
        return new DsgMemRecord
        {
            DsgVarPtr = HubReferenceIO.Read(slice, 0x00),
            MemBufferInitial = HubReferenceIO.Read(slice, 0x04),
            MemBuffer = HubReferenceIO.Read(slice, 0x08)
        };
    }

    public byte[] Write(DsgMemRecord value)
    {
        var bytes = new byte[Size];
        HubReferenceIO.Write(bytes, 0x00, value.DsgVarPtr);
        HubReferenceIO.Write(bytes, 0x04, value.MemBufferInitial);
        HubReferenceIO.Write(bytes, 0x08, value.MemBuffer);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(DsgMemRecord));
    }

    public DsgMemRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<DsgMemRecord>(json, Schema);

    public void ToJson(DsgMemRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}
