using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;
using Astrolabe.Core.Serialization;
using Astrolabe.Core.Serialization.Codecs;

namespace Astrolabe.Core.Rete;

/// <summary>Writes animation family nodes and transform pool entries to wire bytes.</summary>
internal static class AnimationTreeExport
{
    [ThreadStatic]
    private static string? _cachedPackageRoot;

    [ThreadStatic]
    private static AnimationTreeStore? _cachedStore;

    private static AnimationTreeStore GetStore(string packageRoot)
    {
        var full = Path.GetFullPath(packageRoot);
        if (_cachedStore == null ||
            !string.Equals(_cachedPackageRoot, full, StringComparison.OrdinalIgnoreCase))
        {
            _cachedStore = new AnimationTreeStore();
            _cachedStore.Load(packageRoot);
            _cachedPackageRoot = full;
        }

        return _cachedStore;
    }

    /// <summary>Get the stored JSON record for an animation fragment URI (for opaque LUT reads).</summary>
    public static bool TryGetNodeRecordJson(string packageRoot, string dataPath, out JsonElement record)
    {
        record = default;
        var resolved = ReferenceUri.Resolve(packageRoot, dataPath);
        var relative = GetPackageRelative(packageRoot, resolved.FilePath);
        if (!relative.Equals(AnimationFamiliesDocument.RelativePath, StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        if (!AnimationPaths.TryParseFamilyById(resolved.JsonPointer, out var id))
        {
            return false;
        }

        if (!GetStore(packageRoot).TryGetNode(id, out var node) || node.Record is not { } json)
        {
            return false;
        }

        record = json;
        return true;
    }

    public static bool TryWriteElementBytes(
        string packageRoot,
        string dataPath,
        ReferenceAddressResolver resolver,
        out byte[] bytes)
    {
        bytes = [];
        var resolved = ReferenceUri.Resolve(packageRoot, dataPath);
        var relative = GetPackageRelative(packageRoot, resolved.FilePath);

        if (relative.Equals(AnimationTransformsDocument.RelativePath, StringComparison.OrdinalIgnoreCase))
        {
            return TryWriteTransform(packageRoot, resolved.JsonPointer, out bytes);
        }

        if (relative.Equals(AnimationFamiliesDocument.RelativePath, StringComparison.OrdinalIgnoreCase))
        {
            return TryWriteFamilyNode(packageRoot, resolved.JsonPointer, resolver, out bytes);
        }

        // Legacy WIP path
#pragma warning disable CS0618
        if (relative.Equals(AnimationTreeDocument.RelativePath, StringComparison.OrdinalIgnoreCase))
#pragma warning restore CS0618
        {
            throw new InvalidDataException(
                "Legacy animation/level.json is no longer supported. Re-import to families/transforms docs.");
        }

        return false;
    }

    private static bool TryWriteTransform(string packageRoot, string? jsonPointer, out byte[] bytes)
    {
        bytes = [];
        if (!AnimationPaths.TryParseTransformById(jsonPointer, out var id))
        {
            return false;
        }

        if (!GetStore(packageRoot).TryGetTransform(id, out var transform))
        {
            throw new InvalidDataException($"Transform not found: {id}");
        }

        bytes = TransformCodec.Instance.Write(transform);
        return true;
    }

    private static bool TryWriteFamilyNode(
        string packageRoot,
        string? jsonPointer,
        ReferenceAddressResolver resolver,
        out byte[] bytes)
    {
        bytes = [];
        if (!AnimationPaths.TryParseFamilyById(jsonPointer, out var id))
        {
            return false;
        }

        if (!GetStore(packageRoot).TryGetNode(id, out var node))
        {
            throw new InvalidDataException($"Animation node not found: {id}");
        }

        if (string.IsNullOrEmpty(node.Kind) ||
            node.Kind.Equals("group", StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidDataException($"Cannot serialize layout group as leaf: {id}");
        }

        if (node.Record is not { } record)
        {
            throw new InvalidDataException($"Animation node has no record payload: {id}");
        }

        if (!StructCodecRegistry.TryGet(node.Kind, out var codec))
        {
            if (record.ValueKind == JsonValueKind.Object &&
                record.TryGetProperty("data", out var dataProperty) &&
                dataProperty.ValueKind == JsonValueKind.String)
            {
                bytes = Convert.FromBase64String(dataProperty.GetString()!);
                return true;
            }

            throw new InvalidDataException($"Unsupported animation node kind: {node.Kind}");
        }

        if (codec.UsesExternalBinaryPayload)
        {
            // Opaque codecs need a on-disk JSON path to resolve sibling payload paths.
            var tmpDir = Path.Combine(packageRoot, "animation", ".export-tmp");
            Directory.CreateDirectory(tmpDir);
            var tmpJson = Path.Combine(tmpDir, $"{node.Id}.json");
            File.WriteAllText(tmpJson, record.GetRawText());
            try
            {
                bytes = codec.WriteFromJsonPath(packageRoot, tmpJson);
            }
            finally
            {
                try { File.Delete(tmpJson); } catch { /* ignore */ }
            }

            return true;
        }

        using var resolvedDoc = ReferenceJson.ResolvePointersForExport(
            record,
            packageRoot,
            codec,
            resolver);
        bytes = codec.WriteFromJsonElement(resolvedDoc.RootElement);
        return true;
    }

    private static string GetPackageRelative(string packageRoot, string fullPath)
    {
        var root = Path.GetFullPath(packageRoot)
            .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        var full = Path.GetFullPath(fullPath);
        if (full.StartsWith(root, StringComparison.OrdinalIgnoreCase))
        {
            return full[(root.Length + 1)..].Replace('\\', '/');
        }

        return Path.GetFileName(fullPath);
    }
}
