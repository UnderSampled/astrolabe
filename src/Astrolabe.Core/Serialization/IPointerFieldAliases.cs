namespace Astrolabe.Core.Serialization;

public interface IPointerFieldAliases
{
    /// <summary>
    /// Maps legacy JSON pointer property names to canonical pointer field names.
    /// </summary>
    IReadOnlyDictionary<string, string> PointerFieldAliases { get; }
}