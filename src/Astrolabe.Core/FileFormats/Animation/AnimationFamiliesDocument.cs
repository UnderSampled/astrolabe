using System.Text.Json;

namespace Astrolabe.Core.FileFormats.Animation;

/// <summary>
/// Semantic animation package document.
/// <para>
/// <see cref="Families"/> is the authoring tree (Family → State → animation ownership).
/// <see cref="ById"/> holds streamable codec records for export.
/// <see cref="Runs"/> are ordered id lists for content.json expand (stream order, no DFS).
/// </para>
/// </summary>
public sealed class AnimationFamiliesDocument
{
    public const string RelativePath = "animation/families.json";
    public const string SchemaValue = "astrolabe.animation-families.v1";

    public string Schema { get; set; } = SchemaValue;

    /// <summary>Authoring tree keyed by stable family id.</summary>
    public Dictionary<string, AnimationFamilyEntry> Families { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);

    /// <summary>
    /// Streamable leaves (state, animationmontreal, animchannel, …) for expand/export.
    /// Not the primary authoring view.
    /// </summary>
    public Dictionary<string, AnimationNode> ById { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);

    /// <summary>
    /// Contiguous stream-order runs of leaf ids (expand targets from content.json).
    /// Emit each id as a leaf only — do not DFS semantic Children.
    /// </summary>
    public Dictionary<string, List<string>> Runs { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);

    /// <summary>Per SNA block key: ordered expand roots when a forest expand is used.</summary>
    public Dictionary<string, List<string>> LayoutRoots { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);

    /// <summary>Streamable anim leaf ids not claimed under any family/state ownership tree.</summary>
    public List<string> OrphanLeafIds { get; set; } = [];
}

/// <summary>One character/family template in the authoring tree.</summary>
public sealed class AnimationFamilyEntry
{
    public string Id { get; set; } = "";
    public string? Name { get; set; }
    public uint? FamilyIndex { get; set; }

    /// <summary>Optional provenance ref to opaque family blob (import-only).</summary>
    public string? ProvenanceRef { get; set; }

    /// <summary>States in linked-list / discovery order.</summary>
    public List<AnimationStateEntry> States { get; set; } = [];
}

/// <summary>One animation state under a family.</summary>
public sealed class AnimationStateEntry
{
    public string Id { get; set; } = "";
    public string? Name { get; set; }

    /// <summary>byId of the state's AnimationMontreal, when present.</summary>
    public string? AnimationId { get; set; }

    /// <summary>byId of transition records owned by this state.</summary>
    public List<string> TransitionIds { get; set; } = [];
}

/// <summary>One streamable animation record for layout/export.</summary>
public sealed class AnimationNode
{
    public string Id { get; set; } = "";
    public string Kind { get; set; } = "";

    /// <summary>
    /// Semantic ownership children (for authoring graph edges), not stream order.
    /// Stream order uses <see cref="AnimationFamiliesDocument.Runs"/>.
    /// </summary>
    public List<string> Children { get; set; } = [];

    /// <summary>Codec JSON for this node. Null for pure groups.</summary>
    public JsonElement? Record { get; set; }

    /// <summary>Import-only provenance; export layout ignores this.</summary>
    public int? ProvenanceVirtualAddress { get; set; }
}
