using Astrolabe.Core.Hub;
using System.Text.Json;
using Astrolabe.Core.FileFormats.Perso;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class TransitionCodec : IStructCodec<TransitionRecord>
{
    public const int Size = 0x14;

    public static TransitionCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "next", PointerTarget.BlockRelative),
        new PointerField(0x04, "prev", PointerTarget.BlockRelative),
        new PointerField(0x08, "hdr", PointerTarget.BlockRelative),
        new PointerField(0x0C, "targetState", PointerTarget.BlockRelative),
        new PointerField(0x10, "stateToGo", PointerTarget.BlockRelative)
    ];

    public string Kind => "transition";
    public string Schema => "astrolabe.transition.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public TransitionRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(TransitionRecord));
        return new TransitionRecord
        {
            Next = HubReferenceIO.Read(slice, 0x00),
            Prev = HubReferenceIO.Read(slice, 0x04),
            Hdr = HubReferenceIO.Read(slice, 0x08),
            TargetState = HubReferenceIO.Read(slice, 0x0C),
            StateToGo = HubReferenceIO.Read(slice, 0x10)
        };
    }

    public byte[] Write(TransitionRecord value)
    {
        var bytes = new byte[Size];
        HubReferenceIO.Write(bytes, 0x00, value.Next);
        HubReferenceIO.Write(bytes, 0x04, value.Prev);
        HubReferenceIO.Write(bytes, 0x08, value.Hdr);
        HubReferenceIO.Write(bytes, 0x0C, value.TargetState);
        HubReferenceIO.Write(bytes, 0x10, value.StateToGo);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(TransitionRecord));
    }

    public TransitionRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<TransitionRecord>(json, Schema);

    public void ToJson(TransitionRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}