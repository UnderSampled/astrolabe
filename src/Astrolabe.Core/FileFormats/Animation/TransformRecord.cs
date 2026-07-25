namespace Astrolabe.Core.FileFormats.Animation;

/// <summary>
/// Montreal compressed transform wire record (raymap: <c>Matrix.ReadCompressed</c>).
/// </summary>
public sealed class TransformRecord
{
    public string Schema { get; set; } = "astrolabe.transform.v1";

    /// <summary>Stable id within <c>animation/transforms.json</c> (not a VM address).</summary>
    public string Id { get; set; } = "";

    /// <summary>Variable-length transform payload (type byte + int16 fields).</summary>
    public byte[] WireBytes { get; set; } = [];

    /// <summary>Interstitial bytes after the transform in the stream (typically 4 or 6).</summary>
    public byte[] TrailingGap { get; set; } = [];

    /// <summary>Import-only provenance; export must not require this for reconstruction.</summary>
    public int? ProvenanceVirtualAddress { get; set; }
}
