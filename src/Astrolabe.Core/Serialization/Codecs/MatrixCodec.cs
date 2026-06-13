using System.Text.Json;
using Astrolabe.Core.FileFormats;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class MatrixCodec : IStructCodec<MatrixRecord>
{
    public const int Size = 88;

    public static MatrixCodec Instance { get; } = new();

    public string Kind => "matrix";
    public string Schema => "astrolabe.matrix.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];

    public MatrixRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        var translation = new float[3];
        var basisX = new float[3];
        var basisY = new float[3];
        var basisZ = new float[3];

        var matrix = new MatrixRecord
        {
            Type = StructBinaryIO.ReadUInt32(slice, 0x00)
        };

        JsonStructCodec.ReadFloat3(slice, 0x04, translation);
        JsonStructCodec.ReadFloat3(slice, 0x10, basisX);
        JsonStructCodec.ReadFloat3(slice, 0x1C, basisY);
        JsonStructCodec.ReadFloat3(slice, 0x28, basisZ);
        matrix.Translation = translation;
        matrix.BasisX = basisX;
        matrix.BasisY = basisY;
        matrix.BasisZ = basisZ;

        const int headerSize = 0x34;
        if (length > headerSize)
        {
            matrix.ExtraBase64 = Convert.ToBase64String(slice.Slice(headerSize).ToArray());
        }

        return matrix;
    }

    public byte[] Write(MatrixRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteUInt32(bytes, 0x00, value.Type);
        JsonStructCodec.WriteFloat3(bytes, 0x04, value.Translation, nameof(value.Translation));
        JsonStructCodec.WriteFloat3(bytes, 0x10, value.BasisX, nameof(value.BasisX));
        JsonStructCodec.WriteFloat3(bytes, 0x1C, value.BasisY, nameof(value.BasisY));
        JsonStructCodec.WriteFloat3(bytes, 0x28, value.BasisZ, nameof(value.BasisZ));

        if (!string.IsNullOrWhiteSpace(value.ExtraBase64))
        {
            Convert.FromBase64String(value.ExtraBase64).CopyTo(bytes.AsSpan(0x34));
        }

        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(MatrixRecord));
    }

    public MatrixRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<MatrixRecord>(json, Schema);

    public void ToJson(MatrixRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}