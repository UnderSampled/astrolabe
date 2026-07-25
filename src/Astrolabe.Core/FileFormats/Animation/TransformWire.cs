using System.Buffers.Binary;

namespace Astrolabe.Core.FileFormats.Animation;

/// <summary>
/// Wire layout helpers for Montreal compressed transforms (see docs/perso-mesh-animation.md).
/// Decoding matches raymap <c>Matrix.ReadCompressed</c>.
/// </summary>
internal static class TransformWire
{
    public static int GetPayloadLength(ReadOnlySpan<byte> wireBytes)
    {
        if (wireBytes.Length < 2)
        {
            return wireBytes.Length;
        }

        return GetPayloadLength(BinaryPrimitives.ReadUInt16LittleEndian(wireBytes));
    }

    public static int GetPayloadLength(ushort type)
    {
        var actualType = type < 128 ? type & 0xF : 128;
        var size = 2;
        if (actualType is 1 or 3 or 7 or 11 or 15)
        {
            size += 6;
        }

        if (actualType is 2 or 3 or 7 or 11 or 15)
        {
            size += 8;
        }

        if (actualType == 7)
        {
            size += 2;
        }
        else if (actualType == 11)
        {
            size += 6;
        }
        else if (actualType == 15)
        {
            size += 12;
        }

        return Math.Max(size, 2);
    }

    public static bool IsLikelyTransform(ReadOnlySpan<byte> wireBytes) =>
        wireBytes.Length >= 2 && GetPayloadLength(wireBytes) <= wireBytes.Length;

    public static int GetTrailingGapLength(
        ReadOnlySpan<byte> blockData,
        int transformOffset,
        int wireLength,
        int? nextTransformOffset)
    {
        if (nextTransformOffset.HasValue)
        {
            var gap = nextTransformOffset.Value - (transformOffset + wireLength);
            return gap is >= 0 and <= 6 ? gap : 0;
        }

        var remaining = blockData.Length - (transformOffset + wireLength);
        return remaining is 4 or 6 ? remaining : 0;
    }
}