namespace Astrolabe.Core.Serialization;

public interface IPointerArrayCodec
{
    bool IsPointerArray => true;
    string PointerArrayPropertyName { get; }
    IReadOnlyList<PointerField> GetPointerFieldsForLength(int byteLength);
}