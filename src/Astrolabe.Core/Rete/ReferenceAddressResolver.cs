using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;
using Astrolabe.Core.Serialization;
using Astrolabe.Core.Serialization.Codecs;

namespace Astrolabe.Core.Rete;

internal sealed class ReferenceAddressResolver
{
    private readonly Dictionary<string, RetePackageAddressIndex> _indexes =
        new(StringComparer.OrdinalIgnoreCase);

    private readonly JsonSerializerOptions _jsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    public ReferenceAddressResolver(string packageRoot)
    {
        LoadPackage(packageRoot);
    }

    internal static ReferenceAddressResolver CreateForExport(string packageRoot)
    {
        var resolver = new ReferenceAddressResolver(packageRoot);
        var normalizedRoot = Path.GetFullPath(packageRoot);
        var packageParent = Directory.GetParent(normalizedRoot)?.FullName;
        if (packageParent == null)
        {
            return resolver;
        }

        var manifestPath = Path.Combine(normalizedRoot, OpenSpacePackageCodec.ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            return resolver;
        }

        var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
            File.ReadAllText(manifestPath),
            resolver._jsonOptions) ?? throw new InvalidDataException($"Could not read Rete manifest: {manifestPath}");

        if (manifest.PackageRole.Equals("level", StringComparison.OrdinalIgnoreCase))
        {
            var fixPackageDir = Path.Combine(packageParent, "fix");
            if (File.Exists(Path.Combine(fixPackageDir, OpenSpacePackageCodec.ManifestFileName)))
            {
                resolver.LoadPackage(fixPackageDir);
            }

            return resolver;
        }

        if (!manifest.PackageRole.Equals("fix", StringComparison.OrdinalIgnoreCase))
        {
            return resolver;
        }

        foreach (var siblingDir in Directory.EnumerateDirectories(packageParent))
        {
            if (siblingDir.Equals(normalizedRoot, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            var siblingManifestPath = Path.Combine(siblingDir, OpenSpacePackageCodec.ManifestFileName);
            if (!File.Exists(siblingManifestPath))
            {
                continue;
            }

            var siblingManifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(siblingManifestPath),
                resolver._jsonOptions);
            if (siblingManifest?.PackageRole.Equals("level", StringComparison.OrdinalIgnoreCase) == true)
            {
                resolver.LoadPackage(siblingDir);
            }
        }

        return resolver;
    }

    public void LoadPackage(string packageRoot)
    {
        var normalizedRoot = Path.GetFullPath(packageRoot);
        if (_indexes.ContainsKey(normalizedRoot))
        {
            return;
        }

        var manifestPath = Path.Combine(normalizedRoot, OpenSpacePackageCodec.ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            return;
        }

        _indexes[normalizedRoot] = RetePackageAddressIndex.Load(normalizedRoot, _jsonOptions);
    }

    public bool TryGetReferenceUri(int virtualAddress, string referringPackageRoot, out string uri)
    {
        foreach (var index in _indexes.Values)
        {
            if (index.TryGetPath(virtualAddress, out var targetPath, out var byteOffset))
            {
                uri = ReferenceUri.MakeRelative(referringPackageRoot, targetPath);
                if (byteOffset != 0)
                {
                    uri = $"{uri}#byteOffset={byteOffset}";
                }

                return true;
            }
        }

        uri = "";
        return false;
    }

