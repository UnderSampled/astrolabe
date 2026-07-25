using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;
using Astrolabe.Core.Hub;
using Astrolabe.Core.Serialization;
using Astrolabe.Core.Serialization.Codecs;

namespace Astrolabe.Core.Rete;

internal static class AnimationTreeImporter
{
    private static readonly HashSet<string> AnimationKinds = new(StringComparer.OrdinalIgnoreCase)
    {
        "state",
        "animationmontreal",
        "animframes",
        "animchannel",
        "animchannelptrs",
        "animhierarchiesheader",
        "animhierarchies",
        "compressedmatrix",
        "transform"
    };

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    public static void AggregateLevelPackage(string packageDir, RetePackageManifest manifest)
    {
        if (!manifest.PackageRole.Equals("level", StringComparison.OrdinalIgnoreCase))
        {
            return;
        }

        var blocks = manifest.SnaFiles
            .SelectMany(file => file.Blocks)
            .Where(block => block.ContentPath != null &&
                            block.Key is "05:01" or "06:02")
            .ToList();
        if (blocks.Count == 0)
        {
            return;
        }

        var contexts = blocks
            .Select(block => LoadBlockContext(packageDir, block))
            .ToDictionary(context => context.BlockKey, StringComparer.OrdinalIgnoreCase);

        if (!contexts.ContainsKey("05:01"))
        {
            return;
        }

        var tree = BuildTree(packageDir, contexts);
        if (tree.Elements.Count == 0 && tree.Transforms.Count == 0)
        {
            return;
        }

        AnimationTreeStore.Write(packageDir, tree);
        foreach (var context in contexts.Values)
        {
            ApplyTreeToBlock(packageDir, context, tree);
        }

        DeleteLegacyAnimationFiles(packageDir);
    }

    private sealed class BlockContext
    {
        public required string BlockKey { get; init; }
        public required string ContentPath { get; init; }
        public required SnaBlockContentDocument Document { get; init; }
        public required byte[] BlockData { get; init; }
        public required Dictionary<int, SnaBlockContentElement> ElementsByAddress { get; init; }
    }

    private static BlockContext LoadBlockContext(string packageDir, SnaBlockManifest block)
    {
        var contentPath = ResolvePath(packageDir, block.ContentPath!);
        var document = ReadJson<SnaBlockContentDocument>(contentPath);
        var blockData = RebuildBlockData(packageDir, document);
        var elementsByAddress = document.Elements
            .Where(element => element.VirtualAddress != 0)
            .ToDictionary(element => element.VirtualAddress, element => element);

        return new BlockContext
        {
            BlockKey = block.Key,
            ContentPath = block.ContentPath!,
            Document = document,
            BlockData = blockData,
            ElementsByAddress = elementsByAddress
        };
    }

    private static byte[] RebuildBlockData(string packageDir, SnaBlockContentDocument document)
    {
        using var stream = new MemoryStream();
        foreach (var element in document.Elements.OrderBy(entry => entry.Order))
        {
            var path = ResolvePath(packageDir, element.DataPath);
            if (!File.Exists(path))
            {
                throw new FileNotFoundException($"Missing element payload: {path}");
            }

            stream.Write(File.ReadAllBytes(path));
        }

        return stream.ToArray();
    }

