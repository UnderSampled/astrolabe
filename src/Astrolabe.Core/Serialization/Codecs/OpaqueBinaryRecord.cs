using System.Text.Json.Serialization;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class OpaqueBinaryRecord
{
    public string Schema { get; set; } = "";
    public string Path { get; set; } = "";
    public string Sha256 { get; set; } = "";
    public Dictionary<string, string?> Pointers { get; set; } = new(StringComparer.OrdinalIgnoreCase);

    [JsonIgnore]
    public byte[] Data { get; set; } = [];

    [JsonPropertyName("data")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public byte[]? LegacyData { get; set; }

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
