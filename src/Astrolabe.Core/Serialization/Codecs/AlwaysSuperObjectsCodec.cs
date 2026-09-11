using System.Text.Json;
using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Perso;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class AlwaysSuperObjectsCodec :
    IStructCodec<AlwaysSuperObjectsRecord>,
    IPointerArrayCodec,
    IStructuredArrayCodec
{
    public const int EntrySize = SuperObjectCodec.Size;

    public static AlwaysSuperObjectsCodec Instance { get; } = new();

    /// <summary>
    /// SuperObject codec treats typeCode/childrenCount as relocation-scanned ints; they are not
    /// reference slots. Nested JSON URI rewrite must not convert them to hub URIs.
    /// </summary>
    private static readonly PointerField[] TrueEntryPointerFields =
        SuperObjectCodec.Instance.PointerFields
            .Where(f => f.Name is not ("typeCode" or "childrenCount"))
            .ToArray();

    public string Kind => "alwayssuperobjects";
    public string Schema => "astrolabe.always-super-objects.v1";
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];
    public string PointerArrayPropertyName => "entries";
    public int PointerEntryStride => EntrySize;
    public string EntriesPropertyName => "entries";
    public IReadOnlyList<PointerField> EntryPointerFields => TrueEntryPointerFields;
    public int StructuredEntrySize => EntrySize;
    int IStructuredArrayCodec.EntrySize => EntrySize;

    public IReadOnlyList<PointerField> GetPointerFieldsForLength(int byteLength) =>
        BuildPointerFields(byteLength);

    private static IReadOnlyList<PointerField> BuildPointerFields(int byteLength)
    {
        if (byteLength == 0 || byteLength % EntrySize != 0)
        {
            return [];
        }

        var entryCount = byteLength / EntrySize;
        // Keep full SuperObject pointer metadata (incl. typeCode/childrenCount) for RTB generation.
        var template = SuperObjectCodec.Instance.PointerFields;
        var fields = new PointerField[entryCount * template.Count];
        for (var entryIndex = 0; entryIndex < entryCount; entryIndex++)
        {
            var entryOffset = entryIndex * EntrySize;
            for (var fieldIndex = 0; fieldIndex < template.Count; fieldIndex++)
            {
                var field = template[fieldIndex];
                fields[entryIndex * template.Count + fieldIndex] = field with
                {
                    Offset = entryOffset + field.Offset,
                    Name = $"{field.Name}_{entryIndex}"
                };
            }
        }

        return fields;
    }

    public AlwaysSuperObjectsRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        if (length == 0)
        {
            return new AlwaysSuperObjectsRecord { Entries = [] };
        }

        if (length % EntrySize != 0)
        {
            throw new InvalidDataException(
                $"{Kind} length {length} is not a multiple of SuperObject size {EntrySize}.");
        }

        var slice = data.Slice(offset, length);
        var count = length / EntrySize;
        var entries = new SuperObjectRecord[count];
        for (var i = 0; i < count; i++)
        {
            entries[i] = SuperObjectCodec.Instance.Read(slice, i * EntrySize, EntrySize);
        }

        return new AlwaysSuperObjectsRecord { Entries = entries };
    }

    public byte[] Write(AlwaysSuperObjectsRecord value)
    {
        if (value.Entries == null)
        {
            throw new InvalidDataException($"{Schema} requires a non-null entries array.");
        }

        var bytes = new byte[value.Entries.Length * EntrySize];
        for (var i = 0; i < value.Entries.Length; i++)
        {
            var entryBytes = SuperObjectCodec.Instance.Write(value.Entries[i]);
            entryBytes.CopyTo(bytes, i * EntrySize);
        }

        return bytes;
    }

    public AlwaysSuperObjectsRecord FromJson(JsonElement json)
    {
        var schema = json.TryGetProperty("schema", out var schemaElement)
            ? schemaElement.GetString() ?? Schema
            : Schema;
        if (schema != Schema)
        {
            throw new InvalidDataException($"Unsupported always-super-objects schema: {schema}");
        }

        if (!json.TryGetProperty("entries", out var entriesElement) ||
            entriesElement.ValueKind != JsonValueKind.Array)
        {
            throw new InvalidDataException($"{Schema} requires an entries array.");
        }

        var entries = new SuperObjectRecord[entriesElement.GetArrayLength()];
        var index = 0;
        foreach (var entry in entriesElement.EnumerateArray())
        {
            // SuperObjectCodec.FromJson accepts HubReference URI/number forms used after rewrite.
            entries[index++] = SuperObjectCodec.Instance.FromJson(entry);
        }

        return new AlwaysSuperObjectsRecord
        {
            Schema = schema,
            Entries = entries
        };
    }

    public void ToJson(AlwaysSuperObjectsRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}
