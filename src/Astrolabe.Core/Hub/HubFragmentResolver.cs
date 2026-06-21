namespace Astrolabe.Core.Hub;

internal static class HubFragmentResolver
{
    private const string ByteOffsetPrefix = "byteOffset=";

    public static (string Path, int ByteOffset) SplitUri(string uri)
    {
        if (!TrySplitUri(uri, out var path, out var byteOffset))
        {
            throw new InvalidDataException($"Invalid reference URI fragment: {uri}");
        }

        return (path, byteOffset);
    }

    public static bool TrySplitUri(string uri, out string path, out int byteOffset)
    {
        path = uri;
        byteOffset = 0;

        var hashIndex = uri.IndexOf('#');
        if (hashIndex < 0)
        {
            path = uri;
            return true;
        }

        path = uri[..hashIndex];
        var fragment = uri[(hashIndex + 1)..];
        if (fragment.Length == 0)
        {
            return true;
        }

        if (!fragment.StartsWith(ByteOffsetPrefix, StringComparison.Ordinal) ||
            !int.TryParse(fragment[ByteOffsetPrefix.Length..], out byteOffset))
        {
            return false;
        }

        return true;
    }
}