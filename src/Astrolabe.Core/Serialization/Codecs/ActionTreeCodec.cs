using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;
using Astrolabe.Core.Hub;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class ActionTreeCodec : IStructCodec<ActionTreeRecord>
{
    public const int Size = 0x20;

    public static ActionTreeCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x08, "child", PointerTarget.BlockRelative),
        new PointerField(0x10, "sibling", PointerTarget.BlockRelative),
        new PointerField(0x18, "parent", PointerTarget.BlockRelative)
    ];

    public string Kind => "actiontree";
    public string Schema => "astrolabe.action-tree.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public ActionTreeRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(ActionTreeRecord));
        return new ActionTreeRecord
        {
            Unknown00 = StructBinaryIO.ReadUInt32(slice, 0x00),
            Unknown04 = StructBinaryIO.ReadUInt32(slice, 0x04),
            Child = HubReferenceIO.Read(slice, 0x08),
            Sibling = HubReferenceIO.Read(slice, 0x10),
            Parent = HubReferenceIO.Read(slice, 0x18)
        };
    }

    public byte[] Write(ActionTreeRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteUInt32(bytes, 0x00, value.Unknown00);
        StructBinaryIO.WriteUInt32(bytes, 0x04, value.Unknown04);
        HubReferenceIO.Write(bytes, 0x08, value.Child);
        HubReferenceIO.Write(bytes, 0x10, value.Sibling);
        HubReferenceIO.Write(bytes, 0x18, value.Parent);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(ActionTreeRecord));
    }

    public ActionTreeRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<ActionTreeRecord>(json, Schema);

    public void ToJson(ActionTreeRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}
