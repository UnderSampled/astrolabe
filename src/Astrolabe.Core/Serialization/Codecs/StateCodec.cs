using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class StateCodec : IStructCodec<StateRecord>
{
    public const int Size = 0x38;

    public static StateCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "next", PointerTarget.BlockRelative),
        new PointerField(0x04, "prev", PointerTarget.BlockRelative),
        new PointerField(0x08, "hdr", PointerTarget.BlockRelative),
        new PointerField(0x0C, "animRef", PointerTarget.BlockRelative),
        new PointerField(0x10, "transitionsHead", PointerTarget.BlockRelative),
        new PointerField(0x14, "transitionsTail", PointerTarget.BlockRelative),
        new PointerField(0x1C, "prohibitsHead", PointerTarget.BlockRelative),
        new PointerField(0x20, "prohibitsTail", PointerTarget.BlockRelative),
        new PointerField(0x28, "nextState", PointerTarget.BlockRelative),
        new PointerField(0x2C, "mechanicsIdCard", PointerTarget.BlockRelative)
    ];

    public string Kind => "state";
    public string Schema => "astrolabe.state.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public StateRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(StateRecord));
        return new StateRecord
        {
            Next = StructBinaryIO.ReadInt32(slice, 0x00),
            Prev = StructBinaryIO.ReadInt32(slice, 0x04),
            Hdr = StructBinaryIO.ReadInt32(slice, 0x08),
            AnimRef = StructBinaryIO.ReadInt32(slice, 0x0C),
            TransitionsHead = StructBinaryIO.ReadInt32(slice, 0x10),
            TransitionsTail = StructBinaryIO.ReadInt32(slice, 0x14),
            TransitionsCount = StructBinaryIO.ReadUInt32(slice, 0x18),
            ProhibitsHead = StructBinaryIO.ReadInt32(slice, 0x1C),
            ProhibitsTail = StructBinaryIO.ReadInt32(slice, 0x20),
            ProhibitsCount = StructBinaryIO.ReadUInt32(slice, 0x24),
            NextState = StructBinaryIO.ReadInt32(slice, 0x28),
            MechanicsIdCard = StructBinaryIO.ReadInt32(slice, 0x2C),
            Unknown30 = StructBinaryIO.ReadUInt32(slice, 0x30),
            Unknown34 = StructBinaryIO.ReadUInt32(slice, 0x34)
        };
    }

    public byte[] Write(StateRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteInt32(bytes, 0x00, value.Next);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.Prev);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.Hdr);
        StructBinaryIO.WriteInt32(bytes, 0x0C, value.AnimRef);
        StructBinaryIO.WriteInt32(bytes, 0x10, value.TransitionsHead);
        StructBinaryIO.WriteInt32(bytes, 0x14, value.TransitionsTail);
        StructBinaryIO.WriteUInt32(bytes, 0x18, value.TransitionsCount);
        StructBinaryIO.WriteInt32(bytes, 0x1C, value.ProhibitsHead);
        StructBinaryIO.WriteInt32(bytes, 0x20, value.ProhibitsTail);
        StructBinaryIO.WriteUInt32(bytes, 0x24, value.ProhibitsCount);
        StructBinaryIO.WriteInt32(bytes, 0x28, value.NextState);
        StructBinaryIO.WriteInt32(bytes, 0x2C, value.MechanicsIdCard);
        StructBinaryIO.WriteUInt32(bytes, 0x30, value.Unknown30);
        StructBinaryIO.WriteUInt32(bytes, 0x34, value.Unknown34);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(StateRecord));
    }

    public StateRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<StateRecord>(json, Schema);

    public void ToJson(StateRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}