    public int ResolveAddress(string referringPackageRoot, string uri)
    {
        var resolved = ReferenceUri.Resolve(referringPackageRoot, uri);
        var packageRoot = FindPackageRoot(resolved.FilePath)
            ?? throw new InvalidDataException($"Reference target is not inside a Rete package: {uri}");

        LoadPackage(packageRoot);
        var index = _indexes[packageRoot];

        if (TryResolveLevelSlotAddress(packageRoot, uri, resolved.FilePath, out var slotAddress))
        {
            return slotAddress;
        }

        // Prefer full URI keys (path + JSON Pointer). Many animation leaves share one
        // families.json / transforms.json file; fragment is the identity, not the path alone.
        var normalizedUri = uri.Replace('\\', '/');
        var byteOffsetSuffix = 0;
        var lookupUri = normalizedUri;
        var semi = lookupUri.IndexOf(";byteOffset=", StringComparison.OrdinalIgnoreCase);
        if (semi >= 0 &&
            int.TryParse(lookupUri[(semi + ";byteOffset=".Length)..], out byteOffsetSuffix))
        {
            lookupUri = lookupUri[..semi];
        }

        if (index.TryGetAddress(lookupUri, out var address))
        {
            return checked(address + byteOffsetSuffix);
        }

        // Relative form as stored on content segments (no scheme).
        var packageRelative = NormalizeToPackageRelativeUri(packageRoot, lookupUri, resolved);
        if (index.TryGetAddress(packageRelative, out address))
        {
            return checked(address + byteOffsetSuffix);
        }

        if (!index.TryGetAddress(resolved.FilePath, out address))
        {
            throw new InvalidDataException($"Reference target is not part of package content: {uri}");
        }

        if (string.IsNullOrWhiteSpace(resolved.JsonPointer))
        {
            return checked(address + byteOffsetSuffix);
        }

        if (TryReadByteOffsetFragment(resolved.JsonPointer, out var byteOffset))
        {
            return checked(address + byteOffset + byteOffsetSuffix);
        }

        // JSON Pointer fragment without index hit (e.g. #/byId/x) — already tried lookupUri.
        if (byteOffsetSuffix != 0)
        {
            return checked(address + byteOffsetSuffix);
        }

        throw new InvalidDataException($"Unsupported reference fragment in URI: {uri}");
    }

    private static string NormalizeToPackageRelativeUri(
        string packageRoot,
        string uri,
        ResolvedReferenceUri resolved)
    {
        // Strip package schemes if present; keep fragment.
        var working = uri;
        foreach (var prefix in new[] { "level:/", "fix:/" })
        {
            if (working.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
            {
                working = working[prefix.Length..];
                break;
            }
        }

        var hash = working.IndexOf('#');
        var pathPart = hash >= 0 ? working[..hash] : working;
        var fragment = hash >= 0 ? working[hash..] : "";

        if (Path.IsPathRooted(pathPart))
        {
            var root = Path.GetFullPath(packageRoot)
                .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
            var full = Path.GetFullPath(pathPart);
            if (full.StartsWith(root, StringComparison.OrdinalIgnoreCase))
            {
                pathPart = full[(root.Length + 1)..].Replace('\\', '/');
            }
        }

        return pathPart.Replace('\\', '/') + fragment;
    }

    private static string? FindPackageRoot(string filePath)
    {
        var directory = Directory.Exists(filePath)
            ? filePath
            : Path.GetDirectoryName(filePath);

        while (!string.IsNullOrEmpty(directory))
        {
            var manifestPath = Path.Combine(directory, OpenSpacePackageCodec.ManifestFileName);
            if (File.Exists(manifestPath))
            {
                return Path.GetFullPath(directory);
            }

            directory = Directory.GetParent(directory)?.FullName;
        }

        return null;
    }

    private bool TryResolveLevelSlotAddress(
        string levelPackageRoot,
        string uri,
        string resolvedFilePath,
        out int address)
    {
        address = 0;
        if (!uri.StartsWith(ReferenceUri.LevelPrefix + "slots/", StringComparison.Ordinal) ||
            !File.Exists(resolvedFilePath))
        {
            return false;
        }

        if (!StructCodecRegistry.TryGet(RawBlobCodec.Instance.Kind, out var codec) ||
            !codec.UsesExternalBinaryPayload)
        {
            return false;
        }

        try
        {
            var record = (OpaqueBinaryRecord)codec.ReadFromJsonPath(levelPackageRoot, resolvedFilePath);
            if (string.IsNullOrWhiteSpace(record.Path))
            {
                return false;
            }

            var innerResolved = ReferenceUri.Resolve(levelPackageRoot, record.Path);
            return _indexes[levelPackageRoot].TryGetAddress(innerResolved.FilePath, out address);
        }
        catch (JsonException)
        {
            return false;
        }
        catch (InvalidDataException)
        {
            return false;
        }
    }

    private static bool TryReadByteOffsetFragment(string fragment, out int byteOffset)
    {
        const string Prefix = "byteOffset=";
        byteOffset = 0;

        if (!fragment.StartsWith(Prefix, StringComparison.Ordinal))
        {
            return false;
        }

        return int.TryParse(fragment[Prefix.Length..], out byteOffset);
    }

    private static bool TryResolveAnimationTreeFragment(
        string packageRoot,
        string? jsonPointer,
        out int virtualAddress)
    {
        // Addresses for animation fragments come from the package address index built by
        // linearizing content.json (export layout). Provenance VAs are never required.
        virtualAddress = 0;
        _ = packageRoot;
        _ = jsonPointer;
        return false;
    }
}

internal sealed class RetePackageAddressIndex
{
    private readonly Dictionary<int, string> _pathByAddress = new();
    private readonly Dictionary<string, int> _addressByPath = new(StringComparer.OrdinalIgnoreCase);
    private readonly List<AddressRange> _ranges = new();

