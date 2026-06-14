using System.Text.Json;
using Astrolabe.Core.FileFormats.Perso;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class SpawnableEntryCodec : IStructCodec<SpawnableEntryRecord>
{
    public const int Size = 0x14;

    public static SpawnableEntryCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "next", PointerTarget.BlockRelative),
        new PointerField(0x04, "prev", PointerTarget.BlockRelative),
        new PointerField(0x08, "hdr", PointerTarget.BlockRelative),
        new PointerField(0x10, "perso", PointerTarget.Any)
    ];

    public string Kind => "spawnableentry";
    public string Schema => "astrolabe.spawnable-entry.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public SpawnableEntryRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(SpawnableEntryRecord));
        return new SpawnableEntryRecord
        {
            Next = StructBinaryIO.ReadInt32(slice, 0x00),
            Prev = StructBinaryIO.ReadInt32(slice, 0x04),
            Hdr = StructBinaryIO.ReadInt32(slice, 0x08),
            Index = StructBinaryIO.ReadUInt32(slice, 0x0C),
            Perso = StructBinaryIO.ReadInt32(slice, 0x10)
        };
    }

    public byte[] Write(SpawnableEntryRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteInt32(bytes, 0x00, value.Next);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.Prev);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.Hdr);
        StructBinaryIO.WriteUInt32(bytes, 0x0C, value.Index);
        StructBinaryIO.WriteInt32(bytes, 0x10, value.Perso);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(SpawnableEntryRecord));
    }

    public SpawnableEntryRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<SpawnableEntryRecord>(json, Schema);

    public void ToJson(SpawnableEntryRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}