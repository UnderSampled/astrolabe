using System.Numerics;
using SharpGLTF.Geometry;
using SharpGLTF.Geometry.VertexTypes;
using SharpGLTF.Materials;
using SharpGLTF.Scenes;

namespace Astrolabe.Core.FileFormats.Geometry;

/// <summary>
/// Exports mesh data to glTF format.
/// </summary>
public static class GltfExporter
{
    /// <summary>
    /// Exports a single mesh to a glTF file.
    /// </summary>
    public static void ExportMesh(MeshData mesh, string outputPath)
    {
        ExportMeshes([mesh], outputPath);
    }

    /// <summary>
    /// Exports multiple meshes to a single glTF file.
    /// </summary>
    public static void ExportMeshes(IEnumerable<MeshData> meshes, string outputPath)
    {
        var scene = new SceneBuilder();
        var material = new MaterialBuilder("default")
            .WithDoubleSide(true)
            .WithMetallicRoughnessShader()
            .WithBaseColor(new Vector4(0.8f, 0.8f, 0.8f, 1.0f));

        foreach (var mesh in meshes)
        {
            if (mesh.Vertices.Length < 3) continue;

            var meshBuilder = CreateMeshBuilder(mesh, material);
            scene.AddRigidMesh(meshBuilder, Matrix4x4.Identity);
        }

        var model = scene.ToGltf2();
        model.SaveGLB(outputPath);
    }

    /// <summary>
    /// Exports meshes as point clouds (no triangles, just vertices as points).
    /// </summary>
    public static void ExportAsPointCloud(IEnumerable<MeshData> meshes, string outputPath)
    {
        var scene = new SceneBuilder();
        var material = new MaterialBuilder("points")
            .WithUnlitShader()
            .WithBaseColor(new Vector4(1.0f, 0.5f, 0.0f, 1.0f));

        foreach (var mesh in meshes)
        {
            if (mesh.Vertices.Length < 1) continue;

            // Create a mesh with points by making tiny triangles at each vertex
            var meshBuilder = new MeshBuilder<VertexPosition, VertexEmpty, VertexEmpty>(mesh.Name);
            var primitive = meshBuilder.UsePrimitive(material);

            // Create a small point indicator at each vertex
            float pointSize = 0.1f;
            foreach (var vertex in mesh.Vertices)
            {
                // Create a tiny triangle to represent the point
                var v1 = new VertexPosition(vertex);
                var v2 = new VertexPosition(vertex + new Vector3(pointSize, 0, 0));
                var v3 = new VertexPosition(vertex + new Vector3(0, pointSize, 0));
                primitive.AddTriangle(v1, v2, v3);
            }

            scene.AddRigidMesh(meshBuilder, Matrix4x4.Identity);
        }

        var model = scene.ToGltf2();
        model.SaveGLB(outputPath);
    }

    private static MeshBuilder<VertexPositionNormal, VertexEmpty, VertexEmpty> CreateMeshBuilder(
        MeshData mesh, MaterialBuilder material)
    {
        var meshBuilder = new MeshBuilder<VertexPositionNormal, VertexEmpty, VertexEmpty>(mesh.Name);
        var primitive = meshBuilder.UsePrimitive(material);

        // If we have normals, use them; otherwise generate flat normals
        bool hasNormals = mesh.Normals != null && mesh.Normals.Length == mesh.Vertices.Length;

        // If we have explicit indices, use them
        if (mesh.Indices != null && mesh.Indices.Length >= 3)
        {
            for (int i = 0; i < mesh.Indices.Length - 2; i += 3)
            {
                int i0 = mesh.Indices[i];
                int i1 = mesh.Indices[i + 1];
                int i2 = mesh.Indices[i + 2];

                if (i0 >= mesh.Vertices.Length || i1 >= mesh.Vertices.Length || i2 >= mesh.Vertices.Length)
                    continue;

                var v0 = mesh.Vertices[i0];
                var v1 = mesh.Vertices[i1];
                var v2 = mesh.Vertices[i2];

                var n0 = hasNormals ? SanitizeVector(mesh.Normals![i0]) : CalculateNormal(v0, v1, v2);
                var n1 = hasNormals ? SanitizeVector(mesh.Normals![i1]) : n0;
                var n2 = hasNormals ? SanitizeVector(mesh.Normals![i2]) : n0;

                primitive.AddTriangle(
                    new VertexPositionNormal(v0, n0),
                    new VertexPositionNormal(v1, n1),
                    new VertexPositionNormal(v2, n2));
            }
        }
        else
        {
            // No indices - create a triangle fan or strip from the vertices
            // This is a simple heuristic that may not be correct for all meshes
            if (mesh.Vertices.Length >= 3)
            {
                // Try triangle fan from first vertex
                var center = mesh.Vertices[0];
                var nCenter = hasNormals ? SanitizeVector(mesh.Normals![0]) : Vector3.UnitY;

                for (int i = 1; i < mesh.Vertices.Length - 1; i++)
                {
                    var v1 = mesh.Vertices[i];
                    var v2 = mesh.Vertices[i + 1];

                    var n = CalculateNormal(center, v1, v2);
                    var n1 = hasNormals ? SanitizeVector(mesh.Normals![i]) : n;
                    var n2 = hasNormals ? SanitizeVector(mesh.Normals![i + 1]) : n;

                    primitive.AddTriangle(
                        new VertexPositionNormal(center, hasNormals ? nCenter : n),
                        new VertexPositionNormal(v1, n1),
                        new VertexPositionNormal(v2, n2));
                }
            }
        }

        return meshBuilder;
    }

    private static Vector3 CalculateNormal(Vector3 v0, Vector3 v1, Vector3 v2)
    {
        var edge1 = v1 - v0;
        var edge2 = v2 - v0;
        var normal = Vector3.Cross(edge1, edge2);

        if (normal.LengthSquared() > 0.0001f)
            normal = Vector3.Normalize(normal);
        else
            normal = Vector3.UnitY;

        return SanitizeVector(normal);
    }

    private static Vector3 SanitizeVector(Vector3 v)
    {
        float x = float.IsFinite(v.X) ? v.X : 0;
        float y = float.IsFinite(v.Y) ? v.Y : 1;
        float z = float.IsFinite(v.Z) ? v.Z : 0;

        // Ensure it's normalized
        var result = new Vector3(x, y, z);
        if (result.LengthSquared() > 0.0001f)
            return Vector3.Normalize(result);
        return Vector3.UnitY;
    }
}
