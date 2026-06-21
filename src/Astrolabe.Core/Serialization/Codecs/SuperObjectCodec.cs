using Astrolabe.Core.Hub;
using System.Text.Json;
using Astrolabe.Core.FileFormats;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class SuperObjectCodec : IStructCodec<SuperObjectRecord>
{
    public const int Size = 0x38;

    public static SuperObjectCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "typeCode", PointerTarget.BlockRelative, RequiresVmRange: true),
        new PointerField(0x04, "offData", PointerTarget.BlockRelative),
        new PointerField(0x08, "childrenHead", PointerTarget.BlockRelative),
        new PointerField(0x0C, "childrenTail", PointerTarget.BlockRelative),
        new PointerField(0x10, "childrenCount", PointerTarget.BlockRelative, RequiresVmRange: true),
        new PointerField(0x14, "brotherNext", PointerTarget.BlockRelative),
        new PointerField(0x18, "brotherPrev", PointerTarget.BlockRelative),
        new PointerField(0x1C, "parent", PointerTarget.BlockRelative),
        new PointerField(0x20, "matrix", PointerTarget.BlockRelative),
        new PointerField(0x24, "staticMatrix", PointerTarget.BlockRelative),
        new PointerField(0x28, "globalMatrix", PointerTarget.BlockRelative),
        new PointerField(0x34, "boundingVolume", PointerTarget.BlockRelative)
    ];

    public string Kind => "superObject";
    public string Schema => "astrolabe.super-object.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public SuperObjectRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        var typeCode = StructBinaryIO.ReadUInt32(slice, 0x00);

        return new SuperObjectRecord
        {
            TypeCode = typeCode,
            Type = TrackingSuperObjectReader.GetSuperObjectType(typeCode).ToString(),
            OffData = HubReferenceIO.Read(slice, 0x04),
            ChildrenHead = HubReferenceIO.Read(slice, 0x08),
            ChildrenTail = HubReferenceIO.Read(slice, 0x0C),
            ChildrenCount = StructBinaryIO.ReadUInt32(slice, 0x10),
            BrotherNext = HubReferenceIO.Read(slice, 0x14),
            BrotherPrev = HubReferenceIO.Read(slice, 0x18),
            Parent = HubReferenceIO.Read(slice, 0x1C),
            Matrix = HubReferenceIO.Read(slice, 0x20),
            StaticMatrix = HubReferenceIO.Read(slice, 0x24),
            GlobalMatrix = HubReferenceIO.Read(slice, 0x28),
            DrawFlags = StructBinaryIO.ReadUInt32(slice, 0x2C),
            Flags = StructBinaryIO.ReadUInt32(slice, 0x30),
            BoundingVolume = HubReferenceIO.Read(slice, 0x34)
        };
    }

    public byte[] Write(SuperObjectRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteUInt32(bytes, 0x00, value.TypeCode);
        HubReferenceIO.Write(bytes, 0x04, value.OffData);
        HubReferenceIO.Write(bytes, 0x08, value.ChildrenHead);
        HubReferenceIO.Write(bytes, 0x0C, value.ChildrenTail);
        StructBinaryIO.WriteUInt32(bytes, 0x10, value.ChildrenCount);
        HubReferenceIO.Write(bytes, 0x14, value.BrotherNext);
        HubReferenceIO.Write(bytes, 0x18, value.BrotherPrev);
        HubReferenceIO.Write(bytes, 0x1C, value.Parent);
        HubReferenceIO.Write(bytes, 0x20, value.Matrix);
        HubReferenceIO.Write(bytes, 0x24, value.StaticMatrix);
        HubReferenceIO.Write(bytes, 0x28, value.GlobalMatrix);
        StructBinaryIO.WriteUInt32(bytes, 0x2C, value.DrawFlags);
        StructBinaryIO.WriteUInt32(bytes, 0x30, value.Flags);
        HubReferenceIO.Write(bytes, 0x34, value.BoundingVolume);
        return bytes;
    }

    public SuperObjectRecord FromJson(JsonElement json)
    {
        var schema = json.TryGetProperty("schema", out var schemaElement)
            ? schemaElement.GetString() ?? Schema
            : Schema;
        if (schema != Schema && schema != "astrolabe.scene-node.v1")
        {
            throw new InvalidDataException($"Unsupported super object schema: {schema}");
        }

        return new SuperObjectRecord
        {
            Schema = schema,
            TypeCode = ReadUInt32Field(json, "typeCode"),
            Type = json.TryGetProperty("type", out var typeElement) ? typeElement.GetString() ?? "" : "",
            OffData = ReadHubReference(json, "offData"),
            ChildrenHead = ReadHubReference(json, "childrenHead"),
            ChildrenTail = ReadHubReference(json, "childrenTail"),
            ChildrenCount = ReadUInt32Field(json, "childrenCount"),
            BrotherNext = ReadHubReference(json, "brotherNext"),
            BrotherPrev = ReadHubReference(json, "brotherPrev"),
            Parent = ReadHubReference(json, "parent"),
            Matrix = ReadHubReference(json, "matrix"),
            StaticMatrix = ReadHubReference(json, "staticMatrix"),
            GlobalMatrix = ReadHubReference(json, "globalMatrix"),
            DrawFlags = ReadUInt32Field(json, "drawFlags"),
            Flags = ReadUInt32Field(json, "flags"),
            BoundingVolume = ReadHubReference(json, "boundingVolume")
        };
    }

    private static HubReference ReadHubReference(JsonElement json, string name)
    {
        if (!json.TryGetProperty(name, out var element))
        {
            return HubReference.Null;
        }

        return element.ValueKind switch
        {
            JsonValueKind.Null => HubReference.Null,
            JsonValueKind.String => HubReference.FromUri(element.GetString()),
            JsonValueKind.Number when element.TryGetInt32(out var value) => HubReference.FromWire(value),
            JsonValueKind.Number when element.TryGetUInt32(out var unsigned) && unsigned <= int.MaxValue =>
                HubReference.FromWire((int)unsigned),
            _ => HubReference.Null
        };
    }

    private static uint ReadUInt32Field(JsonElement json, string name)
    {
        if (!json.TryGetProperty(name, out var element) || element.ValueKind == JsonValueKind.Null)
        {
            return 0;
        }

        if (element.ValueKind == JsonValueKind.Number)
        {
            if (element.TryGetUInt32(out var unsigned))
            {
                return unsigned;
            }

            if (element.TryGetInt32(out var signed) && signed >= 0)
            {
                return (uint)signed;
            }
        }

        return 0;
    }

    public void ToJson(SuperObjectRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}
