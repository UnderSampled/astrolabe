using System.Text;
using System.Text.Json;
using Astrolabe.Core.FileFormats.Perso;

namespace Astrolabe.Core.Serialization.Codecs;

/// <summary>
/// Null-terminated (or raw) name blobs: <c>objecttypename</c>, <c>sectorname</c>.
/// </summary>
public sealed class TextNameCodec : IStructCodec<TextNameRecord>
{
    private TextNameCodec(string kind)
    {
        Kind = kind;
    }

    public string Kind { get; }
    public string Schema => "astrolabe.text-name.v1";
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } = [];

    public static TextNameCodec ObjectTypeName { get; } = new("objecttypename");
    public static TextNameCodec SectorName { get; } = new("sectorname");

    public TextNameRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length).ToArray();
        return new TextNameRecord
        {
            Type = Kind,
            Text = DecodeText(slice),
            Bytes = slice
        };
    }

    public byte[] Write(TextNameRecord value)
    {
        if (value.Bytes is { Length: > 0 })
        {
            return value.Bytes.ToArray();
        }

        // Rebuild C-string from text when bytes were omitted by an editor.
        var text = value.Text ?? "";
        var encoded = Encoding.ASCII.GetBytes(text);
        var bytes = new byte[encoded.Length + 1];
        encoded.CopyTo(bytes, 0);
        return bytes;
    }

    public TextNameRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<TextNameRecord>(json, Schema);

    public void ToJson(TextNameRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);

    private static string DecodeText(ReadOnlySpan<byte> data)
    {
        if (data.IsEmpty)
        {
            return "";
        }

        var nul = data.IndexOf((byte)0);
        var span = nul >= 0 ? data[..nul] : data;
        if (span.IsEmpty)
        {
            return "";
        }

        // Prefer ASCII when printable; otherwise leave text empty (bytes carry fidelity).
        for (var i = 0; i < span.Length; i++)
        {
            var b = span[i];
            if (b is < 0x20 or > 0x7E)
            {
                return "";
            }
        }

        return Encoding.ASCII.GetString(span);
    }
}