    private RetePackageAddressIndex()
    {
    }

    public static RetePackageAddressIndex Load(string packageRoot, JsonSerializerOptions jsonOptions)
    {
        var index = new RetePackageAddressIndex();
        var manifestPath = Path.Combine(packageRoot, OpenSpacePackageCodec.ManifestFileName);
        var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
            File.ReadAllText(manifestPath),
            jsonOptions) ?? throw new InvalidDataException($"Could not read Rete manifest: {manifestPath}");

        foreach (var snaFile in manifest.SnaFiles)
        {
            foreach (var block in snaFile.Blocks.OrderBy(b => b.Order))
            {
                if (block.ContentPath == null)
                {
                    continue;
                }

                var contentPath = ResolvePackagePath(packageRoot, block.ContentPath);
                var content = JsonSerializer.Deserialize<SnaBlockContentDocument>(
                    File.ReadAllText(contentPath),
                    jsonOptions) ?? throw new InvalidDataException($"Could not read SNA block content: {contentPath}");

                var cursor = 0;
                AnimationTreeStore? animStore = null;
                AnimationTreeStore GetAnimStore()
                {
                    if (animStore == null)
                    {
                        animStore = new AnimationTreeStore();
                        animStore.Load(packageRoot);
                    }

                    return animStore;
                }

                foreach (var leaf in SnaBlockContentLinearizer.Linearize(packageRoot, content))
                {
                    var length = DetermineLeafLength(packageRoot, leaf, GetAnimStore);
                    var virtualAddress = checked(content.BaseInMemory + cursor);
                    var resolved = ReferenceUri.Resolve(packageRoot, leaf.DataPath);
                    var hasFragment = !string.IsNullOrEmpty(resolved.JsonPointer);

                    // Fragment URIs (animation pool leaves) share one file — index by full URI only.
                    if (hasFragment)
                    {
                        index.AddUriKey(virtualAddress, length, leaf.DataPath.Replace('\\', '/'));
                    }
                    else
                    {
                        index.Add(virtualAddress, length, resolved.FilePath);
                    }

                    cursor = checked(cursor + length);
                }
            }
        }

        return index;
    }

    public bool TryGetPath(int virtualAddress, out string path, out int byteOffset)
    {
        if (_pathByAddress.TryGetValue(virtualAddress, out path!))
        {
            byteOffset = 0;
            return true;
        }

        AddressRange? bestRange = null;
        foreach (var range in _ranges)
        {
            if (!range.Contains(virtualAddress))
            {
                continue;
            }

            if (bestRange == null || range.Length < bestRange.Length)
            {
                bestRange = range;
            }
        }

        if (bestRange is { } match)
        {
            path = match.Path;
            byteOffset = checked(virtualAddress - match.Start);
            return true;
        }

        path = "";
        byteOffset = 0;
        return false;
    }

    public bool TryGetAddress(string path, out int virtualAddress)
    {
        if (_addressByPath.TryGetValue(Path.GetFullPath(path), out virtualAddress))
        {
            return true;
        }

        // URI keys (package-relative with fragment)
        return _addressByUri.TryGetValue(path.Replace('\\', '/'), out virtualAddress);
    }

