using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;
using Astrolabe.Core.Hub;
using Astrolabe.Core.Serialization;
using Astrolabe.Core.Serialization.Codecs;

namespace Astrolabe.Core.Rete;

internal static class AnimationTreeExport
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    public static bool TryWriteElementBytes(
        string packageRoot,
        string dataPath,
        ReferenceAddressResolver resolver,
        out byte[] bytes)
    {
        bytes = [];
        if (!dataPath.StartsWith(AnimationTreeDocument.RelativePath, StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        var resolved = ReferenceUri.Resolve(packageRoot, dataPath);
        var treePath = Path.Combine(packageRoot, AnimationTreeDocument.RelativePath);
        if (!File.Exists(treePath))
        {
            throw new InvalidDataException($"Animation tree not found: {treePath}");
        }

        var store = new AnimationTreeStore();
        store.Load(packageRoot);

        if (AnimationTreePaths.TryParseTransformFragment(resolved.JsonPointer, out var transformIndex) &&
            store.TryGetTransform(transformIndex, out var transform))
        {
            bytes = TransformCodec.Instance.Write(transform);
            return true;
        }

        if (!store.TryGetElementRecord(resolved.JsonPointer, out var entry))
        {
            throw new InvalidDataException($"Animation tree fragment not found: {dataPath}");
        }

        bytes = WritePromotedElementBytes(packageRoot, entry, resolver);
        return true;
    }

    private static byte[] WritePromotedElementBytes(
        string packageRoot,
        AnimationTreeElementEntry entry,
        ReferenceAddressResolver resolver)
    {
        if (!StructCodecRegistry.TryGet(entry.Kind, out var codec))
        {
            if (entry.Record.TryGetProperty("data", out var dataProperty) &&
                dataProperty.ValueKind == JsonValueKind.String)
            {
                return Convert.FromBase64String(dataProperty.GetString()!);
            }

            throw new InvalidDataException($"Unsupported animation tree element kind: {entry.Kind}");
        }

        using var resolvedDocument = ReferenceJson.ResolvePointersForExport(
            entry.Record,
            packageRoot,
            codec,
            resolver);
        return codec.WriteFromJsonElement(resolvedDocument.RootElement);
    }
}