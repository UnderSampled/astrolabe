namespace Astrolabe.Core.FileFormats.Animation;

/// <summary>
/// Montreal compressed transform wire record (raymap: <c>Matrix.ReadCompressed</c>).
/// </summary>
public sealed class TransformRecord
{
    public string Schema { get; set; } = "astrolabe.transform.v1";

    /// <summary>Original VM address of this transform in block 06:02 (export pointer target).</summary>
    public int VirtualAddress { get; set; }

    /// <summary>Variable-length transform payload (type byte + int16 fields).</summary>
    public byte[] WireBytes { get; set; } = [];

    /// <summary>Interstitial bytes after the transform in the 06:02 stream (typically 4 or 6).</summary>
    public byte[] TrailingGap { get; set; } = [];
}