    private readonly Dictionary<string, int> _addressByUri = new(StringComparer.OrdinalIgnoreCase);

    private void Add(int virtualAddress, int length, string dataPath)
    {
        var normalizedPath = Path.GetFullPath(dataPath);
        _pathByAddress.TryAdd(virtualAddress, normalizedPath);
        _addressByPath.TryAdd(normalizedPath, virtualAddress);
        if (length > 0)
        {
            _ranges.Add(new AddressRange(virtualAddress, length, normalizedPath));
        }
    }

    public void AddUriKey(int virtualAddress, int length, string uri)
    {
        var key = uri.Replace('\\', '/');
        _addressByUri.TryAdd(key, virtualAddress);
        _pathByAddress.TryAdd(virtualAddress, key);
        if (length > 0)
        {
            _ranges.Add(new AddressRange(virtualAddress, length, key));
        }
    }

    private static int DetermineLeafLength(
        string packageRoot,
        SnaBlockContentLinearizer.Leaf leaf,
        Func<AnimationTreeStore>? getAnimStore = null)
    {
        // Prefer fixed-size codecs (no disk/JSON work).
        if (StructCodecRegistry.TryGet(leaf.Kind, out var codec) && codec.FixedSize is { } fixedSize)
        {
            return fixedSize;
        }

        AnimationTreeStore? store = null;
        AnimationTreeStore Store()
        {
            store ??= getAnimStore?.Invoke() ?? LoadAnimStore(packageRoot);
            return store;
        }

        if (leaf.Kind.Equals("transform", StringComparison.OrdinalIgnoreCase) &&
            AnimationPaths.TryParseTransformById(
                ReferenceUri.Resolve(packageRoot, leaf.DataPath).JsonPointer,
                out var transformId) &&
            Store().TryGetTransform(transformId, out var transform))
        {
            return transform.WireBytes.Length + transform.TrailingGap.Length;
        }

        if (AnimationPaths.TryParseFamilyById(
                ReferenceUri.Resolve(packageRoot, leaf.DataPath).JsonPointer,
                out var nodeId) &&
            Store().TryGetNode(nodeId, out var node) &&
            node.Record is { } record)
        {
            if (record.ValueKind == JsonValueKind.Object &&
                record.TryGetProperty("path", out var pathProp) &&
                pathProp.ValueKind == JsonValueKind.String)
            {
                var rel = pathProp.GetString();
                if (!string.IsNullOrWhiteSpace(rel))
                {
                    var full = Path.Combine(rel.Split('/').Prepend(packageRoot).ToArray());
                    if (File.Exists(full))
                    {
                        return checked((int)new FileInfo(full).Length);
                    }
                }
            }

            if (StructCodecRegistry.TryGet(node.Kind, out var nodeCodec) &&
                nodeCodec.FixedSize is { } nodeFixed)
            {
                return nodeFixed;
            }

            if (StructCodecRegistry.TryGet(node.Kind, out nodeCodec) &&
                !nodeCodec.UsesExternalBinaryPayload)
            {
                return nodeCodec.WriteFromJsonElement(record).Length;
            }
        }

        var dataPath = ReferenceUri.Resolve(packageRoot, leaf.DataPath).FilePath;
        if (StructCodecRegistry.TryGet(leaf.Kind, out codec) &&
            dataPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase) &&
            File.Exists(dataPath))
        {
            return codec.WriteFromJsonPath(packageRoot, dataPath).Length;
        }

        if (File.Exists(dataPath))
        {
            return checked((int)new FileInfo(dataPath).Length);
        }

        return 0;
    }

    private static AnimationTreeStore LoadAnimStore(string packageRoot)
    {
        var store = new AnimationTreeStore();
        store.Load(packageRoot);
        return store;
    }

    private static string ResolvePackagePath(string packageRoot, string relativePath) =>
        Path.Combine(relativePath.Split('/').Prepend(packageRoot).ToArray());

    private sealed record AddressRange(int Start, int Length, string Path)
    {
        public bool Contains(int address) =>
            address >= Start && address < checked(Start + Length);
    }
}
