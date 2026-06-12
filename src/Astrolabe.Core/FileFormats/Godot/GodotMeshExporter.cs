using System.Globalization;
using System.Numerics;
using System.Text;
using Astrolabe.Core.FileFormats.Geometry;

namespace Astrolabe.Core.FileFormats.Godot;

/// <summary>
/// Exports OpenSpace mesh data as Godot-native ArrayMesh resources.
/// </summary>
public static class GodotMeshExporter
{
    private const ulong ArrayFormatVertex = 1UL << 0;
    private const ulong ArrayFormatNormal = 1UL << 1;
    private const ulong ArrayFormatTangent = 1UL << 2;
    private const ulong ArrayFormatTexUv = 1UL << 4;
    private const ulong ArrayFlagFormatCurrentVersion = 1UL << 35;
    private const ulong SurfaceFormat = ArrayFlagFormatCurrentVersion
                                      | ArrayFormatVertex
                                      | ArrayFormatNormal
                                      | ArrayFormatTangent
                                      | ArrayFormatTexUv;

    public static void ExportMesh(
        MeshData mesh,
        string outputPath,
        Func<string?, string?> textureResourceLookup)
    {
        var surfaces = BuildSurfaces(mesh, textureResourceLookup).ToList();
        var materialIds = surfaces
            .Select(s => s.Material)
            .DistinctBy(m => m.Key)
            .Select((m, index) => m with { ResourceId = $"StandardMaterial3D_{index}" })
            .ToDictionary(m => m.Key, m => m);

        foreach (var surface in surfaces)
        {
            surface.Material = materialIds[surface.Material.Key];
        }

        var textureIds = materialIds.Values
            .Where(m => !string.IsNullOrEmpty(m.TextureResourcePath))
            .Select(m => m.TextureResourcePath!)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .Select((path, index) => new { Path = path, Id = $"Texture2D_{index}" })
            .ToDictionary(t => t.Path, t => t.Id, StringComparer.OrdinalIgnoreCase);

        Directory.CreateDirectory(Path.GetDirectoryName(outputPath) ?? ".");

        using var writer = new StreamWriter(outputPath, false, new UTF8Encoding(false));
        var loadSteps = 1 + materialIds.Count + textureIds.Count;
        writer.WriteLine($"[gd_resource type=\"ArrayMesh\" load_steps={loadSteps} format=3]");
        writer.WriteLine();

        foreach (var (texturePath, id) in textureIds)
        {
            writer.WriteLine($"[ext_resource type=\"Texture2D\" path=\"{EscapeString(texturePath)}\" id=\"{id}\"]");
        }

        if (textureIds.Count > 0)
        {
            writer.WriteLine();
        }

        foreach (var material in materialIds.Values.OrderBy(m => m.ResourceId, StringComparer.Ordinal))
        {
            writer.WriteLine($"[sub_resource type=\"StandardMaterial3D\" id=\"{material.ResourceId}\"]");
            writer.WriteLine($"resource_name = \"{EscapeString(material.Name)}\"");
            writer.WriteLine("albedo_color = Color(0.8, 0.8, 0.8, 1)");

            if (!string.IsNullOrEmpty(material.TextureResourcePath))
            {
                writer.WriteLine($"albedo_texture = ExtResource(\"{textureIds[material.TextureResourcePath!]}\")");
            }

            if (material.IsTransparent)
            {
                writer.WriteLine("transparency = 1");
                writer.WriteLine("alpha_scissor_threshold = 0.5");
            }

            if (material.IsLight)
            {
                writer.WriteLine("emission_enabled = true");
                writer.WriteLine("emission = Color(1, 1, 1, 1)");
            }

            writer.WriteLine();
        }

        writer.WriteLine("[resource]");
        writer.WriteLine($"resource_name = \"{EscapeString(mesh.Name)}\"");
        writer.WriteLine("_surfaces = [");

        for (var i = 0; i < surfaces.Count; i++)
        {
            WriteSurface(writer, surfaces[i], i == surfaces.Count - 1);
        }

        writer.WriteLine("]");
    }

