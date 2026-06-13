using System.Text.Json;
using Astrolabe.Core.FileFormats.Geometry;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class SectorCodec : IStructCodec<SectorRecord>
{
    public const int Size = 0xD0;

    public static SectorCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "collideObj", PointerTarget.BlockRelative),
        new PointerField(0x04, "envHead", PointerTarget.BlockRelative),
        new PointerField(0x08, "envTail", PointerTarget.BlockRelative),
        new PointerField(0x0C, "envHdr", PointerTarget.BlockRelative),
        new PointerField(0x14, "surfHead", PointerTarget.BlockRelative),
        new PointerField(0x18, "surfTail", PointerTarget.BlockRelative),
        new PointerField(0x1C, "surfHdr", PointerTarget.BlockRelative),
        new PointerField(0x24, "persosHead", PointerTarget.BlockRelative),
        new PointerField(0x28, "persosTail", PointerTarget.BlockRelative),
        new PointerField(0x2C, "persosHdr", PointerTarget.BlockRelative),
        new PointerField(0x34, "staticLightsHead", PointerTarget.BlockRelative),
        new PointerField(0x38, "staticLightsTail", PointerTarget.BlockRelative),
        new PointerField(0x3C, "staticLightsHdr", PointerTarget.BlockRelative),
        new PointerField(0x44, "dynLightsHead", PointerTarget.BlockRelative),
        new PointerField(0x48, "dynLightsTail", PointerTarget.BlockRelative),
        new PointerField(0x4C, "dynLightsHdr", PointerTarget.BlockRelative),
        new PointerField(0x54, "streamsHead", PointerTarget.BlockRelative),
        new PointerField(0x58, "streamsTail", PointerTarget.BlockRelative),
        new PointerField(0x5C, "streamsHdr", PointerTarget.BlockRelative),
        new PointerField(0x64, "graphicSectorsHead", PointerTarget.BlockRelative),
        new PointerField(0x68, "graphicSectorsTail", PointerTarget.BlockRelative),
        new PointerField(0x6C, "graphicSectorsHdr", PointerTarget.BlockRelative),
        new PointerField(0x74, "collisionSectorsHead", PointerTarget.BlockRelative),
        new PointerField(0x78, "collisionSectorsTail", PointerTarget.BlockRelative),
        new PointerField(0x7C, "collisionSectorsHdr", PointerTarget.BlockRelative),
        new PointerField(0x84, "activitySectorsHead", PointerTarget.BlockRelative),
        new PointerField(0x88, "activitySectorsTail", PointerTarget.BlockRelative),
        new PointerField(0x8C, "activitySectorsHdr", PointerTarget.BlockRelative),
        new PointerField(0x94, "soundSectorsHead", PointerTarget.BlockRelative),
        new PointerField(0x98, "soundSectorsTail", PointerTarget.BlockRelative),
        new PointerField(0x9C, "soundSectorsHdr", PointerTarget.BlockRelative),
        new PointerField(0xA4, "placeholderHead", PointerTarget.BlockRelative),
        new PointerField(0xA8, "placeholderTail", PointerTarget.BlockRelative),
        new PointerField(0xAC, "placeholderHdr", PointerTarget.BlockRelative),
        new PointerField(0xC8, "name", PointerTarget.BlockRelative)
    ];

    public string Kind => "sector";
    public string Schema => "astrolabe.sector.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public SectorRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(SectorRecord));
        return new SectorRecord
        {
            CollideObj = StructBinaryIO.ReadInt32(slice, 0x00),
            EnvHead = StructBinaryIO.ReadInt32(slice, 0x04),
            EnvTail = StructBinaryIO.ReadInt32(slice, 0x08),
            EnvHdr = StructBinaryIO.ReadInt32(slice, 0x0C),
            EnvCount = StructBinaryIO.ReadUInt32(slice, 0x10),
            SurfHead = StructBinaryIO.ReadInt32(slice, 0x14),
            SurfTail = StructBinaryIO.ReadInt32(slice, 0x18),
            SurfHdr = StructBinaryIO.ReadInt32(slice, 0x1C),
            SurfCount = StructBinaryIO.ReadUInt32(slice, 0x20),
            PersosHead = StructBinaryIO.ReadInt32(slice, 0x24),
            PersosTail = StructBinaryIO.ReadInt32(slice, 0x28),
            PersosHdr = StructBinaryIO.ReadInt32(slice, 0x2C),
            PersosCount = StructBinaryIO.ReadUInt32(slice, 0x30),
            StaticLightsHead = StructBinaryIO.ReadInt32(slice, 0x34),
            StaticLightsTail = StructBinaryIO.ReadInt32(slice, 0x38),
            StaticLightsHdr = StructBinaryIO.ReadInt32(slice, 0x3C),
            StaticLightsCount = StructBinaryIO.ReadUInt32(slice, 0x40),
            DynLightsHead = StructBinaryIO.ReadInt32(slice, 0x44),
            DynLightsTail = StructBinaryIO.ReadInt32(slice, 0x48),
            DynLightsHdr = StructBinaryIO.ReadInt32(slice, 0x4C),
            DynLightsCount = StructBinaryIO.ReadUInt32(slice, 0x50),
            StreamsHead = StructBinaryIO.ReadInt32(slice, 0x54),
            StreamsTail = StructBinaryIO.ReadInt32(slice, 0x58),
            StreamsHdr = StructBinaryIO.ReadInt32(slice, 0x5C),
            StreamsCount = StructBinaryIO.ReadUInt32(slice, 0x60),
            GraphicSectorsHead = StructBinaryIO.ReadInt32(slice, 0x64),
            GraphicSectorsTail = StructBinaryIO.ReadInt32(slice, 0x68),
            GraphicSectorsHdr = StructBinaryIO.ReadInt32(slice, 0x6C),
            GraphicSectorsCount = StructBinaryIO.ReadUInt32(slice, 0x70),
            CollisionSectorsHead = StructBinaryIO.ReadInt32(slice, 0x74),
            CollisionSectorsTail = StructBinaryIO.ReadInt32(slice, 0x78),
            CollisionSectorsHdr = StructBinaryIO.ReadInt32(slice, 0x7C),
            CollisionSectorsCount = StructBinaryIO.ReadUInt32(slice, 0x80),
            ActivitySectorsHead = StructBinaryIO.ReadInt32(slice, 0x84),
            ActivitySectorsTail = StructBinaryIO.ReadInt32(slice, 0x88),
            ActivitySectorsHdr = StructBinaryIO.ReadInt32(slice, 0x8C),
            ActivitySectorsCount = StructBinaryIO.ReadUInt32(slice, 0x90),
            SoundSectorsHead = StructBinaryIO.ReadInt32(slice, 0x94),
            SoundSectorsTail = StructBinaryIO.ReadInt32(slice, 0x98),
            SoundSectorsHdr = StructBinaryIO.ReadInt32(slice, 0x9C),
            SoundSectorsCount = StructBinaryIO.ReadUInt32(slice, 0xA0),
            PlaceholderHead = StructBinaryIO.ReadInt32(slice, 0xA4),
            PlaceholderTail = StructBinaryIO.ReadInt32(slice, 0xA8),
            PlaceholderHdr = StructBinaryIO.ReadInt32(slice, 0xAC),
            PlaceholderCount = StructBinaryIO.ReadUInt32(slice, 0xB0),
            UnknownB4 = StructBinaryIO.ReadUInt32(slice, 0xB4),
            UnknownB8 = StructBinaryIO.ReadUInt32(slice, 0xB8),
            UnknownBC = StructBinaryIO.ReadUInt32(slice, 0xBC),
            IsSectorVirtual = StructBinaryIO.ReadUInt32(slice, 0xC0),
            ActivationFlag = StructBinaryIO.ReadUInt32(slice, 0xC4),
            Name = StructBinaryIO.ReadInt32(slice, 0xC8),
            UnknownCC = StructBinaryIO.ReadUInt32(slice, 0xCC)
        };
    }

    public byte[] Write(SectorRecord value)
    {
        var bytes = new byte[Size];
        StructBinaryIO.WriteInt32(bytes, 0x00, value.CollideObj);
        StructBinaryIO.WriteInt32(bytes, 0x04, value.EnvHead);
        StructBinaryIO.WriteInt32(bytes, 0x08, value.EnvTail);
        StructBinaryIO.WriteInt32(bytes, 0x0C, value.EnvHdr);
        StructBinaryIO.WriteUInt32(bytes, 0x10, value.EnvCount);
        StructBinaryIO.WriteInt32(bytes, 0x14, value.SurfHead);
        StructBinaryIO.WriteInt32(bytes, 0x18, value.SurfTail);
        StructBinaryIO.WriteInt32(bytes, 0x1C, value.SurfHdr);
        StructBinaryIO.WriteUInt32(bytes, 0x20, value.SurfCount);
        StructBinaryIO.WriteInt32(bytes, 0x24, value.PersosHead);
        StructBinaryIO.WriteInt32(bytes, 0x28, value.PersosTail);
        StructBinaryIO.WriteInt32(bytes, 0x2C, value.PersosHdr);
        StructBinaryIO.WriteUInt32(bytes, 0x30, value.PersosCount);
        StructBinaryIO.WriteInt32(bytes, 0x34, value.StaticLightsHead);
        StructBinaryIO.WriteInt32(bytes, 0x38, value.StaticLightsTail);
        StructBinaryIO.WriteInt32(bytes, 0x3C, value.StaticLightsHdr);
        StructBinaryIO.WriteUInt32(bytes, 0x40, value.StaticLightsCount);
        StructBinaryIO.WriteInt32(bytes, 0x44, value.DynLightsHead);
        StructBinaryIO.WriteInt32(bytes, 0x48, value.DynLightsTail);
        StructBinaryIO.WriteInt32(bytes, 0x4C, value.DynLightsHdr);
        StructBinaryIO.WriteUInt32(bytes, 0x50, value.DynLightsCount);
        StructBinaryIO.WriteInt32(bytes, 0x54, value.StreamsHead);
        StructBinaryIO.WriteInt32(bytes, 0x58, value.StreamsTail);
        StructBinaryIO.WriteInt32(bytes, 0x5C, value.StreamsHdr);
        StructBinaryIO.WriteUInt32(bytes, 0x60, value.StreamsCount);
        StructBinaryIO.WriteInt32(bytes, 0x64, value.GraphicSectorsHead);
        StructBinaryIO.WriteInt32(bytes, 0x68, value.GraphicSectorsTail);
        StructBinaryIO.WriteInt32(bytes, 0x6C, value.GraphicSectorsHdr);
        StructBinaryIO.WriteUInt32(bytes, 0x70, value.GraphicSectorsCount);
        StructBinaryIO.WriteInt32(bytes, 0x74, value.CollisionSectorsHead);
        StructBinaryIO.WriteInt32(bytes, 0x78, value.CollisionSectorsTail);
        StructBinaryIO.WriteInt32(bytes, 0x7C, value.CollisionSectorsHdr);
        StructBinaryIO.WriteUInt32(bytes, 0x80, value.CollisionSectorsCount);
        StructBinaryIO.WriteInt32(bytes, 0x84, value.ActivitySectorsHead);
        StructBinaryIO.WriteInt32(bytes, 0x88, value.ActivitySectorsTail);
        StructBinaryIO.WriteInt32(bytes, 0x8C, value.ActivitySectorsHdr);
        StructBinaryIO.WriteUInt32(bytes, 0x90, value.ActivitySectorsCount);
        StructBinaryIO.WriteInt32(bytes, 0x94, value.SoundSectorsHead);
        StructBinaryIO.WriteInt32(bytes, 0x98, value.SoundSectorsTail);
        StructBinaryIO.WriteInt32(bytes, 0x9C, value.SoundSectorsHdr);
        StructBinaryIO.WriteUInt32(bytes, 0xA0, value.SoundSectorsCount);
        StructBinaryIO.WriteInt32(bytes, 0xA4, value.PlaceholderHead);
        StructBinaryIO.WriteInt32(bytes, 0xA8, value.PlaceholderTail);
        StructBinaryIO.WriteInt32(bytes, 0xAC, value.PlaceholderHdr);
        StructBinaryIO.WriteUInt32(bytes, 0xB0, value.PlaceholderCount);
        StructBinaryIO.WriteUInt32(bytes, 0xB4, value.UnknownB4);
        StructBinaryIO.WriteUInt32(bytes, 0xB8, value.UnknownB8);
        StructBinaryIO.WriteUInt32(bytes, 0xBC, value.UnknownBC);
        StructBinaryIO.WriteUInt32(bytes, 0xC0, value.IsSectorVirtual);
        StructBinaryIO.WriteUInt32(bytes, 0xC4, value.ActivationFlag);
        StructBinaryIO.WriteInt32(bytes, 0xC8, value.Name);
        StructBinaryIO.WriteUInt32(bytes, 0xCC, value.UnknownCC);
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(SectorRecord));
    }

    public SectorRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<SectorRecord>(json, Schema);

    public void ToJson(SectorRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}