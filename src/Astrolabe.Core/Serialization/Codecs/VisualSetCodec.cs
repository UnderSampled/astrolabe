using System.Text.Json;
using Astrolabe.Core.FileFormats.Geometry;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class VisualSetCodec : IStructCodec<VisualSetRecord>
{
    public const int Size = 0x10;

    public static VisualSetCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x08, "lodDistances", PointerTarget.BlockRelative),
        new PointerField(0x0C, "lodDataOffsets", PointerTarget.BlockRelative)
    ];

    public string Kind => "visualset";
    public string Schema => "astrolabe.visual-set.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public VisualSetRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        return new VisualSetRecord
        {
            Unknown0 = StructBinaryIO.ReadUInt32(slice, 0x00),
            NumberOfLod = StructBinaryIO.ReadUInt16(slice, 0x04),
            VisualSetType = StructBinaryIO.ReadUInt16(slice, 0x06),
            LodDistances = StructBinaryIO.ReadInt32(slice, 0x08),
            LodDataOffsets = StructBinaryIO.ReadInt32(slice, 0x0C)
        };
    }

    public byte[] Write(VisualSetRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteUInt32(bytes, 0x00, value.Unknown0);
        StructBinaryIO.WriteUInt16(bytes, 0x04, value.NumberOfLod);
        StructBinaryIO.WriteUInt16(bytes, 0x06, value.VisualSetType);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.LodDistances);
        StructBinaryIO.WriteInt32(bytes, 0x0C, value.LodDataOffsets);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(VisualSetRecord));
    }

    public VisualSetRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<VisualSetRecord>(json, Schema);

    public void ToJson(VisualSetRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}