    private static IEnumerable<MeshSurface> BuildSurfaces(
        MeshData mesh,
        Func<string?, string?> textureResourceLookup)
    {
        if (mesh.SubMeshes.Count > 0)
        {
            var subMeshIndex = 0;
            foreach (var subMesh in mesh.SubMeshes)
            {
                var vertices = BuildSubMeshVertices(mesh, subMesh);
                if (vertices.Count == 0)
                {
                    subMeshIndex++;
                    continue;
                }

                yield return new MeshSurface(
                    $"surface_{subMeshIndex}",
                    vertices,
                    CreateMaterial(subMesh.TextureName, subMesh.MaterialFlags, subMesh.IsLight,
                        subMesh.VisualMaterial?.IsTransparent ?? false, textureResourceLookup));
                subMeshIndex++;
            }

            yield break;
        }

        if (mesh.Indices is { Length: >= 3 })
        {
            var vertices = BuildIndexedVertices(mesh, mesh.Indices);
            if (vertices.Count > 0)
            {
                yield return new MeshSurface(
                    "surface_0",
                    vertices,
                    CreateMaterial(mesh.TextureName, 0, isLight: false, isTransparent: false, textureResourceLookup));
            }
        }
    }

    private static List<MeshVertex> BuildSubMeshVertices(MeshData mesh, SubMeshData subMesh)
    {
        var vertices = new List<MeshVertex>(subMesh.Triangles.Length);
        var hasNormals = mesh.Normals != null && mesh.Normals.Length == mesh.Vertices.Length;
        var hasUVs = subMesh.UVs.Length > 0 && subMesh.UVIndices.Length > 0;

        for (var i = 0; i < subMesh.Triangles.Length - 2; i += 3)
        {
            var i0 = subMesh.Triangles[i];
            var i1 = subMesh.Triangles[i + 1];
            var i2 = subMesh.Triangles[i + 2];

            if (!IsValidVertexIndex(mesh, i0) || !IsValidVertexIndex(mesh, i1) || !IsValidVertexIndex(mesh, i2))
            {
                continue;
            }

            var v0 = mesh.Vertices[i0];
            var v1 = mesh.Vertices[i1];
            var v2 = mesh.Vertices[i2];
            var faceNormal = CalculateNormal(v0, v1, v2);

            vertices.Add(new MeshVertex(v0, GetNormal(mesh, i0, faceNormal, hasNormals), GetSubMeshUV(subMesh, i, hasUVs)));
            vertices.Add(new MeshVertex(v1, GetNormal(mesh, i1, faceNormal, hasNormals), GetSubMeshUV(subMesh, i + 1, hasUVs)));
            vertices.Add(new MeshVertex(v2, GetNormal(mesh, i2, faceNormal, hasNormals), GetSubMeshUV(subMesh, i + 2, hasUVs)));
        }

        return vertices;
    }

    private static List<MeshVertex> BuildIndexedVertices(MeshData mesh, int[] indices)
    {
        var vertices = new List<MeshVertex>(indices.Length);
        var hasNormals = mesh.Normals != null && mesh.Normals.Length == mesh.Vertices.Length;
        var hasUVs = mesh.UVs != null && mesh.UVIndices != null && mesh.UVIndices.Length > 0;

        for (var i = 0; i < indices.Length - 2; i += 3)
        {
            var i0 = indices[i];
            var i1 = indices[i + 1];
            var i2 = indices[i + 2];

            if (!IsValidVertexIndex(mesh, i0) || !IsValidVertexIndex(mesh, i1) || !IsValidVertexIndex(mesh, i2))
            {
                continue;
            }

            var v0 = mesh.Vertices[i0];
            var v1 = mesh.Vertices[i1];
            var v2 = mesh.Vertices[i2];
            var faceNormal = CalculateNormal(v0, v1, v2);

            vertices.Add(new MeshVertex(v0, GetNormal(mesh, i0, faceNormal, hasNormals), GetMeshUV(mesh, i, hasUVs)));
            vertices.Add(new MeshVertex(v1, GetNormal(mesh, i1, faceNormal, hasNormals), GetMeshUV(mesh, i + 1, hasUVs)));
            vertices.Add(new MeshVertex(v2, GetNormal(mesh, i2, faceNormal, hasNormals), GetMeshUV(mesh, i + 2, hasUVs)));
        }

        return vertices;
    }

    private static MaterialResource CreateMaterial(
        string? textureName,
        uint materialFlags,
        bool isLight,
        bool isTransparent,
        Func<string?, string?> textureResourceLookup)
    {
        var texturePath = textureResourceLookup(textureName);
        var transparent = isTransparent || IsTransparentFromFlags(materialFlags);
        var key = string.Join('|', texturePath ?? "", transparent, isLight);

        return new MaterialResource(
            key,
            string.IsNullOrEmpty(textureName) ? "material" : Path.GetFileNameWithoutExtension(textureName),
            texturePath,
            transparent,
            isLight,
            "");
    }

