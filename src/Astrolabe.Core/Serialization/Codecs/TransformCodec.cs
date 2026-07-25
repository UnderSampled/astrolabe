using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;

namespace Astrolabe.Core.Serialization.Codecs;

/// <summary>
/// Variable-length Montreal transform wire codec (raymap: <c>Matrix.ReadCompressed</c>).
/// </summary>
public sealed class TransformCodec : IStructCodec<TransformRecord>
{
    public static TransformCodec Instance { get; } = new();

    public string Kind => "transform";
    public string Schema => "astrolabe.transform.v1";
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];

    public TransformRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        var wireLength = TransformWire.GetPayloadLength(slice);
        if (wireLength > slice.Length)
        {
            wireLength = slice.Length;
        }

        return new TransformRecord
        {
            WireBytes = slice[..wireLength].ToArray(),
            TrailingGap = slice.Length > wireLength ? slice[wireLength..].ToArray() : []
        };
    }

    public byte[] Write(TransformRecord value)
    {
        if (value.WireBytes.Length == 0)
        {
            throw new InvalidDataException($"{Schema} requires non-empty wireBytes.");
        }

        if (value.TrailingGap.Length is > 6)
        {
            throw new InvalidDataException($"{Schema} trailingGap must be at most 6 bytes.");
        }

        var bytes = new byte[value.WireBytes.Length + value.TrailingGap.Length];
        value.WireBytes.CopyTo(bytes, 0);
        value.TrailingGap.CopyTo(bytes, value.WireBytes.Length);
        return bytes;
    }

    public TransformRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<TransformRecord>(json, Schema);

    public void ToJson(TransformRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}