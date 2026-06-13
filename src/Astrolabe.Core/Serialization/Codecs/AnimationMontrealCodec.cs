using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class AnimationMontrealCodec : IStructCodec<AnimationMontrealRecord>
{
    public const int Size = 0x70;
    private const int SpeedMatrixOffset = 0x14;
    private const int SpeedMatrixFloatCount = 13;
    private const int TailOffset = 0x48;
    private const int TailUIntCount = 10;

    public static AnimationMontrealCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "offFrames", PointerTarget.BlockRelative),
        new PointerField(0x08, "offUnk", PointerTarget.BlockRelative)
    ];

    public string Kind => "animationmontreal";
    public string Schema => "astrolabe.animation-montreal.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public AnimationMontrealRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(AnimationMontrealRecord));
        var record = new AnimationMontrealRecord
        {
            OffFrames = StructBinaryIO.ReadInt32(slice, 0x00),
            NumFrames = StructBinaryIO.ReadByte(slice, 0x04),
            Speed = StructBinaryIO.ReadByte(slice, 0x05),
            NumChannels = StructBinaryIO.ReadByte(slice, 0x06),
            UnkByte = StructBinaryIO.ReadByte(slice, 0x07),
            OffUnk = StructBinaryIO.ReadInt32(slice, 0x08),
            Unk0C = StructBinaryIO.ReadUInt32(slice, 0x0C),
            Unk10 = StructBinaryIO.ReadUInt32(slice, 0x10),
            SpeedMatrix = ReadFloats(slice, SpeedMatrixOffset, SpeedMatrixFloatCount),
            Tail = ReadUInts(slice, TailOffset, TailUIntCount)
        };
        return record;
    }

    public byte[] Write(AnimationMontrealRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteInt32(bytes, 0x00, value.OffFrames);
        StructBinaryIO.WriteByte(bytes, 0x04, value.NumFrames);
        StructBinaryIO.WriteByte(bytes, 0x05, value.Speed);
        StructBinaryIO.WriteByte(bytes, 0x06, value.NumChannels);
        StructBinaryIO.WriteByte(bytes, 0x07, value.UnkByte);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.OffUnk);
        StructBinaryIO.WriteUInt32(bytes, 0x0C, value.Unk0C);
        StructBinaryIO.WriteUInt32(bytes, 0x10, value.Unk10);
        WriteFloats(bytes, SpeedMatrixOffset, value.SpeedMatrix, SpeedMatrixFloatCount);
        WriteUInts(bytes, TailOffset, value.Tail, TailUIntCount);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(AnimationMontrealRecord));
    }

    public AnimationMontrealRecord FromJson(JsonElement json)
    {
        var record = JsonStructCodec.Deserialize<AnimationMontrealRecord>(json, Schema);
        record.Tail = NormalizeTail(record.Tail);
        return record;
    }

    public void ToJson(AnimationMontrealRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);

    private static float[] ReadFloats(ReadOnlySpan<byte> slice, int offset, int count)
    {
        var values = new float[count];
        for (var i = 0; i < count; i++)
        {
            values[i] = StructBinaryIO.ReadSingle(slice, offset + i * 4);
        }

        return values;
    }

    private static uint[] ReadUInts(ReadOnlySpan<byte> slice, int offset, int count)
    {
        var values = new uint[count];
        for (var i = 0; i < count; i++)
        {
            values[i] = StructBinaryIO.ReadUInt32(slice, offset + i * 4);
        }

        return values;
    }

    private static void WriteFloats(Span<byte> bytes, int offset, IReadOnlyList<float> values, int expectedCount)
    {
        if (values.Count != expectedCount)
        {
            throw new InvalidDataException($"speedMatrix must contain exactly {expectedCount} values.");
        }

        for (var i = 0; i < values.Count; i++)
        {
            StructBinaryIO.WriteSingle(bytes, offset + i * 4, values[i]);
        }
    }

    private static void WriteUInts(Span<byte> bytes, int offset, IReadOnlyList<uint> values, int expectedCount)
    {
        var normalized = NormalizeTail(values);
        for (var i = 0; i < normalized.Length; i++)
        {
            StructBinaryIO.WriteUInt32(bytes, offset + i * 4, normalized[i]);
        }
    }

    private static uint[] NormalizeTail(IReadOnlyList<uint> values)
    {
        var normalized = new uint[TailUIntCount];
        for (var i = 0; i < values.Count && i < TailUIntCount; i++)
        {
            normalized[i] = values[i];
        }

        return normalized;
    }
}