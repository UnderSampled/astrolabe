namespace Astrolabe.Core.Serialization;

public enum PointerTarget
{
    BlockRelative,
    Fix,
    Any
}

public readonly record struct PointerField(
    int Offset,
    string Name,
    PointerTarget Target,
    int[]? IgnoreValues = null,
    bool RequiresVmRange = false,
    bool RequiresDecompressedTarget = false);