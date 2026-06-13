using System.Text.Json;
using Astrolabe.Core.FileFormats.Geometry;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class ElementTrianglesCodec : IStructCodec<ElementTrianglesRecord>
{
    public const int Size = 0x28;

    public static ElementTrianglesCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "material", PointerTarget.Any),
        new PointerField(0x08, "triangles", PointerTarget.Any),
        new PointerField(0x0C, "mappingUvs", PointerTarget.Any),
        new PointerField(0x10, "normals", PointerTarget.Any),
        new PointerField(0x14, "uvs", PointerTarget.Any),
        new PointerField(0x1C, "vertexIndices", PointerTarget.Any)
    ];

    public string Kind => "elementtriangles";
    public string Schema => "astrolabe.element-triangles.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public ElementTrianglesRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        return new ElementTrianglesRecord
        {
            Material = StructBinaryIO.ReadInt32(slice, 0x00),
            NumTriangles = StructBinaryIO.ReadUInt16(slice, 0x04),
            NumUvs = StructBinaryIO.ReadUInt16(slice, 0x06),
            Triangles = StructBinaryIO.ReadInt32(slice, 0x08),
            MappingUvs = StructBinaryIO.ReadInt32(slice, 0x0C),
            Normals = StructBinaryIO.ReadInt32(slice, 0x10),
            Uvs = StructBinaryIO.ReadInt32(slice, 0x14),
            Unknown18 = StructBinaryIO.ReadUInt32(slice, 0x18),
            VertexIndices = StructBinaryIO.ReadInt32(slice, 0x1C),
            NumVertexIndices = StructBinaryIO.ReadUInt16(slice, 0x20),
            ParallelBox = StructBinaryIO.ReadUInt16(slice, 0x22),
            Unknown24 = StructBinaryIO.ReadUInt32(slice, 0x24)
        };
    }

    public byte[] Write(ElementTrianglesRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteInt32(bytes, 0x00, value.Material);
        StructBinaryIO.WriteUInt16(bytes, 0x04, value.NumTriangles);
        StructBinaryIO.WriteUInt16(bytes, 0x06, value.NumUvs);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.Triangles);
        StructBinaryIO.WriteInt32(bytes, 0x0C, value.MappingUvs);
        StructBinaryIO.WriteInt32(bytes, 0x10, value.Normals);
        StructBinaryIO.WriteInt32(bytes, 0x14, value.Uvs);
        StructBinaryIO.WriteUInt32(bytes, 0x18, value.Unknown18);
        StructBinaryIO.WriteInt32(bytes, 0x1C, value.VertexIndices);
        StructBinaryIO.WriteUInt16(bytes, 0x20, value.NumVertexIndices);
        StructBinaryIO.WriteUInt16(bytes, 0x22, value.ParallelBox);
        StructBinaryIO.WriteUInt32(bytes, 0x24, value.Unknown24);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(ElementTrianglesRecord));
    }

    public ElementTrianglesRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<ElementTrianglesRecord>(json, Schema);

    public void ToJson(ElementTrianglesRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}
