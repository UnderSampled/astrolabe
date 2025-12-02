using System.Numerics;
using Astrolabe.Core.FileFormats.Materials;

namespace Astrolabe.Core.FileFormats;

/// <summary>
/// Reads SuperObject hierarchy from SNA memory blocks.
/// </summary>
public class SuperObjectReader
{
    private readonly MemoryContext _memory;
    private readonly HashSet<int> _visitedAddresses = new();
    private readonly GameMaterialReader _gameMaterialReader;

    public SuperObjectReader(MemoryContext memory)
    {
        _memory = memory;
        _gameMaterialReader = new GameMaterialReader(memory);
    }

    private SceneGraph? _currentGraph;

    /// <summary>
    /// Reads a complete scene graph starting from GPT entry points.
    /// </summary>
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

        _currentGraph = null;
        return graph;
    }

    /// <summary>
    /// Reads a SuperObject and its children recursively.
    /// </summary>
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

        // Add to global list
        _currentGraph?.AllNodes.Add(node);

        try
        {
            // SuperObject structure (Montreal engine):
            // +0x00: uint32 typeCode
            // +0x04: Pointer off_data
            // +0x08: LinkedList<SuperObject> children (head, tail, count for Double type)
            // +0x14: Pointer off_brother_next
            // +0x18: Pointer off_brother_prev
            // +0x1C: Pointer off_parent
            // +0x20: Pointer off_matrix
            // +0x24: Pointer off_staticMatrix
            // +0x28: int32 globalMatrix
            // +0x2C: uint32 drawFlags
            // +0x30: uint32 flags
            // +0x34: Pointer off_boundingVolume

            node.TypeCode = reader.ReadUInt32();
            node.Type = GetSuperObjectType(node.TypeCode);
            node.OffData = reader.ReadInt32();

            // LinkedList header (Double type: head, tail, count)
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

            // Read transform matrix if present
            if (node.OffMatrix != 0)
            {
                node.Transform = ReadMatrix(node.OffMatrix);
            }

            // Read data based on type
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

            // LinkedList entry for Double type with element pointers:
            // next pointer, prev pointer are in the SuperObject itself
            // The entry at headAddress IS the first SuperObject

            var child = ReadSuperObject(currentAddress, parent);
            if (child != null)
            {
                parent.Children.Add(child);

                // Get next sibling from the SuperObject's off_brother_next
                // We need to re-read since ReadSuperObject consumed the reader
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
                // off_data points directly to GeometricObject
                node.GeometricObjectAddress = node.OffData;
                break;

            case SuperObjectType.PhysicalObject:
                ReadPhysicalObjectData(node, node.OffData);
                break;

            // World and Sector don't have geometry directly
            case SuperObjectType.World:
            case SuperObjectType.Sector:
            case SuperObjectType.Perso:
            default:
                break;
        }
    }

    private void ReadIPOData(SceneNode node)
    {
        // IPO structure:
        // +0x00: Pointer off_data (-> PhysicalObject)
        // +0x04: Pointer off_radiosity

        var reader = _memory.GetReaderAt(node.OffData);
        if (reader == null) return;

        int offPhysicalObject = reader.ReadInt32();
        // int offRadiosity = reader.ReadInt32();

        if (offPhysicalObject != 0)
        {
            ReadPhysicalObjectData(node, offPhysicalObject);
        }
    }

    private void ReadPhysicalObjectData(SceneNode node, int address)
    {
        // PhysicalObject structure:
        // +0x00: Pointer off_visualSet
        // +0x04: Pointer off_collideSet
        // +0x08: Pointer off_visualBoundingVolume
        // +0x0C: uint32 (padding or additional field)

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        int offVisualSet = reader.ReadInt32();
        int offCollideSet = reader.ReadInt32();

        if (offVisualSet != 0)
        {
            ReadVisualSetData(node, offVisualSet);
        }

        // CollideSet address stored for future use
        if (offCollideSet != 0)
        {
            node.OffCollideSet = offCollideSet;
        }
    }

    private void ReadVisualSetData(SceneNode node, int address)
    {
        // VisualSet structure (Montreal):
        // +0x00: uint32 (0)
        // +0x04: uint16 numberOfLOD
        // +0x06: uint16 visualSetType
        // +0x08: Pointer off_LODDistances
        // +0x0C: Pointer off_LODDataOffsets

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return;

        reader.ReadUInt32(); // 0
        ushort numberOfLOD = reader.ReadUInt16();
        ushort visualSetType = reader.ReadUInt16();

        if (numberOfLOD > 0)
        {
            int offLODDistances = reader.ReadInt32();
            int offLODDataOffsets = reader.ReadInt32();

            // Read first LOD's GeometricObject pointer
            if (offLODDataOffsets != 0)
            {
                var lodReader = _memory.GetReaderAt(offLODDataOffsets);
                if (lodReader != null)
                {
                    node.GeometricObjectAddress = lodReader.ReadInt32();
                }
            }
        }
    }

    private Matrix4x4? ReadMatrix(int address)
    {
        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        // 4x4 matrix stored as 16 floats (row-major or column-major depending on engine)
        var m = new Matrix4x4();

        m.M11 = reader.ReadSingle();
        m.M12 = reader.ReadSingle();
        m.M13 = reader.ReadSingle();
        m.M14 = reader.ReadSingle();

        m.M21 = reader.ReadSingle();
        m.M22 = reader.ReadSingle();
        m.M23 = reader.ReadSingle();
        m.M24 = reader.ReadSingle();

        m.M31 = reader.ReadSingle();
        m.M32 = reader.ReadSingle();
        m.M33 = reader.ReadSingle();
        m.M34 = reader.ReadSingle();

        m.M41 = reader.ReadSingle();
        m.M42 = reader.ReadSingle();
        m.M43 = reader.ReadSingle();
        m.M44 = reader.ReadSingle();

        return m;
    }

    /// <summary>
    /// Gets the SuperObject type from a type code (Montreal engine).
    /// </summary>
    public static SuperObjectType GetSuperObjectType(uint typeCode)
    {
        // Montreal engine type codes
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
