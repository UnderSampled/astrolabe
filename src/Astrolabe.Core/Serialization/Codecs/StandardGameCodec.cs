using Astrolabe.Core.Hub;
using System.Text.Json;
using Astrolabe.Core.FileFormats.Perso;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class StandardGameCodec : IStructCodec<StandardGameRecord>
{
    public const int Size = 0x30;

    public static StandardGameCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x0C, "superObject", PointerTarget.BlockRelative)
    ];

    public string Kind => "standardgame";
    public string Schema => "astrolabe.standard-game.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public StandardGameRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(StandardGameRecord));
        return new StandardGameRecord
        {
            ObjectType0 = StructBinaryIO.ReadUInt32(slice, 0x00),
            ObjectType1 = StructBinaryIO.ReadUInt32(slice, 0x04),
            ObjectType2 = StructBinaryIO.ReadUInt32(slice, 0x08),
            SuperObject = HubReferenceIO.Read(slice, 0x0C),
            Unknown10 = slice.AsSpan(0x10, 0x20).ToArray()
        };
    }

    public byte[] Write(StandardGameRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteUInt32(bytes, 0x00, value.ObjectType0);
        StructBinaryIO.WriteUInt32(bytes, 0x04, value.ObjectType1);
        StructBinaryIO.WriteUInt32(bytes, 0x08, value.ObjectType2);
        HubReferenceIO.Write(bytes, 0x0C, value.SuperObject);
        if (value.Unknown10.Length != 0x20)
        {
            throw new InvalidDataException("unknown10 must contain exactly 32 bytes.");
        }

        value.Unknown10.CopyTo(bytes.AsSpan(0x10));
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(StandardGameRecord));
    }

    public StandardGameRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<StandardGameRecord>(json, Schema);

    public void ToJson(StandardGameRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}