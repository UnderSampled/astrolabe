namespace Astrolabe.Core.Serialization;

public interface IPointerArrayCodec
{
    bool IsPointerArray => true;
    string PointerArrayPropertyName { get; }
    int PointerEntryStride => 4;
    IReadOnlyList<PointerField> GetPointerFieldsForLength(int byteLength);

    IReadOnlyList<PointerField> EnumeratePointerFields(ReadOnlySpan<byte> data) =>
        GetPointerFieldsForLength(data.Length);
}