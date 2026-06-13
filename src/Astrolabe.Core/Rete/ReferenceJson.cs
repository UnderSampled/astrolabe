using System.Text.Json;
using Astrolabe.Core.Serialization;

namespace Astrolabe.Core.Rete;

internal static class ReferenceJson
{
    public static bool RewritePointersToUris(
        string jsonPath,
        string packageRoot,
        IReadOnlyList<PointerField> pointerFields,
        ReferenceAddressResolver resolver)
    {
        if (pointerFields.Count == 0)
        {
            return false;
        }

        using var document = JsonDocument.Parse(File.ReadAllText(jsonPath));
        using var stream = new MemoryStream();
        using (var writer = CreateWriter(stream))
        {
            var changed = WriteObject(document.RootElement, writer, pointerFields, property =>
            {
                if (property.Value.ValueKind != JsonValueKind.Number ||
                    !property.Value.TryGetInt32(out var value))
                {
                    return WriteReplacement.Unchanged;
                }

                if (value == 0)
                {
                    return WriteReplacement.Null;
                }

                return resolver.TryGetReferenceUri(value, packageRoot, out var uri)
                    ? WriteReplacement.String(uri)
                    : WriteReplacement.Unchanged;
            });

            writer.Flush();
            if (!changed)
            {
                return false;
            }
        }

        File.WriteAllBytes(jsonPath, stream.ToArray());
        return true;
    }

    public static JsonDocument ResolvePointersForExport(
        JsonElement root,
        string packageRoot,
        IReadOnlyList<PointerField> pointerFields,
        ReferenceAddressResolver resolver)
    {
        if (pointerFields.Count == 0)
        {
            return JsonDocument.Parse(root.GetRawText());
        }

        using var stream = new MemoryStream();
        using (var writer = CreateWriter(stream))
        {
            WriteObject(root, writer, pointerFields, property =>
            {
                if (property.Value.ValueKind == JsonValueKind.Null)
                {
                    return WriteReplacement.Number(0);
                }

                if (property.Value.ValueKind != JsonValueKind.String)
                {
                    return WriteReplacement.Unchanged;
                }

                var uri = property.Value.GetString();
                if (string.IsNullOrWhiteSpace(uri))
                {
                    return WriteReplacement.Number(0);
                }

                return WriteReplacement.Number(resolver.ResolveAddress(packageRoot, uri));
            });

            writer.Flush();
        }

        stream.Position = 0;
        return JsonDocument.Parse(stream);
    }

    private static Utf8JsonWriter CreateWriter(Stream stream) =>
        new(stream, new JsonWriterOptions { Indented = true });

    private static bool WriteObject(
        JsonElement root,
        Utf8JsonWriter writer,
        IReadOnlyList<PointerField> pointerFields,
        Func<JsonProperty, WriteReplacement> replacementFactory)
    {
        if (root.ValueKind != JsonValueKind.Object)
        {
            root.WriteTo(writer);
            return false;
        }

        var changed = false;
        var pointerNames = pointerFields
            .Select(f => f.Name)
            .ToHashSet(StringComparer.OrdinalIgnoreCase);

        writer.WriteStartObject();
        foreach (var property in root.EnumerateObject())
        {
            writer.WritePropertyName(property.Name);
            if (pointerNames.Contains(property.Name))
            {
                var replacement = replacementFactory(property);
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
