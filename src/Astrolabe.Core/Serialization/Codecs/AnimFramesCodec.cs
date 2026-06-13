using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class AnimFramesCodec : IStructCodec<AnimFramesRecord>, IPointerArrayCodec
{
    public const int FrameSize = 0x10;

    public static AnimFramesCodec Instance { get; } = new();

    public string Kind => "animframes";
    public string Schema => "astrolabe.anim-frames.v1";
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];
    public string PointerArrayPropertyName => "frames";

    public IReadOnlyList<PointerField> GetPointerFieldsForLength(int byteLength)
    {
        if (byteLength == 0)
        {
            return [];
        }

        if (byteLength % FrameSize != 0)
        {
            throw new InvalidDataException(
                $"{Kind} serialized length {byteLength} is not a multiple of {FrameSize}.");
        }

        var frameCount = byteLength / FrameSize;
        var fields = new PointerField[frameCount * 4];
        for (var frameIndex = 0; frameIndex < frameCount; frameIndex++)
        {
            var frameOffset = frameIndex * FrameSize;
            fields[frameIndex * 4 + 0] = new PointerField(frameOffset + 0x00, "channels", PointerTarget.BlockRelative);
            fields[frameIndex * 4 + 1] = new PointerField(frameOffset + 0x04, "mat", PointerTarget.BlockRelative);
            fields[frameIndex * 4 + 2] = new PointerField(frameOffset + 0x08, "vec", PointerTarget.BlockRelative);
            fields[frameIndex * 4 + 3] = new PointerField(frameOffset + 0x0C, "hierarchies", PointerTarget.BlockRelative);
        }

        return fields;
    }

    public AnimFramesRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        if (length % FrameSize != 0)
        {
            throw new InvalidDataException($"{Kind} length {length} is not a multiple of {FrameSize}.");
        }

        var slice = data.Slice(offset, length);
        var frameCount = length / FrameSize;
        var frames = new AnimFrameRecord[frameCount];
        for (var i = 0; i < frameCount; i++)
        {
            var frameOffset = i * FrameSize;
            frames[i] = new AnimFrameRecord
            {
                Channels = StructBinaryIO.ReadInt32(slice, frameOffset + 0x00),
                Mat = StructBinaryIO.ReadInt32(slice, frameOffset + 0x04),
                Vec = StructBinaryIO.ReadInt32(slice, frameOffset + 0x08),
                Hierarchies = StructBinaryIO.ReadInt32(slice, frameOffset + 0x0C)
            };
        }

        return new AnimFramesRecord { Frames = frames };
    }

    public byte[] Write(AnimFramesRecord value)
    {
        if (value.Frames == null)
        {
            throw new InvalidDataException($"{Schema} ({Kind}) requires a non-null frames array.");
        }

        var bytes = new byte[value.Frames.Length * FrameSize];
        for (var i = 0; i < value.Frames.Length; i++)
        {
            var frame = value.Frames[i];
            var frameOffset = i * FrameSize;
            StructBinaryIO.WriteInt32(bytes, frameOffset + 0x00, frame.Channels);
            StructBinaryIO.WriteInt32(bytes, frameOffset + 0x04, frame.Mat);
            StructBinaryIO.WriteInt32(bytes, frameOffset + 0x08, frame.Vec);
            StructBinaryIO.WriteInt32(bytes, frameOffset + 0x0C, frame.Hierarchies);
        }

        return bytes;
    }

    public AnimFramesRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<AnimFramesRecord>(json, Schema);

    public void ToJson(AnimFramesRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}