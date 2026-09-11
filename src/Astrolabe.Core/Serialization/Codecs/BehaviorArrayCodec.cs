using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;
using Astrolabe.Core.Hub;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class BehaviorArrayCodec :
    IStructCodec<BehaviorArrayRecord>,
    IPointerArrayCodec,
    IStructuredArrayCodec
{
    public const int EntrySize = 0x10;

    private static readonly PointerField[] EntryPointerFieldsList =
    [
        new PointerField(0x00, "scripts", PointerTarget.BlockRelative),
        new PointerField(0x04, "scheduleScript", PointerTarget.BlockRelative)
    ];

    private BehaviorArrayCodec(string kind, string schema)
    {
        Kind = kind;
        Schema = schema;
    }

    public string Kind { get; }
    public string Schema { get; }
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];
    public string PointerArrayPropertyName => "entries";
    public int PointerEntryStride => EntrySize;
    public string EntriesPropertyName => "entries";
    public IReadOnlyList<PointerField> EntryPointerFields => EntryPointerFieldsList;
    public int StructuredEntrySize => EntrySize;
    int IStructuredArrayCodec.EntrySize => EntrySize;

    public static BehaviorArrayCodec BehaviorsNormal { get; } = new(
        "behaviors_normal",
        "astrolabe.behaviors-normal.v1");

    public static BehaviorArrayCodec BehaviorsReflex { get; } = new(
        "behaviors_reflex",
        "astrolabe.behaviors-reflex.v1");

    public IReadOnlyList<PointerField> GetPointerFieldsForLength(int byteLength) =>
        BuildPointerFields(byteLength);

    private static IReadOnlyList<PointerField> BuildPointerFields(int byteLength)
    {
        if (byteLength == 0 || byteLength % EntrySize != 0)
        {
            return [];
        }

        var entryCount = byteLength / EntrySize;
        var fields = new PointerField[entryCount * 2];
        for (var entryIndex = 0; entryIndex < entryCount; entryIndex++)
        {
            var entryOffset = entryIndex * EntrySize;
            fields[entryIndex * 2 + 0] = new PointerField(entryOffset + 0x00, "scripts", PointerTarget.BlockRelative);
            fields[entryIndex * 2 + 1] = new PointerField(entryOffset + 0x04, "scheduleScript", PointerTarget.BlockRelative);
        }

        return fields;
    }

    public BehaviorArrayRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        if (length == 0)
        {
            return new BehaviorArrayRecord { Schema = Schema, Type = Kind, Entries = [] };
        }

        if (length % EntrySize != 0)
        {
            throw new InvalidDataException($"{Kind} length {length} is not a multiple of {EntrySize}.");
        }

        var slice = data.Slice(offset, length);
        var count = length / EntrySize;
        var entries = new BehaviorEntryRecord[count];
        for (var i = 0; i < count; i++)
        {
            var entryOffset = i * EntrySize;
            entries[i] = new BehaviorEntryRecord
            {
                Scripts = HubReferenceIO.Read(slice, entryOffset + 0x00),
                ScheduleScript = HubReferenceIO.Read(slice, entryOffset + 0x04),
                Unknown08 = StructBinaryIO.ReadUInt32(slice, entryOffset + 0x08),
                Unknown0C = StructBinaryIO.ReadUInt32(slice, entryOffset + 0x0C)
            };
        }

        return new BehaviorArrayRecord { Schema = Schema, Type = Kind, Entries = entries };
    }

    public byte[] Write(BehaviorArrayRecord value)
    {
        if (value.Entries == null)
        {
            throw new InvalidDataException($"{Schema} requires a non-null entries array.");
        }

        var bytes = new byte[value.Entries.Length * EntrySize];
        for (var i = 0; i < value.Entries.Length; i++)
        {
            var entry = value.Entries[i];
            var entryOffset = i * EntrySize;
            HubReferenceIO.Write(bytes, entryOffset + 0x00, entry.Scripts);
            HubReferenceIO.Write(bytes, entryOffset + 0x04, entry.ScheduleScript);
            StructBinaryIO.WriteUInt32(bytes, entryOffset + 0x08, entry.Unknown08);
            StructBinaryIO.WriteUInt32(bytes, entryOffset + 0x0C, entry.Unknown0C);
        }

        return bytes;
    }

    public BehaviorArrayRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<BehaviorArrayRecord>(json, Schema);

    public void ToJson(BehaviorArrayRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}
