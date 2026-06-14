using System.Buffers.Binary;
using System.Text.Json;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class RawBlobCodec : IStructCodec<OpaqueBinaryRecord>, IPointerArrayCodec
{
    public static RawBlobCodec Instance { get; } = new();

    public string Kind => "raw";
    public string Schema => "astrolabe.raw-blob.v1";
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];
    public string PointerArrayPropertyName => "data";

    public IReadOnlyList<PointerField> GetPointerFieldsForLength(int byteLength) =>
        [];

    public IReadOnlyList<PointerField> EnumeratePointerFields(ReadOnlySpan<byte> data)
    {
        var fields = new List<PointerField>();
        for (var offset = 0; offset <= data.Length - sizeof(int); offset += sizeof(int))
        {
            var value = BinaryPrimitives.ReadInt32LittleEndian(data.Slice(offset, sizeof(int)));
            if (!VmPointerScanning.IsLikelyVirtualAddress(value))
            {
                continue;
            }

            fields.Add(new PointerField(
                offset,
                $"ptr_{offset:X}",
                PointerTarget.BlockRelative,
                RequiresVmRange: true,
                RequiresDecompressedTarget: true));
        }

        return fields;
    }

    public OpaqueBinaryRecord Read(ReadOnlySpan<byte> data, int offset, int length) =>
        OpaqueBinaryRecord.FromSlice(Schema, data, offset, length);

    public byte[] Write(OpaqueBinaryRecord value) => value.Data;

    public OpaqueBinaryRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<OpaqueBinaryRecord>(json, Schema);

    public void ToJson(OpaqueBinaryRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}