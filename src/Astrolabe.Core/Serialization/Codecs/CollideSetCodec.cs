using Astrolabe.Core.Hub;
using System.Text.Json;
using Astrolabe.Core.FileFormats.Geometry;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class CollideSetCodec : IStructCodec<CollideSetRecord>
{
    public const int Size = 0x14;

    public static CollideSetCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "zdxList", PointerTarget.BlockRelative),
        new PointerField(0x04, "zddList", PointerTarget.BlockRelative),
        new PointerField(0x08, "zdeList", PointerTarget.BlockRelative)
    ];

    public string Kind => "collideset";
    public string Schema => "astrolabe.collide-set.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public CollideSetRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(CollideSetRecord));
        return new CollideSetRecord
        {
            ZdxList = HubReferenceIO.Read(slice, 0x00),
            ZddList = HubReferenceIO.Read(slice, 0x04),
            ZdeList = HubReferenceIO.Read(slice, 0x08),
            Unknown0C = slice.AsSpan(0x0C, 8).ToArray()
        };
    }

    public byte[] Write(CollideSetRecord value)
    {
        var bytes = new byte[Size];
        HubReferenceIO.Write(bytes, 0x00, value.ZdxList);
        HubReferenceIO.Write(bytes, 0x04, value.ZddList);
        HubReferenceIO.Write(bytes, 0x08, value.ZdeList);
        WriteBytes(bytes, 0x0C, value.Unknown0C, 8);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(CollideSetRecord));
    }

    public CollideSetRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<CollideSetRecord>(json, Schema);

    public void ToJson(CollideSetRecord value, Utf8JsonWriter writer) =>
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