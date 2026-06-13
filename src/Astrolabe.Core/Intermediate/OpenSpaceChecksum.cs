namespace Astrolabe.Core.Intermediate;

public static class OpenSpaceChecksum
{
    private const uint Modulo = 0xFFF1;

    public static uint Calculate(ReadOnlySpan<byte> data)
    {
        uint sum = 1;
        uint weighted = 0;

        foreach (var value in data)
        {
            sum += value;
            weighted += sum;
            sum %= Modulo;
            weighted %= Modulo;
        }

        return sum | (weighted << 16);
    }
}
