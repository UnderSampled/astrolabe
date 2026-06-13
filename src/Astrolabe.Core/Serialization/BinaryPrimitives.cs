using System.Buffers.Binary;

namespace Astrolabe.Core.Serialization;

internal static class StructBinaryIO
{
    public const int Float4Size = 16;

    public static void ReadFloat4(ReadOnlySpan<byte> data, int offset, float[] destination)
    {
        if (destination.Length != 4)
        {
            throw new ArgumentException("Destination must contain exactly 4 floats.", nameof(destination));
        }

        for (var i = 0; i < 4; i++)
        {
            destination[i] = ReadSingle(data, offset + i * 4);
        }
    }

    public static void WriteFloat4(Span<byte> destination, int offset, IReadOnlyList<float> values)
    {
        if (values.Count != 4)
        {
            throw new InvalidDataException("Float4 fields must contain exactly 4 values.");
        }

        for (var i = 0; i < 4; i++)
        {
            WriteSingle(destination, offset + i * 4, values[i]);
        }
    }

    public static float ReadSingle(ReadOnlySpan<byte> data, int offset) =>
        BinaryPrimitives.ReadSingleLittleEndian(data.Slice(offset, 4));

    public static void WriteSingle(Span<byte> destination, int offset, float value) =>
        BinaryPrimitives.WriteSingleLittleEndian(destination.Slice(offset, 4), value);

    public static uint ReadUInt32(ReadOnlySpan<byte> data, int offset) =>
        BinaryPrimitives.ReadUInt32LittleEndian(data.Slice(offset, 4));

    public static void WriteUInt32(Span<byte> destination, int offset, uint value) =>
        BinaryPrimitives.WriteUInt32LittleEndian(destination.Slice(offset, 4), value);

    public static int ReadInt32(ReadOnlySpan<byte> data, int offset) =>
        BinaryPrimitives.ReadInt32LittleEndian(data.Slice(offset, 4));

    public static void WriteInt32(Span<byte> destination, int offset, int value) =>
        BinaryPrimitives.WriteInt32LittleEndian(destination.Slice(offset, 4), value);

    public static ushort ReadUInt16(ReadOnlySpan<byte> data, int offset) =>
        BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(offset, 2));

    public static void WriteUInt16(Span<byte> destination, int offset, ushort value) =>
        BinaryPrimitives.WriteUInt16LittleEndian(destination.Slice(offset, 2), value);

    public static byte ReadByte(ReadOnlySpan<byte> data, int offset)
    {
        return data[offset];
    }

    public static void WriteByte(Span<byte> destination, int offset, byte value)
    {
        destination[offset] = value;
    }

    public static byte[] RequireExactSize(ReadOnlySpan<byte> slice, int expectedLength, string typeName)
    {
        if (slice.Length != expectedLength)
        {
            throw new InvalidDataException(
                $"{typeName} must be exactly {expectedLength} bytes, but was {slice.Length}.");
        }

        return slice.ToArray();
    }
}