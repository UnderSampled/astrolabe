using Astrolabe.Core.FileFormats.Animation;

namespace Astrolabe.Core.Rete;

internal static class AnimationTreePaths
{
    public static string TreeRelativePath => AnimationTreeDocument.RelativePath;

    public static string ElementFragment(int virtualAddress) =>
        $"/elements/{virtualAddress:X8}";

    public static string TransformFragment(int transformIndex) =>
        $"{AnimationTreeDocument.TransformFragmentPrefix}{transformIndex}";

    public static string ElementUri(int virtualAddress) =>
        $"{TreeRelativePath}#/elements/{virtualAddress:X8}";

    public static string TransformUri(int transformIndex) =>
        $"{TreeRelativePath}#/transforms/{transformIndex}";

    public static bool TryParseTransformFragment(string? jsonPointer, out int transformIndex)
    {
        transformIndex = -1;
        if (string.IsNullOrWhiteSpace(jsonPointer))
        {
            return false;
        }

        var fragment = jsonPointer.TrimStart('/');
        const string prefix = "transforms/";
        if (!fragment.StartsWith(prefix, StringComparison.Ordinal))
        {
            return false;
        }

        return int.TryParse(fragment[prefix.Length..], out transformIndex) && transformIndex >= 0;
    }

    public static bool TryParseElementFragment(string? jsonPointer, out int virtualAddress)
    {
        virtualAddress = 0;
        if (string.IsNullOrWhiteSpace(jsonPointer))
        {
            return false;
        }

        var fragment = jsonPointer.TrimStart('/');
        const string prefix = "elements/";
        if (!fragment.StartsWith(prefix, StringComparison.Ordinal))
        {
            return false;
        }

        return int.TryParse(
            fragment[prefix.Length..],
            System.Globalization.NumberStyles.HexNumber,
            null,
            out virtualAddress);
    }
}