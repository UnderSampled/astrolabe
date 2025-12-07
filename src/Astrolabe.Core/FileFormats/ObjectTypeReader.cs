using System.Text;

namespace Astrolabe.Core.FileFormats;

/// <summary>
/// Reads object type names from level data (Family, Model, Perso names).
/// </summary>
public class ObjectTypeReader
{
    private readonly MemoryContext _memory;
    private readonly SnaReader _sna;

    public ObjectTypeReader(MemoryContext memory, SnaReader sna)
    {
        _memory = memory;
        _sna = sna;
    }

    /// <summary>
    /// Object type entry containing name and indices.
    /// </summary>
    public class ObjectType
    {
        public int Address { get; set; }
        public string Name { get; set; } = "";
        public byte Unk1 { get; set; }
        public byte Id { get; set; }
        public ushort Unk2 { get; set; }
    }

    /// <summary>
    /// Tries to find and read the object types table by scanning SNA data.
    /// The object types are stored as 3 consecutive linked list headers in the level data.
    /// Returns a dictionary mapping family_index to name.
    /// </summary>
    public Dictionary<uint, string> TryFindFamilyNames()
    {
        var result = new Dictionary<uint, string>();

        // The object types are in the main level block (typically module 0)
        // They're stored as 3 linked list headers followed by the family linked list
        // We can find them by looking for a sequence that makes sense

        foreach (var block in _sna.Blocks)
        {
            if (block.Data == null || block.Data.Length < 0x1000) continue;

            // Try to find the object types table by scanning for valid patterns
            var names = TryScanBlockForObjectTypes(block);
            if (names.Count > 0)
            {
                foreach (var kvp in names)
                {
                    result[kvp.Key] = kvp.Value;
                }
                break;
            }
        }

        return result;
    }

