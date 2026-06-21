using Astrolabe.Core.Hub;
using System.Text.Json;
using Astrolabe.Core.FileFormats.Perso;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class PersoSectorInfoCodec : IStructCodec<PersoSectorInfoRecord>
{
    public const int Size = 0x10;

    public static PersoSectorInfoCodec Instance { get; } = new();

    private static readonly PointerField[] PointerFieldsList =
    [
        new PointerField(0x00, "sector", PointerTarget.BlockRelative)
    ];

    public string Kind => "persosectorinfo";
    public string Schema => "astrolabe.perso-sector-info.v1";
    public int? FixedSize => Size;
    public IReadOnlyList<PointerField> PointerFields => PointerFieldsList;

    public PersoSectorInfoRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = StructBinaryIO.RequireExactSize(data.Slice(offset, length), Size, nameof(PersoSectorInfoRecord));
        return new PersoSectorInfoRecord
        {
            Sector = HubReferenceIO.Read(slice, 0x00),
            Unknown04 = slice.AsSpan(0x04, 0x0C).ToArray()
        };
    }

    public byte[] Write(PersoSectorInfoRecord value)
    {
        var bytes = new byte[Size];
        HubReferenceIO.Write(bytes, 0x00, value.Sector);
        if (value.Unknown04.Length != 0x0C)
        {
            throw new InvalidDataException("unknown04 must contain exactly 12 bytes.");
        }

        value.Unknown04.CopyTo(bytes.AsSpan(0x04));
        return JsonStructCodec.RequireExactSize(bytes, Size, nameof(PersoSectorInfoRecord));
    }

    public PersoSectorInfoRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<PersoSectorInfoRecord>(json, Schema);

    public void ToJson(PersoSectorInfoRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);
}