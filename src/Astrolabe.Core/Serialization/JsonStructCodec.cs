using System.Text.Json;

namespace Astrolabe.Core.Serialization;

internal static class JsonStructCodec
{
    internal static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    public static T Deserialize<T>(JsonElement json, string schema) where T : class
    {
        var value = json.Deserialize<T>(Options)
            ?? throw new InvalidDataException($"Could not deserialize {schema} JSON.");

        var schemaProperty = typeof(T).GetProperty("Schema");
        if (schemaProperty?.GetValue(value) is string actualSchema && actualSchema != schema)
        {
            throw new InvalidDataException($"Unsupported schema: {actualSchema}");
        }

        return value;
    }

    public static void Serialize<T>(Utf8JsonWriter writer, T value) =>
        JsonSerializer.Serialize(writer, value, Options);

    public static void WriteFloat3(Span<byte> destination, int offset, IReadOnlyList<float> values, string fieldName)
    {
        if (values.Count != 3)
        {
            throw new InvalidDataException($"{fieldName} must contain exactly 3 values.");
        }

        for (var i = 0; i < 3; i++)
        {
            StructBinaryIO.WriteSingle(destination, offset + i * 4, values[i]);
        }
    }

    public static void ReadFloat3(ReadOnlySpan<byte> data, int offset, float[] destination)
    {
        if (destination.Length != 3)
        {
            throw new ArgumentException("Destination must contain exactly 3 floats.", nameof(destination));
        }

        for (var i = 0; i < 3; i++)
        {
            destination[i] = StructBinaryIO.ReadSingle(data, offset + i * 4);
        }
    }

    public static void WriteIntArray(Span<byte> destination, int offset, IReadOnlyList<int> values, int expectedLength, string fieldName)
    {
        if (values.Count != expectedLength)
        {
            throw new InvalidDataException($"{fieldName} must contain exactly {expectedLength} integers.");
        }

        for (var i = 0; i < values.Count; i++)
        {
            StructBinaryIO.WriteInt32(destination, offset + i * 4, values[i]);
        }
    }

    public static byte[] RequireExactSize(byte[] data, int expectedLength, string typeName) =>
        StructBinaryIO.RequireExactSize(data, expectedLength, typeName);

    public static void RequireValuesArray<T>(T[]? values, string schema, string kind)
    {
        if (values == null)
        {
            throw new InvalidDataException($"{schema} ({kind}) requires a non-null values array.");
        }
    }
}