using System.Text.Json;
using Astrolabe.Core.FileFormats.Geometry;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class GeometricObjectCodec : IStructCodec<GeometricObjectRecord>
{
    public const int Size = 0x40;

    public static GeometricObjectCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x04, "vertices", PointerTarget.Any),
        new PointerField(0x08, "normals", PointerTarget.Any),
        new PointerField(0x0C, "materials", PointerTarget.Any),
        new PointerField(0x18, "elementTypes", PointerTarget.Any),
        new PointerField(0x1C, "elements", PointerTarget.Any)
    ];

    public string Kind => "geometricobject";
    public string Schema => "astrolabe.geometric-object.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public GeometricObjectRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        var unknowns = new int[4];
        var sphereCenter = new float[3];

        for (var i = 0; i < unknowns.Length; i++)
        {
            unknowns[i] = StructBinaryIO.ReadInt32(slice, 0x20 + i * 4);
        }

        JsonStructCodec.ReadFloat3(slice, 0x34, sphereCenter);

        return new GeometricObjectRecord
        {
            NumVertices = StructBinaryIO.ReadUInt32(slice, 0x00),
            Vertices = StructBinaryIO.ReadInt32(slice, 0x04),
            Normals = StructBinaryIO.ReadInt32(slice, 0x08),
            Materials = StructBinaryIO.ReadInt32(slice, 0x0C),
            Unknown0 = StructBinaryIO.ReadInt32(slice, 0x10),
            NumElements = StructBinaryIO.ReadUInt32(slice, 0x14),
            ElementTypes = StructBinaryIO.ReadInt32(slice, 0x18),
            Elements = StructBinaryIO.ReadInt32(slice, 0x1C),
            Unknowns = unknowns,
            SphereRadius = StructBinaryIO.ReadSingle(slice, 0x30),
            SphereCenterRaw = sphereCenter
        };
    }

    public byte[] Write(GeometricObjectRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteUInt32(bytes, 0x00, value.NumVertices);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.Vertices);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.Normals);
        StructBinaryIO.WriteInt32(bytes, 0x0C, value.Materials);
        StructBinaryIO.WriteInt32(bytes, 0x10, value.Unknown0);
        StructBinaryIO.WriteUInt32(bytes, 0x14, value.NumElements);
        StructBinaryIO.WriteInt32(bytes, 0x18, value.ElementTypes);
        StructBinaryIO.WriteInt32(bytes, 0x1C, value.Elements);
        JsonStructCodec.WriteIntArray(bytes, 0x20, value.Unknowns, 4, nameof(value.Unknowns));
        StructBinaryIO.WriteSingle(bytes, 0x30, value.SphereRadius);
        JsonStructCodec.WriteFloat3(bytes, 0x34, value.SphereCenterRaw, nameof(value.SphereCenterRaw));
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(GeometricObjectRecord));
    }

    public GeometricObjectRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<GeometricObjectRecord>(json, Schema);

    public void ToJson(GeometricObjectRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}