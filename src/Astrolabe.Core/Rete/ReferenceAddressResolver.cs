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

        if (TryResolveLevelSlotAddress(packageRoot, uri, resolved.FilePath, out var slotAddress))
        {
            return slotAddress;
        }

        if (!_indexes[packageRoot].TryGetAddress(resolved.FilePath, out var address))
        {
            throw new InvalidDataException($"Reference target is not part of package content: {uri}");
        }

        if (string.IsNullOrWhiteSpace(resolved.JsonPointer))
        {
            return address;
        }

        if (TryReadByteOffsetFragment(resolved.JsonPointer, out var byteOffset))
        {
            return checked(address + byteOffset);
        }

        if (TryResolveAnimationTreeFragment(packageRoot, resolved.JsonPointer, out var animationAddress))
        {
            return animationAddress;
        }

        throw new InvalidDataException($"Unsupported reference fragment in URI: {uri}");
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
        virtualAddress = 0;
        var store = new AnimationTreeStore();
        store.Load(packageRoot);
        if (!store.IsLoaded)
        {
            return false;
        }

        if (AnimationTreePaths.TryParseTransformFragment(jsonPointer, out _))
        {
            return store.TryResolveTransformAddress(jsonPointer, out virtualAddress);
        }

        if (AnimationTreePaths.TryParseElementFragment(jsonPointer, out var elementAddress) &&
            store.TryGetElementRecord(jsonPointer, out var entry))
        {
            virtualAddress = entry.VirtualAddress != 0 ? entry.VirtualAddress : elementAddress;
            return virtualAddress != 0;
        }

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
                foreach (var element in content.Elements.OrderBy(e => e.Order))
                {
                    var offset = element.Length > 0 ? element.OffsetInBlock : cursor;
                    var length = element.Length > 0
                        ? element.Length
                        : DetermineElementLength(packageRoot, element);
                    var virtualAddress = element.Length > 0
                        ? element.VirtualAddress
                        : checked(content.BaseInMemory + offset);
                    var dataPath = ReferenceUri.Resolve(packageRoot, element.DataPath).FilePath;

                    index.Add(virtualAddress, length, dataPath);
                    cursor = checked(offset + length);
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

    public bool TryGetAddress(string path, out int virtualAddress) =>
        _addressByPath.TryGetValue(Path.GetFullPath(path), out virtualAddress);

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

    private static int DetermineElementLength(string packageRoot, SnaBlockContentElement element)
    {
        if (element.DataPath.StartsWith(AnimationTreeDocument.RelativePath, StringComparison.OrdinalIgnoreCase) &&
            AnimationTreeExport.TryWriteElementBytes(
                packageRoot,
                element.DataPath,
                new ReferenceAddressResolver(packageRoot),
                out var treeBytes))
        {
            return treeBytes.Length;
        }

        var dataPath = ReferenceUri.Resolve(packageRoot, element.DataPath).FilePath;
        if (StructCodecRegistry.TryGet(element.Kind, out var codec))
        {
            if (codec.FixedSize is { } fixedSize)
            {
                return fixedSize;
            }

            if (dataPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
            {
                return codec.WriteFromJsonPath(packageRoot, dataPath).Length;
            }
        }

        return checked((int)new FileInfo(dataPath).Length);
    }

    private static string ResolvePackagePath(string packageRoot, string relativePath) =>
        Path.Combine(relativePath.Split('/').Prepend(packageRoot).ToArray());

    private sealed record AddressRange(int Start, int Length, string Path)
    {
        public bool Contains(int address) =>
            address >= Start && address < checked(Start + Length);
    }
}