    private static void WriteSurface(StreamWriter writer, MeshSurface surface, bool isLast)
    {
        var (position, size) = CalculateAabb(surface.Vertices);
        var vertexData = BuildVertexData(surface.Vertices);
        var attributeData = BuildAttributeData(surface.Vertices);

        writer.WriteLine("{");
        writer.WriteLine($"\"aabb\": AABB({Format(position.X)}, {Format(position.Y)}, {Format(position.Z)}, {Format(size.X)}, {Format(size.Y)}, {Format(size.Z)}),");
        writer.WriteLine($"\"format\": {SurfaceFormat},");
        writer.WriteLine("\"primitive\": 3,");
        writer.Write("\"vertex_data\": ");
        WritePackedByteArray(writer, vertexData);
        writer.WriteLine(",");
        writer.WriteLine($"\"vertex_count\": {surface.Vertices.Count},");
        writer.Write("\"attribute_data\": ");
        WritePackedByteArray(writer, attributeData);
        writer.WriteLine(",");
        writer.WriteLine($"\"material\": SubResource(\"{surface.Material.ResourceId}\"),");
        writer.WriteLine($"\"name\": \"{EscapeString(surface.Name)}\"");
        writer.WriteLine(isLast ? "}" : "},");
    }

    private static byte[] BuildVertexData(IReadOnlyList<MeshVertex> vertices)
    {
        using var stream = new MemoryStream();
        using var writer = new BinaryWriter(stream, Encoding.UTF8, leaveOpen: true);

        foreach (var vertex in vertices)
        {
            writer.Write(vertex.Position.X);
            writer.Write(vertex.Position.Y);
            writer.Write(vertex.Position.Z);
        }

        foreach (var vertex in vertices)
        {
            writer.Write(PackOctahedron(vertex.Normal));
            writer.Write(PackOctahedronTangent(CreateTangent(vertex.Normal), 1.0f));
        }

        writer.Flush();
        return stream.ToArray();
    }

    private static byte[] BuildAttributeData(IReadOnlyList<MeshVertex> vertices)
    {
        using var stream = new MemoryStream();
        using var writer = new BinaryWriter(stream, Encoding.UTF8, leaveOpen: true);

        foreach (var vertex in vertices)
        {
            writer.Write(vertex.UV.X);
            writer.Write(vertex.UV.Y);
        }

        writer.Flush();
        return stream.ToArray();
    }

    private static void WritePackedByteArray(StreamWriter writer, byte[] bytes)
    {
        writer.Write("PackedByteArray(");
        for (var i = 0; i < bytes.Length; i++)
        {
            if (i > 0)
            {
                writer.Write(", ");
            }

            writer.Write(bytes[i].ToString(CultureInfo.InvariantCulture));
        }
        writer.Write(")");
    }

    private static (Vector3 Position, Vector3 Size) CalculateAabb(IReadOnlyList<MeshVertex> vertices)
    {
        var min = new Vector3(float.MaxValue, float.MaxValue, float.MaxValue);
        var max = new Vector3(float.MinValue, float.MinValue, float.MinValue);

        foreach (var vertex in vertices)
        {
            min = Vector3.Min(min, vertex.Position);
            max = Vector3.Max(max, vertex.Position);
        }

        var size = max - min;
        size.X = MathF.Max(size.X, 0.00001f);
        size.Y = MathF.Max(size.Y, 0.00001f);
        size.Z = MathF.Max(size.Z, 0.00001f);
        return (min, size);
    }

    private static bool IsValidVertexIndex(MeshData mesh, int index) =>
        index >= 0 && index < mesh.Vertices.Length;

    private static Vector3 GetNormal(MeshData mesh, int index, Vector3 fallback, bool hasNormals)
    {
        if (!hasNormals || mesh.Normals == null)
        {
            return fallback;
        }

        return SanitizeNormal(mesh.Normals[index], fallback);
    }

    private static Vector2 GetSubMeshUV(SubMeshData subMesh, int triangleVertexIndex, bool hasUVs)
    {
        if (!hasUVs || triangleVertexIndex >= subMesh.UVIndices.Length)
        {
            return Vector2.Zero;
        }

        var uvIndex = subMesh.UVIndices[triangleVertexIndex];
        if (uvIndex < 0 || uvIndex >= subMesh.UVs.Length)
        {
            return Vector2.Zero;
        }

        return subMesh.UVs[uvIndex];
    }

