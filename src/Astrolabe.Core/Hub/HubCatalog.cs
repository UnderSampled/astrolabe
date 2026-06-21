using System.Text.Json;
using Astrolabe.Core.Rete;
using Astrolabe.Core.Serialization;
using Astrolabe.Core.Serialization.Codecs;

namespace Astrolabe.Core.Hub;

public sealed class HubCatalog
{
    private readonly Dictionary<string, HubElement> _byPath =
        new(StringComparer.OrdinalIgnoreCase);

    private readonly Dictionary<int, HubElement> _byVirtualAddress = new();

    public string PackageDir { get; }
    public RetePackageManifest Manifest { get; }

    public HubCatalog(string packageDir, RetePackageManifest manifest)
    {
        PackageDir = Path.GetFullPath(packageDir);
        Manifest = manifest;
    }

    public IReadOnlyCollection<HubElement> Elements => _byPath.Values;

    public static HubCatalog Load(string packageDir)
    {
        var manifest = OpenSpacePackageCodec.ReadReteManifest(packageDir);
        var catalog = new HubCatalog(packageDir, manifest);
        catalog.IndexElements();
        return catalog;
    }

    public bool TryGetByPath(string dataPath, out HubElement element)
    {
        element = null!;
        if (!HubFragmentResolver.TrySplitUri(dataPath, out var path, out _))
        {
            return false;
        }

        return _byPath.TryGetValue(NormalizePath(path), out element!);
    }

    public bool TryGetByVirtualAddress(int virtualAddress, out HubElement element) =>
        _byVirtualAddress.TryGetValue(virtualAddress, out element!);

    public IEnumerable<HubElement> GetElementsOfKind(string kind) =>
        _byPath.Values.Where(element =>
            element.Kind.Equals(kind, StringComparison.OrdinalIgnoreCase));

    public bool TryHydrate(HubElement element) => TryHydrateElement(element);

    public int ResolveVirtualAddress(HubReference? reference)
    {
        if (reference == null || reference.IsNull)
        {
            return 0;
        }

        if (reference.ResolvedAddress != 0)
        {
            return reference.ResolvedAddress;
        }

        if (reference.WireValue != 0)
        {
            return reference.WireValue;
        }

        if (string.IsNullOrWhiteSpace(reference.Uri) ||
            !HubFragmentResolver.TrySplitUri(reference.Uri, out _, out var byteOffset) ||
            !TryGetByPath(reference.Uri, out var element))
        {
            return 0;
        }

        return checked(element.VirtualAddress + byteOffset);
    }

    public T? Resolve<T>(HubReference? reference) where T : class
    {
        if (reference == null || reference.IsNull)
        {
            return null;
        }

        if (reference.Target is T typed)
        {
            return typed;
        }

        if (string.IsNullOrWhiteSpace(reference.Uri) ||
            !HubFragmentResolver.TrySplitUri(reference.Uri, out _, out var byteOffset) ||
            !TryGetByPath(reference.Uri, out var element))
        {
            return null;
        }

        if (byteOffset == 0)
        {
            if (!TryHydrateElement(element) || element.Value is not T resolved)
            {
                return null;
            }

            reference.Target = resolved;
            return resolved;
        }

        var fragmentAddress = checked(element.VirtualAddress + byteOffset);
        if (!TryGetByVirtualAddress(fragmentAddress, out var fragmentElement) ||
            !TryHydrateElement(fragmentElement) ||
            fragmentElement.Value is not T fragment)
        {
            return null;
        }

        reference.Target = fragment;
        return fragment;
    }

    public void ResolveReference(HubReference? reference)
    {
        if (reference == null || reference.IsNull || reference.Target != null)
        {
            return;
        }

        if (string.IsNullOrWhiteSpace(reference.Uri) ||
            !HubFragmentResolver.TrySplitUri(reference.Uri, out _, out var byteOffset) ||
            !TryGetByPath(reference.Uri, out var element))
        {
            return;
        }

        if (byteOffset == 0)
        {
            if (TryHydrateElement(element))
            {
                reference.Target = element.Value;
            }

            return;
        }

        var fragmentAddress = checked(element.VirtualAddress + byteOffset);
        if (TryGetByVirtualAddress(fragmentAddress, out var fragmentElement) &&
            TryHydrateElement(fragmentElement))
        {
            reference.Target = fragmentElement.Value;
        }
    }

    private void IndexElements()
    {
        foreach (var element in EnumerateManifestIndexEntries())
        {
            _byPath[NormalizePath(element.DataPath)] = element;
            if (element.VirtualAddress != 0)
            {
                _byVirtualAddress[element.VirtualAddress] = element;
            }
        }

        IndexSlotElements();
    }

