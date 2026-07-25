namespace Astrolabe.Core.FileFormats.Animation;

/// <summary>
/// Shared Montreal transform pool. <see cref="Stream"/> is the authoritative
/// emission order for dense transform spans; <see cref="ById"/> holds payloads.
/// Channel records link here by URI — they do not own exclusive matrix copies.
/// </summary>
public sealed class AnimationTransformsDocument
{
    public const string RelativePath = "animation/transforms.json";
    public const string SchemaValue = "astrolabe.animation-transforms.v1";

    public string Schema { get; set; } = SchemaValue;

    /// <summary>Ordered transform ids (block linearization for pure transform runs).</summary>
    public List<string> Stream { get; set; } = [];

    /// <summary>
    /// Contiguous stream runs for content.json expand (avoids tens of thousands of nested segments).
    /// </summary>
    public Dictionary<string, List<string>> Runs { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);

    /// <summary>Transform payloads keyed by stable id (not virtual address).</summary>
    public Dictionary<string, TransformRecord> ById { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);
}
