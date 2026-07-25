using System.Text.Json;
using Astrolabe.Core.FileFormats.Semantic;
using Astrolabe.Core.Serialization;
using Astrolabe.Core.Serialization.Codecs;

namespace Astrolabe.Core.Rete;

/// <summary>Writes dual-layer semantic pool nodes (scene/geometry/ai/character/sector) to wire bytes.</summary>
internal static class SemanticPoolExport
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    [ThreadStatic]
    private static string? _cacheRoot;

    [ThreadStatic]
    private static Dictionary<string, Dictionary<string, SemanticPoolNode>>? _byDoc;

    public static bool TryWriteElementBytes(
        string packageRoot,
        string dataPath,
        ReferenceAddressResolver resolver,
        out byte[] bytes)
    {
        bytes = [];
        var resolved = ReferenceUri.Resolve(packageRoot, dataPath);
        var relative = GetPackageRelative(packageRoot, resolved.FilePath);
        var docKey = SemanticPoolPaths.MatchDocumentRelative(relative);
        if (docKey == null)
        {
            return false;
        }

        EnsureCache(packageRoot);
        if (_byDoc == null || !_byDoc.TryGetValue(docKey, out var byId))
        {
            return false;
        }

        var pointer = (resolved.JsonPointer ?? "").TrimStart('/');
        if (pointer.StartsWith("byId/", StringComparison.OrdinalIgnoreCase))
        {
            var rest = pointer["byId/".Length..];
            string id;
            string? field = null;
            var slash = rest.IndexOf('/');
            if (slash >= 0)
            {
                id = rest[..slash];
                field = rest[(slash + 1)..];
            }
            else
            {
                id = rest;
            }

            if (!byId.TryGetValue(id, out var node))
            {
                throw new InvalidDataException($"Semantic pool node not found: {dataPath}");
            }

            if (field != null)
            {
                if (field.Equals("matrix", StringComparison.OrdinalIgnoreCase) && node.Matrix is { } matrix)
                {
                    bytes = WriteRecord("matrix", matrix, packageRoot, resolver);
                    return true;
                }

                if (field.Equals("staticMatrix", StringComparison.OrdinalIgnoreCase) &&
                    node.StaticMatrix is { } staticMatrix)
                {
                    bytes = WriteRecord("matrix", staticMatrix, packageRoot, resolver);
                    return true;
                }

                throw new InvalidDataException($"Unknown semantic pool field: {dataPath}");
            }

            bytes = WriteNode(packageRoot, node, resolver);
            return true;
        }

        return false;
    }

    public static bool TryGetNode(
        string packageRoot,
        string dataPath,
        out SemanticPoolNode node)
    {
        node = null!;
        var resolved = ReferenceUri.Resolve(packageRoot, dataPath);
        var relative = GetPackageRelative(packageRoot, resolved.FilePath);
        var docKey = SemanticPoolPaths.MatchDocumentRelative(relative);
        if (docKey == null)
        {
            return false;
        }

        var pointer = (resolved.JsonPointer ?? "").TrimStart('/');
        if (!pointer.StartsWith("byId/", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        var rest = pointer["byId/".Length..];
        var slash = rest.IndexOf('/');
        var id = slash >= 0 ? rest[..slash] : rest;
        if (id.Length == 0)
        {
            return false;
        }

        EnsureCache(packageRoot);
        if (_byDoc == null || !_byDoc.TryGetValue(docKey, out var byId) || !byId.TryGetValue(id, out node!))
        {
            return false;
        }

        return true;
    }

    private static byte[] WriteNode(
        string packageRoot,
        SemanticPoolNode node,
        ReferenceAddressResolver resolver)
    {
        if (!string.IsNullOrEmpty(node.BufferPath))
        {
            var bin = Path.Combine(packageRoot, node.BufferPath.Replace('/', Path.DirectorySeparatorChar));
            if (File.Exists(bin))
            {
                return File.ReadAllBytes(bin);
            }
        }

        if (node.Record is not { } record)
        {
            throw new InvalidDataException($"Semantic pool node has no record: {node.Id}");
        }

        // Opaque base64 payload (binary leaves).
        if (record.ValueKind == JsonValueKind.Object &&
            record.TryGetProperty("data", out var dataProp) &&
            dataProp.ValueKind == JsonValueKind.String &&
            !StructCodecRegistry.TryGet(node.Kind, out _))
        {
            return Convert.FromBase64String(dataProp.GetString()!);
        }

        // Dense buffer descriptor with path.
        if (record.ValueKind == JsonValueKind.Object &&
            record.TryGetProperty("path", out var pathProp) &&
            pathProp.ValueKind == JsonValueKind.String)
        {
            var rel = pathProp.GetString()!;
            var bin = Path.Combine(packageRoot, rel.Replace('/', Path.DirectorySeparatorChar));
            if (File.Exists(bin))
            {
                return File.ReadAllBytes(bin);
            }
        }

        return WriteRecord(node.Kind, record, packageRoot, resolver);
    }

    private static byte[] WriteRecord(
        string kind,
        JsonElement record,
        string packageRoot,
        ReferenceAddressResolver resolver)
    {
        if (!StructCodecRegistry.TryGet(kind, out var codec))
        {
            if (record.ValueKind == JsonValueKind.Object &&
                record.TryGetProperty("data", out var dataProp) &&
                dataProp.ValueKind == JsonValueKind.String)
            {
                return Convert.FromBase64String(dataProp.GetString()!);
            }

            throw new InvalidDataException($"Unsupported semantic pool kind: {kind}");
        }

        if (codec.UsesExternalBinaryPayload)
        {
            var tmpDir = Path.Combine(packageRoot, ".export-tmp");
            Directory.CreateDirectory(tmpDir);
            var tmpJson = Path.Combine(tmpDir, $"{Guid.NewGuid():N}.json");
            File.WriteAllText(tmpJson, record.GetRawText());
            try
            {
                return codec.WriteFromJsonPath(packageRoot, tmpJson);
            }
            finally
            {
                try { File.Delete(tmpJson); } catch { /* ignore */ }
            }
        }

        using var resolvedDoc = ReferenceJson.ResolvePointersForExport(
            record,
            packageRoot,
            codec,
            resolver);
        return codec.WriteFromJsonElement(resolvedDoc.RootElement);
    }

    private static void EnsureCache(string packageRoot)
    {
        var full = Path.GetFullPath(packageRoot);
        if (string.Equals(_cacheRoot, full, StringComparison.OrdinalIgnoreCase) && _byDoc != null)
        {
            return;
        }

        _cacheRoot = full;
        _byDoc = new Dictionary<string, Dictionary<string, SemanticPoolNode>>(StringComparer.OrdinalIgnoreCase);

        TryLoad(full, SceneTreeDocument.RelativePath, d =>
            JsonSerializer.Deserialize<SceneTreeDocument>(d, JsonOptions)?.ById);
        TryLoad(full, GeometryPoolDocument.RelativePath, d =>
            JsonSerializer.Deserialize<GeometryPoolDocument>(d, JsonOptions)?.ById);
        TryLoad(full, AiPoolDocument.RelativePath, d =>
            JsonSerializer.Deserialize<AiPoolDocument>(d, JsonOptions)?.ById);
        TryLoad(full, CharacterPoolDocument.RelativePath, d =>
            JsonSerializer.Deserialize<CharacterPoolDocument>(d, JsonOptions)?.ById);
        TryLoad(full, SectorPoolDocument.RelativePath, d =>
            JsonSerializer.Deserialize<SectorPoolDocument>(d, JsonOptions)?.ById);
    }

    private static void TryLoad(
        string packageRoot,
        string relative,
        Func<string, Dictionary<string, SemanticPoolNode>?> loader)
    {
        var path = Path.Combine(packageRoot, relative.Replace('/', Path.DirectorySeparatorChar));
        if (!File.Exists(path))
        {
            return;
        }

        var byId = loader(File.ReadAllText(path));
        if (byId != null)
        {
            _byDoc![relative] = byId;
        }
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
