using System.Numerics;

namespace Astrolabe.Core.FileFormats;

/// <summary>
/// Reads SuperObject hierarchy while tracking which bytes are read.
/// This version tracks ALL data including mesh vertices, triangles, materials,
/// AI scripts, animations, families, states, waypoints, etc.
/// </summary>
public class TrackingSuperObjectReader
{
    private readonly MemoryContext _memory;
    private readonly ByteRangeTracker _tracker;
    private readonly HashSet<int> _visitedAddresses = new();
    private readonly HashSet<int> _visitedGeometricObjects = new();
    private readonly HashSet<int> _visitedMaterials = new();
    private readonly HashSet<int> _visitedAI = new();
    private readonly HashSet<int> _visitedFamilies = new();
    private readonly HashSet<int> _visitedStates = new();
    private readonly HashSet<int> _visitedAnimations = new();
    private readonly HashSet<int> _visitedObjectLists = new();
    private readonly HashSet<int> _visitedWaypoints = new();
    private readonly HashSet<int> _visitedGraphs = new();

    public ByteRangeTracker Tracker => _tracker;

    public TrackingSuperObjectReader(MemoryContext memory, ByteRangeTracker tracker)
    {
        _memory = memory;
        _tracker = tracker;
    }

    private SceneGraph? _currentGraph;

    public SceneGraph ReadSceneGraph(GptReader gpt)
    {
        var graph = new SceneGraph();
        _currentGraph = graph;

        if (gpt.OffActualWorld != 0)
        {
            graph.ActualWorld = ReadSuperObject(gpt.OffActualWorld);
        }

        if (gpt.OffDynamicWorld != 0)
        {
            graph.DynamicWorld = ReadSuperObject(gpt.OffDynamicWorld);
        }

        if (gpt.OffFatherSector != 0)
        {
            graph.FatherSector = ReadSuperObject(gpt.OffFatherSector);
        }

        // Track additional GPT structures
        TrackGptExtras(gpt);

        _currentGraph = null;
        return graph;
    }

    /// <summary>
    /// Tracks additional GPT structures not reached through scene graph traversal.
    /// </summary>
    private void TrackGptExtras(GptReader gpt)
    {
        // Track spawnable persos linked list
        if (gpt.SpawnableCount > 0 && gpt.OffSpawnableHead != 0)
        {
            TrackSpawnablePersos(gpt.OffSpawnableHead, gpt.SpawnableCount);
        }

        // Track always reusable SuperObjects
        if (gpt.NumAlways > 0 && gpt.OffAlwaysReusableSO != 0)
        {
            TrackAlwaysSuperObjects(gpt.OffAlwaysReusableSO, gpt.NumAlways);
        }

        // Track object type tables (Family/Model/Instance names)
        foreach (var table in gpt.ObjectTypeTables)
        {
            if (table.Count > 0 && table.Head != 0)
            {
                TrackObjectTypeTable(table.Head, table.Count);
            }
        }

        // Track families linked list
        if (gpt.FamiliesCount > 0 && gpt.OffFamiliesHead != 0)
        {
            TrackFamiliesLinkedList(gpt.OffFamiliesHead, gpt.FamiliesCount);
        }
    }

    private void TrackSpawnablePersos(int headAddress, uint count)
    {
        // Each spawnable entry is: index (4) + off_perso (4) = 8 bytes
        // Plus linked list pointers: next (4) + prev (4) + hdr (4) = 12 bytes
        const int EntrySize = 20;
        int current = headAddress;

        for (uint i = 0; i < count && current != 0; i++)
        {
            _tracker.Record(current, EntrySize, "SpawnableEntry");

            var reader = _memory.GetReaderAt(current);
            if (reader == null) break;

            int next = reader.ReadInt32();
            reader.ReadInt32(); // prev
            reader.ReadInt32(); // hdr
            reader.ReadUInt32(); // index
            int offPerso = reader.ReadInt32();

            // Track the spawnable perso itself
            if (offPerso != 0)
            {
                if (!_visitedAddresses.Contains(offPerso))
                {
                    ReadSuperObject(offPerso);
                }
                else
                {
                    TrackSpawnablePersoDynam(offPerso);
                }
            }

            current = next;
        }
    }

    private void TrackAlwaysSuperObjects(int startAddress, uint count)
    {
        // Always SuperObjects are empty SO structures
        const int EmptySOSize = 0x38;
        int totalSize = (int)(count * EmptySOSize);
        _tracker.Record(startAddress, totalSize, "AlwaysSuperObjects");
    }

    private void TrackObjectTypeTable(int headAddress, uint count)
    {
        // Each entry: next (4) + prev (4) + hdr (4) + off_name (4) + index (4) = 20 bytes
        const int EntrySize = 20;
        int current = headAddress;

        for (uint i = 0; i < count && current != 0; i++)
        {
            _tracker.Record(current, EntrySize, "ObjectTypeEntry");

            var reader = _memory.GetReaderAt(current);
            if (reader == null) break;

            int next = reader.ReadInt32();
            reader.ReadInt32(); // prev
            reader.ReadInt32(); // hdr
            int offName = reader.ReadInt32();

            // Track the name string
            if (offName != 0)
            {
                string? name = ReadNullTerminatedString(offName, out int nameLen);
                if (nameLen > 0)
                {
                    _tracker.Record(offName, nameLen, "ObjectTypeName");
                }
            }

            current = next;
        }
    }

    private void TrackFamiliesLinkedList(int headAddress, uint count)
    {
        // Track families from GPT linked list that might not be reached via scene graph
        int current = headAddress;

        for (uint i = 0; i < count && current != 0; i++)
        {
            // Linked list entry header: next (4) + prev (4) + hdr (4) = 12 bytes
            _tracker.Record(current, 12, "FamilyListEntry");

            var reader = _memory.GetReaderAt(current);
            if (reader == null) break;

            int next = reader.ReadInt32();
            reader.ReadInt32(); // prev
            reader.ReadInt32(); // hdr

            // The family data follows the linked list header
            int familyDataAddr = current + 12;
            if (!_visitedFamilies.Contains(familyDataAddr))
            {
                ReadFamily(0, familyDataAddr); // Use 0 as nodeAddr since this is global
            }

            current = next;
        }
    }

    public SceneNode? ReadSuperObject(int address, SceneNode? parent = null)
    {
        if (address == 0) return null;
        if (_visitedAddresses.Contains(address)) return null;

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        _visitedAddresses.Add(address);

        var node = new SceneNode
        {
            Address = address,
            Parent = parent
        };

        _currentGraph?.AllNodes.Add(node);

        try
        {
            // SuperObject structure: 0x38 bytes (56 bytes)
            const int SuperObjectSize = 0x38;
            _tracker.RecordForNode(address, address, SuperObjectSize, "SuperObject");

            node.TypeCode = reader.ReadUInt32();
            node.Type = GetSuperObjectType(node.TypeCode);
            node.OffData = reader.ReadInt32();

            int childrenHead = reader.ReadInt32();
            int childrenTail = reader.ReadInt32();
            uint childrenCount = reader.ReadUInt32();

            int offBrotherNext = reader.ReadInt32();
            int offBrotherPrev = reader.ReadInt32();
            int offParent = reader.ReadInt32();

            node.OffMatrix = reader.ReadInt32();
            node.OffStaticMatrix = reader.ReadInt32();
            int globalMatrix = reader.ReadInt32();

            node.DrawFlags = reader.ReadUInt32();
            node.Flags = reader.ReadUInt32();
            node.OffBoundingVolume = reader.ReadInt32();

            // Read transform matrices
            if (node.OffMatrix != 0)
            {
                node.Transform = ReadMatrix(node.Address, node.OffMatrix);
            }
            if (node.OffStaticMatrix != 0)
            {
                ReadMatrix(node.Address, node.OffStaticMatrix);
            }

            // Read bounding volume
            if (node.OffBoundingVolume != 0)
            {
                ReadBoundingVolume(node.Address, node.OffBoundingVolume);
            }

            // Read type-specific data
            ReadSuperObjectData(node);

            // Read children
            if (childrenHead != 0 && childrenCount > 0)
            {
                ReadChildren(node, childrenHead, childrenCount);
            }

            return node;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error reading SuperObject at 0x{address:X8}: {ex.Message}");
            return node;
        }
    }

