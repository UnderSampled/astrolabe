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
        if (AnimationTreeExport.TryWriteElementBytes(packageRoot, dataPath, resolver, out var treeBytes))
        {
            return treeBytes;
        }

        if (!StructCodecRegistry.TryGet(kind, out var codec))
        {
            return File.ReadAllBytes(ReferenceUri.Resolve(packageRoot, dataPath).FilePath);
        }

        var elementPath = ReferenceUri.Resolve(packageRoot, dataPath).FilePath;
        if (!elementPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
        {
            return File.ReadAllBytes(elementPath);
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
        return codec.WriteFromJsonElement(resolvedDocument.RootElement);
    }

    private static byte[] ApplyInlinePointerOverlay(
        byte[] data,
        ReferenceAddressResolver resolver,
        string packageRoot,
        IReadOnlyDictionary<string, string?> inlineOverlay,
        bool explicitInlineOverlay)
    {
        if (inlineOverlay.Count == 0)
        {
            return data;
        }

        var bytes = data.ToArray();
        foreach (var (offsetKey, uri) in inlineOverlay)
        {
            if (!TryParsePointerOffset(offsetKey, out var offset))
            {
                throw new InvalidDataException($"Invalid opaque pointer offset '{offsetKey}'.");
            }

            if (!TryValidateRelocationPointerOffset(offset, bytes.Length, out _))
            {
                // Fringe RT* sites can span element tail bytes that are not representable after
                // JSON promotion without reshaping the block segment. Leave bytes unchanged.
                continue;
            }

            if (string.IsNullOrWhiteSpace(uri))
            {
                // Null LUT entries track relocation sites discovered from RT* tables. Preserve
                // the imported .bin value so export stays byte-identical to the source disc.
                continue;
            }

            int address;
            try
            {
                address = resolver.ResolveAddress(packageRoot, uri);
            }
            catch (InvalidDataException)
            {
                // Unresolved LUT URIs preserve imported bytes (consistent with null/out-of-range entries).
                continue;
            }

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

    internal static bool MergePointerLut(
        IDictionary<string, string?> lut,
        string offsetKey,
        string? uri)
    {
        if (lut.TryGetValue(offsetKey, out var existing) &&
            string.Equals(existing, uri, StringComparison.Ordinal))
        {
            return false;
        }

        lut[offsetKey] = uri;
        return true;
    }

    internal static bool MergePointerLut(
        IDictionary<string, string?> lut,
        IReadOnlyDictionary<string, string?> discovered)
    {
        var changed = false;
        foreach (var (offsetKey, uri) in discovered)
        {
            if (MergePointerLut(lut, offsetKey, uri))
            {
                changed = true;
            }
        }

        return changed;
    }

    internal static string FormatPointerOffset(int offset) => $"0x{offset:X}";

    internal static void ValidateRelocationPointerOffset(int offset, int spanLength, string context)
    {
        if (!TryValidateRelocationPointerOffset(offset, spanLength, out var error))
        {
            throw new InvalidDataException($"{error} in {context}.");
        }
    }

    internal static bool TryValidateRelocationPointerOffset(int offset, int spanLength, out string error)
    {
        if (offset < 0)
        {
            error = $"Negative pointer offset 0x{offset:X}";
            return false;
        }

        if (offset + sizeof(int) > spanLength)
        {
            error = $"Pointer offset 0x{offset:X} is out of range (span length 0x{spanLength:X})";
            return false;
        }

        error = "";
        return true;
    }

    internal static bool MergeStructPointerLut(string jsonPath, string offsetKey, string? uri)
    {
        var discovered = new Dictionary<string, string?>(StringComparer.OrdinalIgnoreCase)
        {
            [offsetKey] = uri
        };
        return ApplyStructPointerLut(jsonPath, discovered);
    }

    internal static bool ApplyStructPointerLut(
        string jsonPath,
        IReadOnlyDictionary<string, string?> discovered)
    {
        if (discovered.Count == 0)
        {
            return false;
        }

        using var document = JsonDocument.Parse(File.ReadAllText(jsonPath));
        var pointers = new Dictionary<string, string?>(StringComparer.OrdinalIgnoreCase);
        if (document.RootElement.TryGetProperty("pointers", out var existing) &&
            existing.ValueKind == JsonValueKind.Object)
        {
            foreach (var property in existing.EnumerateObject())
            {
                pointers[property.Name] = property.Value.ValueKind == JsonValueKind.Null
                    ? null
                    : property.Value.GetString();
            }
        }

        if (!MergePointerLut(pointers, discovered))
        {
            return false;
        }

        WriteStructPointerLut(jsonPath, pointers);
        return true;
    }

    internal static bool TryReadStructPointerLut(
        string jsonPath,
        out Dictionary<string, string?> pointers)
    {
        pointers = new Dictionary<string, string?>(StringComparer.OrdinalIgnoreCase);
        using var document = JsonDocument.Parse(File.ReadAllText(jsonPath));
        if (!document.RootElement.TryGetProperty("pointers", out var pointersElement) ||
            pointersElement.ValueKind != JsonValueKind.Object)
        {
            return false;
        }

        foreach (var property in pointersElement.EnumerateObject())
        {
            pointers[property.Name] = property.Value.ValueKind == JsonValueKind.Null
                ? null
                : property.Value.GetString();
        }

        return pointers.Count > 0;
    }

    private static void WriteStructPointerLut(
        string jsonPath,
        IReadOnlyDictionary<string, string?> pointers)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(jsonPath));
        using var stream = new MemoryStream();
        using (var writer = CreateWriter(stream))
        {
            writer.WriteStartObject();
            foreach (var property in document.RootElement.EnumerateObject())
            {
                if (property.Name.Equals("pointers", StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                writer.WritePropertyName(property.Name);
                property.Value.WriteTo(writer);
            }

            writer.WritePropertyName("pointers");
            writer.WriteStartObject();
            foreach (var (offsetKey, uri) in pointers.OrderBy(pair => OrderPointerOffset(pair.Key)))
            {
                writer.WritePropertyName(offsetKey);
                if (uri == null)
                {
                    writer.WriteNullValue();
                }
                else
                {
                    writer.WriteStringValue(uri);
                }
            }

            writer.WriteEndObject();
            writer.WriteEndObject();
            writer.Flush();
        }

        File.WriteAllBytes(jsonPath, stream.ToArray());
    }

    private static int OrderPointerOffset(string value) =>
        TryParsePointerOffset(value, out var offset) ? offset : int.MaxValue;

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
        return ApplyInlinePointerOverlay(
            data,
            resolver,
            packageRoot,
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