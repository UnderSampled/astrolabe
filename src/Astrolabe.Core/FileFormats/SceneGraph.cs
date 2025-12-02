using System.Numerics;

namespace Astrolabe.Core.FileFormats;

/// <summary>
/// Types of SuperObjects in the scene graph.
/// </summary>
public enum SuperObjectType
{
    Unknown = 0,
    World = 1,
    IPO = 2,          // Instantiated Physical Object
    IPO_2 = 3,
    Perso = 4,        // Character/Actor
    Sector = 5,
    PhysicalObject = 6,
    GeometricObject = 7,
    GeometricShadowObject = 8
}

/// <summary>
/// A node in the scene graph hierarchy.
/// </summary>
public class SceneNode
{
    public int Address { get; set; }
    public SuperObjectType Type { get; set; }
    public uint TypeCode { get; set; }
    public string? Name { get; set; }

    // Pointers
    public int OffData { get; set; }
    public int OffMatrix { get; set; }
    public int OffStaticMatrix { get; set; }
    public int OffBoundingVolume { get; set; }

    // Transform
    public Matrix4x4? Transform { get; set; }

    // Flags
    public uint DrawFlags { get; set; }
    public uint Flags { get; set; }

    // Hierarchy
    public List<SceneNode> Children { get; set; } = new();
    public SceneNode? Parent { get; set; }

    // Data (depends on type)
    public int GeometricObjectAddress { get; set; }
    public int OffCollideSet { get; set; }

    public override string ToString()
    {
        return $"{Type} @ 0x{Address:X8}" + (Name != null ? $" ({Name})" : "");
    }
}

/// <summary>
/// Complete scene graph extracted from level data.
/// </summary>
public class SceneGraph
{
    public SceneNode? ActualWorld { get; set; }
    public SceneNode? DynamicWorld { get; set; }
    public SceneNode? FatherSector { get; set; }

    public List<SceneNode> AllNodes { get; } = new();

    /// <summary>
    /// Gets all nodes of a specific type.
    /// </summary>
    public IEnumerable<SceneNode> GetNodesOfType(SuperObjectType type)
    {
        return AllNodes.Where(n => n.Type == type);
    }

    /// <summary>
    /// Gets all nodes that have geometry (IPO, GeometricObject types).
    /// </summary>
    public IEnumerable<SceneNode> GetGeometryNodes()
    {
        return AllNodes.Where(n =>
            n.Type == SuperObjectType.IPO ||
            n.Type == SuperObjectType.IPO_2 ||
            n.Type == SuperObjectType.GeometricObject ||
            n.Type == SuperObjectType.PhysicalObject);
    }

    /// <summary>
    /// Prints the scene hierarchy to a text writer.
    /// </summary>
    public void PrintHierarchy(TextWriter writer, SceneNode? node = null, int indent = 0)
    {
        node ??= ActualWorld;
        if (node == null) return;

        var prefix = new string(' ', indent * 2);
        writer.WriteLine($"{prefix}{node}");

        foreach (var child in node.Children)
        {
            PrintHierarchy(writer, child, indent + 1);
        }
    }
}