    private void ReadChildren(SceneNode parent, int headAddress, uint count)
    {
        int currentAddress = headAddress;

        for (uint i = 0; i < count && currentAddress != 0; i++)
        {
            var reader = _memory.GetReaderAt(currentAddress);
            if (reader == null) break;

            var child = ReadSuperObject(currentAddress, parent);
            if (child != null)
            {
                parent.Children.Add(child);

                var siblingReader = _memory.GetReaderAt(currentAddress + 0x14);
                if (siblingReader != null)
                {
                    currentAddress = siblingReader.ReadInt32();
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

    private void ReadSuperObjectData(SceneNode node)
    {
        if (node.OffData == 0) return;

        switch (node.Type)
        {
            case SuperObjectType.IPO:
            case SuperObjectType.IPO_2:
                ReadIPOData(node);
                break;

            case SuperObjectType.GeometricObject:
                node.GeometricObjectAddress = node.OffData;
                ReadGeometricObjectFull(node.Address, node.OffData);
                break;

            case SuperObjectType.PhysicalObject:
                ReadPhysicalObjectData(node, node.OffData);
                break;

            case SuperObjectType.Sector:
                ReadSectorData(node);
                break;

            case SuperObjectType.Perso:
                ReadPersoDataFull(node);
                break;

            case SuperObjectType.World:
            default:
                break;
        }
    }

    private void ReadSectorData(SceneNode node)
    {
        if (node.OffData == 0) return;

        // Sector structure (Montreal) - ~0xC0 bytes:
        // +0x00: off_collideObj
        // +0x04: environments list header (16 bytes)
        // +0x14: surface list header (16 bytes)
        // +0x24: persos list header (16 bytes)
        // +0x34: staticLights list (16 bytes)
        // +0x44: dynamicLights list header (16 bytes)
        // +0x54: streams list header (16 bytes)
        // +0x64: graphicSectors list header (16 bytes)
        // +0x74: collisionSectors list header (16 bytes)
        // +0x84: activitySectors list header (16 bytes)
        // +0x94: soundSectors list header (16 bytes)
        // +0xA4: placeholder list header (16 bytes)
        // +0xB4: 3 x uint32 (Montreal padding)
        // +0xC0: isSectorVirtual (4 bytes)
        // +0xC4: activation flag (4 bytes)
        // +0xC8: off_name
        const int SectorSize = 0xD0;
        _tracker.RecordForNode(node.Address, node.OffData, SectorSize, "Sector");

        var reader = _memory.GetReaderAt(node.OffData);
        if (reader == null) return;

        try
        {
            int offCollideObj = reader.ReadInt32();

            // Read collision geometry
            if (offCollideObj != 0)
            {
                ReadSectorCollisionGeometry(node.Address, offCollideObj);
            }

            // Read linked list headers and their entries
            // environments list
            ReadLinkedListData(node.Address, reader, "EnvList");
            // surface list
            ReadLinkedListData(node.Address, reader, "SurfList");
            // persos list
            ReadLinkedListData(node.Address, reader, "PersosList");
            // static lights
            ReadLightsLinkedList(node.Address, reader);
            // dynamic lights
            ReadLinkedListData(node.Address, reader, "DynLightsList");
            // streams list
            ReadLinkedListData(node.Address, reader, "StreamsList");
            // graphic sectors
            ReadNeighborSectorsList(node.Address, reader, "GraphicSectors");
            // collision sectors
            ReadNeighborSectorsList(node.Address, reader, "CollisionSectors");
            // activity sectors
            ReadLinkedListData(node.Address, reader, "ActivitySectors");
            // sound sectors
            ReadLinkedListData(node.Address, reader, "SoundSectors");
            // placeholder
            ReadLinkedListData(node.Address, reader, "PlaceholderList");

            // Skip Montreal padding
            reader.ReadInt32();
            reader.ReadInt32();
            reader.ReadInt32();

            // Virtual flag
            reader.ReadInt32();
            // Activation flag
            reader.ReadInt32();

            // Name pointer
            int offName = reader.ReadInt32();
            if (offName != 0)
            {
                string? name = ReadNullTerminatedString(offName, out int strLen);
                if (name != null && strLen > 0)
                {
                    node.Name = name;
                    _tracker.RecordForNode(node.Address, offName, strLen, "SectorName");
                }
            }
        }
        catch { }
    }

    private void ReadLinkedListData(int nodeAddr, BinaryReader reader, string label)
    {
        // Read linked list header: head (4) + tail (4) + hdr (4) + count (4) = 16 bytes
        int head = reader.ReadInt32();
        int tail = reader.ReadInt32();
        reader.ReadInt32(); // hdr
        uint count = reader.ReadUInt32();

        if (count > 0 && count < 10000 && head != 0)
        {
            // Track linked list entries
            TrackLinkedListEntries(nodeAddr, head, count, label);
        }
    }

    private void TrackLinkedListEntries(int nodeAddr, int headAddress, uint count, string label)
    {
        int current = headAddress;
        for (uint i = 0; i < count && current != 0 && i < 1000; i++)
        {
            // Each linked list entry: next (4) + prev (4) + hdr (4) + data... (variable)
            // Track minimum entry size
            _tracker.Record(current, 16, $"{label}Entry");

            var reader = _memory.GetReaderAt(current);
            if (reader == null) break;
            current = reader.ReadInt32(); // next
        }
    }

    private void ReadLightsLinkedList(int nodeAddr, BinaryReader reader)
    {
        int head = reader.ReadInt32();
        int tail = reader.ReadInt32();
        reader.ReadInt32(); // hdr
        uint count = reader.ReadUInt32();

        if (count > 0 && count < 1000 && head != 0)
        {
            // LightInfo structures are ~0x90 bytes
            TrackLightInfoEntries(nodeAddr, head, count);
        }
    }

    private void TrackLightInfoEntries(int nodeAddr, int headAddress, uint count)
    {
        int current = headAddress;
        for (uint i = 0; i < count && current != 0; i++)
        {
            // LightInfo is ~0x90 bytes
            _tracker.Record(current, 0x90, "LightInfo");

            var reader = _memory.GetReaderAt(current);
            if (reader == null) break;
            current = reader.ReadInt32(); // next
        }
    }

    private void ReadNeighborSectorsList(int nodeAddr, BinaryReader reader, string label)
    {
        int head = reader.ReadInt32();
        int tail = reader.ReadInt32();
        reader.ReadInt32(); // hdr
        uint count = reader.ReadUInt32();

        if (count > 0 && count < 10000 && head != 0)
        {
            // Neighbor sector entries: short0 (2) + short2 (2) + off_sector (4) + next (4) + prev (4) + hdr (4) = 20 bytes
            TrackNeighborSectorEntries(nodeAddr, head, count, label);
        }
    }

    private void TrackNeighborSectorEntries(int nodeAddr, int headAddress, uint count, string label)
    {
        int current = headAddress;
        for (uint i = 0; i < count && current != 0 && i < 1000; i++)
        {
            // Entry size: 4 + 4 + 4 + 4 + 4 = 20 bytes (Montreal)
            _tracker.Record(current, 20, $"{label}Entry");

            var reader = _memory.GetReaderAt(current);
            if (reader == null) break;

            reader.ReadInt32(); // short0, short2
            reader.ReadInt32(); // off_sector
            current = reader.ReadInt32(); // next
        }
    }

    private void ReadSectorCollisionGeometry(int nodeAddr, int address)
    {
        // Sector collision is a GeometricObjectCollide used as bounding volume
        // Track similar to regular collision geometry
        const int CollideHeaderSize = 0x30;
        _tracker.Record(address, CollideHeaderSize, "SectorCollideGeo");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            uint numVertices = reader.ReadUInt32();
            int offVertices = reader.ReadInt32();
            reader.ReadInt32(); // padding/normals
            uint numElements = reader.ReadUInt32();
            int offElementTypes = reader.ReadInt32();
            int offElements = reader.ReadInt32();

            if (numVertices > 0 && numVertices < 100000 && offVertices != 0)
            {
                _tracker.Record(offVertices, (int)(numVertices * 12), "SectorCollideVerts");
            }

            if (numElements > 0 && numElements < 10000)
            {
                if (offElementTypes != 0)
                {
                    _tracker.Record(offElementTypes, (int)(numElements * 2), "SectorCollideElemTypes");
                }
                if (offElements != 0)
                {
                    _tracker.Record(offElements, (int)(numElements * 4), "SectorCollideElemPtrs");
                }
            }
        }
        catch { }
    }

    // ==================== PERSO & RELATED STRUCTURES ====================

    private void ReadPersoDataFull(SceneNode node)
    {
        if (node.OffData == 0) return;

        // Perso structure (Montreal): ~0x40 bytes
        // +0x00: off_3dData
        // +0x04: off_stdGame
        // +0x08: off_dynam
        // +0x0C: uint (Montreal padding)
        // +0x10: off_brain
        // +0x14: off_camera
        // +0x18: off_collSet
        // +0x1C: off_msWay
        // +0x20: off_msLight
        // +0x24: uint (Montreal padding)
        // +0x28: off_sectInfo
        // ... more fields
        const int PersoSize = 0x40;
        _tracker.RecordForNode(node.Address, node.OffData, PersoSize, "Perso");

        var reader = _memory.GetReaderAt(node.OffData);
        if (reader == null) return;

        try
        {
            int off3dData = reader.ReadInt32();      // 0x00
            int offStdGame = reader.ReadInt32();     // 0x04
            int offDynam = reader.ReadInt32();       // 0x08
            reader.ReadInt32();                       // 0x0C Montreal padding
            int offBrain = reader.ReadInt32();       // 0x10
            int offCamera = reader.ReadInt32();      // 0x14
            int offCollSet = reader.ReadInt32();     // 0x18
            int offMsWay = reader.ReadInt32();       // 0x1C
            int offMsLight = reader.ReadInt32();     // 0x20
            reader.ReadInt32();                       // 0x24 Montreal padding
            int offSectInfo = reader.ReadInt32();    // 0x28

            if (offStdGame != 0)
            {
                ReadStandardGame(node, offStdGame);
            }

            if (off3dData != 0)
            {
                ReadPerso3dData(node.Address, off3dData);
            }

            if (offDynam != 0)
            {
                ReadDynam(node.Address, offDynam);
            }

            if (offBrain != 0)
            {
                ReadBrain(node.Address, offBrain);
            }

            if (offCollSet != 0)
            {
                ReadCollideSetFull(node.Address, offCollSet);
            }

            if (offMsWay != 0)
            {
                ReadMSWay(node.Address, offMsWay);
            }

            if (offSectInfo != 0)
            {
                ReadPersoSectorInfo(node.Address, offSectInfo);
            }
        }
        catch { }
    }

    private void ReadPerso3dData(int nodeAddr, int address)
    {
        if (_visitedFamilies.Contains(address)) return;
        _visitedFamilies.Add(address);

        // Perso3dData structure: ~0x20 bytes
        // +0x00: off_stateInitial
        // +0x04: off_stateCurrent
        // +0x08: off_state2
        // +0x0C: off_objectList
        // +0x10: off_objectListInitial
        // +0x14: off_family
        const int Perso3dDataSize = 0x20;
        _tracker.RecordForNode(nodeAddr, address, Perso3dDataSize, "Perso3dData");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offStateInitial = reader.ReadInt32();
            int offStateCurrent = reader.ReadInt32();
            int offState2 = reader.ReadInt32();
            int offObjectList = reader.ReadInt32();
            int offObjectListInitial = reader.ReadInt32();
            int offFamily = reader.ReadInt32();

            // Read family (which contains states and object lists)
            if (offFamily != 0)
            {
                ReadFamily(nodeAddr, offFamily);
            }

            // Read object lists
            if (offObjectList != 0)
            {
                ReadObjectList(nodeAddr, offObjectList);
            }
            if (offObjectListInitial != 0 && offObjectListInitial != offObjectList)
            {
                ReadObjectList(nodeAddr, offObjectListInitial);
            }

            // Read states if not already read via family
            if (offStateCurrent != 0)
            {
                ReadState(nodeAddr, offStateCurrent);
            }
        }
        catch { }
    }

    private void ReadFamily(int nodeAddr, int address)
    {
        if (_visitedFamilies.Contains(address)) return;
        _visitedFamilies.Add(address);

        // Family structure (Montreal): ~0x40 bytes
        // +0x00: off_next (linked list)
        // +0x04: off_prev
        // +0x08: off_hdr
        // +0x0C: family_index
        // +0x10: states LinkedList (12 bytes: head, tail, count)
        // +0x1C: off_physical_list_default
        // +0x20: objectLists LinkedList (12 bytes)
        // +0x2C: off_bounding_volume
        // +0x30: more fields...
        const int FamilySize = 0x40;
        _tracker.RecordForNode(nodeAddr, address, FamilySize, "Family");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            reader.ReadInt32(); // off_next
            reader.ReadInt32(); // off_prev
            reader.ReadInt32(); // off_hdr
            uint familyIndex = reader.ReadUInt32();

            // States linked list
            int statesHead = reader.ReadInt32();
            int statesTail = reader.ReadInt32();
            uint statesCount = reader.ReadUInt32();

            int offPhysListDefault = reader.ReadInt32();

            // Object lists linked list
            int objListHead = reader.ReadInt32();
            int objListTail = reader.ReadInt32();
            uint objListCount = reader.ReadUInt32();

            int offBoundingVolume = reader.ReadInt32();

            // Read states
            if (statesHead != 0 && statesCount > 0 && statesCount < 500)
            {
                ReadStateList(nodeAddr, statesHead, statesCount);
            }

            // Read object lists
            if (objListHead != 0 && objListCount > 0 && objListCount < 100)
            {
                ReadObjectListLinkedList(nodeAddr, objListHead, objListCount);
            }

            if (offBoundingVolume != 0)
            {
                ReadBoundingVolume(nodeAddr, offBoundingVolume);
            }
        }
        catch { }
    }

    private void ReadStateList(int nodeAddr, int headAddress, uint count)
    {
        int current = headAddress;
        for (uint i = 0; i < count && current != 0; i++)
        {
            ReadState(nodeAddr, current);

            // Get next pointer - State has linked list pointers at start (Montreal has no names)
            // State structure starts with next/prev/hdr pointers
            var reader = _memory.GetReaderAt(current);
            if (reader == null) break;
            current = reader.ReadInt32(); // off_next
        }
    }

    private void ReadState(int nodeAddr, int address)
    {
        if (_visitedStates.Contains(address)) return;
        _visitedStates.Add(address);

        // State structure (Montreal, no names): ~0x30 bytes
        // +0x00: off_next
        // +0x04: off_prev
        // +0x08: off_hdr
        // +0x0C: off_anim_ref
        // +0x10: transitions LinkedList (12 bytes)
        // +0x1C: prohibits LinkedList (12 bytes)
        // +0x28: off_nextState
        // +0x2C: off_mechanicsIDCard
        // +0x30: more fields (unk, unk, byte, speed)
        const int StateSize = 0x38;
        _tracker.RecordForNode(nodeAddr, address, StateSize, "State");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            reader.ReadInt32(); // off_next
            reader.ReadInt32(); // off_prev
            reader.ReadInt32(); // off_hdr
            int offAnimRef = reader.ReadInt32();

            // Transitions linked list
            int transHead = reader.ReadInt32();
            int transTail = reader.ReadInt32();
            uint transCount = reader.ReadUInt32();

            // Prohibits linked list
            int prohibHead = reader.ReadInt32();
            int prohibTail = reader.ReadInt32();
            uint prohibCount = reader.ReadUInt32();

            int offNextState = reader.ReadInt32();
            int offMechIDCard = reader.ReadInt32();

            // Read animation reference (Montreal format)
            if (offAnimRef != 0)
            {
                ReadAnimationMontreal(nodeAddr, offAnimRef);
            }

            // Read transitions
            if (transHead != 0 && transCount > 0 && transCount < 100)
            {
                ReadTransitionList(nodeAddr, transHead, transCount);
            }

            // Read mechanics ID card
            if (offMechIDCard != 0)
            {
                ReadMechanicsIDCard(nodeAddr, offMechIDCard);
            }
        }
        catch { }
    }

