using Astrolabe.Core.FileFormats.Semantic;

namespace Astrolabe.Core.Rete;

internal static class SemanticPoolPaths
{
    public static string SceneNodeUri(string id) =>
        $"{SceneTreeDocument.RelativePath}#/byId/{id}";

    public static string SceneRunUri(string runId) =>
        $"{SceneTreeDocument.RelativePath}#/runs/{runId}";

    public static string GeometryNodeUri(string id) =>
        $"{GeometryPoolDocument.RelativePath}#/byId/{id}";

    public static string GeometryRunUri(string runId) =>
        $"{GeometryPoolDocument.RelativePath}#/runs/{runId}";

    public static string AiNodeUri(string id) =>
        $"{AiPoolDocument.RelativePath}#/byId/{id}";

    public static string AiRunUri(string runId) =>
        $"{AiPoolDocument.RelativePath}#/runs/{runId}";

    public static string CharacterNodeUri(string id) =>
        $"{CharacterPoolDocument.RelativePath}#/byId/{id}";

    public static string CharacterRunUri(string runId) =>
        $"{CharacterPoolDocument.RelativePath}#/runs/{runId}";

    public static string SectorNodeUri(string id) =>
        $"{SectorPoolDocument.RelativePath}#/byId/{id}";

    public static string SectorRunUri(string runId) =>
        $"{SectorPoolDocument.RelativePath}#/runs/{runId}";

    public static bool TryParseById(string? jsonPointer, out string id)
    {
        if (!TryParseByIdField(jsonPointer, out id, out _))
        {
            id = "";
            return false;
        }

        return true;
    }

    /// <summary>
    /// Parses <c>#/byId/{id}</c> or <c>#/byId/{id}/{field}</c> (e.g. matrix / staticMatrix).
    /// <paramref name="id"/> is the bare node id (no field suffix).
    /// </summary>
    public static bool TryParseByIdField(string? jsonPointer, out string id, out string? field)
    {
        id = "";
        field = null;
        if (string.IsNullOrWhiteSpace(jsonPointer))
        {
            return false;
        }

        var fragment = jsonPointer.TrimStart('/');
        const string prefix = "byId/";
        if (!fragment.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        var rest = fragment[prefix.Length..];
        if (rest.Length == 0)
        {
            return false;
        }

        var slash = rest.IndexOf('/');
        if (slash < 0)
        {
            id = rest;
            return true;
        }

        id = rest[..slash];
        field = rest[(slash + 1)..];
        return id.Length > 0 && field.Length > 0;
    }

    /// <summary>Stream-leaf key for runs: bare id or <c>id/matrix</c> / <c>id/staticMatrix</c>.</summary>
    public static string SceneStreamLeafKey(string id, string? field) =>
        string.IsNullOrEmpty(field) ? id : $"{id}/{field}";

    public static bool TryParseRun(string? jsonPointer, out string runId)
    {
        runId = "";
        if (string.IsNullOrWhiteSpace(jsonPointer))
        {
            return false;
        }

        var fragment = jsonPointer.TrimStart('/');
        const string prefix = "runs/";
        if (!fragment.StartsWith(prefix, StringComparison.Ordinal))
        {
            return false;
        }

        runId = fragment[prefix.Length..];
        return runId.Length > 0;
    }

    public static string? MatchDocumentRelative(string relative)
    {
        if (relative.Equals(SceneTreeDocument.RelativePath, StringComparison.OrdinalIgnoreCase))
        {
            return SceneTreeDocument.RelativePath;
        }

        if (relative.Equals(GeometryPoolDocument.RelativePath, StringComparison.OrdinalIgnoreCase))
        {
            return GeometryPoolDocument.RelativePath;
        }

        if (relative.Equals(AiPoolDocument.RelativePath, StringComparison.OrdinalIgnoreCase))
        {
            return AiPoolDocument.RelativePath;
        }

        if (relative.Equals(CharacterPoolDocument.RelativePath, StringComparison.OrdinalIgnoreCase))
        {
            return CharacterPoolDocument.RelativePath;
        }

        if (relative.Equals(SectorPoolDocument.RelativePath, StringComparison.OrdinalIgnoreCase))
        {
            return SectorPoolDocument.RelativePath;
        }

        return null;
    }
}
