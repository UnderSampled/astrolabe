namespace Astrolabe.Core.FileFormats.Animation;

/// <summary>
/// Legacy WIP aggregate path. Prefer <see cref="AnimationFamiliesDocument"/> and
/// <see cref="AnimationTransformsDocument"/>. Kept only so old packages fail clearly.
/// </summary>
[Obsolete("Use AnimationFamiliesDocument + AnimationTransformsDocument.")]
public static class AnimationTreeDocument
{
    public const string RelativePath = "animation/level.json";
    public const string SchemaValue = "astrolabe.animation-tree.v1";
}
