using System.Text.Json;
using Astrolabe.Core.FileFormats.Geometry;
using Astrolabe.Core.Hub;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class CollideZoneListCodec : IStructCodec<CollideZoneListRecord>
{
    public const int Size = 0x0C;

    private CollideZoneListCodec(string kind)
    {
        Kind = kind;
        Schema = $"astrolabe.{kind.Replace('_', '-')}.v1";
    }

    public string Kind { get; }
    public string Schema { get; }
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields { get; } =
    [
        new PointerField(0x00, "head", PointerTarget.BlockRelative),
        new PointerField(0x04, "tail", PointerTarget.BlockRelative)
    ];

    public static CollideZoneListCodec Zdx { get; } = new("collidezdxlist");
    public static CollideZoneListCodec Zdd { get; } = new("collidezddlist");
    public static CollideZoneListCodec Zde { get; } = new("collidezdelist");

    public CollideZoneListRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(CollideZoneListRecord));
        return new CollideZoneListRecord
        {
            Schema = Schema,
            Type = Kind,
            Head = HubReferenceIO.Read(slice, 0x00),
            Tail = HubReferenceIO.Read(slice, 0x04),
            Count = StructBinaryIO.ReadUInt32(slice, 0x08)
        };
    }

    public byte[] Write(CollideZoneListRecord value)
    {
        var bytes = new byte[Size];
        HubReferenceIO.Write(bytes, 0x00, value.Head);
        HubReferenceIO.Write(bytes, 0x04, value.Tail);
        StructBinaryIO.WriteUInt32(bytes, 0x08, value.Count);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(CollideZoneListRecord));
    }

    public CollideZoneListRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<CollideZoneListRecord>(json, Schema);

    public void ToJson(CollideZoneListRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}

public sealed class CollideZoneCodec : IStructCodec<CollideZoneRecord>
{
    public const int Size = 0x20;

    private CollideZoneCodec(string kind)
    {
        Kind = kind;
        Schema = $"astrolabe.{kind.Replace('_', '-')}.v1";
    }

    public string Kind { get; }
    public string Schema { get; }
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields { get; } =
    [
        new PointerField(0x00, "next", PointerTarget.BlockRelative),
        new PointerField(0x04, "prev", PointerTarget.BlockRelative),
        new PointerField(0x08, "collideObj", PointerTarget.BlockRelative)
    ];

    public static CollideZoneCodec Zdx { get; } = new("collidezdxzone");
    public static CollideZoneCodec Zdd { get; } = new("collidezddzone");
    public static CollideZoneCodec Zde { get; } = new("collidezdezone");

    public CollideZoneRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(CollideZoneRecord));
        return new CollideZoneRecord
        {
            Schema = Schema,
            Type = Kind,
            Next = HubReferenceIO.Read(slice, 0x00),
            Prev = HubReferenceIO.Read(slice, 0x04),
            CollideObj = HubReferenceIO.Read(slice, 0x08),
            Unknown0C = slice.AsSpan(0x0C, Size - 0x0C).ToArray()
        };
    }

    public byte[] Write(CollideZoneRecord value)
    {
        var bytes = new byte[Size];
        HubReferenceIO.Write(bytes, 0x00, value.Next);
        HubReferenceIO.Write(bytes, 0x04, value.Prev);
        HubReferenceIO.Write(bytes, 0x08, value.CollideObj);
        WriteBytes(bytes, 0x0C, value.Unknown0C, Size - 0x0C);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(CollideZoneRecord));
    }

    public CollideZoneRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<CollideZoneRecord>(json, Schema);

    public void ToJson(CollideZoneRecord value, Utf8JsonWriter writer) =>
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
