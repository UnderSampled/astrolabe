using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;
using Astrolabe.Core.Hub;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class ActionTableCodec : IStructCodec<ActionTableRecord>
{
    public const int Size = 0x40;

    public static ActionTableCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "header", PointerTarget.BlockRelative)
    ];

    public string Kind => "actiontable";
    public string Schema => "astrolabe.action-table.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public ActionTableRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(ActionTableRecord));
        return new ActionTableRecord
        {
            Header = HubReferenceIO.Read(slice, 0x00),
            Unknown04 = StructBinaryIO.ReadUInt32(slice, 0x04),
            Unknown08 = StructBinaryIO.ReadUInt32(slice, 0x08),
            Unknown0C = slice.AsSpan(0x0C, Size - 0x0C).ToArray()
        };
    }

    public byte[] Write(ActionTableRecord value)
    {
        var bytes = new byte[Size];
        HubReferenceIO.Write(bytes, 0x00, value.Header);
        StructBinaryIO.WriteUInt32(bytes, 0x04, value.Unknown04);
        StructBinaryIO.WriteUInt32(bytes, 0x08, value.Unknown08);
        WriteBytes(bytes, 0x0C, value.Unknown0C, Size - 0x0C);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(ActionTableRecord));
    }

    public ActionTableRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<ActionTableRecord>(json, Schema);

    public void ToJson(ActionTableRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);

    private static void WriteBytes(Span<byte> destination, int offset, IReadOnlyList<byte> values, int expectedLength)
    {
        if (values.Count != expectedLength)
        {
            throw new InvalidDataException($"unknown0C must contain exactly {expectedLength} bytes.");
        }

        for (var i = 0; i < values.Count; i++)
        {
            destination[offset + i] = values[i];
        }
    }
}