    /// <summary>
    /// Scans a block looking for Family type entries and builds a map from linked list position to name.
    /// ObjectType entries have the structure:
    /// +0x00: next pointer (4 bytes)
    /// +0x04: prev pointer (4 bytes)
    /// +0x08: header pointer (4 bytes)
    /// +0x0C: name pointer (4 bytes) - points to the inline name string
    /// +0x10: marker 01 00 00 00
    /// +0x14: internal id (4 bytes) - NOT the objectType index!
    /// +0x18: name string (inline after the entry)
    ///
    /// Family names start with lowercase letters (not I/M prefix).
    /// We need to traverse the linked list to determine the actual objectType index.
    /// </summary>
    private Dictionary<uint, string> TryScanBlockForObjectTypes(SnaBlock block)
    {
        var result = new Dictionary<uint, string>();
        if (block.Data == null) return result;

        // First pass: Find all Family type entries (lowercase names) and store their addresses
        var familyEntries = new List<(int address, string name)>();

        for (int i = 0; i < block.Data.Length - 32; i++)
        {
            // Look for the marker pattern: 01 00 00 00 followed by an id byte, then a name
            if (block.Data[i] == 0x01 && block.Data[i+1] == 0x00 &&
                block.Data[i+2] == 0x00 && block.Data[i+3] == 0x00)
            {
                // Read internal id (next 4 bytes) - just for validation
                uint internalId = BitConverter.ToUInt32(block.Data, i + 4);
                if (internalId > 100) continue;

                // Check for name string starting at i+8
                int nameStart = i + 8;
                if (nameStart >= block.Data.Length) continue;

                // Exclude Instance and Model type names, keep Family names
                // Instance names: I/i + uppercase, or I/i + underscore
                // Model names: M/m + uppercase, or M/m + underscore
                // Family names: Everything else (including proper nouns like "World", "Actor3")
                byte firstChar = block.Data[nameStart];
                if (firstChar < 'A' || (firstChar > 'Z' && firstChar < 'a') || firstChar > 'z') continue;

                // Check for Instance/Model patterns
                if (nameStart + 1 < block.Data.Length)
                {
                    byte secondChar = block.Data[nameStart + 1];
                    // I/i + uppercase = Instance name
                    if ((firstChar == 'I' || firstChar == 'i') && secondChar >= 'A' && secondChar <= 'Z') continue;
                    // M/m + uppercase = Model name
                    if ((firstChar == 'M' || firstChar == 'm') && secondChar >= 'A' && secondChar <= 'Z') continue;
                    // I/i + underscore = Instance name
                    if ((firstChar == 'I' || firstChar == 'i') && secondChar == '_') continue;
                    // M/m + underscore = Model name
                    if ((firstChar == 'M' || firstChar == 'm') && secondChar == '_') continue;
                }

                // Read the name until null terminator
                int j = nameStart;
                while (j < block.Data.Length && block.Data[j] >= 0x20 && block.Data[j] < 0x7F && j - nameStart < 64)
                    j++;

                if (j - nameStart < 2) continue;
                if (j < block.Data.Length && block.Data[j] != 0) continue;

                string name = Encoding.ASCII.GetString(block.Data, nameStart, j - nameStart);
                if (!name.All(c => char.IsLetterOrDigit(c) || c == '_' || c == '-')) continue;

                // Calculate the entry address (the struct starts 0x10 bytes before the marker)
                // Entry: [next 4][prev 4][header 4][name_ptr 4][marker 4][id 4][name...]
                int entryAddr = block.BaseInMemory + i - 0x10;
                familyEntries.Add((entryAddr, name));

                // Skip past this name to avoid false duplicates
                i = j;
            }
        }

        // Now we have all Family entries with their addresses.
        // We need to find which one is at which position in the linked list.
        // The linked list is ordered by the "next" pointer at offset 0.

        // Build a map from entry address to (next address, name)
        var entryMap = new Dictionary<int, (int next, string name)>();
        foreach (var (entryAddr, name) in familyEntries)
        {
            // Read the next pointer from the entry
            int blockOffset = entryAddr - block.BaseInMemory;
            if (blockOffset >= 0 && blockOffset + 4 <= block.Data.Length)
            {
                int nextAddr = BitConverter.ToInt32(block.Data, blockOffset);
                entryMap[entryAddr] = (nextAddr, name);
            }
        }

        // Find the longest chain that doesn't contain Instance/Model prefixed names
        // This is the Family types chain
        if (entryMap.Count > 0)
        {
            // For each entry, check if it's part of an Instance or Model chain
            // by looking at whether other entries in its chain have I/M+uppercase prefix
            var familyChainEntries = new HashSet<int>();

            foreach (var startAddr in entryMap.Keys)
            {
                // Traverse forward and backward from this entry
                var chain = new List<(int addr, string name)>();
                var visited = new HashSet<int>();

                // Go forward
                int current = startAddr;
                while (current != 0 && entryMap.TryGetValue(current, out var entry) && !visited.Contains(current))
                {
                    visited.Add(current);
                    chain.Add((current, entry.name));
                    current = entry.next;
                }

                // Go backward using prev pointer
                int blockOffset = startAddr - block.BaseInMemory;
                if (blockOffset >= 4 && blockOffset + 8 <= block.Data.Length)
                {
                    int prevAddr = BitConverter.ToInt32(block.Data, blockOffset + 4);
                    while (prevAddr != 0 && !visited.Contains(prevAddr))
                    {
                        int prevBlockOffset = prevAddr - block.BaseInMemory;
                        if (prevBlockOffset < 0 || prevBlockOffset + 20 > block.Data.Length) break;

                        // Read the name at prev entry
                        int prevNamePtrOffset = prevBlockOffset + 0x0C;
                        int prevMarkerOffset = prevBlockOffset + 0x10;
                        int prevIdOffset = prevBlockOffset + 0x14;
                        int prevNameOffset = prevBlockOffset + 0x18;

                        if (prevNameOffset >= block.Data.Length) break;
                        if (BitConverter.ToUInt32(block.Data, prevMarkerOffset) != 1) break;

                        // Read name
                        int j = prevNameOffset;
                        while (j < block.Data.Length && block.Data[j] >= 0x20 && block.Data[j] < 0x7F && j - prevNameOffset < 64)
                            j++;
                        if (j - prevNameOffset < 2) break;

                        string prevName = Encoding.ASCII.GetString(block.Data, prevNameOffset, j - prevNameOffset);
                        visited.Add(prevAddr);
                        chain.Insert(0, (prevAddr, prevName));

                        prevAddr = BitConverter.ToInt32(block.Data, prevBlockOffset + 4);
                    }
                }

                // Check if this chain contains any Instance/Model prefixed names
                bool hasInstanceModelPrefix = false;
                foreach (var (addr, name) in chain)
                {
                    if (name.Length >= 2)
                    {
                        char first = name[0];
                        char second = name[1];
                        if ((first == 'I' || first == 'M') && second >= 'A' && second <= 'Z')
                        {
                            hasInstanceModelPrefix = true;
                            break;
                        }
                    }
                }

                if (!hasInstanceModelPrefix && chain.Count > familyChainEntries.Count)
                {
                    // This is a candidate for the Family chain
                    familyChainEntries.Clear();
                    result.Clear();

                    // Find the objectType index offset by reading the Family struct near the first entry
                    // The Family struct is near each objectType entry and contains FamilyIndex at offset 0x0C
                    uint baseIndex = 0;
                    if (chain.Count > 0)
                    {
                        // Try to find a Family struct near the first entry
                        // Family structs are typically 0x58 bytes after the objectType entry
                        int firstEntryAddr = chain[0].addr;

                        // Look for a Family struct pattern nearby
                        for (int offset = 0x30; offset < 0x100; offset += 4)
                        {
                            int checkOffset = firstEntryAddr + offset - block.BaseInMemory;
                            if (checkOffset >= 0 && checkOffset + 0x30 < block.Data.Length)
                            {
                                // Check if this looks like a Family struct (has valid linked list pointers)
                                int potentialNext = BitConverter.ToInt32(block.Data, checkOffset);
                                int potentialIndex = BitConverter.ToInt32(block.Data, checkOffset + 0x0C);
                                if (potentialNext > 0x09000000 && potentialNext < 0x0A000000 &&
                                    potentialIndex >= 0 && potentialIndex < 100)
                                {
                                    baseIndex = (uint)potentialIndex;
                                    break;
                                }
                            }
                        }
                    }

                    for (int idx = 0; idx < chain.Count; idx++)
                    {
                        uint objectTypeIndex = baseIndex + (uint)idx;
                        result[objectTypeIndex] = chain[idx].name;
                        familyChainEntries.Add(chain[idx].addr);
                    }
                }
            }
        }

        return result;
    }

