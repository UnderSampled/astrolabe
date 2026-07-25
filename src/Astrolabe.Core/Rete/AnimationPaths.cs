using Astrolabe.Core.FileFormats.Animation;

namespace Astrolabe.Core.Rete;

/// <summary>URI helpers for animation families + transform pool documents.</summary>
internal static class AnimationPaths
{
    public static string TransformUri(string id) =>
        $"{AnimationTransformsDocument.RelativePath}#/byId/{id}";

    public static string TransformStreamUri() =>
        $"{AnimationTransformsDocument.RelativePath}#/stream";

    public static string FamilyNodeUri(string id) =>
        $"{AnimationFamiliesDocument.RelativePath}#/byId/{id}";

    public static string FamilyRunUri(string runId) =>
        $"{AnimationFamiliesDocument.RelativePath}#/runs/{runId}";

    public static string LayoutRootsUri(string blockKey) =>
        $"{AnimationFamiliesDocument.RelativePath}#/layoutRoots/{blockKey.Replace(':', '_')}";

    public static bool TryParseTransformById(string? jsonPointer, out string id)
    {
        id = "";
        if (string.IsNullOrWhiteSpace(jsonPointer))
        {
            return false;
        }

        var fragment = jsonPointer.TrimStart('/');
        const string prefix = "byId/";
        if (!fragment.StartsWith(prefix, StringComparison.Ordinal))
        {
            return false;
        }

        id = fragment[prefix.Length..];
        return id.Length > 0;
    }

    public static bool TryParseFamilyById(string? jsonPointer, out string id)
    {
        id = "";
        if (string.IsNullOrWhiteSpace(jsonPointer))
        {
            return false;
        }

        var fragment = jsonPointer.TrimStart('/');
        const string prefix = "byId/";
        if (!fragment.StartsWith(prefix, StringComparison.Ordinal))
        {
            return false;
        }

        id = fragment[prefix.Length..];
        return id.Length > 0;
    }

    public static bool TryParseFamilyRun(string? jsonPointer, out string runId)
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
}
