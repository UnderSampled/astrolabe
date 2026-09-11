using System.Text.Json;
using Astrolabe.Core.FileFormats.Perso;
using Astrolabe.Core.Hub;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class DynamCodec : IStructCodec<DynamRecord>
{
    public const int Size = 0x80;

    public static DynamCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x10, "field10", PointerTarget.BlockRelative),
        new PointerField(0x14, "field14", PointerTarget.BlockRelative)
    ];

    public string Kind => "dynam";
    public string Schema => "astrolabe.dynam.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public DynamRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(DynamRecord));
        return new DynamRecord
        {
            Unknown00 = StructBinaryIO.ReadUInt32(slice, 0x00),
            Unknown04 = StructBinaryIO.ReadUInt32(slice, 0x04),
            Unknown08 = StructBinaryIO.ReadUInt32(slice, 0x08),
            Unknown0C = StructBinaryIO.ReadUInt32(slice, 0x0C),
            Field10 = HubReferenceIO.Read(slice, 0x10),
            Field14 = HubReferenceIO.Read(slice, 0x14),
            Unknown18 = slice.AsSpan(0x18, Size - 0x18).ToArray()
        };
    }

    public byte[] Write(DynamRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteUInt32(bytes, 0x00, value.Unknown00);
        StructBinaryIO.WriteUInt32(bytes, 0x04, value.Unknown04);
        StructBinaryIO.WriteUInt32(bytes, 0x08, value.Unknown08);
        StructBinaryIO.WriteUInt32(bytes, 0x0C, value.Unknown0C);
        HubReferenceIO.Write(bytes, 0x10, value.Field10);
        HubReferenceIO.Write(bytes, 0x14, value.Field14);
        WriteBytes(bytes, 0x18, value.Unknown18, Size - 0x18);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(DynamRecord));
    }

    public DynamRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<DynamRecord>(json, Schema);

    public void ToJson(DynamRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);

    private static void WriteBytes(Span<byte> destination, int offset, IReadOnlyList<byte> values, int expectedLength)
    {
        if (values.Count != expectedLength)
        {
            throw new InvalidDataException($"unknown18 must contain exactly {expectedLength} bytes.");
        }

        for (var i = 0; i < values.Count; i++)
        {
            destination[offset + i] = values[i];
        }
    }
}
