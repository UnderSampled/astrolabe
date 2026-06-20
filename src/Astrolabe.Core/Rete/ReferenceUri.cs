namespace Astrolabe.Core.Rete;

internal readonly record struct ResolvedReferenceUri(string FilePath, string? JsonPointer);

internal static class ReferenceUri
{
    public const string FixPrefix = "fix:/";
    public const string LevelPrefix = "level:/";

    public static bool TryResolve(
        string referringPackageRoot,
        string uri,
        out string filePath,
        out string? jsonPointer,
        string? levelPackageRoot = null)
    {
        filePath = "";
        jsonPointer = null;

        if (string.IsNullOrWhiteSpace(uri))
        {
            return false;
        }

        var hashIndex = uri.IndexOf('#');
        var pathPart = hashIndex >= 0 ? uri[..hashIndex] : uri;
        jsonPointer = hashIndex >= 0 ? uri[(hashIndex + 1)..] : null;

        if (string.IsNullOrWhiteSpace(pathPart))
        {
            return false;
        }

        if (pathPart.Contains("../", StringComparison.Ordinal) ||
            pathPart.StartsWith("..", StringComparison.Ordinal))
        {
            return false;
        }

        string? baseRoot = null;
        string relativePath;

        if (pathPart.StartsWith(FixPrefix, StringComparison.Ordinal))
        {
            baseRoot = TryGetFixPackageRoot(referringPackageRoot);
            relativePath = pathPart[FixPrefix.Length..];
        }
        else if (pathPart.StartsWith(LevelPrefix, StringComparison.Ordinal))
        {
            relativePath = pathPart[LevelPrefix.Length..];
            baseRoot = TryGetLevelPackageRoot(referringPackageRoot);
            if (baseRoot == null && !string.IsNullOrWhiteSpace(levelPackageRoot))
            {
                baseRoot = Path.GetFullPath(levelPackageRoot);
            }

            if (baseRoot == null)
            {
                baseRoot = TryFindSiblingLevelPackageRoot(referringPackageRoot, relativePath);
            }
        }
        else
        {
            baseRoot = Path.GetFullPath(referringPackageRoot);
            relativePath = pathPart;
        }

        if (baseRoot == null || string.IsNullOrWhiteSpace(relativePath))
        {
            return false;
        }

        filePath = Path.GetFullPath(Path.Combine(baseRoot, FromUriPath(relativePath)));
        return true;
    }

    public static ResolvedReferenceUri Resolve(
        string referringPackageRoot,
        string uri,
        string? levelPackageRoot = null)
    {
        if (!TryResolve(referringPackageRoot, uri, out var filePath, out var jsonPointer, levelPackageRoot))
        {
            throw new InvalidDataException($"Invalid reference URI: {uri}");
        }

        return new ResolvedReferenceUri(filePath, jsonPointer);
    }

    public static string MakeReference(string referringPackageRoot, string targetPath)
    {
        var referringRoot = Path.GetFullPath(referringPackageRoot);
        var targetFullPath = Path.GetFullPath(targetPath);

        if (IsUnderRoot(targetFullPath, referringRoot))
        {
            return ToUriPath(Path.GetRelativePath(referringRoot, targetFullPath));
        }

        var fixRoot = TryGetFixPackageRoot(referringRoot);
        if (fixRoot != null && IsUnderRoot(targetFullPath, fixRoot))
        {
            return FixPrefix + ToUriPath(Path.GetRelativePath(fixRoot, targetFullPath));
        }

        if (TryFindLevelPackageRootContaining(referringRoot, targetFullPath, out var levelRoot)
            && IsUnderRoot(targetFullPath, levelRoot))
        {
            return LevelPrefix + ToUriPath(Path.GetRelativePath(levelRoot, targetFullPath));
        }

        throw new InvalidDataException(
            $"Cannot build reference URI for target outside referring package, Fix, and level roots: {targetFullPath}");
    }

    public static string MakeRelative(string packageRoot, string targetPath) =>
        MakeReference(packageRoot, targetPath);

    public static string ToUriPath(string path) =>
        path.Replace(Path.DirectorySeparatorChar, '/').Replace(Path.AltDirectorySeparatorChar, '/');

    private static string FromUriPath(string path) =>
        path.Replace('/', Path.DirectorySeparatorChar);

    private static bool IsUnderRoot(string path, string root)
    {
        var relative = Path.GetRelativePath(root, path);
        return relative != "."
            && !relative.StartsWith("..", StringComparison.Ordinal)
            && !Path.IsPathRooted(relative);
    }

    private static string? TryGetFixPackageRoot(string referringPackageRoot)
    {
        var referringRoot = Path.GetFullPath(referringPackageRoot);
        if (TryReadPackageRole(referringRoot) == "fix")
        {
            return referringRoot;
        }

        var siblingFix = Path.Combine(Directory.GetParent(referringRoot)?.FullName ?? "", "fix");
        return HasReteManifest(siblingFix) ? Path.GetFullPath(siblingFix) : null;
    }

    private static string? TryGetLevelPackageRoot(string referringPackageRoot)
    {
        var referringRoot = Path.GetFullPath(referringPackageRoot);
        return TryReadPackageRole(referringRoot) == "level" ? referringRoot : null;
    }

    private static string? TryFindSiblingLevelPackageRoot(string referringPackageRoot, string relativePath)
    {
        var parent = Directory.GetParent(Path.GetFullPath(referringPackageRoot))?.FullName;
        if (parent == null)
        {
            return null;
        }

        foreach (var candidate in Directory.GetDirectories(parent))
        {
            if (!string.Equals(TryReadPackageRole(candidate), "level", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            var candidateRoot = Path.GetFullPath(candidate);
            var candidatePath = Path.GetFullPath(Path.Combine(candidateRoot, FromUriPath(relativePath)));
            if (File.Exists(candidatePath) || Directory.Exists(candidatePath))
            {
                return candidateRoot;
            }
        }

        return null;
    }

    private static bool TryFindLevelPackageRootContaining(
        string referringPackageRoot,
        string targetFullPath,
        out string levelRoot)
    {
        levelRoot = "";

        var directLevelRoot = TryGetLevelPackageRoot(referringPackageRoot);
        if (directLevelRoot != null && IsUnderRoot(targetFullPath, directLevelRoot))
        {
            levelRoot = directLevelRoot;
            return true;
        }

        var parent = Directory.GetParent(Path.GetFullPath(referringPackageRoot))?.FullName;
        if (parent == null)
        {
            return false;
        }

        foreach (var candidate in Directory.GetDirectories(parent))
        {
            if (!string.Equals(TryReadPackageRole(candidate), "level", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            var normalized = Path.GetFullPath(candidate);
            if (!IsUnderRoot(targetFullPath, normalized))
            {
                continue;
            }

            levelRoot = normalized;
            return true;
        }

        return false;
    }

    private static string? TryReadPackageRole(string packageRoot)
    {
        var manifestPath = Path.Combine(packageRoot, OpenSpacePackageCodec.ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            return null;
        }

        try
        {
            using var document = System.Text.Json.JsonDocument.Parse(File.ReadAllText(manifestPath));
            if (document.RootElement.TryGetProperty("packageRole", out var role))
            {
                return role.GetString();
            }
        }
        catch
        {
        }

        return null;
    }

    private static bool HasReteManifest(string packageRoot) =>
        File.Exists(Path.Combine(packageRoot, OpenSpacePackageCodec.ManifestFileName));
}