using System.Numerics;
using Astrolabe.Core.FileFormats.Materials;
using SharpGLTF.Geometry;
using SharpGLTF.Geometry.VertexTypes;
using SharpGLTF.Materials;
using SharpGLTF.Memory;
using SharpGLTF.Scenes;
using SixLabors.ImageSharp;
using SixLabors.ImageSharp.Formats.Png;

namespace Astrolabe.Core.FileFormats.Geometry;

/// <summary>
/// Exports mesh data to glTF format.
/// </summary>
public static class GltfExporter
{
    /// <summary>
    /// Exports a single mesh to a glTF file.
    /// </summary>
    public static void ExportMesh(MeshData mesh, string outputPath, string? texturePath = null)
    {
        ExportMeshes([mesh], outputPath, texturePath);
    }

    /// <summary>
    /// Exports a single mesh with texture lookup function for multi-material support.
    /// </summary>
    public static void ExportMesh(MeshData mesh, string outputPath, Func<string?, string?> textureLookup)
    {
        var scene = new SceneBuilder();

        if (mesh.Vertices.Length < 3) return;

        var meshBuilder = CreateMeshBuilderWithSubMeshes(mesh, textureLookup);
        scene.AddRigidMesh(meshBuilder, Matrix4x4.Identity);

        var model = scene.ToGltf2();
        model.SaveGLB(outputPath);
    }

    /// <summary>
    /// Exports multiple meshes to a single glTF file.
    /// </summary>
    public static void ExportMeshes(IEnumerable<MeshData> meshes, string outputPath, string? texturePath = null)
    {
        var scene = new SceneBuilder();
        var material = CreateMaterial(texturePath);

        foreach (var mesh in meshes)
        {
            if (mesh.Vertices.Length < 3) continue;

            var meshBuilder = CreateMeshBuilder(mesh, material);
            scene.AddRigidMesh(meshBuilder, Matrix4x4.Identity);
        }

        var model = scene.ToGltf2();
        model.SaveGLB(outputPath);
    }

    private static MaterialBuilder CreateMaterial(string? texturePath) =>
        CreateMaterial(texturePath, forceTransparent: false);

    private static MaterialBuilder CreateMaterial(string? texturePath, bool forceTransparent)
    {
        var material = new MaterialBuilder("default")
            .WithDoubleSide(true)
            .WithMetallicRoughnessShader()
            .WithMetallicRoughness(0.0f, 1.0f); // Non-metallic, rough surface

        if (!string.IsNullOrEmpty(texturePath) && File.Exists(texturePath))
        {
            try
            {
                byte[] imageBytes;
                bool hasAlpha = false;

                // Convert TGA to PNG if needed (GLTF only supports PNG/JPEG)
                if (texturePath.EndsWith(".tga", StringComparison.OrdinalIgnoreCase))
                {
                    using var img = Image.Load(texturePath);
                    // Check if image has alpha channel based on bits per pixel
                    // 32bpp = RGBA (has alpha), 24bpp = RGB (no alpha)
                    // AlphaRepresentation isn't reliably set for TGA files in ImageSharp
                    hasAlpha = img.PixelType.BitsPerPixel == 32;
                    using var ms = new MemoryStream();
                    img.Save(ms, new PngEncoder());
                    imageBytes = ms.ToArray();
                }
                else
                {
                    imageBytes = File.ReadAllBytes(texturePath);
                    // PNG files may have alpha
                    hasAlpha = texturePath.EndsWith(".png", StringComparison.OrdinalIgnoreCase);
                }

                var image = new MemoryImage(imageBytes);
                material.WithBaseColor(image);

                // Enable alpha blending if texture has alpha channel or material flags say so
                if (forceTransparent || hasAlpha)
                {
                    material.WithAlpha(SharpGLTF.Materials.AlphaMode.BLEND);
                }
            }
            catch
            {
                // Fall back to solid color if texture loading fails
                material.WithBaseColor(new Vector4(0.8f, 0.8f, 0.8f, 1.0f));
            }
        }
        else
        {
            material.WithBaseColor(new Vector4(0.8f, 0.8f, 0.8f, 1.0f));
        }

        return material;
    }