    private void ReadTransitionList(int nodeAddr, int headAddress, uint count)
    {
        int current = headAddress;
        for (uint i = 0; i < count && current != 0; i++)
        {
            // Transition structure: ~0x14 bytes
            const int TransitionSize = 0x14;
            _tracker.RecordForNode(nodeAddr, current, TransitionSize, "Transition");

            var reader = _memory.GetReaderAt(current);
            if (reader == null) break;
            current = reader.ReadInt32(); // off_next
        }
    }

    private void ReadMechanicsIDCard(int nodeAddr, int address)
    {
        // MechanicsIDCard: ~0x20 bytes
        const int MechIDCardSize = 0x20;
        _tracker.RecordForNode(nodeAddr, address, MechIDCardSize, "MechanicsIDCard");
    }

    // ==================== ANIMATION STRUCTURES ====================

    private void ReadAnimationMontreal(int nodeAddr, int address)
    {
        if (_visitedAnimations.Contains(address)) return;
        _visitedAnimations.Add(address);

        // AnimationMontreal structure: ~0x70 bytes
        // +0x00: off_frames
        // +0x04: num_frames (byte), speed (byte), num_channels (byte), unkbyte
        // +0x08: off_unk
        // +0x0C: padding
        // +0x10: padding
        // +0x14: speedMatrix (52 bytes)
        // ... more
        const int AnimMontrealSize = 0x70;
        _tracker.RecordForNode(nodeAddr, address, AnimMontrealSize, "AnimationMontreal");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offFrames = reader.ReadInt32();
            byte numFrames = reader.ReadByte();
            byte speed = reader.ReadByte();
            byte numChannels = reader.ReadByte();
            reader.ReadByte(); // unkbyte

            if (numFrames > 0 && numFrames < 255 && offFrames != 0)
            {
                ReadAnimFrames(nodeAddr, offFrames, numFrames, numChannels);
            }
        }
        catch { }
    }

    private void ReadAnimFrames(int nodeAddr, int address, int numFrames, int numChannels)
    {
        // AnimFrameMontreal is inline array, each frame ~0x10 bytes
        const int FrameSize = 0x10;
        int totalSize = numFrames * FrameSize;
        _tracker.RecordForNode(nodeAddr, address, totalSize, "AnimFrames");

        // Read each frame to get channel pointers
        for (int i = 0; i < numFrames; i++)
        {
            int frameAddr = address + i * FrameSize;
            var reader = _memory.GetReaderAt(frameAddr);
            if (reader == null) continue;

            try
            {
                int offChannels = reader.ReadInt32();
                int offMat = reader.ReadInt32();
                int offVec = reader.ReadInt32();
                int offHierarchies = reader.ReadInt32();

                // Read channel pointers array
                if (offChannels != 0 && numChannels > 0)
                {
                    int channelPtrsSize = numChannels * 4;
                    _tracker.RecordForNode(nodeAddr, offChannels, channelPtrsSize, "AnimChannelPtrs");

                    // Read each channel
                    var channelReader = _memory.GetReaderAt(offChannels);
                    if (channelReader != null)
                    {
                        for (int c = 0; c < numChannels; c++)
                        {
                            int channelAddr = channelReader.ReadInt32();
                            if (channelAddr != 0)
                            {
                                ReadAnimChannel(nodeAddr, channelAddr);
                            }
                        }
                    }
                }

                // Read hierarchies
                if (offHierarchies != 0)
                {
                    ReadAnimHierarchies(nodeAddr, offHierarchies);
                }
            }
            catch { }
        }
    }

    private void ReadAnimChannel(int nodeAddr, int address)
    {
        if (_visitedAnimations.Contains(address)) return;
        _visitedAnimations.Add(address);

        // AnimChannelMontreal: ~0x14 bytes
        // This may point to a compressed matrix if isIdentity != 1 && != 0
        const int ChannelSize = 0x14;
        _tracker.RecordForNode(nodeAddr, address, ChannelSize, "AnimChannel");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            uint isIdentity = reader.ReadUInt32();
            // If isIdentity is a pointer (not 0 or 1), it points to compressed matrix data
            if (isIdentity != 0 && isIdentity != 1 && isIdentity > 0x08000000 && isIdentity < 0x10000000)
            {
                ReadTransform(nodeAddr, (int)isIdentity);
            }
        }
        catch { }
    }

    private void ReadTransform(int nodeAddr, int address)
    {
        // Transform wire size depends on type field (raymap Matrix.ReadCompressed):
        // Type 1: translation only = 2 + 6 = 8 bytes
        // Type 2: rotation only = 2 + 8 = 10 bytes
        // Type 3: translation + rotation = 2 + 6 + 8 = 16 bytes
        // Type 7: translation + rotation + zoom = 2 + 6 + 8 + 2 = 18 bytes
        // Type 11: translation + rotation + axial scale = 2 + 6 + 8 + 6 = 22 bytes
        // Type 15: translation + rotation + matrix scale = 2 + 6 + 8 + 12 = 28 bytes

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            ushort type = reader.ReadUInt16();
            int actualType = type < 128 ? (type & 0xF) : 128;

            int size = 2; // Type field
            if (actualType == 1 || actualType == 3 || actualType == 7 || actualType == 11 || actualType == 15)
                size += 6; // Translation (3 shorts)
            if (actualType == 2 || actualType == 3 || actualType == 7 || actualType == 11 || actualType == 15)
                size += 8; // Rotation quaternion (4 shorts)
            if (actualType == 7)
                size += 2; // Uniform scale (1 short)
            else if (actualType == 11)
                size += 6; // Axial scale (3 shorts)
            else if (actualType == 15)
                size += 12; // Matrix scale (6 shorts)

            if (size < 8) size = 8; // Minimum size
            _tracker.RecordForNode(nodeAddr, address, size, "Transform");
        }
        catch
        {
            // Fallback to safe estimate
            _tracker.RecordForNode(nodeAddr, address, 28, "Transform");
        }
    }

    private void ReadAnimHierarchies(int nodeAddr, int address)
    {
        // Hierarchies structure: 8 bytes header + array
        _tracker.RecordForNode(nodeAddr, address, 8, "AnimHierarchiesHeader");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            uint numHierarchies = reader.ReadUInt32();
            int offHierarchies = reader.ReadInt32();

            if (numHierarchies > 0 && numHierarchies < 1000 && offHierarchies != 0)
            {
                // Each hierarchy entry is ~8 bytes
                int hierSize = (int)(numHierarchies * 8);
                _tracker.RecordForNode(nodeAddr, offHierarchies, hierSize, "AnimHierarchies");
            }
        }
        catch { }
    }

    // ==================== OBJECT LISTS ====================

    private void ReadObjectListLinkedList(int nodeAddr, int headAddress, uint count)
    {
        int current = headAddress;
        for (uint i = 0; i < count && current != 0; i++)
        {
            ReadObjectList(nodeAddr, current);

            // Get next pointer
            var reader = _memory.GetReaderAt(current);
            if (reader == null) break;
            current = reader.ReadInt32();
        }
    }

    private void ReadObjectList(int nodeAddr, int address)
    {
        if (_visitedObjectLists.Contains(address)) return;
        _visitedObjectLists.Add(address);

        // ObjectList structure: ~0x14 bytes
        // +0x00: off_next
        // +0x04: off_prev
        // +0x08: off_hdr
        // +0x0C: off_entries
        // +0x10: num_entries
        const int ObjectListSize = 0x14;
        _tracker.RecordForNode(nodeAddr, address, ObjectListSize, "ObjectList");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            reader.ReadInt32(); // off_next
            reader.ReadInt32(); // off_prev
            reader.ReadInt32(); // off_hdr
            int offEntries = reader.ReadInt32();
            uint numEntries = reader.ReadUInt32();

            if (offEntries != 0 && numEntries > 0 && numEntries < 1000)
            {
                // Each entry is a pointer to PhysicalObject (4 bytes)
                int entriesSize = (int)(numEntries * 4);
                _tracker.RecordForNode(nodeAddr, offEntries, entriesSize, "ObjectListEntries");

                // Read each PhysicalObject
                var entriesReader = _memory.GetReaderAt(offEntries);
                if (entriesReader != null)
                {
                    for (int i = 0; i < numEntries; i++)
                    {
                        int poAddr = entriesReader.ReadInt32();
                        if (poAddr != 0)
                        {
                            ReadPhysicalObjectFromList(nodeAddr, poAddr);
                        }
                    }
                }
            }
        }
        catch { }
    }

    private void ReadPhysicalObjectFromList(int nodeAddr, int address)
    {
        // Check if we've seen this already via scene graph
        if (_visitedGeometricObjects.Contains(address)) return;

        // Same as PhysicalObject from scene graph
        const int PhysicalObjectSize = 0x10;
        _tracker.RecordForNode(nodeAddr, address, PhysicalObjectSize, "PhysicalObject");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offVisualSet = reader.ReadInt32();
            int offCollideSet = reader.ReadInt32();
            int offVisualBoundingVolume = reader.ReadInt32();

            if (offVisualSet != 0)
            {
                ReadVisualSetData(nodeAddr, offVisualSet);
            }

            if (offCollideSet != 0)
            {
                ReadCollideSetFull(nodeAddr, offCollideSet);
            }

            if (offVisualBoundingVolume != 0)
            {
                ReadBoundingVolume(nodeAddr, offVisualBoundingVolume);
            }
        }
        catch { }
    }

    // ==================== WAYPOINTS & GRAPHS ====================

    private void ReadMSWay(int nodeAddr, int address)
    {
        if (_visitedWaypoints.Contains(address)) return;
        _visitedWaypoints.Add(address);

        // MSWay structure: ~0x10 bytes
        const int MSWaySize = 0x10;
        _tracker.RecordForNode(nodeAddr, address, MSWaySize, "MSWay");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offGraph = reader.ReadInt32();
            if (offGraph != 0)
            {
                ReadGraph(nodeAddr, offGraph);
            }
        }
        catch { }
    }

    private void ReadGraph(int nodeAddr, int address)
    {
        if (_visitedGraphs.Contains(address)) return;
        _visitedGraphs.Add(address);

        // Graph structure: ~0x18 bytes (linked list of nodes + name pointers)
        const int GraphSize = 0x18;
        _tracker.RecordForNode(nodeAddr, address, GraphSize, "Graph");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            // Nodes linked list (double linked with header pointers)
            int nodesHead = reader.ReadInt32();
            int nodesTail = reader.ReadInt32();
            int nodesHdr = reader.ReadInt32();
            uint nodesCount = reader.ReadUInt32();

            if (nodesHead != 0 && nodesCount > 0 && nodesCount < 10000)
            {
                ReadGraphNodes(nodeAddr, nodesHead, nodesCount);
            }
        }
        catch { }
    }

    private void ReadGraphNodes(int nodeAddr, int headAddress, uint count)
    {
        int current = headAddress;
        for (uint i = 0; i < count && current != 0; i++)
        {
            // GraphNode structure: ~0x20 bytes
            const int GraphNodeSize = 0x20;
            _tracker.RecordForNode(nodeAddr, current, GraphNodeSize, "GraphNode");

            var reader = _memory.GetReaderAt(current);
            if (reader == null) break;

            int next = reader.ReadInt32();
            reader.ReadInt32(); // prev
            reader.ReadInt32(); // hdr
            int offWayPoint = reader.ReadInt32();

            if (offWayPoint != 0)
            {
                ReadWayPoint(nodeAddr, offWayPoint);
            }

            current = next;
        }
    }

    private void ReadWayPoint(int nodeAddr, int address)
    {
        if (_visitedWaypoints.Contains(address)) return;
        _visitedWaypoints.Add(address);

        // WayPoint structure (Montreal): ~0x18 bytes
        // +0x00: uint (Montreal padding)
        // +0x04: float x, y, z
        // +0x10: float radius
        // +0x14: off_perso_so
        const int WayPointSize = 0x18;
        _tracker.RecordForNode(nodeAddr, address, WayPointSize, "WayPoint");
    }

    private void ReadPersoSectorInfo(int nodeAddr, int address)
    {
        // PersoSectorInfo: ~0x10 bytes
        const int SectorInfoSize = 0x10;
        _tracker.RecordForNode(nodeAddr, address, SectorInfoSize, "PersoSectorInfo");
    }

    private void TrackSpawnablePersoDynam(int persoDataAddress)
    {
        var reader = _memory.GetReaderAt(persoDataAddress);
        if (reader == null)
        {
            return;
        }

        try
        {
            reader.ReadInt32();
            reader.ReadInt32();
            var offDynam = reader.ReadInt32();
            if (offDynam != 0)
            {
                ReadDynam(persoDataAddress, offDynam);
            }
        }
        catch { }
    }

    private void ReadDynam(int nodeAddr, int address)
    {
        // Dynam structure: ~0x80 bytes
        const int DynamSize = 0x80;
        _tracker.RecordForNode(nodeAddr, address, DynamSize, "Dynam");
    }

    private void ReadStandardGame(SceneNode node, int address)
    {
        // StandardGame structure: approximately 0x30 bytes
        const int StdGameSize = 0x30;
        _tracker.RecordForNode(node.Address, address, StdGameSize, "StandardGame");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        node.FamilyIndex = reader.ReadUInt32();
        node.ModelIndex = reader.ReadUInt32();
        node.InstanceIndex = reader.ReadUInt32();

        node.Name = $"Family{node.FamilyIndex}/Model{node.ModelIndex}/Instance{node.InstanceIndex}";
    }

    private string? ReadNullTerminatedString(int address, out int length)
    {
        length = 0;
        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        var chars = new List<char>();
        while (true)
        {
            byte b = reader.ReadByte();
            if (b == 0) break;
            chars.Add((char)b);
            if (chars.Count > 256) break;
        }

        length = chars.Count + 1; // Include null terminator
        return chars.Count > 0 ? new string(chars.ToArray()) : null;
    }

    private void ReadIPOData(SceneNode node)
    {
        // IPO structure: 8 bytes (2 pointers)
        const int IPOSize = 8;
        _tracker.RecordForNode(node.Address, node.OffData, IPOSize, "IPO");

        var reader = _memory.GetReaderAt(node.OffData);
        if (reader == null) return;

        int offPhysicalObject = reader.ReadInt32();
        int offRadiosity = reader.ReadInt32();

        if (offPhysicalObject != 0)
        {
            ReadPhysicalObjectData(node, offPhysicalObject);
        }

        if (offRadiosity != 0)
        {
            ReadRadiosityData(node.Address, offRadiosity);
        }
    }

    private void ReadRadiosityData(int nodeAddr, int address)
    {
        const int RadiosityHeaderSize = 0x10;
        _tracker.RecordForNode(nodeAddr, address, RadiosityHeaderSize, "RadiosityHeader");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offColors = reader.ReadInt32();
            uint numColors = reader.ReadUInt32();

            if (offColors != 0 && numColors > 0 && numColors < 100000)
            {
                int colorDataSize = (int)(numColors * 4);
                _tracker.RecordForNode(nodeAddr, offColors, colorDataSize, "VertexColors");
            }
        }
        catch { }
    }

    private void ReadPhysicalObjectData(SceneNode node, int address)
    {
        const int PhysicalObjectSize = 0x10;
        _tracker.RecordForNode(node.Address, address, PhysicalObjectSize, "PhysicalObject");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        int offVisualSet = reader.ReadInt32();
        int offCollideSet = reader.ReadInt32();
        int offVisualBoundingVolume = reader.ReadInt32();

        if (offVisualSet != 0)
        {
            ReadVisualSetData(node.Address, offVisualSet);
        }

        if (offCollideSet != 0)
        {
            node.OffCollideSet = offCollideSet;
            ReadCollideSetFull(node.Address, offCollideSet);
        }

        if (offVisualBoundingVolume != 0)
        {
            ReadBoundingVolume(node.Address, offVisualBoundingVolume);
        }
    }

    private void ReadVisualSetData(int nodeAddr, int address)
    {
        const int VisualSetSize = 0x10;
        _tracker.RecordForNode(nodeAddr, address, VisualSetSize, "VisualSet");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        reader.ReadUInt32(); // 0
        ushort numberOfLOD = reader.ReadUInt16();
        ushort visualSetType = reader.ReadUInt16();

        if (numberOfLOD > 0 && numberOfLOD < 10)
        {
            int offLODDistances = reader.ReadInt32();
            int offLODDataOffsets = reader.ReadInt32();

            if (offLODDistances != 0)
            {
                _tracker.RecordForNode(nodeAddr, offLODDistances, numberOfLOD * 4, "LODDistances");
            }

            if (offLODDataOffsets != 0)
            {
                _tracker.RecordForNode(nodeAddr, offLODDataOffsets, numberOfLOD * 4, "LODDataOffsets");

                for (int i = 0; i < numberOfLOD; i++)
                {
                    var lodReader = _memory.GetReaderAt(offLODDataOffsets + i * 4);
                    if (lodReader == null) continue;
                    int geoAddr = lodReader.ReadInt32();
                    if (geoAddr != 0)
                    {
                        ReadGeometricObjectFull(nodeAddr, geoAddr);
                    }
                }
            }
        }
    }

    private void ReadCollideSetFull(int nodeAddr, int address)
    {
        if (_visitedGeometricObjects.Contains(address + 0x10000000)) return;
        _visitedGeometricObjects.Add(address + 0x10000000);

        const int CollideSetSize = 0x14;
        _tracker.RecordForNode(nodeAddr, address, CollideSetSize, "CollideSet");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offZdxList = reader.ReadInt32();
            int offZddList = reader.ReadInt32();
            int offZdeList = reader.ReadInt32();

            if (offZdxList != 0)
            {
                ReadCollisionZoneList(nodeAddr, offZdxList, "CollideZDX");
            }
            if (offZddList != 0)
            {
                ReadCollisionZoneList(nodeAddr, offZddList, "CollideZDD");
            }
            if (offZdeList != 0)
            {
                ReadCollisionZoneList(nodeAddr, offZdeList, "CollideZDE");
            }
        }
        catch { }
    }

    private void ReadCollisionZoneList(int nodeAddr, int address, string label)
    {
        _tracker.RecordForNode(nodeAddr, address, 12, label + "List");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int head = reader.ReadInt32();
            int tail = reader.ReadInt32();
            uint count = reader.ReadUInt32();

            int current = head;
            for (uint i = 0; i < count && current != 0 && i < 1000; i++)
            {
                ReadCollisionZone(nodeAddr, current, label);

                var nextReader = _memory.GetReaderAt(current);
                if (nextReader == null) break;
                current = nextReader.ReadInt32();
            }
        }
        catch { }
    }

    private void ReadCollisionZone(int nodeAddr, int address, string label)
    {
        const int ZoneSize = 0x20;
        _tracker.RecordForNode(nodeAddr, address, ZoneSize, label + "Zone");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offNext = reader.ReadInt32();
            int offPrev = reader.ReadInt32();
            int offCollideObj = reader.ReadInt32();

            if (offCollideObj != 0)
            {
                ReadCollideObject(nodeAddr, offCollideObj);
            }
        }
        catch { }
    }

    private void ReadCollideObject(int nodeAddr, int address)
    {
        if (_visitedGeometricObjects.Contains(address)) return;
        _visitedGeometricObjects.Add(address);

        const int CollideObjHeaderSize = 0x30;
        _tracker.RecordForNode(nodeAddr, address, CollideObjHeaderSize, "CollideObject");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            uint numVertices = reader.ReadUInt32();
            int offVertices = reader.ReadInt32();
            reader.ReadInt32();
            uint numElements = reader.ReadUInt32();
            int offElementTypes = reader.ReadInt32();
            int offElements = reader.ReadInt32();

            if (numVertices > 0 && numVertices < 100000 && offVertices != 0)
            {
                int vertexDataSize = (int)(numVertices * 12);
                _tracker.RecordForNode(nodeAddr, offVertices, vertexDataSize, "CollideVertices");
            }

            if (numElements > 0 && numElements < 10000)
            {
                if (offElementTypes != 0)
                {
                    int typeDataSize = (int)(numElements * 2);
                    _tracker.RecordForNode(nodeAddr, offElementTypes, typeDataSize, "CollideElementTypes");
                }

                if (offElements != 0)
                {
                    int elemPtrSize = (int)(numElements * 4);
                    _tracker.RecordForNode(nodeAddr, offElements, elemPtrSize, "CollideElementPtrs");

                    for (int i = 0; i < numElements; i++)
                    {
                        var elemReader = _memory.GetReaderAt(offElements + i * 4);
                        if (elemReader == null) continue;
                        int elemAddr = elemReader.ReadInt32();
                        if (elemAddr != 0)
                        {
                            ReadCollideElement(nodeAddr, elemAddr);
                        }
                    }
                }
            }
        }
        catch { }
    }

    private void ReadCollideElement(int nodeAddr, int address)
    {
        const int CollideElemSize = 0x18;
        _tracker.RecordForNode(nodeAddr, address, CollideElemSize, "CollideElement");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offMaterial = reader.ReadInt32();
            ushort numTriangles = reader.ReadUInt16();
            reader.ReadUInt16();
            int offTriangles = reader.ReadInt32();

            if (offMaterial != 0)
            {
                ReadGameMaterial(nodeAddr, offMaterial);
            }

            if (numTriangles > 0 && offTriangles != 0)
            {
                int triDataSize = numTriangles * 6;
                _tracker.RecordForNode(nodeAddr, offTriangles, triDataSize, "CollideTriangles");
            }
        }
        catch { }
    }

    private void ReadGeometricObjectFull(int nodeAddr, int address)
    {
        if (_visitedGeometricObjects.Contains(address)) return;
        _visitedGeometricObjects.Add(address);

        const int GeoObjHeaderSize = 0x40;
        _tracker.RecordForNode(nodeAddr, address, GeoObjHeaderSize, "GeometricObject");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            uint numVertices = reader.ReadUInt32();
            int offVertices = reader.ReadInt32();
            int offNormals = reader.ReadInt32();
            int offMaterials = reader.ReadInt32();
            reader.ReadInt32();
            uint numElements = reader.ReadUInt32();
            int offElementTypes = reader.ReadInt32();
            int offElements = reader.ReadInt32();

            if (numVertices == 0 || numVertices > 100000) return;
            if (numElements == 0 || numElements > 10000) return;

            if (offVertices != 0)
            {
                int vertexDataSize = (int)(numVertices * 12);
                _tracker.RecordForNode(nodeAddr, offVertices, vertexDataSize, "Vertices");
            }

            if (offNormals != 0)
            {
                int normalDataSize = (int)(numVertices * 12);
                _tracker.RecordForNode(nodeAddr, offNormals, normalDataSize, "Normals");
            }

            if (offElementTypes != 0)
            {
                int typeDataSize = (int)(numElements * 2);
                _tracker.RecordForNode(nodeAddr, offElementTypes, typeDataSize, "ElementTypes");
            }

            if (offElements != 0)
            {
                int elemPtrSize = (int)(numElements * 4);
                _tracker.RecordForNode(nodeAddr, offElements, elemPtrSize, "ElementPtrs");

                ushort[]? elementTypes = null;
                if (offElementTypes != 0)
                {
                    elementTypes = new ushort[numElements];
                    var typeReader = _memory.GetReaderAt(offElementTypes);
                    if (typeReader != null)
                    {
                        for (int i = 0; i < numElements; i++)
                        {
                            elementTypes[i] = typeReader.ReadUInt16();
                        }
                    }
                }

                for (int i = 0; i < numElements; i++)
                {
                    var elemReader = _memory.GetReaderAt(offElements + i * 4);
                    if (elemReader == null) continue;
                    int elemAddr = elemReader.ReadInt32();
                    if (elemAddr == 0) continue;

                    ushort elemType = elementTypes?[i] ?? 1;
                    ReadGeometricElement(nodeAddr, elemAddr, elemType, numVertices);
                }
            }
        }
        catch { }
    }

    private void ReadGeometricElement(int nodeAddr, int address, ushort elementType, uint numVertices)
    {
        if (elementType == 1)
        {
            const int ElemTriSize = 0x28;
            _tracker.RecordForNode(nodeAddr, address, ElemTriSize, "ElementTriangles");

            var reader = _memory.GetReaderAt(address);
            if (reader == null) return;

            try
            {
                int offMaterial = reader.ReadInt32();
                ushort numTriangles = reader.ReadUInt16();
                ushort numUvs = reader.ReadUInt16();
                int offTriangles = reader.ReadInt32();
                int offMappingUvs = reader.ReadInt32();
                int offNormals = reader.ReadInt32();
                int offUvs = reader.ReadInt32();
                reader.ReadInt32();
                int offVertexIndices = reader.ReadInt32();
                ushort numVertexIndices = reader.ReadUInt16();

                if (offMaterial != 0)
                {
                    ReadGameMaterial(nodeAddr, offMaterial);
                }

                if (numTriangles > 0 && offTriangles != 0)
                {
                    _tracker.RecordForNode(nodeAddr, offTriangles, numTriangles * 6, "Triangles");
                }

                if (numTriangles > 0 && offMappingUvs != 0)
                {
                    _tracker.RecordForNode(nodeAddr, offMappingUvs, numTriangles * 6, "UVMapping");
                }

                if (numTriangles > 0 && offNormals != 0)
                {
                    _tracker.RecordForNode(nodeAddr, offNormals, numTriangles * 12, "TriangleNormals");
                }

                if (numUvs > 0 && offUvs != 0)
                {
                    _tracker.RecordForNode(nodeAddr, offUvs, numUvs * 8, "UVs");
                }

                if (numVertexIndices > 0 && offVertexIndices != 0)
                {
                    _tracker.RecordForNode(nodeAddr, offVertexIndices, numVertexIndices * 2, "VertexIndices");
                }
            }
            catch { }
        }
        else if (elementType == 3)
        {
            const int ElemSpriteSize = 0x20;
            _tracker.RecordForNode(nodeAddr, address, ElemSpriteSize, "ElementSprites");
        }
        else if (elementType == 13 || elementType == 15)
        {
            // DeformSet (bone/weight data for skinned meshes)
            ReadDeformSet(nodeAddr, address, numVertices);
        }
        else
        {
            _tracker.RecordForNode(nodeAddr, address, 0x10, $"Element_Type{elementType}");
        }
    }

    private void ReadDeformSet(int nodeAddr, int address, uint numVertices)
    {
        // DeformSet structure (R2/Montreal):
        // +0x00: off_weights (pointer)
        // +0x04: off_bones (pointer)
        // +0x08: num_weights (ushort)
        // +0x0A: num_bones (byte)
        // +0x0B: padding (byte)
        const int DeformSetHeaderSize = 0x0C;
        _tracker.RecordForNode(nodeAddr, address, DeformSetHeaderSize, "DeformSet");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offWeights = reader.ReadInt32();
            int offBones = reader.ReadInt32();
            ushort numWeights = reader.ReadUInt16();
            byte numBones = reader.ReadByte();
            numBones++; // Root bone is implicit

            // Read weights array
            // Each weight entry: pointer (4) + vertex_index (2) + num_weightsForVertex (1) + padding (1) = 8 bytes
            if (offWeights != 0 && numWeights > 0 && numWeights < 10000)
            {
                int weightsArraySize = numWeights * 8;
                _tracker.RecordForNode(nodeAddr, offWeights, weightsArraySize, "DeformWeights");

                // Each weight entry points to per-vertex weight data
                var weightsReader = _memory.GetReaderAt(offWeights);
                if (weightsReader != null)
                {
                    for (int i = 0; i < numWeights; i++)
                    {
                        int offWeightsForVertex = weightsReader.ReadInt32();
                        weightsReader.ReadUInt16(); // vertex_index
                        byte numWeightsForVertex = weightsReader.ReadByte();
                        weightsReader.ReadByte(); // padding

                        // Per-vertex weights: weight (2) + boneIndex (1) + padding (1) = 4 bytes each
                        if (offWeightsForVertex != 0 && numWeightsForVertex > 0)
                        {
                            int perVertexWeightsSize = numWeightsForVertex * 4;
                            _tracker.RecordForNode(nodeAddr, offWeightsForVertex, perVertexWeightsSize, "VertexWeights");
                        }
                    }
                }
            }

            // Read bones array
            // Each bone: 0x38 bytes (56 bytes) - matrix + additional data
            if (offBones != 0 && numBones > 0 && numBones < 256)
            {
                const int BoneSize = 0x38;
                int bonesArraySize = (numBones - 1) * BoneSize; // First bone is root (not stored)
                _tracker.RecordForNode(nodeAddr, offBones, bonesArraySize, "DeformBones");
            }
        }
        catch { }
    }

    private void ReadGameMaterial(int nodeAddr, int address)
    {
        if (_visitedMaterials.Contains(address)) return;
        _visitedMaterials.Add(address);

        const int GameMaterialSize = 0x10;
        _tracker.RecordForNode(nodeAddr, address, GameMaterialSize, "GameMaterial");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offVisualMaterial = reader.ReadInt32();
            int offMechanicsMaterial = reader.ReadInt32();
            uint soundMaterial = reader.ReadUInt32();
            int offCollideMaterial = reader.ReadInt32();

            if (offVisualMaterial != 0)
            {
                ReadVisualMaterial(nodeAddr, offVisualMaterial);
            }

            if (offCollideMaterial != 0 && offCollideMaterial != -1)
            {
                ReadCollideMaterial(nodeAddr, offCollideMaterial);
            }
        }
        catch { }
    }

    private void ReadVisualMaterial(int nodeAddr, int address)
    {
        if (_visitedMaterials.Contains(address)) return;
        _visitedMaterials.Add(address);

        const int VisualMaterialSize = 0x78;
        _tracker.RecordForNode(nodeAddr, address, VisualMaterialSize, "VisualMaterial");
    }

    private void ReadCollideMaterial(int nodeAddr, int address)
    {
        if (_visitedMaterials.Contains(address)) return;
        _visitedMaterials.Add(address);

        const int CollideMaterialSize = 8;
        _tracker.RecordForNode(nodeAddr, address, CollideMaterialSize, "CollideMaterial");
    }

    private void ReadBoundingVolume(int nodeAddr, int address)
    {
        const int BoundingVolumeSize = 0x1C;
        _tracker.RecordForNode(nodeAddr, address, BoundingVolumeSize, "BoundingVolume");
    }

    private Matrix4x4? ReadMatrix(int nodeAddr, int address)
    {
        const int MatrixSize = 88;
        _tracker.RecordForNode(nodeAddr, address, MatrixSize, "Matrix");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        uint type = reader.ReadUInt32();

        var m = Matrix4x4.Identity;

        m.M14 = reader.ReadSingle();
        m.M24 = reader.ReadSingle();
        m.M34 = reader.ReadSingle();

        m.M11 = reader.ReadSingle();
        m.M21 = reader.ReadSingle();
        m.M31 = reader.ReadSingle();

        m.M12 = reader.ReadSingle();
        m.M22 = reader.ReadSingle();
        m.M32 = reader.ReadSingle();

        m.M13 = reader.ReadSingle();
        m.M23 = reader.ReadSingle();
        m.M33 = reader.ReadSingle();

        return m;
    }

    // ==================== AI STRUCTURES ====================

    private void ReadBrain(int nodeAddr, int address)
    {
        if (_visitedAI.Contains(address)) return;
        _visitedAI.Add(address);

        const int BrainSize = 0x0C;
        _tracker.RecordForNode(nodeAddr, address, BrainSize, "Brain");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offMind = reader.ReadInt32();
            if (offMind != 0)
            {
                ReadMind(nodeAddr, offMind);
            }
        }
        catch { }
    }

    private void ReadMind(int nodeAddr, int address)
    {
        if (_visitedAI.Contains(address)) return;
        _visitedAI.Add(address);

        const int MindSize = 0x18;
        _tracker.RecordForNode(nodeAddr, address, MindSize, "Mind");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offAIModel = reader.ReadInt32();
            int offIntelNormal = reader.ReadInt32();
            int offIntelReflex = reader.ReadInt32();
            int offDsgMem = reader.ReadInt32();

            if (offAIModel != 0)
            {
                ReadAIModel(nodeAddr, offAIModel);
            }

            if (offIntelNormal != 0)
            {
                ReadIntelligence(nodeAddr, offIntelNormal);
            }

            if (offIntelReflex != 0)
            {
                ReadIntelligence(nodeAddr, offIntelReflex);
            }

            if (offDsgMem != 0)
            {
                ReadDsgMem(nodeAddr, offDsgMem);
            }
        }
        catch { }
    }

    private void ReadIntelligence(int nodeAddr, int address)
    {
        if (_visitedAI.Contains(address)) return;
        _visitedAI.Add(address);

        // Intelligence structure (R2/Montreal):
        // +0x00: off_aiModel (4)
        // +0x04: off_actionTree (4)
        // +0x08: off_comport (4)
        // +0x0C: off_lastComport (4)
        // +0x10: off_actionTable (4)
        // +0x14: off_defaultComport (4)
        const int IntelSize = 0x18;
        _tracker.RecordForNode(nodeAddr, address, IntelSize, "Intelligence");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offAIModel = reader.ReadInt32();
            int offActionTree = reader.ReadInt32();
            int offComport = reader.ReadInt32();
            int offLastComport = reader.ReadInt32();
            int offActionTable = reader.ReadInt32();
            int offDefaultComport = reader.ReadInt32();

            // Track action tree if present
            if (offActionTree != 0)
            {
                // Action tree is a hierarchy of actions, estimate size
                _tracker.RecordForNode(nodeAddr, offActionTree, 0x20, "ActionTree");
            }

            // Track action table if present
            if (offActionTable != 0)
            {
                // Action table - variable size array
                _tracker.RecordForNode(nodeAddr, offActionTable, 0x40, "ActionTable");
            }
        }
        catch { }
    }

    private void ReadDsgMem(int nodeAddr, int address)
    {
        if (_visitedAI.Contains(address)) return;
        _visitedAI.Add(address);

        // DsgMem structure:
        // +0x00: pointer to pointer (to DsgVar)
        // +0x04: memBufferInitial
        // +0x08: memBuffer
        const int DsgMemSize = 0x0C;
        _tracker.RecordForNode(nodeAddr, address, DsgMemSize, "DsgMem");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offDsgVarPtr = reader.ReadInt32();
            int offMemBufferInitial = reader.ReadInt32();
            int offMemBuffer = reader.ReadInt32();

            // Read the DsgVar pointer from the indirect pointer
            if (offDsgVarPtr != 0)
            {
                _tracker.RecordForNode(nodeAddr, offDsgVarPtr, 4, "DsgVarPtrIndirect");
                var ptrReader = _memory.GetReaderAt(offDsgVarPtr);
                if (ptrReader != null)
                {
                    int offDsgVar = ptrReader.ReadInt32();
                    if (offDsgVar != 0)
                    {
                        ReadDsgVar(nodeAddr, offDsgVar);
                    }
                }
            }

            // Estimate buffer sizes based on DsgVar (if already read)
            // For now, use a reasonable estimate
            if (offMemBufferInitial != 0)
            {
                _tracker.RecordForNode(nodeAddr, offMemBufferInitial, 0x100, "DsgMemBufferInitial");
            }

            if (offMemBuffer != 0)
            {
                _tracker.RecordForNode(nodeAddr, offMemBuffer, 0x100, "DsgMemBufferCurrent");
            }
        }
        catch { }
    }

    private void ReadAIModel(int nodeAddr, int address)
    {
        if (_visitedAI.Contains(address)) return;
        _visitedAI.Add(address);

        const int AIModelSize = 0x14;
        _tracker.RecordForNode(nodeAddr, address, AIModelSize, "AIModel");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offBehaviorsNormal = reader.ReadInt32();
            int offBehaviorsReflex = reader.ReadInt32();
            int offDsgVar = reader.ReadInt32();
            int offMacros = reader.ReadInt32();

            if (offBehaviorsNormal != 0)
            {
                ReadBehaviorList(nodeAddr, offBehaviorsNormal, "Normal");
            }

            if (offBehaviorsReflex != 0)
            {
                ReadBehaviorList(nodeAddr, offBehaviorsReflex, "Reflex");
            }

            if (offDsgVar != 0)
            {
                ReadDsgVar(nodeAddr, offDsgVar);
            }

            if (offMacros != 0)
            {
                ReadMacroList(nodeAddr, offMacros);
            }
        }
        catch { }
    }

    private void ReadDsgVar(int nodeAddr, int address)
    {
        if (_visitedAI.Contains(address)) return;
        _visitedAI.Add(address);

        const int DsgVarSize = 0x10;
        _tracker.RecordForNode(nodeAddr, address, DsgVarSize, "DsgVar");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offInfos = reader.ReadInt32();
            uint numInfos = reader.ReadUInt32();

            if (offInfos != 0 && numInfos > 0 && numInfos < 1000)
            {
                int infosSize = (int)(numInfos * 0x0C);
                _tracker.RecordForNode(nodeAddr, offInfos, infosSize, "DsgVarInfos");
            }
        }
        catch { }
    }

    private void ReadBehaviorList(int nodeAddr, int address, string prefix)
    {
        if (_visitedAI.Contains(address)) return;
        _visitedAI.Add(address);

        const int BehaviorListSize = 8;
        _tracker.RecordForNode(nodeAddr, address, BehaviorListSize, $"BehaviorList_{prefix}");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offEntries = reader.ReadInt32();
            uint numEntries = reader.ReadUInt32();

            if (offEntries == 0 || numEntries == 0 || numEntries > 100) return;

            const int BehaviorSize = 0x10;
            int totalSize = (int)(numEntries * BehaviorSize);
            _tracker.RecordForNode(nodeAddr, offEntries, totalSize, $"Behaviors_{prefix}");

            for (int i = 0; i < numEntries; i++)
            {
                int behaviorAddr = offEntries + i * BehaviorSize;
                ReadBehavior(nodeAddr, behaviorAddr);
            }
        }
        catch { }
    }

    private void ReadBehavior(int nodeAddr, int address)
    {
        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offScripts = reader.ReadInt32();
            int offScheduleScript = reader.ReadInt32();
            byte numScripts = reader.ReadByte();

            if (numScripts > 0 && numScripts < 50 && offScripts != 0)
            {
                if (TryReadScriptPointerEntries(offScripts, numScripts, out var scriptAddrs))
                {
                    _tracker.RecordForNode(nodeAddr, offScripts, scriptAddrs.Count * 4, "ScriptPtrs");

                    foreach (var scriptAddr in scriptAddrs)
                    {
                        if (scriptAddr != 0)
                        {
                            ReadScript(nodeAddr, scriptAddr);
                        }
                    }
                }
            }

            if (offScheduleScript != 0)
            {
                ReadScript(nodeAddr, offScheduleScript);
            }
        }
        catch { }
    }

    private bool TryReadScriptPointerEntries(int address, int count, out List<int> scriptAddrs)
    {
        scriptAddrs = [];
        var scriptsReader = _memory.GetReaderAt(address);
        if (scriptsReader == null) return false;

        try
        {
            for (int i = 0; i < count; i++)
            {
                int scriptAddr = scriptsReader.ReadInt32();
                if (scriptAddr == 0)
                {
                    scriptAddrs.Add(scriptAddr);
                    continue;
                }

                if (!TryGetScriptSize(scriptAddr, out _))
                {
                    break;
                }

                scriptAddrs.Add(scriptAddr);
            }
        }
        catch
        {
            return false;
        }

        return scriptAddrs.Count > 0 && scriptAddrs.Any(addr => addr != 0);
    }

    private void ReadMacroList(int nodeAddr, int address)
    {
        if (_visitedAI.Contains(address)) return;
        _visitedAI.Add(address);

        const int MacroListSize = 8;
        _tracker.RecordForNode(nodeAddr, address, MacroListSize, "MacroList");

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        try
        {
            int offEntries = reader.ReadInt32();
            byte numEntries = reader.ReadByte();

            if (offEntries == 0 || numEntries == 0 || numEntries > 100) return;

            const int MacroSize = 8;
            int totalSize = numEntries * MacroSize;
            _tracker.RecordForNode(nodeAddr, offEntries, totalSize, "Macros");

            for (int i = 0; i < numEntries; i++)
            {
                int macroAddr = offEntries + i * MacroSize;
                var macroReader = _memory.GetReaderAt(macroAddr);
                if (macroReader == null) continue;

                int offScript = macroReader.ReadInt32();
                if (offScript != 0)
                {
                    ReadScript(nodeAddr, offScript);
                }
            }
        }
        catch { }
    }

    private void ReadScript(int nodeAddr, int address)
    {
        if (_visitedAI.Contains(address)) return;
        _visitedAI.Add(address);

        if (TryGetScriptSize(address, out var scriptSize))
        {
            _tracker.RecordForNode(nodeAddr, address, scriptSize, "Script");
        }
    }

    private bool TryGetScriptSize(int address, out int scriptSize)
    {
        const int MaxNodes = 500;
        const int NodeSize = 8;
        scriptSize = 0;

        int nodeCount = 0;
        int pos = address;
        bool foundTerminator = false;

        while (nodeCount < MaxNodes)
        {
            var reader = _memory.GetReaderAt(pos);
            if (reader == null) return false;

            try
            {
                reader.ReadUInt32();
                reader.ReadUInt16();
                byte indent = reader.ReadByte();
                byte type = reader.ReadByte();

                if (type > 50 || indent > 30)
                {
                    return false;
                }

                nodeCount++;
                pos += NodeSize;

                if (indent == 0)
                {
                    foundTerminator = true;
                    break;
                }
            }
            catch
            {
                return false;
            }
        }

        if (nodeCount == 0 || !foundTerminator)
        {
            return false;
        }

        scriptSize = nodeCount * NodeSize;
        return true;
    }

    // ==================== END STRUCTURES ====================

    public static SuperObjectType GetSuperObjectType(uint typeCode)
    {
        return typeCode switch
        {
            0x0 => SuperObjectType.World,
            0x4 => SuperObjectType.Perso,
            0x8 => SuperObjectType.Sector,
            0xD => SuperObjectType.IPO,
            0x15 => SuperObjectType.IPO_2,
            _ => SuperObjectType.Unknown
        };
    }
}
