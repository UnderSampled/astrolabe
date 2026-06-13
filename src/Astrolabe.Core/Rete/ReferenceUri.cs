namespace Astrolabe.Core.Rete;

internal readonly record struct ResolvedReferenceUri(string FilePath, string? JsonPointer);

internal static class ReferenceUri
{
    public static bool TryResolve(string packageRoot, string uri, out string filePath, out string? jsonPointer)
    {
        filePath = "";
        jsonPointer = null;

        if (string.IsNullOrWhiteSpace(uri))
        {
            return false;
        }

        var hashIndex = uri.IndexOf('#');
        var relativePath = hashIndex >= 0 ? uri[..hashIndex] : uri;
        jsonPointer = hashIndex >= 0 ? uri[(hashIndex + 1)..] : null;

        if (string.IsNullOrWhiteSpace(relativePath))
        {
            return false;
        }

        filePath = Path.GetFullPath(Path.Combine(packageRoot, FromUriPath(relativePath)));
        return true;
    }

    public static ResolvedReferenceUri Resolve(string packageRoot, string uri)
    {
        if (!TryResolve(packageRoot, uri, out var filePath, out var jsonPointer))
        {
            throw new InvalidDataException($"Invalid reference URI: {uri}");
        }

        return new ResolvedReferenceUri(filePath, jsonPointer);
    }

    public static string MakeRelative(string packageRoot, string targetPath)
    {
        var relativePath = Path.GetRelativePath(
            Path.GetFullPath(packageRoot),
            Path.GetFullPath(targetPath));

        return ToUriPath(relativePath);
    }

    public static string ToUriPath(string path) =>
        path.Replace(Path.DirectorySeparatorChar, '/').Replace(Path.AltDirectorySeparatorChar, '/');

    private static string FromUriPath(string path) =>
        path.Replace('/', Path.DirectorySeparatorChar);
}