    /// <summary>
    /// Checks if a name looks valid (printable ASCII).
    /// </summary>
    private static bool IsValidName(string name)
    {
        if (string.IsNullOrEmpty(name) || name.Length > 100) return false;
        return name.All(c => c >= 0x20 && c < 0x7F);
    }

    /// <summary>
    /// Tries to find and read the object types table starting from a known address.
    /// Returns arrays for [Family types, Model types, Perso types].
    /// </summary>
    public ObjectType[][] TryReadObjectTypesFrom(int startAddress)
    {
        var result = new ObjectType[3][];

        var reader = _memory.GetReaderAt(startAddress);
        if (reader == null)
        {
            return result;
        }

        try
        {
            // Read 3 linked list headers (Family, Model, Perso types)
            for (int i = 0; i < 3; i++)
            {
                // Linked list header (Double type)
                int firstEntry = reader.ReadInt32();
                int lastEntry = reader.ReadInt32();
                uint count = reader.ReadUInt32();

                if (count > 0 && count < 1000 && firstEntry != 0)
                {
                    result[i] = ReadObjectTypeList(firstEntry, count);
                }
                else
                {
                    result[i] = [];
                }
            }

            return result;
        }
        catch
        {
            return result;
        }
    }

    /// <summary>
    /// Reads a linked list of object types.
    /// </summary>
    private ObjectType[] ReadObjectTypeList(int firstAddress, uint count)
    {
        var types = new List<ObjectType>();
        int current = firstAddress;

        for (uint i = 0; i < count && current != 0; i++)
        {
            var objType = ReadObjectType(current);
            if (objType != null)
            {
                types.Add(objType);

                // Get next entry address (first field of linked list entry)
                var nextReader = _memory.GetReaderAt(current);
                if (nextReader != null)
                {
                    current = nextReader.ReadInt32();
                }
                else
                {
                    break;
                }
            }
            else
            {
                break;
            }
        }

        return types.ToArray();
    }

    /// <summary>
    /// Reads a single object type entry.
    /// </summary>
    private ObjectType? ReadObjectType(int address)
    {
        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var objType = new ObjectType { Address = address };

            // ObjectType structure (Montreal with hasLinkedListHeaderPointers):
            // +0x00: Pointer off_next
            // +0x04: Pointer off_prev
            // +0x08: Pointer off_header
            // +0x0C: Pointer off_name
            // +0x10: byte unk1
            // +0x11: byte id
            // +0x12: ushort unk2

            reader.ReadInt32(); // off_next
            reader.ReadInt32(); // off_prev
            reader.ReadInt32(); // off_header
            int offName = reader.ReadInt32();
            objType.Unk1 = reader.ReadByte();
            objType.Id = reader.ReadByte();
            objType.Unk2 = reader.ReadUInt16();

            // Read the name string
            if (offName != 0)
            {
                var nameReader = _memory.GetReaderAt(offName);
                if (nameReader != null)
                {
                    objType.Name = ReadNullTerminatedString(nameReader, 256);
                }
            }

            return objType;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Reads a null-terminated string.
    /// </summary>
    private static string ReadNullTerminatedString(BinaryReader reader, int maxLength)
    {
        var bytes = new List<byte>();
        for (int i = 0; i < maxLength; i++)
        {
            byte b = reader.ReadByte();
            if (b == 0) break;
            bytes.Add(b);
        }
        return Encoding.ASCII.GetString(bytes.ToArray());
    }
}
