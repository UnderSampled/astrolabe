using System.Text.Json;
using System.Text.Json.Nodes;

namespace Astrolabe.Core.Serialization;

internal readonly record struct RelocationOverlayTarget(byte Module, byte Id, byte Byte6 = 0, byte Byte7 = 0);

internal static class RelocationPointerOverlay
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true
    };

    public static string GetOverlayPath(string elementPath) =>
        elementPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase)
            ? Path.ChangeExtension(elementPath, ".reloc.json")
            : elementPath + ".reloc.json";

    public static bool TryRead(
        string jsonPath,
        out Dictionary<string, string?> pointers,
        out Dictionary<string, RelocationOverlayTarget> targets)
    {
        pointers = new Dictionary<string, string?>(StringComparer.OrdinalIgnoreCase);
        targets = new Dictionary<string, RelocationOverlayTarget>(StringComparer.OrdinalIgnoreCase);
        if (!File.Exists(jsonPath))
        {
            return false;
        }

        using var document = JsonDocument.Parse(File.ReadAllText(jsonPath));
        var root = document.RootElement;
        if (root.TryGetProperty("pointers", out var pointersElement) &&
            pointersElement.ValueKind == JsonValueKind.Object)
        {
            foreach (var entry in pointersElement.EnumerateObject())
            {
                pointers[entry.Name] = entry.Value.ValueKind switch
                {
                    JsonValueKind.Null => null,
                    JsonValueKind.String => entry.Value.GetString(),
                    _ => throw new InvalidDataException(
                        $"Pointer overlay entry '{entry.Name}' in {jsonPath} must be a string or null.")
                };
            }
        }

        if (root.TryGetProperty("targets", out var targetsElement) &&
            targetsElement.ValueKind == JsonValueKind.Object)
        {
            foreach (var entry in targetsElement.EnumerateObject())
            {
                if (entry.Value.ValueKind != JsonValueKind.Object)
                {
                    throw new InvalidDataException(
                        $"Target overlay entry '{entry.Name}' in {jsonPath} must be an object.");
                }

                if (!entry.Value.TryGetProperty("module", out var moduleElement) ||
                    !entry.Value.TryGetProperty("id", out var idElement))
                {
                    throw new InvalidDataException(
                        $"Target overlay entry '{entry.Name}' in {jsonPath} requires 'module' and 'id'.");
                }

                var byte6 = entry.Value.TryGetProperty("byte6", out var byte6Element)
                    ? byte6Element.GetByte()
                    : (byte)0;
                var byte7 = entry.Value.TryGetProperty("byte7", out var byte7Element)
                    ? byte7Element.GetByte()
                    : (byte)0;
                targets[entry.Name] = new RelocationOverlayTarget(
                    moduleElement.GetByte(),
                    idElement.GetByte(),
                    byte6,
                    byte7);
            }
        }

        return pointers.Count > 0 || targets.Count > 0;
    }

    public static void Merge(
        string jsonPath,
        IReadOnlyDictionary<string, string?> pointerUpdates,
        IReadOnlyDictionary<string, RelocationOverlayTarget>? targetUpdates = null)
    {
        if (pointerUpdates.Count == 0 && targetUpdates is not { Count: > 0 })
        {
            return;
        }

        JsonObject root;
        if (File.Exists(jsonPath))
        {
            root = JsonNode.Parse(File.ReadAllText(jsonPath)) as JsonObject
                ?? throw new InvalidDataException($"Expected JSON object in {jsonPath}.");
        }
        else
        {
            root = new JsonObject
            {
                ["schema"] = "astrolabe.relocation-overlay.v1"
            };
            Directory.CreateDirectory(Path.GetDirectoryName(jsonPath)!);
        }

        var changed = false;
        if (pointerUpdates.Count > 0)
        {
            var pointers = root["pointers"] as JsonObject ?? new JsonObject();
            foreach (var (offsetKey, uri) in pointerUpdates)
            {
                var existingUri = pointers[offsetKey]?.GetValue<string?>();
                if (string.Equals(existingUri, uri, StringComparison.Ordinal))
                {
                    continue;
                }

                pointers[offsetKey] = uri == null ? null : JsonValue.Create(uri);
                changed = true;
            }

            root["pointers"] = pointers;
        }

        if (targetUpdates is { Count: > 0 })
        {
            var targets = root["targets"] as JsonObject ?? new JsonObject();
            foreach (var (offsetKey, target) in targetUpdates)
            {
                if (targets[offsetKey] is JsonObject existing &&
                    existing["module"]?.GetValue<byte>() == target.Module &&
                    existing["id"]?.GetValue<byte>() == target.Id &&
                    (existing["byte6"]?.GetValue<byte>() ?? 0) == target.Byte6 &&
                    (existing["byte7"]?.GetValue<byte>() ?? 0) == target.Byte7)
                {
                    continue;
                }

                var targetObject = new JsonObject
                {
                    ["module"] = target.Module,
                    ["id"] = target.Id
                };
                if (target.Byte6 != 0)
                {
                    targetObject["byte6"] = target.Byte6;
                }

                if (target.Byte7 != 0)
                {
                    targetObject["byte7"] = target.Byte7;
                }

                targets[offsetKey] = targetObject;
                changed = true;
            }

            root["targets"] = targets;
        }

        if (!changed)
        {
            return;
        }

        File.WriteAllText(jsonPath, root.ToJsonString(JsonOptions));
    }
}