    private IEnumerable<HubElement> EnumerateManifestIndexEntries()
    {
        foreach (var snaFile in Manifest.SnaFiles)
        {
            foreach (var block in snaFile.Blocks)
            {
                if (block.ContentPath == null)
                {
                    continue;
                }

                var contentPath = ResolvePath(PackageDir, block.ContentPath);
                var content = ReadJson<SnaBlockContentDocument>(contentPath);
                foreach (var entry in content.Elements.OrderBy(e => e.Order))
                {
                    var dataPath = NormalizePath(entry.DataPath);
                    if (dataPath.StartsWith("scene/", StringComparison.OrdinalIgnoreCase))
                    {
                        continue;
                    }

                    if (!StructCodecRegistry.TryGet(entry.Kind, out var codec))
                    {
                        continue;
                    }

                    var filePath = ReferenceUri.Resolve(PackageDir, dataPath).FilePath;
                    if (!File.Exists(filePath))
                    {
                        continue;
                    }

                    yield return new HubElement
                    {
                        Kind = entry.Kind,
                        DataPath = dataPath,
                        Schema = entry.Kind,
                        Value = null,
                        VirtualAddress = entry.VirtualAddress,
                        OffsetInBlock = entry.OffsetInBlock,
                        Length = entry.Length,
                        BlockModule = block.Module,
                        BlockId = block.Id,
                        BlockKey = block.Key
                    };
                }
            }
        }
    }

    private void IndexSlotElements()
    {
        var slotsDir = Path.Combine(PackageDir, "slots");
        if (!Directory.Exists(slotsDir))
        {
            return;
        }

        foreach (var jsonPath in Directory.EnumerateFiles(slotsDir, "*.json", SearchOption.TopDirectoryOnly))
        {
            var relative = NormalizePath(Path.Combine("slots", Path.GetFileName(jsonPath)));
            if (_byPath.ContainsKey(relative))
            {
                continue;
            }

            _byPath[relative] = new HubElement
            {
                Kind = InferSlotKind(relative),
                DataPath = relative,
                Schema = InferSlotKind(relative),
                Value = null,
                VirtualAddress = 0,
                OffsetInBlock = 0,
                Length = 0,
                BlockKey = "slots"
            };
        }
    }

    private bool TryHydrateElement(HubElement element)
    {
        if (element.IsHydrated)
        {
            return true;
        }

        var filePath = ReferenceUri.Resolve(PackageDir, element.DataPath).FilePath;
        if (!File.Exists(filePath))
        {
            return false;
        }

        try
        {
            if (element.BlockKey.Equals("slots", StringComparison.OrdinalIgnoreCase))
            {
                if (!TryHydrateSlotElement(filePath, element))
                {
                    return false;
                }
            }
            else if (!StructCodecRegistry.TryGet(element.Kind, out var codec))
            {
                element.Value = File.ReadAllBytes(filePath);
                element.Schema = element.Kind;
            }
            else if (filePath.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
            {
                element.Value = codec.ReadFromJsonPath(PackageDir, filePath);
                element.Schema = TryGetSchema(element.Value) ?? codec.Kind;
            }
            else
            {
                element.Value = File.ReadAllBytes(filePath);
                element.Schema = element.Kind;
            }

            ResolveRecordReferences(element.Value!);
            return element.IsHydrated;
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

    private bool TryHydrateSlotElement(string jsonPath, HubElement element)
    {
        var text = File.ReadAllText(jsonPath).TrimStart();
        if (!text.StartsWith('{'))
        {
            return false;
        }

        using var document = JsonDocument.Parse(text);
        var root = document.RootElement;
        var kind = root.TryGetProperty("schema", out var schemaElement)
            ? schemaElement.GetString()?.Replace("astrolabe.", "", StringComparison.Ordinal)
                .Replace(".v1", "", StringComparison.Ordinal)
            : null;
        kind = NormalizeKind(kind, element.DataPath);
        if (!StructCodecRegistry.TryGet(kind, out var codec))
        {
            return false;
        }

        element.Value = codec.ReadFromJsonElement(root);
        element.Schema = TryGetSchema(element.Value) ?? kind;
        return true;
    }

    private void ResolveRecordReferences(object record)
    {
        var type = record.GetType();
        foreach (var property in type.GetProperties())
        {
            if (property.PropertyType != typeof(HubReference))
            {
                continue;
            }

            if (property.GetValue(record) is not HubReference reference || reference.IsNull)
            {
                continue;
            }

            ResolveReference(reference);
        }
    }

    private static string NormalizePath(string path) =>
        path.Replace('\\', '/').TrimStart('/');

    private static string ResolvePath(string rootDir, string relativePath) =>
        Path.Combine(relativePath.Split('/').Prepend(rootDir).ToArray());

    private static string? TryGetSchema(object value)
    {
        var property = value.GetType().GetProperty("Schema");
        return property?.GetValue(value) as string;
    }

    private static string InferSlotKind(string _) => "objectlist";

    private static T ReadJson<T>(string path) where T : class
    {
        return JsonSerializer.Deserialize<T>(
            File.ReadAllText(path),
            JsonStructCodec.Options) ?? throw new InvalidDataException($"Could not read {path}");
    }

    private static string NormalizeKind(string? kind, string path)
    {
        if (!string.IsNullOrWhiteSpace(kind))
        {
            kind = kind.Replace("-", "", StringComparison.Ordinal);
            if (StructCodecRegistry.TryGet(kind, out _))
            {
                return kind;
            }
        }

        return InferSlotKind(path);
    }
}