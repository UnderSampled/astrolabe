using System.Text.Json;
using Astrolabe.Core.FileFormats.Materials;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class GameMaterialCodec : IStructCodec<GameMaterialRecord>
{
    public const int Size = 0x10;

    public static GameMaterialCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "visualMaterial", PointerTarget.Any),
        new PointerField(0x04, "mechanicsMaterial", PointerTarget.Any),
        new PointerField(0x0C, "collideMaterial", PointerTarget.Any)
    ];

    public string Kind => "gamematerial";
    public string Schema => "astrolabe.game-material.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public GameMaterialRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        return new GameMaterialRecord
        {
            VisualMaterial = StructBinaryIO.ReadInt32(slice, 0x00),
            MechanicsMaterial = StructBinaryIO.ReadInt32(slice, 0x04),
            SoundMaterial = StructBinaryIO.ReadUInt32(slice, 0x08),
            CollideMaterial = StructBinaryIO.ReadInt32(slice, 0x0C)
        };
    }

    public byte[] Write(GameMaterialRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteInt32(bytes, 0x00, value.VisualMaterial);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.MechanicsMaterial);
        StructBinaryIO.WriteUInt32(bytes, 0x08, value.SoundMaterial);
        StructBinaryIO.WriteInt32(bytes, 0x0C, value.CollideMaterial);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(GameMaterialRecord));
    }

    public GameMaterialRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<GameMaterialRecord>(json, Schema);

    public void ToJson(GameMaterialRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}