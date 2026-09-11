using System.Text.Json;
using Astrolabe.Core.FileFormats.Geometry;
using Astrolabe.Core.Hub;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class SectorCollideGeoCodec : IStructCodec<SectorCollideGeoRecord>
{
    public const int Size = 0x30;

    public static SectorCollideGeoCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x04, "vertices", PointerTarget.BlockRelative),
        new PointerField(0x08, "normals", PointerTarget.BlockRelative),
        new PointerField(0x18, "elementTypes", PointerTarget.BlockRelative),
        new PointerField(0x1C, "elements", PointerTarget.BlockRelative)
    ];

    public string Kind => "sectorcollidegeo";
    public string Schema => "astrolabe.sector-collide-geo.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public SectorCollideGeoRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(SectorCollideGeoRecord));
        return new SectorCollideGeoRecord
        {
            Unknown00 = StructBinaryIO.ReadUInt32(slice, 0x00),
            Vertices = HubReferenceIO.Read(slice, 0x04),
            Normals = HubReferenceIO.Read(slice, 0x08),
            Unknown0C = StructBinaryIO.ReadUInt32(slice, 0x0C),
            Unknown10 = StructBinaryIO.ReadUInt32(slice, 0x10),
            NumElements = StructBinaryIO.ReadUInt32(slice, 0x14),
            ElementTypes = HubReferenceIO.Read(slice, 0x18),
            Elements = HubReferenceIO.Read(slice, 0x1C),
            Unknown20 = slice.AsSpan(0x20, 0x10).ToArray()
        };
    }

    public byte[] Write(SectorCollideGeoRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteUInt32(bytes, 0x00, value.Unknown00);
        HubReferenceIO.Write(bytes, 0x04, value.Vertices);
        HubReferenceIO.Write(bytes, 0x08, value.Normals);
        StructBinaryIO.WriteUInt32(bytes, 0x0C, value.Unknown0C);
        StructBinaryIO.WriteUInt32(bytes, 0x10, value.Unknown10);
        StructBinaryIO.WriteUInt32(bytes, 0x14, value.NumElements);
        HubReferenceIO.Write(bytes, 0x18, value.ElementTypes);
        HubReferenceIO.Write(bytes, 0x1C, value.Elements);
        WriteBytes(bytes, 0x20, value.Unknown20, 0x10);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(SectorCollideGeoRecord));
    }

    public SectorCollideGeoRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<SectorCollideGeoRecord>(json, Schema);

    public void ToJson(SectorCollideGeoRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);

    private static void WriteBytes(Span<byte> destination, int offset, IReadOnlyList<byte> values, int expectedLength)
    {
        if (values.Count != expectedLength)
        {
            throw new InvalidDataException($"unknown20 must contain exactly {expectedLength} bytes.");
        }

        for (var i = 0; i < values.Count; i++)
        {
            destination[offset + i] = values[i];
        }
    }
}
