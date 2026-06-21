namespace Astrolabe.Core.Hub;

public sealed class HubElement
{
    public required string Kind { get; init; }
    public required string DataPath { get; init; }
    public required string Schema { get; set; }
    public object? Value { get; set; }
    public int VirtualAddress { get; init; }
    public int OffsetInBlock { get; init; }
    public int Length { get; init; }
    public byte BlockModule { get; init; }
    public byte BlockId { get; init; }
    public string BlockKey { get; init; } = "";

    public bool IsHydrated => Value != null;
}