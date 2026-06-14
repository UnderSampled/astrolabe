namespace Astrolabe.Core.Serialization;

internal static class VmPointerScanning
{
    public static bool IsLikelyVirtualAddress(int value) =>
        value >= 0x0800_0000 && value < 0x1000_0000;
}