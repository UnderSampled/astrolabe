using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class AnimHierarchiesHeaderCodec : IStructCodec<AnimHierarchiesHeaderRecord>
{
    public const int Size = 0x08;

    public static AnimHierarchiesHeaderCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x04, "offHierarchies", PointerTarget.BlockRelative)
    ];

    public string Kind => "animhierarchiesheader";
    public string Schema => "astrolabe.anim-hierarchies-header.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public AnimHierarchiesHeaderRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(AnimHierarchiesHeaderRecord));
        return new AnimHierarchiesHeaderRecord
        {
            Count = StructBinaryIO.ReadUInt32(slice, 0x00),
            OffHierarchies = StructBinaryIO.ReadInt32(slice, 0x04)
        };
    }

    public byte[] Write(AnimHierarchiesHeaderRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteUInt32(bytes, 0x00, value.Count);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.OffHierarchies);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(AnimHierarchiesHeaderRecord));
    }

    public AnimHierarchiesHeaderRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<AnimHierarchiesHeaderRecord>(json, Schema);

    public void ToJson(AnimHierarchiesHeaderRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}