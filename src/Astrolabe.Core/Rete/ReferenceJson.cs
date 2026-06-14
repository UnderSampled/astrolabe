using System.Buffers.Binary;
using System.Text.Json;
using Astrolabe.Core.Serialization;
using Astrolabe.Core.Serialization.Codecs;

namespace Astrolabe.Core.Rete;

internal static class ReferenceJson
{
    public static bool RewritePointersToUris(
        string jsonPath,
        string packageRoot,
        IStructCodecBinding codec,
        ReferenceAddressResolver resolver)
    {
        if (codec.PointerFields.Count == 0 && !codec.IsPointerArray)
        {
            return false;
        }

        if (codec.UsesExternalBinaryPayload)
        {
            return RewriteOpaquePointersToUris(jsonPath, packageRoot, codec, resolver);
        }

        using var document = JsonDocument.Parse(File.ReadAllText(jsonPath));
        using var stream = new MemoryStream();
        using (var writer = CreateWriter(stream))
        {
            var changed = WriteObject(
                document.RootElement,
                writer,
                codec,
                value => RewritePointerValue(value, packageRoot, resolver));
            writer.Flush();
            if (!changed)
            {
                return false;
            }
        }

        File.WriteAllBytes(jsonPath, stream.ToArray());
        return true;
    }

