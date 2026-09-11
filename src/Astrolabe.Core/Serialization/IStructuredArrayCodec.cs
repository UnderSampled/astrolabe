namespace Astrolabe.Core.Serialization;

/// <summary>
/// Codec whose JSON form is a top-level array of fixed-size entry objects with named pointer fields.
/// Used by ReferenceJson to rewrite nested HubReference/wire pointer properties to URIs.
/// </summary>
public interface IStructuredArrayCodec
{
    string EntriesPropertyName { get; }
    IReadOnlyList<PointerField> EntryPointerFields { get; }
    int EntrySize { get; }
}
