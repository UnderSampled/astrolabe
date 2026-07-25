using System.Text.Json;

namespace Astrolabe.Core.FileFormats.Semantic;

/// <summary>
/// Generic dual-layer semantic pool: authoring roots + byId stream leaves + expand runs.
/// Mirrors <c>animation/families.json</c> for non-animation domains.
/// </summary>
public sealed class SemanticPoolDocument
{
    public string Schema { get; set; } = "";
    public string Domain { get; set; } = "";

    /// <summary>Optional authoring forest roots (id list).</summary>
    public List<string> Roots { get; set; } = [];

    /// <summary>Named authoring roots (e.g. scene actual_world).</summary>
    public Dictionary<string, string> NamedRoots { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);

    /// <summary>Streamable codec leaves keyed by stable id.</summary>
    public Dictionary<string, SemanticPoolNode> ById { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);

    /// <summary>Contiguous stream-order runs for content.json expand.</summary>
    public Dictionary<string, List<string>> Runs { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);
}

/// <summary>One streamable leaf or group in a semantic pool.</summary>
public sealed class SemanticPoolNode
{
    public string Id { get; set; } = "";
    public string Kind { get; set; } = "";

    /// <summary>Semantic children (authoring graph), not stream order.</summary>
    public List<string> Children { get; set; } = [];

    /// <summary>Codec JSON payload. Null for pure groups.</summary>
    public JsonElement? Record { get; set; }

    /// <summary>Optional matrix / static matrix records (scene nodes).</summary>
    public JsonElement? Matrix { get; set; }
    public JsonElement? StaticMatrix { get; set; }

    /// <summary>External buffer path for dense data (geometry).</summary>
    public string? BufferPath { get; set; }

    /// <summary>S-expression source path for script AST leaves.</summary>
    public string? SexprPath { get; set; }

    public int? ProvenanceVirtualAddress { get; set; }
}

/// <summary>Scene hierarchy: nested authoring tree + byId stream leaves.</summary>
public sealed class SceneTreeDocument
{
    public const string RelativePath = "scene/tree.json";
    public const string SchemaValue = "astrolabe.scene-tree.v2";

    public string Schema { get; set; } = SchemaValue;

    /// <summary>Root id per world: actual_world, dynamic_world, father_sector.</summary>
    public Dictionary<string, string?> Roots { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);

    public Dictionary<string, SemanticPoolNode> ById { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);

    public Dictionary<string, List<string>> Runs { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);
}

/// <summary>Geometry / materials aggregate with buffer-backed dense arrays.</summary>
public sealed class GeometryPoolDocument
{
    public const string RelativePath = "geometry/meshes.json";
    public const string SchemaValue = "astrolabe.geometry-pool.v1";
    public const string BufferDir = "geometry/buffers";

    public string Schema { get; set; } = SchemaValue;
    public Dictionary<string, SemanticPoolNode> ById { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);
    public Dictionary<string, List<string>> Runs { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);
}

/// <summary>AI models: JSON metadata + optional S-expression AST sources.</summary>
public sealed class AiPoolDocument
{
    public const string RelativePath = "ai/models.json";
    public const string SchemaValue = "astrolabe.ai-pool.v1";
    public const string SexprDir = "ai/scripts";
    public const string PayloadDir = "ai/payloads";

    public string Schema { get; set; } = SchemaValue;

    /// <summary>Authoring roots (typically brain ids).</summary>
    public List<string> Roots { get; set; } = [];

    public Dictionary<string, SemanticPoolNode> ById { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);
    public Dictionary<string, List<string>> Runs { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);
}

/// <summary>Perso / object-list / standard-game aggregate (non-animation character package).</summary>
public sealed class CharacterPoolDocument
{
    public const string RelativePath = "characters/persos.json";
    public const string SchemaValue = "astrolabe.character-pool.v1";

    public string Schema { get; set; } = SchemaValue;
    public List<string> Roots { get; set; } = [];
    public Dictionary<string, SemanticPoolNode> ById { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);
    public Dictionary<string, List<string>> Runs { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);
}

/// <summary>Sector / collision semantic pool.</summary>
public sealed class SectorPoolDocument
{
    public const string RelativePath = "sectors/sectors.json";
    public const string SchemaValue = "astrolabe.sector-pool.v1";

    public string Schema { get; set; } = SchemaValue;
    public Dictionary<string, SemanticPoolNode> ById { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);
    public Dictionary<string, List<string>> Runs { get; set; } =
        new(StringComparer.OrdinalIgnoreCase);
}

/// <summary>Promoted level sidecars (GPT/PTX/SDA/SND) with texture:/ and sound:/ URIs.</summary>
public sealed class SidecarDocument
{
    public const string RelativePath = "sidecars/level.json";
    public const string SchemaValue = "astrolabe.level-sidecars.v1";

    public string Schema { get; set; } = SchemaValue;
    public SidecarPointerFile? Gpt { get; set; }
    public SidecarPointerFile? Ptx { get; set; }
    public SidecarPointerFile? Sda { get; set; }
    public SidecarPointerFile? Snd { get; set; }
}

public sealed class SidecarPointerFile
{
    public string Kind { get; set; } = "";
    public string SourceFileName { get; set; } = "";
    /// <summary>Wire payload as base64 when not further structured (parity-safe).</summary>
    public string? WireBase64 { get; set; }
    /// <summary>Extracted uint32 pointer slots as reference URIs where resolvable.</summary>
    public List<string?> Pointers { get; set; } = [];
    /// <summary>Texture names from PTX → texture:/ URIs for referenced PNG assets.</summary>
    public List<string> TextureUris { get; set; } = [];
    /// <summary>Sound event URIs for SDA/SND.</summary>
    public List<string> SoundUris { get; set; } = [];
}
