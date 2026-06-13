using System.Text.Json;

namespace Astrolabe.Core.Serialization;

public interface IStructCodec<T>
{
    string Kind { get; }
    string Schema { get; }
    int? FixedSize { get; }

    T Read(ReadOnlySpan<byte> data, int offset, int length);
    byte[] Write(T value);

    T FromJson(JsonElement json);
    void ToJson(T value, Utf8JsonWriter writer);

    IReadOnlyList<PointerField> PointerFields { get; }
}