    /// <summary>
    /// Checks if the material should be transparent based on OpenSpace/Montreal engine flags.
    /// Based on raymap's VisualMaterial.IsTransparent implementation for pre-R3 engines.
    /// NOTE: Currently disabled because flag reading appears to give incorrect results.
    /// Transparency is instead determined by checking actual texture alpha.
    /// </summary>
    private static bool IsTransparentFromFlags(uint flags)
    {
        // Disabled for now - almost all materials have the flags set, which isn't correct
        // Need to investigate the material structure parsing further
        return false;

        // Pre-R3 (Montreal/Rayman 2) transparency flags
        // return (flags & 0x4000000) != 0;  // VisualMaterial transparency flag
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

    private static MeshBuilder<VertexPositionNormal, VertexTexture1, VertexEmpty> CreateMeshBuilderWithSubMeshes(
        MeshData mesh, Func<string?, string?> textureLookup)
    {
        var meshBuilder = new MeshBuilder<VertexPositionNormal, VertexTexture1, VertexEmpty>(mesh.Name);
        bool hasNormals = mesh.Normals != null && mesh.Normals.Length == mesh.Vertices.Length;

        // Cache materials by texture path + transparency flag combination
        var materialCache = new Dictionary<string, MaterialBuilder>();

        foreach (var subMesh in mesh.SubMeshes)
        {
            // Get or create material for this submesh's texture
            string? resolvedTexture = textureLookup(subMesh.TextureName);

            // Check if this material should be transparent based on engine flags or VisualMaterial
            bool isTransparent = IsTransparentFromFlags(subMesh.MaterialFlags) ||
                                 (subMesh.VisualMaterial?.IsTransparent ?? false);
            string materialKey = (resolvedTexture ?? "__default__") + (isTransparent ? "_transparent" : "");

            if (!materialCache.TryGetValue(materialKey, out var material))
            {
                material = CreateMaterialFromVisual(resolvedTexture, isTransparent, subMesh.VisualMaterial);
                materialCache[materialKey] = material;
            }

            var primitive = meshBuilder.UsePrimitive(material);
            bool hasUVs = subMesh.UVs.Length > 0 && subMesh.UVIndices.Length > 0;

            for (int i = 0; i < subMesh.Triangles.Length - 2; i += 3)
            {
                int i0 = subMesh.Triangles[i];
                int i1 = subMesh.Triangles[i + 1];
                int i2 = subMesh.Triangles[i + 2];

                if (i0 >= mesh.Vertices.Length || i1 >= mesh.Vertices.Length || i2 >= mesh.Vertices.Length)
                    continue;

                var v0 = mesh.Vertices[i0];
                var v1 = mesh.Vertices[i1];
                var v2 = mesh.Vertices[i2];

                var n0 = hasNormals ? SanitizeVector(mesh.Normals![i0]) : CalculateNormal(v0, v1, v2);
                var n1 = hasNormals ? SanitizeVector(mesh.Normals![i1]) : n0;
                var n2 = hasNormals ? SanitizeVector(mesh.Normals![i2]) : n0;

                var uv0 = GetSubMeshUV(subMesh, i, hasUVs);
                var uv1 = GetSubMeshUV(subMesh, i + 1, hasUVs);
                var uv2 = GetSubMeshUV(subMesh, i + 2, hasUVs);

                primitive.AddTriangle(
                    (new VertexPositionNormal(v0, n0), new VertexTexture1(uv0)),
                    (new VertexPositionNormal(v1, n1), new VertexTexture1(uv1)),
                    (new VertexPositionNormal(v2, n2), new VertexTexture1(uv2)));
            }
        }

        return meshBuilder;
    }

    /// <summary>
    /// Creates a mesh builder that includes vertex colors.
    /// </summary>
    private static MeshBuilder<VertexPositionNormal, VertexColor1Texture1, VertexEmpty> CreateMeshBuilderWithVertexColors(
        MeshData mesh, Func<string?, string?> textureLookup)
    {
        var meshBuilder = new MeshBuilder<VertexPositionNormal, VertexColor1Texture1, VertexEmpty>(mesh.Name);
        bool hasNormals = mesh.Normals != null && mesh.Normals.Length == mesh.Vertices.Length;
        bool hasVertexColors = mesh.VertexColors != null && mesh.VertexColors.Length == mesh.Vertices.Length;

        // Cache materials by texture path + transparency flag combination
        var materialCache = new Dictionary<string, MaterialBuilder>();

        foreach (var subMesh in mesh.SubMeshes)
        {
            // Get or create material for this submesh's texture
            string? resolvedTexture = textureLookup(subMesh.TextureName);

            // Check if this material should be transparent based on engine flags or VisualMaterial
            bool isTransparent = IsTransparentFromFlags(subMesh.MaterialFlags) ||
                                 (subMesh.VisualMaterial?.IsTransparent ?? false);
            string materialKey = (resolvedTexture ?? "__default__") + (isTransparent ? "_transparent" : "");

            if (!materialCache.TryGetValue(materialKey, out var material))
            {
                material = CreateMaterialFromVisual(resolvedTexture, isTransparent, subMesh.VisualMaterial);
                materialCache[materialKey] = material;
            }

            var primitive = meshBuilder.UsePrimitive(material);
            bool hasUVs = subMesh.UVs.Length > 0 && subMesh.UVIndices.Length > 0;

            for (int i = 0; i < subMesh.Triangles.Length - 2; i += 3)
            {
                int i0 = subMesh.Triangles[i];
                int i1 = subMesh.Triangles[i + 1];
                int i2 = subMesh.Triangles[i + 2];

                if (i0 >= mesh.Vertices.Length || i1 >= mesh.Vertices.Length || i2 >= mesh.Vertices.Length)
                    continue;

                var v0 = mesh.Vertices[i0];
                var v1 = mesh.Vertices[i1];
                var v2 = mesh.Vertices[i2];

                var n0 = hasNormals ? SanitizeVector(mesh.Normals![i0]) : CalculateNormal(v0, v1, v2);
                var n1 = hasNormals ? SanitizeVector(mesh.Normals![i1]) : n0;
                var n2 = hasNormals ? SanitizeVector(mesh.Normals![i2]) : n0;

                var uv0 = GetSubMeshUV(subMesh, i, hasUVs);
                var uv1 = GetSubMeshUV(subMesh, i + 1, hasUVs);
                var uv2 = GetSubMeshUV(subMesh, i + 2, hasUVs);

                var c0 = hasVertexColors ? mesh.VertexColors![i0] : Vector4.One;
                var c1 = hasVertexColors ? mesh.VertexColors![i1] : Vector4.One;
                var c2 = hasVertexColors ? mesh.VertexColors![i2] : Vector4.One;

                primitive.AddTriangle(
                    (new VertexPositionNormal(v0, n0), new VertexColor1Texture1(c0, uv0)),
                    (new VertexPositionNormal(v1, n1), new VertexColor1Texture1(c1, uv1)),
                    (new VertexPositionNormal(v2, n2), new VertexColor1Texture1(c2, uv2)));
            }
        }

        return meshBuilder;
    }

    /// <summary>
    /// Creates a material from VisualMaterial data.
    /// </summary>
    private static MaterialBuilder CreateMaterialFromVisual(string? texturePath, bool forceTransparent, VisualMaterial? visMat)
    {
        var material = new MaterialBuilder("material")
            .WithDoubleSide(true)
            .WithMetallicRoughnessShader()
            .WithMetallicRoughness(0.0f, 1.0f); // Non-metallic, rough surface

        // Apply base color from visual material if available
        if (visMat != null)
        {
            // Use diffuse coefficient as base color tint
            var diffuse = visMat.DiffuseCoef;
            if (diffuse.W > 0)
            {
                // The diffuse coefficient modulates the texture color
                // For now we just use it as a base color factor
                // material.WithBaseColor(new Vector4(diffuse.X, diffuse.Y, diffuse.Z, diffuse.W));
            }
        }

        if (!string.IsNullOrEmpty(texturePath) && File.Exists(texturePath))
        {
            try
            {
                byte[] imageBytes;
                bool hasAlpha = false;

                // Convert TGA to PNG if needed (GLTF only supports PNG/JPEG)
                if (texturePath.EndsWith(".tga", StringComparison.OrdinalIgnoreCase))
                {
                    using var img = Image.Load(texturePath);
                    hasAlpha = img.PixelType.BitsPerPixel == 32;
                    using var ms = new MemoryStream();
                    img.Save(ms, new PngEncoder());
                    imageBytes = ms.ToArray();
                }
                else
                {
                    imageBytes = File.ReadAllBytes(texturePath);
                    hasAlpha = texturePath.EndsWith(".png", StringComparison.OrdinalIgnoreCase);
                }

                var image = new MemoryImage(imageBytes);
                material.WithBaseColor(image);

                // Enable alpha blending if texture has alpha channel or material flags say so
                if (forceTransparent || hasAlpha)
                {
                    material.WithAlpha(SharpGLTF.Materials.AlphaMode.BLEND);
                }
            }
            catch
            {
                material.WithBaseColor(new Vector4(0.8f, 0.8f, 0.8f, 1.0f));
            }
        }
        else
        {
            material.WithBaseColor(new Vector4(0.8f, 0.8f, 0.8f, 1.0f));
        }

        return material;
    }

    private static Vector2 GetSubMeshUV(SubMeshData subMesh, int triangleVertexIndex, bool hasUVs)
    {
        if (!hasUVs || triangleVertexIndex >= subMesh.UVIndices.Length)
            return new Vector2(0, 0);

        int uvIndex = subMesh.UVIndices[triangleVertexIndex];
        if (uvIndex < 0 || uvIndex >= subMesh.UVs.Length)
            return new Vector2(0, 0);

        return subMesh.UVs[uvIndex];
    }

    private static MeshBuilder<VertexPositionNormal, VertexTexture1, VertexEmpty> CreateMeshBuilder(
        MeshData mesh, MaterialBuilder material)
    {
        var meshBuilder = new MeshBuilder<VertexPositionNormal, VertexTexture1, VertexEmpty>(mesh.Name);
        var primitive = meshBuilder.UsePrimitive(material);

        // If we have normals, use them; otherwise generate flat normals
        bool hasNormals = mesh.Normals != null && mesh.Normals.Length == mesh.Vertices.Length;

        // Check if we have UV data
        bool hasUVs = mesh.UVs != null && mesh.UVIndices != null && mesh.UVIndices.Length > 0;

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

                // Get UVs for this triangle (UVIndices maps each triangle vertex to a UV)
                var uv0 = GetUV(mesh, i, hasUVs);
                var uv1 = GetUV(mesh, i + 1, hasUVs);
                var uv2 = GetUV(mesh, i + 2, hasUVs);

                primitive.AddTriangle(
                    (new VertexPositionNormal(v0, n0), new VertexTexture1(uv0)),
                    (new VertexPositionNormal(v1, n1), new VertexTexture1(uv1)),
                    (new VertexPositionNormal(v2, n2), new VertexTexture1(uv2)));
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
                var uvCenter = new Vector2(0.5f, 0.5f);

                for (int i = 1; i < mesh.Vertices.Length - 1; i++)
                {
                    var v1 = mesh.Vertices[i];
                    var v2 = mesh.Vertices[i + 1];

                    var n = CalculateNormal(center, v1, v2);
                    var n1 = hasNormals ? SanitizeVector(mesh.Normals![i]) : n;
                    var n2 = hasNormals ? SanitizeVector(mesh.Normals![i + 1]) : n;

                    primitive.AddTriangle(
                        (new VertexPositionNormal(center, hasNormals ? nCenter : n), new VertexTexture1(uvCenter)),
                        (new VertexPositionNormal(v1, n1), new VertexTexture1(new Vector2(0, 0))),
                        (new VertexPositionNormal(v2, n2), new VertexTexture1(new Vector2(1, 0))));
                }
            }
        }

        return meshBuilder;
    }

    private static Vector2 GetUV(MeshData mesh, int triangleVertexIndex, bool hasUVs)
    {
        if (!hasUVs || mesh.UVIndices == null || mesh.UVs == null)
            return new Vector2(0, 0);

        if (triangleVertexIndex >= mesh.UVIndices.Length)
            return new Vector2(0, 0);

        int uvIndex = mesh.UVIndices[triangleVertexIndex];
        if (uvIndex < 0 || uvIndex >= mesh.UVs.Length)
            return new Vector2(0, 0);

        return mesh.UVs[uvIndex];
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
