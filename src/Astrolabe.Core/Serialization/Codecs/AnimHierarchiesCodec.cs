using System.Buffers.Binary;
using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;
using Astrolabe.Core.Hub;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class AnimHierarchiesCodec : IStructCodec<AnimHierarchiesRecord>, IPointerArrayCodec
{
    /// <summary>Documented hierarchy pair: childChannelId + parentChannelId.</summary>
    public const int PairSize = 4;

    public static AnimHierarchiesCodec Instance { get; } = new();

    public string Kind => "animhierarchies";
    public string Schema => "astrolabe.anim-hierarchies.v1";
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];
    public string PointerArrayPropertyName => "entries";

    public IReadOnlyList<PointerField> GetPointerFieldsForLength(int byteLength) =>
        [];

    public IReadOnlyList<PointerField> EnumeratePointerFields(ReadOnlySpan<byte> data)
    {
        // Historical scan starts after 0x20 (matches prior opaque codec) to avoid false hits on channel ids.
        const int scanStart = 0x20;
        var fields = new List<PointerField>();
        for (var offset = scanStart; offset <= data.Length - sizeof(int); offset += sizeof(int))
        {
            var value = BinaryPrimitives.ReadInt32LittleEndian(data.Slice(offset, sizeof(int)));
            if (!VmPointerScanning.IsLikelyVirtualAddress(value))
            {
                continue;
            }

            fields.Add(new PointerField(
                offset,
                $"ptr_{offset:X}",
                PointerTarget.BlockRelative,
                RequiresVmRange: true,
                RequiresDecompressedTarget: true));
        }

        return fields;
    }

    public AnimHierarchiesRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        if (slice.Length == 0)
        {
            return new AnimHierarchiesRecord { Entries = [], TrailingBytes = [] };
        }

        var pairCount = slice.Length / PairSize;
        var pairs = new AnimHierarchyEntryRecord[pairCount];
        for (var i = 0; i < pairCount; i++)
        {
            var eoff = i * PairSize;
            pairs[i] = new AnimHierarchyEntryRecord
            {
                ChildChannelId = unchecked((short)StructBinaryIO.ReadUInt16(slice, eoff)),
                ParentChannelId = unchecked((short)StructBinaryIO.ReadUInt16(slice, eoff + 2))
            };
        }

        var trailing = slice.Length % PairSize == 0
            ? []
            : slice[(pairCount * PairSize)..].ToArray();

        return new AnimHierarchiesRecord { Entries = pairs, TrailingBytes = trailing };
    }

    public byte[] Write(AnimHierarchiesRecord value)
    {
        if (value.Entries == null)
        {
            throw new InvalidDataException($"{Schema} requires a non-null entries array.");
        }

        var total = value.Entries.Length * PairSize + (value.TrailingBytes?.Length ?? 0);
        var bytes = new byte[total];
        for (var i = 0; i < value.Entries.Length; i++)
        {
            var e = value.Entries[i];
            var eoff = i * PairSize;
            StructBinaryIO.WriteUInt16(bytes, eoff, unchecked((ushort)e.ChildChannelId));
            StructBinaryIO.WriteUInt16(bytes, eoff + 2, unchecked((ushort)e.ParentChannelId));
            // Unknown04 is not part of the 4-byte wire pair; ignore if present.
        }

        if (value.TrailingBytes is { Length: > 0 })
        {
            value.TrailingBytes.CopyTo(bytes, value.Entries.Length * PairSize);
        }

        return bytes;
    }

    public AnimHierarchiesRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<AnimHierarchiesRecord>(json, Schema);

    public void ToJson(AnimHierarchiesRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}