    private static AnimationTreeDocument BuildTree(
        string packageDir,
        IReadOnlyDictionary<string, BlockContext> contexts)
    {
        var animationContext = contexts["05:01"];
        var transformContext = contexts.TryGetValue("06:02", out var block06) ? block06 : null;
        var tree = new AnimationTreeDocument();
        var transformIndexByAddress = new Dictionary<int, int>();
        var orderedTransformAddresses = new List<int>();

        var addressByDataPath = animationContext.Document.Elements
            .Concat(transformContext?.Document.Elements ?? [])
            .ToDictionary(
                entry => ReferenceUri.Resolve(packageDir, entry.DataPath).FilePath,
                entry => entry.VirtualAddress,
                StringComparer.OrdinalIgnoreCase);

        foreach (var element in animationContext.Document.Elements
                     .Where(entry => AnimationKinds.Contains(entry.Kind))
                     .OrderBy(entry => entry.VirtualAddress))
        {
            var recordJson = ReadElementJson(packageDir, element);
            if (element.Kind.Equals("animchannel", StringComparison.OrdinalIgnoreCase))
            {
                recordJson = RewriteChannelTransformReferences(
                    packageDir,
                    transformContext,
                    recordJson,
                    transformIndexByAddress,
                    orderedTransformAddresses,
                    tree);
            }

            recordJson = RewriteAnimationElementReferences(recordJson, packageDir, addressByDataPath);

            tree.Elements[element.VirtualAddress.ToString("X8")] = new AnimationTreeElementEntry
            {
                Kind = NormalizePromotedKind(element.Kind),
                VirtualAddress = element.VirtualAddress,
                OffsetInBlock = element.OffsetInBlock,
                Length = element.Length,
                Record = recordJson
            };
        }

        if (transformContext != null)
        {
            AbsorbTransformStream(
                packageDir,
                transformContext,
                tree,
                transformIndexByAddress,
                orderedTransformAddresses);
        }

        return tree;
    }

