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

    /// <summary>
    /// Number of always-active objects.
    /// </summary>
    public uint NumAlways { get; private set; }

    /// <summary>
    /// Spawnable persos linked list header.
    /// </summary>
    public int OffSpawnableHead { get; private set; }
    public int OffSpawnableTail { get; private set; }
    public uint SpawnableCount { get; private set; }

    /// <summary>
    /// Always reusable SuperObject.
    /// </summary>
    public int OffAlwaysReusableSO { get; private set; }

    /// <summary>
    /// Object type tables (3 tables: Family, Model, Instance names).
    /// </summary>
    public List<(int Head, int Tail, uint Count)> ObjectTypeTables { get; } = new();

    /// <summary>
    /// Families linked list.
    /// </summary>
    public int OffFamiliesHead { get; private set; }
    public int OffFamiliesTail { get; private set; }
    public uint FamiliesCount { get; private set; }

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

        // Continue parsing more GPT fields
        reader.ReadUInt32(); // soundEventIndex

        NumAlways = reader.ReadUInt32();

        // Spawnable persos linked list header
        OffSpawnableHead = reader.ReadInt32();
        OffSpawnableTail = reader.ReadInt32();
        reader.ReadInt32(); // hdr pointer
        SpawnableCount = reader.ReadUInt32();

        OffAlwaysReusableSO = reader.ReadInt32();
        reader.ReadUInt32(); // Montreal: pointer table for always

        // Object type tables (3 tables)
        for (int i = 0; i < 3; i++)
        {
            int head = reader.ReadInt32();
            int tail = reader.ReadInt32();
            uint count = reader.ReadUInt32();
            ObjectTypeTables.Add((head, tail, count));
        }

        // Engine structure: skip map names and unknown data
        reader.ReadByte();
        reader.ReadBytes(0x104); // mapName1
        reader.ReadBytes(0x104); // mapName2
        reader.ReadBytes(0x104); // mapName3
        reader.ReadBytes(0x2627); // Hype: unknown data

        // Unknown linked list
        reader.ReadInt32(); // off_unknown_first
        reader.ReadInt32(); // off_unknown_last
        reader.ReadUInt32(); // num_unknown

        // Families linked list header
        OffFamiliesHead = reader.ReadInt32();
        OffFamiliesTail = reader.ReadInt32();
        reader.ReadInt32(); // hdr pointer
        FamiliesCount = reader.ReadUInt32();
    }

    /// <summary>
    /// All pointers found in the GPT header, for tracking orphan references.
    /// </summary>
    public List<(int Offset, int Pointer, string Label)> AllPointers { get; } = new();

    /// <summary>
    /// Creates a BinaryReader positioned at the start of the GPT data.
    /// </summary>
    public BinaryReader GetReader()
    {
        return new BinaryReader(new MemoryStream(Data));
    }

    /// <summary>
    /// Scans the entire GPT for pointer-like values.
    /// </summary>
    public void ScanForPointers()
    {
        AllPointers.Clear();
        using var reader = new BinaryReader(new MemoryStream(Data));

        // Scan every 4 bytes looking for pointer-like values
        for (int offset = 0; offset < Data.Length - 4; offset += 4)
        {
            reader.BaseStream.Position = offset;
            int value = reader.ReadInt32();

            // Check if this looks like a valid pointer (in typical SNA memory range)
            if (value > 0x08000000 && value < 0x10000000)
            {
                string label = offset switch
                {
                    0x10 => "ActualWorld",
                    0x14 => "DynamicWorld",
                    0x18 => "FatherSector",
                    _ => $"GPT+0x{offset:X3}"
                };
                AllPointers.Add((offset, value, label));
            }
        }
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