    private static Vector2 GetMeshUV(MeshData mesh, int triangleVertexIndex, bool hasUVs)
    {
        if (!hasUVs || mesh.UVs == null || mesh.UVIndices == null || triangleVertexIndex >= mesh.UVIndices.Length)
        {
            return Vector2.Zero;
        }

        var uvIndex = mesh.UVIndices[triangleVertexIndex];
        if (uvIndex < 0 || uvIndex >= mesh.UVs.Length)
        {
            return Vector2.Zero;
        }

        return mesh.UVs[uvIndex];
    }

    private static Vector3 CalculateNormal(Vector3 v0, Vector3 v1, Vector3 v2)
    {
        var cross = Vector3.Cross(v1 - v0, v2 - v0);
        return SanitizeNormal(cross, Vector3.UnitY);
    }

    private static Vector3 SanitizeNormal(Vector3 normal, Vector3 fallback)
    {
        if (!IsFinite(normal) || normal.LengthSquared() < 0.000001f)
        {
            normal = fallback;
        }

        if (!IsFinite(normal) || normal.LengthSquared() < 0.000001f)
        {
            return Vector3.UnitY;
        }

        return Vector3.Normalize(normal);
    }

    private static Vector3 CreateTangent(Vector3 normal)
    {
        var tangent = Vector3.Cross(new Vector3(normal.Z, -normal.X, normal.Y), normal);
        if (!IsFinite(tangent) || tangent.LengthSquared() < 0.000001f)
        {
            tangent = Vector3.Cross(Vector3.UnitX, normal);
        }
        if (!IsFinite(tangent) || tangent.LengthSquared() < 0.000001f)
        {
            tangent = Vector3.UnitX;
        }

        return Vector3.Normalize(tangent);
    }

    private static uint PackOctahedron(Vector3 normal)
    {
        var encoded = OctahedronEncode(normal);
        return PackUnit(encoded.X) | ((uint)PackUnit(encoded.Y) << 16);
    }

    private static uint PackOctahedronTangent(Vector3 tangent, float sign)
    {
        const float bias = 1.0f / 32767.0f;
        var encoded = OctahedronEncode(tangent);
        encoded.Y = MathF.Max(encoded.Y, bias);
        encoded.Y = encoded.Y * 0.5f + 0.5f;
        if (sign < 0.0f)
        {
            encoded.Y = 1.0f - encoded.Y;
        }

        return PackUnit(encoded.X) | ((uint)PackUnit(encoded.Y) << 16);
    }

    private static Vector2 OctahedronEncode(Vector3 vector)
    {
        var denominator = MathF.Abs(vector.X) + MathF.Abs(vector.Y) + MathF.Abs(vector.Z);
        if (denominator < 0.000001f)
        {
            return new Vector2(0.5f, 0.5f);
        }

        var n = vector / denominator;
        Vector2 encoded;

        if (n.Z >= 0.0f)
        {
            encoded = new Vector2(n.X, n.Y);
        }
        else
        {
            encoded = new Vector2(
                (1.0f - MathF.Abs(n.Y)) * (n.X >= 0.0f ? 1.0f : -1.0f),
                (1.0f - MathF.Abs(n.X)) * (n.Y >= 0.0f ? 1.0f : -1.0f));
        }

        return encoded * 0.5f + new Vector2(0.5f, 0.5f);
    }

    private static uint PackUnit(float value)
    {
        var clamped = Math.Clamp(value, 0.0f, 1.0f);
        return (uint)Math.Clamp((int)(clamped * 65535.0f), 0, 65535);
    }

    private static bool IsTransparentFromFlags(uint flags)
    {
        return false;
    }

    private static bool IsFinite(Vector3 value) =>
        float.IsFinite(value.X) && float.IsFinite(value.Y) && float.IsFinite(value.Z);

    private static string Format(float value) =>
        value.ToString("G9", CultureInfo.InvariantCulture);

    private static string EscapeString(string value) =>
        value.Replace("\\", "\\\\").Replace("\"", "\\\"");

    private sealed class MeshSurface
    {
        public MeshSurface(string name, List<MeshVertex> vertices, MaterialResource material)
        {
            Name = name;
            Vertices = vertices;
            Material = material;
        }

        public string Name { get; }
        public List<MeshVertex> Vertices { get; }
        public MaterialResource Material { get; set; }
    }

    private readonly record struct MeshVertex(Vector3 Position, Vector3 Normal, Vector2 UV);

    private sealed record MaterialResource(
        string Key,
        string Name,
        string? TextureResourcePath,
        bool IsTransparent,
        bool IsLight,
        string ResourceId);
}