    private static JsonElement RewriteChannelTransformReferences(
        string packageDir,
        BlockContext? transformContext,
        JsonElement recordJson,
        Dictionary<int, int> transformIndexByAddress,
        List<int> orderedTransformAddresses,
        AnimationTreeDocument tree)
    {
        if (recordJson.ValueKind != JsonValueKind.Object)
        {
            return recordJson;
        }

        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream, new JsonWriterOptions { Indented = false }))
        {
            writer.WriteStartObject();
            foreach (var property in recordJson.EnumerateObject())
            {
                if (property.Name is "isIdentity" or "unknown10")
                {
                    writer.WritePropertyName(property.Name);
                    WriteTransformReference(
                        property.Value,
                        packageDir,
                        transformContext,
                        writer,
                        transformIndexByAddress,
                        orderedTransformAddresses,
                        tree);
                    continue;
                }

                property.WriteTo(writer);
            }

            writer.WriteEndObject();
        }

        using var document = JsonDocument.Parse(stream.ToArray());
        return document.RootElement.Clone();
    }

    private static JsonElement RewriteAnimationElementReferences(
        JsonElement recordJson,
        string packageDir,
        IReadOnlyDictionary<string, int> addressByDataPath)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream))
        {
            RewriteReferenceValues(recordJson, packageDir, addressByDataPath, writer);
        }

        using var document = JsonDocument.Parse(stream.ToArray());
        return document.RootElement.Clone();
    }

    private static void RewriteReferenceValues(
        JsonElement value,
        string packageDir,
        IReadOnlyDictionary<string, int> addressByDataPath,
        Utf8JsonWriter writer)
    {
        switch (value.ValueKind)
        {
            case JsonValueKind.Object:
                writer.WriteStartObject();
                foreach (var property in value.EnumerateObject())
                {
                    writer.WritePropertyName(property.Name);
                    RewriteReferenceValues(property.Value, packageDir, addressByDataPath, writer);
                }

                writer.WriteEndObject();
                break;
            case JsonValueKind.Array:
                writer.WriteStartArray();
                foreach (var item in value.EnumerateArray())
                {
                    RewriteReferenceValues(item, packageDir, addressByDataPath, writer);
                }

                writer.WriteEndArray();
                break;
            case JsonValueKind.String:
                var text = value.GetString();
                if (!string.IsNullOrWhiteSpace(text) &&
                    TryResolveVirtualAddress(packageDir, text, out var virtualAddress))
                {
                    writer.WriteStringValue(AnimationTreePaths.ElementUri(virtualAddress));
                }
                else if (!string.IsNullOrWhiteSpace(text) &&
                         text.StartsWith("types/", StringComparison.OrdinalIgnoreCase) &&
                         ReferenceUri.Resolve(packageDir, text).FilePath is { } resolvedPath &&
                         addressByDataPath.TryGetValue(resolvedPath, out var mappedAddress))
                {
                    writer.WriteStringValue(AnimationTreePaths.ElementUri(mappedAddress));
                }
                else
                {
                    writer.WriteStringValue(text);
                }

                break;
            default:
                value.WriteTo(writer);
                break;
        }
    }

    private static void WriteTransformReference(
        JsonElement value,
        string packageDir,
        BlockContext? transformContext,
        Utf8JsonWriter writer,
        Dictionary<int, int> transformIndexByAddress,
        List<int> orderedTransformAddresses,
        AnimationTreeDocument tree)
    {
        if (value.ValueKind == JsonValueKind.Number && value.TryGetInt32(out var wire))
        {
            if (wire is 0 or 1)
            {
                writer.WriteNumberValue(wire);
                return;
            }

            if (transformContext != null &&
                transformIndexByAddress.TryGetValue(wire, out var existing))
            {
                writer.WriteStringValue(AnimationTreePaths.TransformUri(existing));
                return;
            }
        }

        if (value.ValueKind == JsonValueKind.String)
        {
            var uri = value.GetString();
            if (string.IsNullOrWhiteSpace(uri))
            {
                writer.WriteNullValue();
                return;
            }

            if (TryResolveVirtualAddress(packageDir, uri, out var virtualAddress) &&
                transformContext != null)
            {
                var transformIndex = GetOrCreateTransformIndex(
                    packageDir,
                    transformContext,
                    virtualAddress,
                    transformIndexByAddress,
                    orderedTransformAddresses,
                    tree);
                writer.WriteStringValue(AnimationTreePaths.TransformUri(transformIndex));
                return;
            }
        }

        value.WriteTo(writer);
    }

    private static int GetOrCreateTransformIndex(
        string packageDir,
        BlockContext transformContext,
        int virtualAddress,
        Dictionary<int, int> transformIndexByAddress,
        List<int> orderedTransformAddresses,
        AnimationTreeDocument tree)
    {
        if (transformIndexByAddress.TryGetValue(virtualAddress, out var existing))
        {
            return existing;
        }

        var transform = ReadTransformAtAddress(packageDir, transformContext, virtualAddress);
        var transformIndex = tree.Transforms.Count;
        tree.Transforms.Add(transform);
        transformIndexByAddress[virtualAddress] = transformIndex;
        orderedTransformAddresses.Add(virtualAddress);
        return transformIndex;
    }

    private static void AbsorbTransformStream(
        string packageDir,
        BlockContext transformContext,
        AnimationTreeDocument tree,
        Dictionary<int, int> transformIndexByAddress,
        List<int> orderedTransformAddresses)
    {
        var compressedMatrices = transformContext.Document.Elements
            .Where(element => element.Kind.Equals("compressedmatrix", StringComparison.OrdinalIgnoreCase))
            .OrderBy(element => element.VirtualAddress)
            .ToList();

        foreach (var element in compressedMatrices)
        {
            if (transformIndexByAddress.ContainsKey(element.VirtualAddress))
            {
                continue;
            }

            var transform = ReadTransformAtAddress(packageDir, transformContext, element.VirtualAddress);
            transformIndexByAddress[element.VirtualAddress] = tree.Transforms.Count;
            orderedTransformAddresses.Add(element.VirtualAddress);
            tree.Transforms.Add(transform);
        }
    }

    private static TransformRecord ReadTransformAtAddress(
        string packageDir,
        BlockContext transformContext,
        int virtualAddress)
    {
        if (!transformContext.ElementsByAddress.TryGetValue(virtualAddress, out var element))
        {
            throw new InvalidDataException($"Transform element not found at 0x{virtualAddress:X8}.");
        }

        var path = ResolvePath(packageDir, element.DataPath);
        var bytes = File.ReadAllBytes(path);
        var wireLength = TransformWire.GetPayloadLength(bytes);
        if (wireLength > bytes.Length)
        {
            wireLength = bytes.Length;
        }

        var trailingGap = Array.Empty<byte>();
        var gapElement = FindAdjacentGapElement(transformContext, element);
        if (gapElement != null)
        {
            var gapPath = ResolvePath(packageDir, gapElement.DataPath);
            trailingGap = File.ReadAllBytes(gapPath);
        }
        else
        {
            var offset = element.OffsetInBlock;
            var nextTransformOffset = FindNextTransformOffset(transformContext, element.VirtualAddress);
            var gapLength = TransformWire.GetTrailingGapLength(
                transformContext.BlockData,
                offset,
                wireLength,
                nextTransformOffset);
            if (gapLength > 0)
            {
                trailingGap = transformContext.BlockData.AsSpan(offset + wireLength, gapLength).ToArray();
            }
        }

        return new TransformRecord
        {
            VirtualAddress = virtualAddress,
            WireBytes = bytes.AsSpan(0, wireLength).ToArray(),
            TrailingGap = trailingGap
        };
    }

    private static SnaBlockContentElement? FindAdjacentGapElement(
        BlockContext context,
        SnaBlockContentElement transformElement)
    {
        var expectedOffset = transformElement.OffsetInBlock + transformElement.Length;
        return context.Document.Elements.FirstOrDefault(element =>
            element.Kind.Equals("raw", StringComparison.OrdinalIgnoreCase) &&
            element.OffsetInBlock == expectedOffset &&
            element.Length is 4 or 6);
    }

    private static int? FindNextTransformOffset(BlockContext context, int virtualAddress)
    {
        return context.Document.Elements
            .Where(element => element.Kind.Equals("compressedmatrix", StringComparison.OrdinalIgnoreCase) &&
                              element.VirtualAddress > virtualAddress)
            .OrderBy(element => element.VirtualAddress)
            .Select(element => (int?)element.OffsetInBlock)
            .FirstOrDefault();
    }

    private static void ApplyTreeToBlock(
        string packageDir,
        BlockContext context,
        AnimationTreeDocument tree)
    {
        var elementsToRemove = new HashSet<SnaBlockContentElement>();
        var replacements = new List<(SnaBlockContentElement Original, SnaBlockContentElement Replacement)>();

        foreach (var element in context.Document.Elements.ToList())
        {
            if (element.Kind.Equals("compressedmatrix", StringComparison.OrdinalIgnoreCase))
            {
                var gap = FindAdjacentGapElement(context, element);
                if (gap != null)
                {
                    elementsToRemove.Add(gap);
                }

                if (!tree.Elements.TryGetValue(element.VirtualAddress.ToString("X8"), out _) &&
                    !tree.Transforms.Any(transform => transform.VirtualAddress == element.VirtualAddress))
                {
                    continue;
                }

                var transformIndex = tree.Transforms.FindIndex(
                    transform => transform.VirtualAddress == element.VirtualAddress);
                if (transformIndex < 0)
                {
                    continue;
                }

                var mergedLength = element.Length + (gap?.Length ?? 0);
                replacements.Add((element, new SnaBlockContentElement
                {
                    Order = element.Order,
                    Kind = "transform",
                    DataPath = AnimationTreePaths.TransformUri(transformIndex),
                    OffsetInBlock = element.OffsetInBlock,
                    Length = mergedLength,
                    VirtualAddress = element.VirtualAddress,
                    VirtualAddressHex = element.VirtualAddressHex,
                    Sha256 = element.Sha256,
                    Labels = ["Transform"]
                }));
                elementsToRemove.Add(element);
                continue;
            }

            if (!AnimationKinds.Contains(element.Kind) ||
                element.Kind.Equals("compressedmatrix", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            if (!tree.Elements.ContainsKey(element.VirtualAddress.ToString("X8")))
            {
                continue;
            }

            replacements.Add((element, new SnaBlockContentElement
            {
                Order = element.Order,
                Kind = NormalizePromotedKind(element.Kind),
                DataPath = AnimationTreePaths.ElementUri(element.VirtualAddress),
                OffsetInBlock = element.OffsetInBlock,
                Length = element.Length,
                VirtualAddress = element.VirtualAddress,
                VirtualAddressHex = element.VirtualAddressHex,
                Sha256 = element.Sha256,
                Labels = element.Labels
            }));
            elementsToRemove.Add(element);
        }

        foreach (var (original, replacement) in replacements)
        {
            var index = context.Document.Elements.IndexOf(original);
            if (index >= 0)
            {
                context.Document.Elements[index] = replacement;
            }
        }

        context.Document.Elements.RemoveAll(elementsToRemove.Contains);
        WriteJson(ResolvePath(packageDir, context.ContentPath), context.Document);
    }

    private static void DeleteLegacyAnimationFiles(string packageDir)
    {
        foreach (var kind in new[]
                 {
                     "animchannel", "animframes", "animationmontreal", "animchannelptrs",
                     "animhierarchiesheader", "animhierarchies", "compressedmatrix", "state"
                 })
        {
            var dir = Path.Combine(packageDir, "types", kind);
            if (Directory.Exists(dir))
            {
                Directory.Delete(dir, recursive: true);
            }
        }
    }

    private static JsonElement ReadElementJson(string packageDir, SnaBlockContentElement element)
    {
        var path = ResolvePath(packageDir, element.DataPath);
        if (path.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
        {
            using var document = JsonDocument.Parse(File.ReadAllText(path));
            return document.RootElement.Clone();
        }

        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream))
        {
            writer.WriteStartObject();
            writer.WriteString("schema", GetBinarySchema(element.Kind));
            writer.WriteString("path", element.DataPath);
            writer.WriteBase64String("data", File.ReadAllBytes(path));
            writer.WriteEndObject();
        }

        using var wrapped = JsonDocument.Parse(stream.ToArray());
        return wrapped.RootElement.Clone();
    }

    private static string GetBinarySchema(string kind) =>
        kind.ToLowerInvariant() switch
        {
            "animchannelptrs" => "astrolabe.pointer-array.v1",
            "animhierarchies" => AnimHierarchiesCodec.Instance.Schema,
            _ => $"astrolabe.{kind.ToLowerInvariant()}.v1"
        };

    private static string NormalizePromotedKind(string kind) =>
        kind.Equals("compressedmatrix", StringComparison.OrdinalIgnoreCase) ? "transform" : kind;

    private static bool TryResolveVirtualAddress(string packageDir, string uri, out int virtualAddress)
    {
        virtualAddress = 0;
        var resolved = ReferenceUri.Resolve(packageDir, uri);
        if (!File.Exists(resolved.FilePath))
        {
            return false;
        }

        var manifestPath = Path.Combine(packageDir, OpenSpacePackageCodec.ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            return false;
        }

        var manifest = ReadJson<RetePackageManifest>(manifestPath);
        foreach (var snaFile in manifest.SnaFiles)
        {
            foreach (var block in snaFile.Blocks)
            {
                if (block.ContentPath == null)
                {
                    continue;
                }

                var content = ReadJson<SnaBlockContentDocument>(ResolvePath(packageDir, block.ContentPath));
                var match = content.Elements.FirstOrDefault(element =>
                    element.DataPath.Equals(uri, StringComparison.OrdinalIgnoreCase) ||
                    ReferenceUri.Resolve(packageDir, element.DataPath).FilePath.Equals(
                        resolved.FilePath,
                        StringComparison.OrdinalIgnoreCase));
                if (match != null)
                {
                    virtualAddress = match.VirtualAddress + ReadByteOffset(resolved.JsonPointer);
                    return true;
                }
            }
        }

        return false;
    }

    private static int ReadByteOffset(string? jsonPointer)
    {
        if (string.IsNullOrWhiteSpace(jsonPointer) ||
            !jsonPointer.StartsWith("byteOffset=", StringComparison.Ordinal))
        {
            return 0;
        }

        return int.TryParse(jsonPointer["byteOffset=".Length..], out var offset) ? offset : 0;
    }

    private static string ResolvePath(string rootDir, string relativePath) =>
        Path.Combine(relativePath.Split('/').Prepend(rootDir).ToArray());

    private static T ReadJson<T>(string path) where T : class =>
        JsonSerializer.Deserialize<T>(File.ReadAllText(path), JsonOptions)
        ?? throw new InvalidDataException($"Could not read {path}");

    private static void WriteJson(string path, object value)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllText(path, JsonSerializer.Serialize(value, JsonOptions));
    }
}