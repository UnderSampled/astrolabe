using System.Text.Json;

namespace Astrolabe.Core.FileFormats.Animation;

/// <summary>
/// Aggregate animation data for a level package (states through channels with inline transform indices).
/// </summary>
public sealed class AnimationTreeDocument
{
    public const string RelativePath = "animation/level.json";
    public const string SchemaValue = "astrolabe.animation-tree.v1";
    public const string TransformFragmentPrefix = "/transforms/";

    public string Schema { get; set; } = SchemaValue;

    /// <summary>Transforms in definition order (depth-first tree walk).</summary>
    public List<TransformRecord> Transforms { get; set; } = [];

    /// <summary>Promoted animation elements keyed by virtual address hex (uppercase, no 0x).</summary>
    public Dictionary<string, AnimationTreeElementEntry> Elements { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);
}

public sealed class AnimationTreeElementEntry
{
    public string Kind { get; set; } = "";
    public int VirtualAddress { get; set; }
    public int OffsetInBlock { get; set; }
    public int Length { get; set; }
    public JsonElement Record { get; set; }
}