using System.Text.Json;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class FixedBinaryStructCodec : IStructCodec<OpaqueBinaryRecord>
{
    private FixedBinaryStructCodec(
        string kind,
        string schema,
        int size,
        IReadOnlyList<PointerField> pointerFields)
    {
        Kind = kind;
        Schema = schema;
        FixedSizeValue = size;
        PointerFieldsList = pointerFields;
    }

    public string Kind { get; }
    public string Schema { get; }
    public int? FixedSize => FixedSizeValue;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    private int FixedSizeValue { get; }
    private IReadOnlyList<PointerField> PointerFieldsList { get; }

    public static FixedBinaryStructCodec ActionTable { get; } = new(
        "actiontable",
        "astrolabe.action-table.v1",
        0x40,
        [new PointerField(0x00, "header", PointerTarget.BlockRelative)]);

    public static FixedBinaryStructCodec ActionTree { get; } = new(
        "actiontree",
        "astrolabe.action-tree.v1",
        0x20,
        [
            new PointerField(0x08, "child", PointerTarget.BlockRelative),
            new PointerField(0x10, "sibling", PointerTarget.BlockRelative),
            new PointerField(0x18, "parent", PointerTarget.BlockRelative)
        ]);

    public static FixedBinaryStructCodec DsgVar { get; } = new(
        "dsgvar",
        "astrolabe.dsg-var.v1",
        0x10,
        [
            new PointerField(0x00, "memBuffer", PointerTarget.BlockRelative),
            new PointerField(0x04, "infos", PointerTarget.BlockRelative)
        ]);

    public static FixedBinaryStructCodec DsgMem { get; } = new(
        "dsgmem",
        "astrolabe.dsg-mem.v1",
        0x0C,
        [
            new PointerField(0x00, "dsgVarPtr", PointerTarget.BlockRelative),
            new PointerField(0x04, "memBufferInitial", PointerTarget.BlockRelative),
            new PointerField(0x08, "memBuffer", PointerTarget.BlockRelative)
        ]);

    public static FixedBinaryStructCodec BehaviorListNormal { get; } = new(
        "behaviorlist_normal",
        "astrolabe.behavior-list-normal.v1",
        0x08,
        [new PointerField(0x00, "entries", PointerTarget.BlockRelative)]);

    public static FixedBinaryStructCodec BehaviorListReflex { get; } = new(
        "behaviorlist_reflex",
        "astrolabe.behavior-list-reflex.v1",
        0x08,
        [new PointerField(0x00, "entries", PointerTarget.BlockRelative)]);

    public static FixedBinaryStructCodec ObjectTypeEntry { get; } = new(
        "objecttypeentry",
        "astrolabe.object-type-entry.v1",
        0x14,
        [
            new PointerField(0x00, "next", PointerTarget.BlockRelative),
            new PointerField(0x04, "prev", PointerTarget.BlockRelative),
            new PointerField(0x08, "hdr", PointerTarget.BlockRelative),
            new PointerField(0x0C, "name", PointerTarget.BlockRelative)
        ]);

    public static FixedBinaryStructCodec Dynam { get; } = new(
        "dynam",
        "astrolabe.dynam.v1",
        0x80,
        [
            new PointerField(0x10, "field_10", PointerTarget.BlockRelative),
            new PointerField(0x14, "field_14", PointerTarget.BlockRelative)
        ]);

    public static FixedBinaryStructCodec SectorCollideGeo { get; } = new(
        "sectorcollidegeo",
        "astrolabe.sector-collide-geo.v1",
        0x30,
        [
            new PointerField(0x04, "vertices", PointerTarget.BlockRelative),
            new PointerField(0x08, "normals", PointerTarget.BlockRelative),
            new PointerField(0x18, "elementTypes", PointerTarget.BlockRelative),
            new PointerField(0x1C, "elements", PointerTarget.BlockRelative)
        ]);

    public static FixedBinaryStructCodec CollideZoneList(string kind) => new(
        kind,
        $"astrolabe.{kind.Replace('_', '-')}.v1",
        0x0C,
        [
            new PointerField(0x00, "head", PointerTarget.BlockRelative),
            new PointerField(0x04, "tail", PointerTarget.BlockRelative)
        ]);

    public static FixedBinaryStructCodec CollideZone(string kind) => new(
        kind,
        $"astrolabe.{kind.Replace('_', '-')}.v1",
        0x20,
        [
            new PointerField(0x00, "next", PointerTarget.BlockRelative),
            new PointerField(0x04, "prev", PointerTarget.BlockRelative),
            new PointerField(0x08, "collideObj", PointerTarget.BlockRelative)
        ]);

    public OpaqueBinaryRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), FixedSizeValue, Kind);
        return OpaqueBinaryRecord.FromSlice(Schema, slice, 0, slice.Length);
    }

    public byte[] Write(OpaqueBinaryRecord value) =>
        JsonStructCodec.RequireExactSize(value.Data, FixedSizeValue, Kind);

    public OpaqueBinaryRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<OpaqueBinaryRecord>(json, Schema);

    public void ToJson(OpaqueBinaryRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}