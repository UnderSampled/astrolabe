using System.Text.Json;
using System.Text.Json.Serialization;

namespace Astrolabe.Core.Hub;

/// <summary>
/// In-memory hub pointer: Rete URIs on disk, object links after resolution, VM ints only at export.
/// </summary>
[JsonConverter(typeof(HubReferenceJsonConverter))]
public sealed class HubReference : IEquatable<HubReference>
{
    public static HubReference Null { get; } = new();

    public string? Uri { get; private init; }

    /// <summary>Transient wire value during OpenSpace import before URI rewrite.</summary>
    public int WireValue { get; private init; }

    /// <summary>Materialized VM address during OpenSpace export layout.</summary>
    public int ResolvedAddress { get; set; }

    public object? Target { get; set; }

    public bool IsNull => Target == null && string.IsNullOrWhiteSpace(Uri) && WireValue == 0 && ResolvedAddress == 0;

    public static HubReference FromUri(string? uri) =>
        string.IsNullOrWhiteSpace(uri) ? Null : new HubReference { Uri = uri };

    public static HubReference FromWire(int value) =>
        value == 0 ? Null : new HubReference { WireValue = value };

    public static HubReference FromTarget(object target, string? uri = null) =>
        new() { Target = target, Uri = uri };

    public int MaterializeForWire() =>
        ResolvedAddress != 0 ? ResolvedAddress : WireValue;

    public bool Equals(HubReference? other)
    {
        if (other is null)
        {
            return false;
        }

        return Uri == other.Uri &&
               WireValue == other.WireValue &&
               ResolvedAddress == other.ResolvedAddress &&
               ReferenceEquals(Target, other.Target);
    }

    public override bool Equals(object? obj) => obj is HubReference other && Equals(other);

    public override int GetHashCode() => HashCode.Combine(Uri, WireValue, ResolvedAddress, Target);

    public override string ToString() =>
        Uri ?? (Target != null ? $"<{Target.GetType().Name}>" : WireValue != 0 ? $"0x{WireValue:X8}" : "null");
}

internal sealed class HubReferenceJsonConverter : JsonConverter<HubReference>
{
    public override HubReference Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        return reader.TokenType switch
        {
            JsonTokenType.Null => HubReference.Null,
            JsonTokenType.String => HubReference.FromUri(reader.GetString()),
            JsonTokenType.Number when reader.TryGetInt32(out var value) => HubReference.FromWire(value),
            JsonTokenType.Number when reader.TryGetUInt32(out var unsigned) && unsigned <= int.MaxValue =>
                HubReference.FromWire((int)unsigned),
            _ => throw new JsonException($"Cannot deserialize HubReference from token {reader.TokenType}.")
        };
    }

    public override void Write(Utf8JsonWriter writer, HubReference value, JsonSerializerOptions options)
    {
        if (!string.IsNullOrWhiteSpace(value.Uri))
        {
            writer.WriteStringValue(value.Uri);
            return;
        }

        if (value.WireValue != 0)
        {
            writer.WriteNumberValue(value.WireValue);
            return;
        }

        writer.WriteNullValue();
    }
}