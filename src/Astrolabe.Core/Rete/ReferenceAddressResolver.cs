using System.Text.Json;
using Astrolabe.Core.Serialization;

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
        var packageParent = Directory.GetParent(Path.GetFullPath(packageRoot))?.FullName;
        if (packageParent == null)
        {
            return resolver;
        }

        var fixPackageDir = Path.Combine(packageParent, "fix");
        if (File.Exists(Path.Combine(fixPackageDir, OpenSpacePackageCodec.ManifestFileName)))
        {
            resolver.LoadPackage(fixPackageDir);
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
            if (index.TryGetPath(virtualAddress, out var targetPath))
            {
                uri = ReferenceUri.MakeRelative(referringPackageRoot, targetPath);
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
}

internal sealed class RetePackageAddressIndex
{
    private readonly Dictionary<int, string> _pathByAddress = new();
    private readonly Dictionary<string, int> _addressByPath = new(StringComparer.OrdinalIgnoreCase);

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

                    index.Add(virtualAddress, dataPath);
                    cursor = checked(offset + length);
                }
            }
        }

        return index;
    }

    public bool TryGetPath(int virtualAddress, out string path) =>
        _pathByAddress.TryGetValue(virtualAddress, out path!);

    public bool TryGetAddress(string path, out int virtualAddress) =>
        _addressByPath.TryGetValue(Path.GetFullPath(path), out virtualAddress);

    private void Add(int virtualAddress, string dataPath)
    {
        var normalizedPath = Path.GetFullPath(dataPath);
        _pathByAddress.TryAdd(virtualAddress, normalizedPath);
        _addressByPath.TryAdd(normalizedPath, virtualAddress);
    }

    private static int DetermineElementLength(string packageRoot, SnaBlockContentElement element)
    {
        var dataPath = ReferenceUri.Resolve(packageRoot, element.DataPath).FilePath;
        if (StructCodecRegistry.TryGet(element.Kind, out var codec))
        {
            if (codec.FixedSize is { } fixedSize)
            {
                return fixedSize;
            }

            if (dataPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
            {
                using var document = JsonDocument.Parse(File.ReadAllText(dataPath));
                return codec.WriteFromJsonElement(document.RootElement).Length;
            }
        }

        return checked((int)new FileInfo(dataPath).Length);
    }

    private static string ResolvePackagePath(string packageRoot, string relativePath) =>
        Path.Combine(relativePath.Split('/').Prepend(packageRoot).ToArray());
}
