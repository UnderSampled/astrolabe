using Astrolabe.Core.Hub;
using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class AnimChannelCodec : IStructCodec<AnimChannelRecord>, IPointerFieldAliases
{
    public const int Size = 0x14;

    public static AnimChannelCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "isIdentity", PointerTarget.BlockRelative, IgnoreValues: [0, 1]),
        new PointerField(0x10, "unknown10", PointerTarget.BlockRelative, IgnoreValues: [0], RequiresVmRange: true)
    ];

    private static readonly Dictionary<string, string> PointerFieldAliasesMap =
        new(StringComparer.OrdinalIgnoreCase) { ["matrixPointer"] = "isIdentity" };

    public string Kind => "animchannel";
    public string Schema => "astrolabe.anim-channel.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;
    public IReadOnlyDictionary<string, string> PointerFieldAliases => PointerFieldAliasesMap;

    public AnimChannelRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(AnimChannelRecord));
        return new AnimChannelRecord
        {
            IsIdentity = HubReferenceIO.Read(slice, 0x00),
            ObjectIndex = unchecked((sbyte)StructBinaryIO.ReadByte(slice, 0x04)),
            Unk1 = StructBinaryIO.ReadByte(slice, 0x05),
            Unk2 = unchecked((short)StructBinaryIO.ReadUInt16(slice, 0x06)),
            Unk3 = unchecked((short)StructBinaryIO.ReadUInt16(slice, 0x08)),
            UnkByte1 = StructBinaryIO.ReadByte(slice, 0x0A),
            UnkByte2 = StructBinaryIO.ReadByte(slice, 0x0B),
            UnkUint = StructBinaryIO.ReadUInt32(slice, 0x0C),
            Unknown10 = HubReferenceIO.Read(slice, 0x10)
        };
    }

    public byte[] Write(AnimChannelRecord value)
    {
        var bytes = new byte[Size];
        HubReferenceIO.Write(bytes, 0x00, value.IsIdentity);
        StructBinaryIO.WriteByte(bytes, 0x04, unchecked((byte)value.ObjectIndex));
        StructBinaryIO.WriteByte(bytes, 0x05, value.Unk1);
        StructBinaryIO.WriteUInt16(bytes, 0x06, unchecked((ushort)value.Unk2));
        StructBinaryIO.WriteUInt16(bytes, 0x08, unchecked((ushort)value.Unk3));
        StructBinaryIO.WriteByte(bytes, 0x0A, value.UnkByte1);
        StructBinaryIO.WriteByte(bytes, 0x0B, value.UnkByte2);
        StructBinaryIO.WriteUInt32(bytes, 0x0C, value.UnkUint);
        HubReferenceIO.Write(bytes, 0x10, value.Unknown10);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(AnimChannelRecord));
    }

    public AnimChannelRecord FromJson(JsonElement json)
    {
        var record = JsonStructCodec.Deserialize<AnimChannelRecord>(json, Schema);
        if (!json.TryGetProperty("isIdentity", out _))
        {
            ApplyLegacyMatrixPointer(json, record);
        }

        return record;
    }

    private static void ApplyLegacyMatrixPointer(JsonElement json, AnimChannelRecord record)
    {
        if (!json.TryGetProperty("matrixPointer", out var legacy))
        {
            return;
        }

        if (legacy.ValueKind == JsonValueKind.Number && legacy.TryGetInt32(out var legacyValue))
        {
            record.IsIdentity = HubReference.FromWire(legacyValue);
        }
    }

    public void ToJson(AnimChannelRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}