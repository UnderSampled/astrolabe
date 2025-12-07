using System.Numerics;

namespace Astrolabe.Core.FileFormats.Animation;

/// <summary>
/// Reads Family, State, ObjectList, and Perso structures from memory.
/// </summary>
public class FamilyReader
{
    private readonly MemoryContext _memory;
    private readonly AnimationReader _animReader;
    private readonly Dictionary<int, Family> _familyCache = new();
    private readonly Dictionary<int, ObjectList> _objectListCache = new();
    private readonly Dictionary<int, State> _stateCache = new();

    public FamilyReader(MemoryContext memory)
    {
        _memory = memory;
        _animReader = new AnimationReader(memory);
    }

    /// <summary>
    /// Reads a Perso structure and its associated Family.
    /// </summary>
    public Perso? ReadPerso(int address)
    {
        if (address == 0) return null;

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var perso = new Perso { Address = address };

            // Perso structure (Montreal):
            // +0x00: Pointer off_3dData
            // +0x04: Pointer off_stdGame
            // +0x08: Pointer off_dynam
            // +0x0C: uint32 (Montreal padding)
            // +0x10: Pointer off_brain
            // +0x14: Pointer off_camera
            // +0x18: Pointer off_collSet
            // +0x1C: Pointer off_msWay
            // +0x20: Pointer off_msLight
            // +0x24: uint32 (Montreal padding)
            // +0x28: Pointer off_sectInfo

            perso.Off3dData = reader.ReadInt32();
            perso.OffStdGame = reader.ReadInt32();
            reader.ReadInt32(); // off_dynam
            reader.ReadInt32(); // Montreal padding
            perso.OffBrain = reader.ReadInt32();
            reader.ReadInt32(); // off_camera
            perso.OffCollSet = reader.ReadInt32();

            // Read 3D data to get Family reference
            if (perso.Off3dData != 0)
            {
                perso.P3dData = ReadPerso3dData(perso.Off3dData);
                if (perso.P3dData?.OffFamily != 0)
                {
                    perso.Family = ReadFamily(perso.P3dData!.OffFamily);
                }
            }

            // Try to read names from stdGame
            if (perso.OffStdGame != 0)
            {
                ReadPersoNames(perso);
            }

            return perso;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Reads Perso3dData structure.
    /// </summary>
    private Perso3dData? ReadPerso3dData(int address)
    {
        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var data = new Perso3dData { Address = address };

            // Perso3dData structure (from raymap):
            // +0x00: Pointer off_stateInitial
            // +0x04: Pointer off_stateCurrent
            // +0x08: Pointer off_state2
            // +0x0C: Pointer off_objectList
            // +0x10: Pointer off_objectListInitial
            // +0x14: Pointer off_family

            reader.ReadInt32(); // off_stateInitial
            data.OffStateCurrent = reader.ReadInt32();
            reader.ReadInt32(); // off_state2
            data.OffObjectList = reader.ReadInt32();
            data.OffObjectListInitial = reader.ReadInt32();
            data.OffFamily = reader.ReadInt32();

            return data;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Attempts to read Perso names from StandardGame structure.
    /// </summary>
    private void ReadPersoNames(Perso perso)
    {
        var stdReader = _memory.GetReaderAt(perso.OffStdGame);
        if (stdReader == null)
        {
            // Fallback to address-based names
            perso.NameFamily = $"Family_0x{perso.Off3dData:X8}";
            perso.NameModel = $"Model_0x{perso.Address:X8}";
            perso.NamePerso = $"Perso_0x{perso.Address:X8}";
            return;
        }

        try
        {
            // StandardGame structure (Montreal engine < R3):
            // +0x00: uint32 objectType_Family (index into objectTypes[0])
            // +0x04: uint32 objectType_Model (index into objectTypes[1])
            // +0x08: uint32 objectType_Perso (index into objectTypes[2])
            // +0x0C: Pointer off_superobject

            uint familyTypeIndex = stdReader.ReadUInt32();
            uint modelTypeIndex = stdReader.ReadUInt32();
            uint persoTypeIndex = stdReader.ReadUInt32();

            // Store raw index for name lookup
            perso.FamilyTypeIndex = familyTypeIndex;

            // Use indices as names (actual names would require the objectTypes table)
            perso.NameFamily = $"Family_{familyTypeIndex}";
            perso.NameModel = $"Model_{modelTypeIndex}";
            perso.NamePerso = $"Perso_{persoTypeIndex}";
        }
        catch
        {
            perso.NameFamily = $"Family_0x{perso.Off3dData:X8}";
            perso.NameModel = $"Model_0x{perso.Address:X8}";
            perso.NamePerso = $"Perso_0x{perso.Address:X8}";
        }
    }

    /// <summary>
    /// Reads a Family structure.
    /// </summary>
    public Family? ReadFamily(int address)
    {
        if (address == 0) return null;
        if (_familyCache.TryGetValue(address, out var cached))
            return cached;

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var family = new Family { Address = address };

            // Family structure (Montreal):
            // +0x00: Pointer off_family_next (linked list)
            // +0x04: Pointer off_family_prev
            // +0x08: Pointer off_family_hdr
            // +0x0C: uint32 family_index
            // +0x10: LinkedList<State> states (head, tail, count)
            // +0x1C: Pointer off_physical_list_default
            // +0x20: LinkedList<ObjectList> objectLists (head, tail, count)
            // +0x2C: Pointer off_bounding_volume
            // +0x30: uint32 (skip)
            // +0x34: byte animBank
            // +0x35: byte (skip)
            // +0x36: byte (skip)
            // +0x37: byte (skip)
            // +0x38: byte properties

            reader.ReadInt32(); // off_family_next
            reader.ReadInt32(); // off_family_prev
            reader.ReadInt32(); // off_family_hdr
            family.FamilyIndex = reader.ReadUInt32();

            // Read states linked list header
            int statesHead = reader.ReadInt32();
            int statesTail = reader.ReadInt32();
            uint statesCount = reader.ReadUInt32();

            // Read default object list pointer
            int offPhysicalListDefault = reader.ReadInt32();

            // Read object lists linked list header
            int objectListsHead = reader.ReadInt32();
            int objectListsTail = reader.ReadInt32();
            uint objectListsCount = reader.ReadUInt32();

            family.OffBoundingVolume = reader.ReadInt32();

            // Montreal engine: additional padding then animBank/properties
            reader.ReadUInt32(); // skip
            family.AnimBank = reader.ReadByte();
            reader.ReadByte();
            reader.ReadByte();
            reader.ReadByte();
            family.Properties = reader.ReadByte();

            // Generate name from family index
            family.Name = $"Family_{family.FamilyIndex}";

            // Read states
            if (statesHead != 0 && statesCount > 0 && statesCount < 1000)
            {
                ReadStatesLinkedList(family, statesHead, statesCount);
            }

            // Read default object list - this is the main one
            if (offPhysicalListDefault != 0)
            {
                var defaultList = ReadObjectList(offPhysicalListDefault);
                if (defaultList != null)
                {
                    family.ObjectLists.Add(defaultList);
                }
            }

            _familyCache[address] = family;
            return family;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Reads states from a linked list.
    /// </summary>
    private void ReadStatesLinkedList(Family family, int headAddress, uint count)
    {
        int currentAddr = headAddress;

        for (uint i = 0; i < count && currentAddr != 0; i++)
        {
            var state = ReadState(currentAddr, family, (int)i);
            if (state != null)
            {
                family.States.Add(state);

                // Get next state from linked list pointer
                var nextReader = _memory.GetReaderAt(currentAddr);
                if (nextReader != null)
                {
                    // State structure starts with linked list pointers after name
                    // For Montreal with names: name (0x50) + next (4) + prev (4)
                    // Without names: next (4) at start
                    // Try to read next pointer - it should be at a fixed offset
                    // Actually, State.Read shows name is first (0x50 bytes if hasNames)
                    // Then next pointer

                    // Skip the state data we already read and get next
                    // For simplicity, use a fixed stride based on Montreal state size
                    // Typical state size is around 0x30-0x40 bytes after name
                    currentAddr = GetNextStateAddress(currentAddr, state);
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
    }

    /// <summary>
    /// Gets the next state address from the linked list.
    /// </summary>
    private int GetNextStateAddress(int stateAddr, State state)
    {
        // State linked list: after name (0x50 bytes), next pointer is at +0x00
        // But Montreal may have name inline
        // Looking at raymap: if hasNames, name is 0x50 bytes, then off_entry_next

        // For Montreal without reading name length, we estimate:
        // Try reading at state address directly (assumes no names or name already skipped)
        var reader = _memory.GetReaderAt(stateAddr);
        if (reader == null) return 0;

        // Skip potential name (0x50 bytes if present - Montreal has names)
        // Then read next pointer
        try
        {
            // Check if first bytes look like a string (ASCII printable)
            byte[] probe = new byte[4];
            reader.Read(probe, 0, 4);
            bool looksLikeName = probe.All(b => b >= 0x20 && b < 0x7F || b == 0);

            reader = _memory.GetReaderAt(stateAddr);
            if (reader == null) return 0;

            if (looksLikeName)
            {
                // Skip name (0x50 bytes)
                reader.BaseStream.Seek(0x50, SeekOrigin.Current);
            }

            // Read next pointer
            return reader.ReadInt32();
        }
        catch
        {
            return 0;
        }
    }

    /// <summary>
    /// Reads a State structure.
    /// </summary>
    private State? ReadState(int address, Family family, int index)
    {
        if (address == 0) return null;
        if (_stateCache.TryGetValue(address, out var cached))
            return cached;

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var state = new State { Address = address, Index = index };

            // Check if there's a name (Montreal has names - 0x50 bytes)
            byte[] probe = new byte[4];
            reader.Read(probe, 0, 4);
            bool hasName = probe.All(b => (b >= 0x20 && b < 0x7F) || b == 0);

            reader = _memory.GetReaderAt(address);
            if (reader == null) return null;

            if (hasName)
            {
                // Read name (0x50 bytes, null-terminated)
                byte[] nameBytes = reader.ReadBytes(0x50);
                int nullIndex = Array.IndexOf(nameBytes, (byte)0);
                if (nullIndex > 0)
                {
                    state.Name = System.Text.Encoding.ASCII.GetString(nameBytes, 0, nullIndex);
                }
            }

            // State structure after name:
            // +0x00: Pointer off_entry_next
            // +0x04: Pointer off_entry_prev (if hasLinkedListHeaderPointers)
            // +0x08: Pointer off_entry_hdr (if hasLinkedListHeaderPointers)
            // Then: Pointer off_anim_ref
            // ... transitions, etc.
            // Near end: speed byte

            reader.ReadInt32(); // off_entry_next
            // Montreal has linked list header pointers
            reader.ReadInt32(); // off_entry_prev
            reader.ReadInt32(); // off_entry_hdr

            state.OffAnimRef = reader.ReadInt32();

            // Skip transitions (linked list headers)
            reader.ReadInt32(); // stateTransitions head
            reader.ReadInt32(); // stateTransitions tail
            reader.ReadUInt32(); // stateTransitions count

            reader.ReadInt32(); // prohibitStates head
            reader.ReadInt32(); // prohibitStates tail
            reader.ReadUInt32(); // prohibitStates count

            state.OffNextState = reader.ReadInt32();
            reader.ReadInt32(); // off_mechanicsIDCard

            // Montreal-specific: additional fields then speed
            reader.ReadUInt32();
            reader.ReadUInt32();
            reader.ReadByte();
            reader.ReadByte();
            reader.ReadByte();
            state.Speed = reader.ReadByte();

            // Read animation
            if (state.OffAnimRef != 0)
            {
                state.Animation = _animReader.ReadAnimation(state.OffAnimRef);
            }

            _stateCache[address] = state;
            return state;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Reads an ObjectList structure.
    /// </summary>
    public ObjectList? ReadObjectList(int address)
    {
        if (address == 0) return null;
        if (_objectListCache.TryGetValue(address, out var cached))
            return cached;

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var objectList = new ObjectList { Address = address };

            // ObjectList structure (Montreal engine with linked list headers):
            // +0x00: Pointer off_objList_next
            // +0x04: Pointer off_objList_prev (hasLinkedListHeaderPointers)
            // +0x08: Pointer off_objList_hdr (hasLinkedListHeaderPointers)
            // +0x0C: Pointer off_objList_start (points to entry array)
            // +0x10: Pointer off_objList_2 (copy?)
            // +0x14: uint16 num_entries
            // +0x16: uint16 (padding)

            reader.ReadInt32(); // off_objList_next
            reader.ReadInt32(); // off_objList_prev
            reader.ReadInt32(); // off_objList_hdr
            int offObjListStart = reader.ReadInt32();
            reader.ReadInt32(); // off_objList_2
            ushort numEntries = reader.ReadUInt16();
            reader.ReadUInt16(); // padding

            if (numEntries == 0 || numEntries > 256 || offObjListStart == 0)
            {
                return null;
            }

            // Go to entry array
            var entryReader = _memory.GetReaderAt(offObjListStart);
            if (entryReader == null) return null;

            // Read entries - each entry is 0x14 bytes (20 bytes):
            // +0x00: Pointer off_scale
            // +0x04: Pointer off_po -> PhysicalObject
            // +0x08: uint32 thirdvalue
            // +0x0C: uint16 unk0
            // +0x0E: uint16 unk1
            // +0x10: uint32 lastvalue
            for (int i = 0; i < numEntries; i++)
            {
                var entry = new ObjectListEntry
                {
                    OffScale = entryReader.ReadInt32(),
                    OffPhysicalObject = entryReader.ReadInt32()
                };
                uint thirdvalue = entryReader.ReadUInt32();
                entryReader.ReadUInt16(); // unk0
                entryReader.ReadUInt16(); // unk1
                uint lastvalue = entryReader.ReadUInt32();

                // Only read if lastvalue or thirdvalue != 0 (based on raymap logic)
                if (lastvalue != 0 || thirdvalue != 0)
                {
                    // Read scale if present
                    if (entry.OffScale != 0)
                    {
                        var scaleReader = _memory.GetReaderAt(entry.OffScale);
                        if (scaleReader != null)
                        {
                            float sx = scaleReader.ReadSingle();
                            float sz = scaleReader.ReadSingle();
                            float sy = scaleReader.ReadSingle();
                            entry.Scale = new Vector3(sx, sy, sz); // Y/Z swap happens in file
                        }
                    }

                    // Read PhysicalObject to get GeometricObject address
                    if (entry.OffPhysicalObject != 0)
                    {
                        entry.PhysicalObjectAddress = entry.OffPhysicalObject;
                        entry.GeometricObjectAddress = ReadGeometricObjectFromPhysical(entry.OffPhysicalObject);
                    }
                }

                objectList.Entries.Add(entry);
            }

            _objectListCache[address] = objectList;
            return objectList;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Reads a GeometricObject address from a PhysicalObject.
    /// </summary>
    private int ReadGeometricObjectFromPhysical(int physicalObjectAddress)
    {
        var poReader = _memory.GetReaderAt(physicalObjectAddress);
        if (poReader == null) return 0;

        // PhysicalObject structure:
        // +0x00: Pointer off_visualSet
        int offVisualSet = poReader.ReadInt32();

        if (offVisualSet == 0) return 0;

        // VisualSet structure:
        // +0x00: uint32 (0)
        // +0x04: uint16 numberOfLOD
        // +0x06: uint16 visualSetType
        // +0x08: Pointer off_LODDistances
        // +0x0C: Pointer off_LODDataOffsets
        var vsReader = _memory.GetReaderAt(offVisualSet);
        if (vsReader == null) return 0;

        vsReader.ReadUInt32(); // 0
        ushort numLOD = vsReader.ReadUInt16();
        vsReader.ReadUInt16(); // visualSetType
        vsReader.ReadInt32(); // off_LODDistances
        int offLODDataOffsets = vsReader.ReadInt32();

        if (numLOD > 0 && offLODDataOffsets != 0)
        {
            var lodReader = _memory.GetReaderAt(offLODDataOffsets);
            if (lodReader != null)
            {
                return lodReader.ReadInt32();
            }
        }

        return 0;
    }

    /// <summary>
    /// Finds all Persos in the scene graph.
    /// </summary>
    public List<Perso> FindPersosInSceneGraph(SceneGraph sceneGraph)
    {
        var persos = new List<Perso>();

        foreach (var node in sceneGraph.AllNodes.Where(n => n.Type == SuperObjectType.Perso))
        {
            if (node.OffData != 0)
            {
                var perso = ReadPerso(node.OffData);
                if (perso != null)
                {
                    persos.Add(perso);
                }
            }
        }

        return persos;
    }

    /// <summary>
    /// Gets all unique Families from a list of Persos.
    /// Sets ObjectTypeIndex from the first Perso's stdGame for name lookup.
    /// </summary>
    public List<Family> GetUniqueFamilies(IEnumerable<Perso> persos)
    {
        var families = new Dictionary<int, Family>();

        foreach (var perso in persos)
        {
            if (perso.Family != null && !families.ContainsKey(perso.Family.Address))
            {
                // Store the ObjectType index from stdGame for name lookup
                perso.Family.ObjectTypeIndex = perso.FamilyTypeIndex;
                families[perso.Family.Address] = perso.Family;
            }
        }

        return families.Values.ToList();
    }
}
