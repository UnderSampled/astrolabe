namespace Astrolabe.Core.Serialization.Codecs;

public sealed class OpaqueBinaryRecord
{
    public string Schema { get; set; } = "";
    public byte[] Data { get; set; } = [];

    internal static OpaqueBinaryRecord FromSlice(string schema, ReadOnlySpan<byte> data, int offset, int length)
    {
        if (length == 0)
        {
            return new OpaqueBinaryRecord { Schema = schema, Data = [] };
        }

        return new OpaqueBinaryRecord
        {
            Schema = schema,
            Data = data.Slice(offset, length).ToArray()
        };
    }
}