    internal static byte[] WriteElementBytesForExport(
        string packageRoot,
        string kind,
        string dataPath,
        ReferenceAddressResolver resolver)
    {
        if (!StructCodecRegistry.TryGet(kind, out var codec))
        {
            return File.ReadAllBytes(ReferenceUri.Resolve(packageRoot, dataPath).FilePath);
        }

        var elementPath = ReferenceUri.Resolve(packageRoot, dataPath).FilePath;
        if (!elementPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
        {
            var binary = File.ReadAllBytes(elementPath);
            return ApplyRelocationPointerOverlay(
                packageRoot,
                RelocationPointerOverlay.GetOverlayPath(elementPath),
                binary,
                resolver,
                explicitInlineOverlay: false);
        }

        if (codec.UsesExternalBinaryPayload)
        {
            return WriteOpaqueElementBytesForExport(packageRoot, elementPath, codec, resolver);
        }

        using var document = JsonDocument.Parse(File.ReadAllText(elementPath));
        using var resolvedDocument = ResolvePointersForExport(
            document.RootElement,
            packageRoot,
            codec,
            resolver);
        var bytes = codec.WriteFromJsonElement(resolvedDocument.RootElement);
        return ApplyRelocationPointerOverlay(
            packageRoot,
            RelocationPointerOverlay.GetOverlayPath(elementPath),
            bytes,
            resolver,
            explicitInlineOverlay: false);
    }

    private static byte[] ApplyRelocationPointerOverlay(
        string packageRoot,
        string overlayPath,
        byte[] data,
        ReferenceAddressResolver resolver,
        IReadOnlyDictionary<string, string?>? inlineOverlay = null,
        bool explicitInlineOverlay = false)
    {
        var mergedOverlay = new Dictionary<string, string?>(StringComparer.OrdinalIgnoreCase);
        var inlineKeys = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        Dictionary<string, RelocationOverlayTarget>? targetOverlay = null;

        if (inlineOverlay != null)
        {
            foreach (var pair in inlineOverlay)
            {
                mergedOverlay[pair.Key] = pair.Value;
                inlineKeys.Add(pair.Key);
            }
        }

        if (RelocationPointerOverlay.TryRead(overlayPath, out var fileOverlay, out var fileTargets))
        {
            foreach (var pair in fileOverlay)
            {
                mergedOverlay.TryAdd(pair.Key, pair.Value);
            }

            targetOverlay = fileTargets;
        }

        if (mergedOverlay.Count == 0 && targetOverlay is not { Count: > 0 })
        {
            return data;
        }

        var offsetKeys = mergedOverlay.Keys
            .Concat(targetOverlay == null ? [] : targetOverlay.Keys)
            .Distinct(StringComparer.OrdinalIgnoreCase);
        var bytes = data.ToArray();
        foreach (var offsetKey in offsetKeys)
        {
            if (!TryParsePointerOffset(offsetKey, out var offset))
            {
                throw new InvalidDataException($"Invalid relocation overlay offset '{offsetKey}' in {overlayPath}.");
            }

            if (offset < 0 || offset % sizeof(int) != 0)
            {
                throw new InvalidDataException(
                    $"Misaligned relocation overlay offset '{offsetKey}' in {overlayPath}.");
            }

            if (offset + sizeof(int) > bytes.Length)
            {
                throw new InvalidDataException(
                    $"Relocation overlay offset '{offsetKey}' is out of range for element span in {overlayPath}.");
            }

            mergedOverlay.TryGetValue(offsetKey, out var uri);
            if (string.IsNullOrWhiteSpace(uri))
            {
                if (explicitInlineOverlay && inlineKeys.Contains(offsetKey))
                {
                    BinaryPrimitives.WriteInt32LittleEndian(bytes.AsSpan(offset, sizeof(int)), 0);
                }

                // Sidecar null entries preserve serialized bytes for URI-less relocation rows.
                continue;
            }

            var address = resolver.ResolveAddress(packageRoot, uri);
            BinaryPrimitives.WriteInt32LittleEndian(bytes.AsSpan(offset, sizeof(int)), address);
        }

        return bytes;
    }

    public static JsonDocument ResolvePointersForExport(
        JsonElement root,
        string packageRoot,
        IStructCodecBinding codec,
        ReferenceAddressResolver resolver)
    {
        if (codec.UsesExternalBinaryPayload ||
            (codec.PointerFields.Count == 0 && !codec.IsPointerArray))
        {
            return JsonDocument.Parse(root.GetRawText());
        }

        using var stream = new MemoryStream();
        using (var writer = CreateWriter(stream))
        {
            WriteObject(
                root,
                writer,
                codec,
                value => ResolvePointerValue(value, packageRoot, resolver));
            writer.Flush();
        }

        stream.Position = 0;
        return JsonDocument.Parse(stream);
    }

    private static bool RewriteOpaquePointersToUris(
        string jsonPath,
        string packageRoot,
        IStructCodecBinding codec,
        ReferenceAddressResolver resolver)
    {
        var record = (OpaqueBinaryRecord)codec.ReadFromJsonPath(packageRoot, jsonPath);
        var rewrittenPointers = new Dictionary<string, string?>(StringComparer.OrdinalIgnoreCase);

        foreach (var field in codec.EnumeratePointerFields(record.Data).OrderBy(f => f.Offset))
        {
            if (field.Offset < 0 || field.Offset + sizeof(int) > record.Data.Length)
            {
                continue;
            }

            var value = BinaryPrimitives.ReadInt32LittleEndian(record.Data.AsSpan(field.Offset, sizeof(int)));
            if (value == 0)
            {
                rewrittenPointers[FormatPointerOffset(field.Offset)] = null;
                continue;
            }

            if (resolver.TryGetReferenceUri(value, packageRoot, out var uri))
            {
                rewrittenPointers[FormatPointerOffset(field.Offset)] = uri;
            }
        }

        if (PointerMapsEqual(record.Pointers, rewrittenPointers))
        {
            return false;
        }

        record.Pointers = rewrittenPointers;
        codec.WriteJson(packageRoot, jsonPath, record);
        return true;
    }

    private static byte[] WriteOpaqueElementBytesForExport(
        string packageRoot,
        string jsonPath,
        IStructCodecBinding codec,
        ReferenceAddressResolver resolver)
    {
        var record = (OpaqueBinaryRecord)codec.ReadFromJsonPath(packageRoot, jsonPath);
        var data = record.Data.ToArray();
        return ApplyRelocationPointerOverlay(
            packageRoot,
            RelocationPointerOverlay.GetOverlayPath(jsonPath),
            data,
            resolver,
            record.Pointers,
            explicitInlineOverlay: true);
    }

    private static Utf8JsonWriter CreateWriter(Stream stream) =>
        new(stream, new JsonWriterOptions { Indented = true });

    private static bool WriteObject(
        JsonElement root,
        Utf8JsonWriter writer,
        IStructCodecBinding codec,
        Func<JsonElement, WriteReplacement> replacementFactory)
    {
        if (root.ValueKind != JsonValueKind.Object)
        {
            root.WriteTo(writer);
            return false;
        }

        var changed = false;
        var pointerNames = codec.PointerFields
            .Select(f => f.Name)
            .ToHashSet(StringComparer.OrdinalIgnoreCase);
        var aliases = codec.PointerFieldAliases;

        writer.WriteStartObject();
        foreach (var property in root.EnumerateObject())
        {
            if (ShouldSkipLegacyAliasProperty(root, property.Name, aliases))
            {
                continue;
            }

            var outputName = ResolvePointerPropertyName(property.Name, aliases);
            writer.WritePropertyName(outputName);

            if (codec.IsPointerArray &&
                outputName.Equals(codec.PointerArrayPropertyName, StringComparison.OrdinalIgnoreCase) &&
                property.Value.ValueKind == JsonValueKind.Array)
            {
                if (WritePointerArray(property.Value, writer, replacementFactory))
                {
                    changed = true;
                }

                continue;
            }

            if (pointerNames.Contains(outputName))
            {
                var replacement = replacementFactory(property.Value);
                if (replacement.Kind != ReplacementKind.Unchanged)
                {
                    replacement.WriteTo(writer);
                    changed = true;
                    continue;
                }
            }

            property.Value.WriteTo(writer);
        }

        writer.WriteEndObject();
        return changed;
    }

    private static bool ShouldSkipLegacyAliasProperty(
        JsonElement root,
        string propertyName,
        IReadOnlyDictionary<string, string> aliases)
    {
        if (!aliases.TryGetValue(propertyName, out var canonicalName))
        {
            return false;
        }

        foreach (var property in root.EnumerateObject())
        {
            if (property.Name.Equals(canonicalName, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
        }

        return false;
    }

    private static string ResolvePointerPropertyName(
        string propertyName,
        IReadOnlyDictionary<string, string> aliases) =>
        aliases.TryGetValue(propertyName, out var canonicalName) ? canonicalName : propertyName;

    private static bool WritePointerArray(
        JsonElement array,
        Utf8JsonWriter writer,
        Func<JsonElement, WriteReplacement> replacementFactory)
    {
        var changed = false;
        writer.WriteStartArray();
        foreach (var item in array.EnumerateArray())
        {
            var replacement = replacementFactory(item);
            if (replacement.Kind != ReplacementKind.Unchanged)
            {
                replacement.WriteTo(writer);
                changed = true;
            }
            else
            {
                item.WriteTo(writer);
            }
        }

        writer.WriteEndArray();
        return changed;
    }

    private static WriteReplacement RewritePointerValue(
        JsonElement value,
        string packageRoot,
        ReferenceAddressResolver resolver)
    {
        if (value.ValueKind != JsonValueKind.Number ||
            !TryReadPointerAddress(value, out var address))
        {
            return WriteReplacement.Unchanged;
        }

        if (address == 0)
        {
            return WriteReplacement.Null;
        }

        return resolver.TryGetReferenceUri(address, packageRoot, out var uri)
            ? WriteReplacement.String(uri)
            : WriteReplacement.Unchanged;
    }

    private static bool TryReadPointerAddress(JsonElement value, out int address)
    {
        address = 0;
        if (value.ValueKind != JsonValueKind.Number)
        {
            return false;
        }

        if (value.TryGetInt32(out address))
        {
            return true;
        }

        if (value.TryGetUInt32(out var unsigned) && unsigned <= int.MaxValue)
        {
            address = (int)unsigned;
            return true;
        }

        if (value.TryGetInt64(out var wide) && wide is >= 0 and <= int.MaxValue)
        {
            address = (int)wide;
            return true;
        }

        return false;
    }

    private static WriteReplacement ResolvePointerValue(
        JsonElement value,
        string packageRoot,
        ReferenceAddressResolver resolver)
    {
        if (value.ValueKind == JsonValueKind.Null)
        {
            return WriteReplacement.Number(0);
        }

        if (value.ValueKind != JsonValueKind.String)
        {
            return WriteReplacement.Unchanged;
        }

        var uri = value.GetString();
        if (string.IsNullOrWhiteSpace(uri))
        {
            return WriteReplacement.Number(0);
        }

        return WriteReplacement.Number(resolver.ResolveAddress(packageRoot, uri));
    }

    private static string FormatPointerOffset(int offset) => $"0x{offset:X}";

    private static bool TryParsePointerOffset(string value, out int offset)
    {
        offset = 0;
        if (string.IsNullOrWhiteSpace(value))
        {
            return false;
        }

        if (value.StartsWith("0x", StringComparison.OrdinalIgnoreCase))
        {
            return int.TryParse(
                value.AsSpan(2),
                System.Globalization.NumberStyles.HexNumber,
                System.Globalization.CultureInfo.InvariantCulture,
                out offset);
        }

        return int.TryParse(value, out offset);
    }

    private static bool PointerMapsEqual(
        IReadOnlyDictionary<string, string?> left,
        IReadOnlyDictionary<string, string?> right)
    {
        if (left.Count != right.Count)
        {
            return false;
        }

        foreach (var pair in left)
        {
            if (!right.TryGetValue(pair.Key, out var rightValue))
            {
                return false;
            }

            if (!string.Equals(pair.Value, rightValue, StringComparison.Ordinal))
            {
                return false;
            }
        }

        return true;
    }

    private readonly record struct WriteReplacement(ReplacementKind Kind, int NumberValue, string? StringValue)
    {
        public static WriteReplacement Unchanged { get; } =
            new(ReplacementKind.Unchanged, 0, null);

        public static WriteReplacement Null { get; } =
            new(ReplacementKind.Null, 0, null);

        public static WriteReplacement Number(int value) =>
            new(ReplacementKind.Number, value, null);

        public static WriteReplacement String(string value) =>
            new(ReplacementKind.String, 0, value);

        public void WriteTo(Utf8JsonWriter writer)
        {
            switch (Kind)
            {
                case ReplacementKind.Null:
                    writer.WriteNullValue();
                    break;
                case ReplacementKind.Number:
                    writer.WriteNumberValue(NumberValue);
                    break;
                case ReplacementKind.String:
                    writer.WriteStringValue(StringValue);
                    break;
                case ReplacementKind.Unchanged:
                default:
                    throw new InvalidOperationException("Unchanged replacements must be handled by the caller.");
            }
        }
    }

    private enum ReplacementKind
    {
        Unchanged,
        Null,
        Number,
        String
    }
}
