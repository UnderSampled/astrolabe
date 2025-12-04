using System.Globalization;
using System.Numerics;
using System.Text;

namespace Astrolabe.Core.FileFormats.Godot;

/// <summary>
/// Exports OpenSpace scene graph to Godot TSCN format.
/// </summary>
public class GodotExporter
{
    private readonly Dictionary<int, string> _meshPaths = new();
    private readonly List<ExtResource> _extResources = new();
    private readonly StringBuilder _nodes = new();
    private int _extResourceId = 1;

    /// <summary>
    /// Exports a scene graph to Godot TSCN format.
    /// </summary>
    /// <param name="graph">The scene graph to export</param>
    /// <param name="outputPath">Path for the .tscn file</param>
    /// <param name="meshDirectory">Directory containing exported GLTF meshes (relative to tscn)</param>
    /// <param name="geometryAddressToMeshName">Maps GeometricObject addresses to mesh filenames</param>
    public void Export(SceneGraph graph, string outputPath, string meshDirectory,
        Dictionary<int, string> geometryAddressToMeshName)
    {
        _meshPaths.Clear();
        _extResources.Clear();
        _nodes.Clear();
        _extResourceId = 1;

        // Build mesh path lookup and register external resources
        foreach (var (address, meshName) in geometryAddressToMeshName)
        {
            var relativePath = $"{meshDirectory}/{meshName}.glb";
            var id = _extResourceId++;
            _extResources.Add(new ExtResource
            {
                Id = id,
                Type = "PackedScene",
                Path = relativePath
            });
            _meshPaths[address] = $"ExtResource(\"{id}\")";
        }

        // Export the scene tree
        string rootName = "Level";
        if (graph.ActualWorld != null)
        {
            ExportNode(graph.ActualWorld, null, rootName);
        }

        // Write the TSCN file
        WriteTscn(outputPath, rootName);
    }

    private void ExportNode(SceneNode node, string? parentPath, string nodeName)
    {
        // Sanitize node name for Godot
        nodeName = SanitizeName(nodeName);
        string currentPath = parentPath == null ? "." : $"{parentPath}/{nodeName}";

        // Determine node type based on SuperObject type
        string godotType = node.Type switch
        {
            SuperObjectType.World => "Node3D",
            SuperObjectType.Sector => "Node3D",
            SuperObjectType.IPO or SuperObjectType.IPO_2 => "Node3D",
            SuperObjectType.Perso => "Node3D",
            SuperObjectType.PhysicalObject => "Node3D",
            SuperObjectType.GeometricObject => "Node3D",
            _ => "Node3D"
        };

        // Start node definition
        if (parentPath == null)
        {
            // Root node
            _nodes.AppendLine($"[node name=\"{nodeName}\" type=\"{godotType}\"]");
        }
        else
        {
            _nodes.AppendLine($"[node name=\"{nodeName}\" type=\"{godotType}\" parent=\"{parentPath}\"]");
        }

        // Apply transform if present and valid
        if (node.Transform.HasValue && IsValidTransform(node.Transform.Value))
        {
            WriteTransform(_nodes, node.Transform.Value);
        }

        // Add metadata for unimplemented features
        WriteNodeMetadata(_nodes, node);

        _nodes.AppendLine();

        // If this node has geometry, add a child MeshInstance3D that references the GLTF
        if (node.GeometricObjectAddress != 0 && _meshPaths.TryGetValue(node.GeometricObjectAddress, out var meshRef))
        {
            string meshNodeName = "Mesh";
            string meshPath = currentPath == "." ? meshNodeName : $"{currentPath}/{meshNodeName}";

            // Find the ExtResource ID from the reference
            var match = System.Text.RegularExpressions.Regex.Match(meshRef, @"ExtResource\(""(\d+)""\)");
            if (match.Success)
            {
                int resId = int.Parse(match.Groups[1].Value);

                // Add an instantiated scene node for the GLTF
                _nodes.AppendLine($"[node name=\"{meshNodeName}\" parent=\"{currentPath}\" instance=ExtResource(\"{resId}\")]");
                _nodes.AppendLine();
            }
        }

        // Export children
        int childIndex = 0;
        foreach (var child in node.Children)
        {
            string childName = GetNodeName(child, childIndex);
            ExportNode(child, currentPath, childName);
            childIndex++;
        }
    }

    private static string GetNodeName(SceneNode node, int index)
    {
        if (!string.IsNullOrEmpty(node.Name))
            return node.Name;

        return node.Type switch
        {
            SuperObjectType.World => "World",
            SuperObjectType.Sector => $"Sector_{index}",
            SuperObjectType.IPO => $"IPO_{node.Address:X8}",
            SuperObjectType.IPO_2 => $"IPO2_{node.Address:X8}",
            SuperObjectType.Perso => $"Perso_{index}",
            SuperObjectType.PhysicalObject => $"Physical_{index}",
            SuperObjectType.GeometricObject => $"Geometry_{index}",
            _ => $"Node_{index}"
        };
    }

    private static string SanitizeName(string name)
    {
        // Godot node names can't contain certain characters
        return name
            .Replace("/", "_")
            .Replace("\\", "_")
            .Replace(":", "_")
            .Replace(".", "_")
            .Replace(" ", "_");
    }

