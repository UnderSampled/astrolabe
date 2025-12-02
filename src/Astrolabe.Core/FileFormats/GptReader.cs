namespace Astrolabe.Core.FileFormats;

/// <summary>
/// Reads the GPT (Global Pointer Table) file to extract scene root pointers.
/// Montreal engine format.
/// </summary>
public class GptReader
{
    public byte[] Data { get; }

    /// <summary>
    /// Pointer to the actual world (main scene root).
    /// </summary>
    public int OffActualWorld { get; private set; }

    /// <summary>
    /// Pointer to the dynamic world (dynamic objects).
    /// </summary>
    public int OffDynamicWorld { get; private set; }

    /// <summary>
    /// Pointer to the father sector (sector hierarchy root).
    /// </summary>
    public int OffFatherSector { get; private set; }

    public GptReader(string filePath)
    {
        Data = File.ReadAllBytes(filePath);
        Parse();
    }

    public GptReader(byte[] data)
    {
        Data = data;
        Parse();
    }

    private void Parse()
    {
        using var reader = new BinaryReader(new MemoryStream(Data));

        // Montreal engine LVL GPT structure:
        // +0x00: Pointer (sound related, skip)
        // +0x04: Pointer (skip)
        // +0x08: Pointer (skip)
        // +0x0C: uint32 (skip)
        // +0x10: off_actualWorld
        // +0x14: off_dynamicWorld
        // +0x18: off_fatherSector

        reader.ReadInt32(); // sound related
        reader.ReadInt32(); // skip
        reader.ReadInt32(); // skip
        reader.ReadUInt32(); // skip

        OffActualWorld = reader.ReadInt32();
        OffDynamicWorld = reader.ReadInt32();
        OffFatherSector = reader.ReadInt32();
    }

    /// <summary>
    /// Creates a BinaryReader positioned at the start of the GPT data.
    /// </summary>
    public BinaryReader GetReader()
    {
        return new BinaryReader(new MemoryStream(Data));
    }

    /// <summary>
    /// Prints debug information about the GPT.
    /// </summary>
    public void PrintDebugInfo(TextWriter writer)
    {
        writer.WriteLine("GPT Entry Points:");
        writer.WriteLine($"  off_actualWorld:  0x{OffActualWorld:X8}");
        writer.WriteLine($"  off_dynamicWorld: 0x{OffDynamicWorld:X8}");
        writer.WriteLine($"  off_fatherSector: 0x{OffFatherSector:X8}");
    }
}
