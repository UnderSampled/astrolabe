using System.Text.Json;
using Astrolabe.Core.FileFormats.Materials;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class VisualMaterialCodec : IStructCodec<VisualMaterialRecord>
{
    public const int Size = 0x78;

    public static VisualMaterialCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x48, "offTexture", PointerTarget.BlockRelative),
        new PointerField(0x64, "offAnimTexturesFirst", PointerTarget.BlockRelative),
        new PointerField(0x68, "offAnimTexturesCurrent", PointerTarget.BlockRelative)
    ];

    public string Kind => "visualmaterial";
    public string Schema => "astrolabe.visual-material.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public VisualMaterialRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        var ambient = new float[4];
        var diffuse = new float[4];
        var specular = new float[4];
        var color = new float[4];

        StructBinaryIO.ReadFloat4(slice, 0x04, ambient);
        StructBinaryIO.ReadFloat4(slice, 0x14, diffuse);
        StructBinaryIO.ReadFloat4(slice, 0x24, specular);
        StructBinaryIO.ReadFloat4(slice, 0x34, color);

        return new VisualMaterialRecord
        {
            Flags = StructBinaryIO.ReadUInt32(slice, 0x00),
            AmbientCoef = ambient,
            DiffuseCoef = diffuse,
            SpecularCoef = specular,
            Color = color,
            Unknown44 = StructBinaryIO.ReadUInt32(slice, 0x44),
            OffTexture = StructBinaryIO.ReadInt32(slice, 0x48),
            CurrentScrollX = StructBinaryIO.ReadSingle(slice, 0x4C),
            CurrentScrollY = StructBinaryIO.ReadSingle(slice, 0x50),
            ScrollX = StructBinaryIO.ReadSingle(slice, 0x54),
            ScrollY = StructBinaryIO.ReadSingle(slice, 0x58),
            ScrollMode = StructBinaryIO.ReadUInt32(slice, 0x5C),
            RefreshNumber = StructBinaryIO.ReadInt32(slice, 0x60),
            OffAnimTexturesFirst = StructBinaryIO.ReadInt32(slice, 0x64),
            OffAnimTexturesCurrent = StructBinaryIO.ReadInt32(slice, 0x68),
            NumAnimTextures = StructBinaryIO.ReadUInt16(slice, 0x6C),
            Padding6E = StructBinaryIO.ReadUInt16(slice, 0x6E),
            Unknown70 = StructBinaryIO.ReadUInt32(slice, 0x70),
            Properties = StructBinaryIO.ReadByte(slice, 0x74),
            Padding75 = slice.Slice(0x75, 3).ToArray()
        };
    }

    public byte[] Write(VisualMaterialRecord value)
    {
        ValidateVector(value.AmbientCoef, nameof(value.AmbientCoef));
        ValidateVector(value.DiffuseCoef, nameof(value.DiffuseCoef));
        ValidateVector(value.SpecularCoef, nameof(value.SpecularCoef));
        ValidateVector(value.Color, nameof(value.Color));
        if (value.Padding75.Length != 3)
        {
            throw new InvalidDataException($"{nameof(value.Padding75)} must contain exactly 3 bytes.");
        }

        var bytes = new byte[Size];
        StructBinaryIO.WriteUInt32(bytes, 0x00, value.Flags);
        StructBinaryIO.WriteFloat4(bytes, 0x04, value.AmbientCoef);
        StructBinaryIO.WriteFloat4(bytes, 0x14, value.DiffuseCoef);
        StructBinaryIO.WriteFloat4(bytes, 0x24, value.SpecularCoef);
        StructBinaryIO.WriteFloat4(bytes, 0x34, value.Color);
        StructBinaryIO.WriteUInt32(bytes, 0x44, value.Unknown44);
        StructBinaryIO.WriteInt32(bytes, 0x48, value.OffTexture);
        StructBinaryIO.WriteSingle(bytes, 0x4C, value.CurrentScrollX);
        StructBinaryIO.WriteSingle(bytes, 0x50, value.CurrentScrollY);
        StructBinaryIO.WriteSingle(bytes, 0x54, value.ScrollX);
        StructBinaryIO.WriteSingle(bytes, 0x58, value.ScrollY);
        StructBinaryIO.WriteUInt32(bytes, 0x5C, value.ScrollMode);
        StructBinaryIO.WriteInt32(bytes, 0x60, value.RefreshNumber);
        StructBinaryIO.WriteInt32(bytes, 0x64, value.OffAnimTexturesFirst);
        StructBinaryIO.WriteInt32(bytes, 0x68, value.OffAnimTexturesCurrent);
        StructBinaryIO.WriteUInt16(bytes, 0x6C, value.NumAnimTextures);
        StructBinaryIO.WriteUInt16(bytes, 0x6E, value.Padding6E);
        StructBinaryIO.WriteUInt32(bytes, 0x70, value.Unknown70);
        StructBinaryIO.WriteByte(bytes, 0x74, value.Properties);
        value.Padding75.CopyTo(bytes.AsSpan(0x75, 3));

        return bytes;
    }

    public VisualMaterialRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<VisualMaterialRecord>(json, Schema);

    public void ToJson(VisualMaterialRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);

    private static void ValidateVector(float[] values, string fieldName)
    {
        if (values.Length != 4)
        {
            throw new InvalidDataException($"{fieldName} must contain exactly 4 values.");
        }
    }
}