    private static bool IsValidTransform(Matrix4x4 m)
    {
        // Check if we have any meaningful transform data
        bool hasTranslation = Math.Abs(m.M14) > 0.01f || Math.Abs(m.M24) > 0.01f || Math.Abs(m.M34) > 0.01f;
        bool isNotIdentity = Math.Abs(m.M11 - 1) > 0.01f || Math.Abs(m.M22 - 1) > 0.01f || Math.Abs(m.M33 - 1) > 0.01f ||
                             Math.Abs(m.M12) > 0.01f || Math.Abs(m.M13) > 0.01f || Math.Abs(m.M21) > 0.01f ||
                             Math.Abs(m.M23) > 0.01f || Math.Abs(m.M31) > 0.01f || Math.Abs(m.M32) > 0.01f;

        return hasTranslation || isNotIdentity;
    }

    private static void WriteTransform(StringBuilder sb, Matrix4x4 m)
    {
        // Godot 4 uses Transform3D which is a 3x4 matrix (basis + origin)
        // Our matrix is column-major (from raymap format):
        // - Column 0 (M11, M21, M31) = X basis vector
        // - Column 1 (M12, M22, M32) = Y basis vector
        // - Column 2 (M13, M23, M33) = Z basis vector
        // - Column 3 (M14, M24, M34) = origin/translation

        // Extract basis vectors (columns of the rotation part)
        var basisX = new Vector3(m.M11, m.M21, m.M31);
        var basisY = new Vector3(m.M12, m.M22, m.M32);
        var basisZ = new Vector3(m.M13, m.M23, m.M33);
        var origin = new Vector3(m.M14, m.M24, m.M34);

        // Format: Transform3D(xx, xy, xz, yx, yy, yz, zx, zy, zz, ox, oy, oz)
        sb.AppendLine(string.Format(CultureInfo.InvariantCulture,
            "transform = Transform3D({0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}, {8}, {9}, {10}, {11})",
            basisX.X, basisX.Y, basisX.Z,
            basisY.X, basisY.Y, basisY.Z,
            basisZ.X, basisZ.Y, basisZ.Z,
            origin.X, origin.Y, origin.Z));
    }

    private static void WriteNodeMetadata(StringBuilder sb, SceneNode node)
    {
        // Store OpenSpace addresses and type info as metadata
        sb.AppendLine($"metadata/openspace_address = {node.Address}");
        sb.AppendLine($"metadata/openspace_type = \"{node.Type}\"");
        sb.AppendLine($"metadata/openspace_type_code = {node.TypeCode}");

        if (node.GeometricObjectAddress != 0)
            sb.AppendLine($"metadata/geometry_address = {node.GeometricObjectAddress}");

        if (node.OffCollideSet != 0)
            sb.AppendLine($"metadata/collide_set_address = {node.OffCollideSet}");

        if (node.Flags != 0)
            sb.AppendLine($"metadata/flags = {node.Flags}");

        if (node.DrawFlags != 0)
            sb.AppendLine($"metadata/draw_flags = {node.DrawFlags}");
    }

    private void WriteTscn(string outputPath, string rootName)
    {
        // Use UTF8 without BOM - Godot doesn't like BOM
        using var writer = new StreamWriter(outputPath, false, new UTF8Encoding(false));

        // TSCN header
        writer.WriteLine("[gd_scene load_steps={0} format=3]", _extResources.Count + 1);
        writer.WriteLine();

        // External resources (GLTF meshes)
        foreach (var res in _extResources)
        {
            writer.WriteLine("[ext_resource type=\"{0}\" path=\"res://{1}\" id=\"{2}\"]",
                res.Type, res.Path, res.Id);
        }

        if (_extResources.Count > 0)
            writer.WriteLine();

        // Node tree
        writer.Write(_nodes.ToString());
    }

    /// <summary>
    /// Writes a Godot project file for the exported scene.
    /// </summary>
    /// <param name="projectDir">Directory containing the project</param>
    /// <param name="projectName">Name of the project</param>
    /// <param name="mainScenePath">Path to main scene relative to project (e.g., "level.tscn")</param>
    public static void WriteProjectFile(string projectDir, string projectName, string mainScenePath)
    {
        var projectPath = Path.Combine(projectDir, "project.godot");
        // Use UTF8 without BOM - Godot doesn't like BOM
        using var writer = new StreamWriter(projectPath, false, new UTF8Encoding(false));

        writer.WriteLine("; Engine configuration file.");
        writer.WriteLine("; Generated by Astrolabe");
        writer.WriteLine();
        writer.WriteLine("config_version=5");
        writer.WriteLine();
        writer.WriteLine("[application]");
        writer.WriteLine();
        writer.WriteLine($"config/name=\"{projectName}\"");
        writer.WriteLine("config/features=PackedStringArray(\"4.3\", \"GL Compatibility\")");
        writer.WriteLine($"run/main_scene=\"res://{mainScenePath}\"");
        writer.WriteLine();
        writer.WriteLine("[rendering]");
        writer.WriteLine();
        writer.WriteLine("renderer/rendering_method=\"gl_compatibility\"");
        writer.WriteLine("renderer/rendering_method.mobile=\"gl_compatibility\"");
    }

    private class ExtResource
    {
        public int Id { get; set; }
        public string Type { get; set; } = "";
        public string Path { get; set; } = "";
    }
}
