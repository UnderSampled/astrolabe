using System.Numerics;
using Astrolabe.Core.FileFormats.Animation;
using Astrolabe.Core.FileFormats.Materials;
using SharpGLTF.Geometry;
using SharpGLTF.Geometry.VertexTypes;
using SharpGLTF.Materials;
using SharpGLTF.Memory;
using SharpGLTF.Schema2;
using SharpGLTF.Scenes;
using ImageSharpImage = SixLabors.ImageSharp.Image;
using SixLabors.ImageSharp.Formats.Png;

namespace Astrolabe.Core.FileFormats.Geometry;

/// <summary>
/// Exports Family data (character meshes + animations) to GLTF format.
/// </summary>
public class FamilyExporter
{
    private readonly MemoryContext _memory;
    private readonly TextureTable? _textureTable;
    private readonly string? _textureBasePath;
    private readonly MeshScanner _meshScanner;
    private readonly GameMaterialReader _gameMaterialReader;

    public FamilyExporter(LevelLoader level, TextureTable? textureTable = null, string? textureBasePath = null)
    {
        _memory = new MemoryContext(level.Sna, level.Rtb);
        _textureTable = textureTable;
        _textureBasePath = textureBasePath;
        _meshScanner = new MeshScanner(level, textureTable);
        _gameMaterialReader = new GameMaterialReader(_memory);
    }

    /// <summary>
    /// Exports a Family to a GLTF file with all ObjectList meshes and animations.
    /// </summary>
    public void ExportFamily(Family family, string outputPath, Func<string?, string?>? textureLookup = null)
    {
        // Reset debug counters per family
        _loggedTextureDebug = false;
        _matDebugCount = 0;
        _elemDebugCount = 0;
        string familyDebugName = family.Name ?? $"Family_{family.FamilyIndex}";
        Console.WriteLine($"[{familyDebugName}] Starting export...");

        var scene = new SceneBuilder($"Family_{family.FamilyIndex}");

        // Create root node for the family
        var rootNode = new NodeBuilder($"Family_{family.Name ?? family.FamilyIndex.ToString()}");

        // Get the primary ObjectList (first one)
        var primaryObjectList = family.ObjectLists.FirstOrDefault();
        if (primaryObjectList == null || primaryObjectList.Entries.Count == 0)
        {
            Console.WriteLine($"Family {family.Name} has no ObjectLists");
            return;
        }

        // Get initial transforms from first frame of first animation
        AnimFrameMontreal? initialFrame = null;
        if (family.States.Count > 0 && family.States[0].Animation != null)
        {
            var firstAnim = family.States[0].Animation!;
            if (firstAnim.Frames.Length > 0)
            {
                initialFrame = firstAnim.Frames[0];
            }
        }

        // Create nodes for each channel (mesh part)
        var channelNodes = new List<NodeBuilder>();
        var channelMeshes = new List<MeshBuilder<VertexPositionNormal, VertexTexture1, VertexEmpty>?>();

        // First pass: create channel nodes with transforms (don't add to root yet)
        for (int i = 0; i < primaryObjectList.Entries.Count; i++)
        {
            var entry = primaryObjectList.Entries[i];
            var channelNode = new NodeBuilder($"Channel_{i}");

            // Apply initial transform from first animation frame
            if (initialFrame != null && initialFrame.Channels != null && i < initialFrame.Channels.Length)
            {
                var channel = initialFrame.Channels[i];
                if (channel.Matrix != null)
                {
                    var scale = channel.Matrix.Scale;
                    var rotation = channel.Matrix.Rotation;
                    var translation = channel.Matrix.Position;

                    if (i < 3)
                    {
                        Console.WriteLine($"    Channel_{i}: pos={translation}, rot={rotation}, scale={scale}");
                    }

                    if (entry.Scale.HasValue)
                    {
                        scale *= entry.Scale.Value;
                    }

                    var transform = Matrix4x4.CreateScale(scale) *
                                   Matrix4x4.CreateFromQuaternion(rotation) *
                                   Matrix4x4.CreateTranslation(translation);
                    channelNode.LocalTransform = transform;
                }
                else if (entry.Scale.HasValue)
                {
                    channelNode.LocalTransform = Matrix4x4.CreateScale(entry.Scale.Value);
                }
            }
            else if (entry.Scale.HasValue)
            {
                channelNode.LocalTransform = Matrix4x4.CreateScale(entry.Scale.Value);
            }

            channelNodes.Add(channelNode);

            // Read mesh for this channel
            MeshBuilder<VertexPositionNormal, VertexTexture1, VertexEmpty>? meshBuilder = null;
            if (entry.GeometricObjectAddress != 0)
            {
                var meshData = ReadGeometricObject(entry.GeometricObjectAddress);
                if (meshData != null)
                {
                    meshBuilder = CreateMeshBuilder(meshData, $"Mesh_{i}", textureLookup);
                }
            }
            channelMeshes.Add(meshBuilder);
        }

        // Build hierarchy from first frame
        var childToParent = new Dictionary<int, int>();
        if (initialFrame != null && initialFrame.Hierarchies.Length > 0)
        {
            foreach (var h in initialFrame.Hierarchies)
            {
                if (h.ChildChannelId >= 0 && h.ChildChannelId < channelNodes.Count &&
                    h.ParentChannelId >= 0 && h.ParentChannelId < channelNodes.Count &&
                    h.ChildChannelId != h.ParentChannelId)
                {
                    childToParent[h.ChildChannelId] = h.ParentChannelId;
                }
            }
        }

        // Build scene tree: add root channels and their children recursively
        void AddNodeTree(NodeBuilder parent, int channelId, HashSet<int> added)
        {
            if (added.Contains(channelId)) return;
            added.Add(channelId);

            var node = channelNodes[channelId];
            parent.AddNode(node);

            // Add mesh to node
            var mesh = channelMeshes[channelId];
            if (mesh != null)
            {
                scene.AddRigidMesh(mesh, node);
            }

            // Add children
            for (int childId = 0; childId < channelNodes.Count; childId++)
            {
                if (childToParent.TryGetValue(childId, out var parentId) && parentId == channelId)
                {
                    AddNodeTree(node, childId, added);
                }
            }
        }

        var addedNodes = new HashSet<int>();

        // Find and add root channels (those without parents)
        for (int i = 0; i < channelNodes.Count; i++)
        {
            if (!childToParent.ContainsKey(i))
            {
                AddNodeTree(rootNode, i, addedNodes);
            }
        }

        // Add any remaining orphan nodes directly to root
        for (int i = 0; i < channelNodes.Count; i++)
        {
            if (!addedNodes.Contains(i))
            {
                addedNodes.Add(i);
                rootNode.AddNode(channelNodes[i]);
                var mesh = channelMeshes[i];
                if (mesh != null)
                {
                    scene.AddRigidMesh(mesh, channelNodes[i]);
                }
            }
        }

        // Add root to scene
        scene.AddNode(rootNode);

        // Build the model
        var model = scene.ToGltf2();

        // Add animations
        foreach (var state in family.States)
        {
            if (state.Animation != null && state.Animation.NumFrames > 1)
            {
                try
                {
                    AddAnimation(model, state, channelNodes);
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"  Warning: Failed to add animation {state.Name ?? $"State_{state.Index}"}: {ex.Message}");
                }
            }
        }

        // Save
        Console.Out.Flush();
        model.SaveGLB(outputPath);
        Console.Out.Flush();
        Console.WriteLine($"Exported Family {family.Name} to {outputPath}");
        Console.WriteLine($"  - {primaryObjectList.Entries.Count} mesh parts");
        Console.WriteLine($"  - {family.States.Count} animations");
    }

    /// <summary>
    /// Exports all Families found in a level to separate GLTF files.
    /// </summary>
    public void ExportAllFamilies(IEnumerable<Family> families, string outputDir, Func<string?, string?>? textureLookup = null)
    {
        Directory.CreateDirectory(outputDir);

        foreach (var family in families)
        {
            string safeName = family.Name ?? $"Family_{family.FamilyIndex}";
            safeName = string.Join("_", safeName.Split(Path.GetInvalidFileNameChars()));

            string outputPath = Path.Combine(outputDir, $"{safeName}.glb");

            try
            {
                ExportFamily(family, outputPath, textureLookup);
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Failed to export {safeName}: {ex.Message}");
            }
        }
    }

    /// <summary>
    /// Applies hierarchy relationships to nodes by rebuilding parent-child structure.
    /// </summary>
    private void ApplyHierarchy(List<NodeBuilder> channelNodes, AnimHierarchy[] hierarchies, NodeBuilder rootNode)
    {
        // Build parent-child relationships
        var childToParent = new Dictionary<int, int>();

        foreach (var h in hierarchies)
        {
            if (h.ChildChannelId >= 0 && h.ChildChannelId < channelNodes.Count &&
                h.ParentChannelId >= 0 && h.ParentChannelId < channelNodes.Count &&
                h.ChildChannelId != h.ParentChannelId) // Avoid self-parenting
            {
                childToParent[h.ChildChannelId] = h.ParentChannelId;
            }
        }

        // Find root channels (those without parents or with -1 parent)
        var rootChannels = new HashSet<int>();
        for (int i = 0; i < channelNodes.Count; i++)
        {
            if (!childToParent.ContainsKey(i))
            {
                rootChannels.Add(i);
            }
        }

        // Clear existing children from root
        // (NodeBuilder doesn't have RemoveNode, so we rebuild)
        var newRootNode = new NodeBuilder(rootNode.Name);
        newRootNode.LocalTransform = rootNode.LocalTransform;

        // Build tree recursively
        void AddNodeWithChildren(NodeBuilder parent, int channelId)
        {
            var node = channelNodes[channelId];
            parent.AddNode(node);

            // Add children
            foreach (var (childId, parentId) in childToParent)
            {
                if (parentId == channelId)
                {
                    AddNodeWithChildren(node, childId);
                }
            }
        }

        // Add root channels to new root
        foreach (var rootChannelId in rootChannels)
        {
            AddNodeWithChildren(newRootNode, rootChannelId);
        }

        // Also add any orphan channels that weren't in hierarchy at all
        var allInHierarchy = new HashSet<int>(childToParent.Keys);
        allInHierarchy.UnionWith(childToParent.Values);
        allInHierarchy.UnionWith(rootChannels);

        for (int i = 0; i < channelNodes.Count; i++)
        {
            if (!allInHierarchy.Contains(i))
            {
                newRootNode.AddNode(channelNodes[i]);
            }
        }
    }

    /// <summary>
    /// Adds an animation to the GLTF model.
    /// </summary>
    private void AddAnimation(ModelRoot model, State state, List<NodeBuilder> channelNodes)
    {
        var anim = state.Animation;
        if (anim == null || anim.NumFrames == 0) return;

        string animName = state.Name ?? $"State_{state.Index}";

        // Calculate frame duration (speed is frames per second, typically 30)
        float fps = anim.Speed > 0 ? anim.Speed : 30f;
        float frameDuration = 1f / fps;

        // First, collect all animation data to see if we have any valid channels
        var animationData = new List<(Node targetNode, Dictionary<float, Vector3> translations,
            Dictionary<float, Quaternion> rotations, Dictionary<float, Vector3> scales)>();

        // For each channel, collect animation tracks
        for (int channelIdx = 0; channelIdx < anim.NumChannels && channelIdx < channelNodes.Count; channelIdx++)
        {
            // Find the corresponding node in the model
            var nodeName = channelNodes[channelIdx].Name;
            var targetNode = model.LogicalNodes.FirstOrDefault(n => n.Name == nodeName);
            if (targetNode == null) continue;

            // Collect keyframes for this channel
            var translations = new List<Vector3>();
            var rotations = new List<Quaternion>();
            var scales = new List<Vector3>();
            var times = new List<float>();

            for (int frameIdx = 0; frameIdx < anim.NumFrames; frameIdx++)
            {
                var frame = anim.Frames[frameIdx];
                if (frame.Channels == null || channelIdx >= frame.Channels.Length)
                    continue;

                var channel = frame.Channels[channelIdx];
                float time = frameIdx * frameDuration;

                times.Add(time);

                if (channel.Matrix != null)
                {
                    translations.Add(channel.Matrix.Position);
                    rotations.Add(channel.Matrix.Rotation);
                    scales.Add(channel.Matrix.Scale);
                }
                else
                {
                    // Identity transform
                    translations.Add(Vector3.Zero);
                    rotations.Add(Quaternion.Identity);
                    scales.Add(Vector3.One);
                }
            }

            if (times.Count < 2) continue; // Need at least 2 keyframes

            // Build keyframe dictionaries
            var translationKeyframes = new Dictionary<float, Vector3>();
            var rotationKeyframes = new Dictionary<float, Quaternion>();
            var scaleKeyframes = new Dictionary<float, Vector3>();

            for (int i = 0; i < times.Count; i++)
            {
                float t = times[i];
                // Avoid duplicate keys by using slightly different times
                while (translationKeyframes.ContainsKey(t)) t += 0.0001f;

                translationKeyframes[t] = translations[i];
                rotationKeyframes[t] = rotations[i];
                scaleKeyframes[t] = scales[i];
            }

            // Check if any channel has non-trivial animation
            bool hasTranslation = translations.Any(tr => tr.Length() > 0.001f);
            bool hasRotation = rotations.Any(r => Math.Abs(r.W - 1f) > 0.001f || r.X != 0 || r.Y != 0 || r.Z != 0);
            bool hasScale = scales.Any(s => Math.Abs(s.X - 1f) > 0.001f || Math.Abs(s.Y - 1f) > 0.001f || Math.Abs(s.Z - 1f) > 0.001f);

            if (hasTranslation || hasRotation || hasScale)
            {
                animationData.Add((targetNode, translationKeyframes, rotationKeyframes, scaleKeyframes));
            }
        }

        // Only create animation if we have valid data
        if (animationData.Count == 0) return;

        var gltfAnim = model.CreateAnimation(animName);

        foreach (var (targetNode, translationKeyframes, rotationKeyframes, scaleKeyframes) in animationData)
        {
            try
            {
                // Translation track
                if (translationKeyframes.Values.Any(t => t != Vector3.Zero))
                {
                    gltfAnim.CreateTranslationChannel(targetNode, translationKeyframes, linear: true);
                }

                // Rotation track
                if (rotationKeyframes.Values.Any(r => r != Quaternion.Identity))
                {
                    gltfAnim.CreateRotationChannel(targetNode, rotationKeyframes, linear: true);
                }

                // Scale track
                if (scaleKeyframes.Values.Any(s => s != Vector3.One))
                {
                    gltfAnim.CreateScaleChannel(targetNode, scaleKeyframes, linear: true);
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Warning: Failed to create animation channel for {targetNode.Name}: {ex.Message}");
            }
        }

        // Note: Empty animations are avoided by checking animationData.Count before creating
    }

    /// <summary>
    /// Reads a GeometricObject at the given address and returns mesh data.
    /// </summary>
    private MeshData? ReadGeometricObject(int address)
    {
        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            // GeometricObject structure:
            // uint32 num_vertices
            // ptr off_vertices
            // ptr off_normals
            // ptr off_materials
            // int32 unknown
            // uint32 num_elements
            // ptr off_element_types
            // ptr off_elements
            // ...

            uint numVertices = reader.ReadUInt32();
            if (numVertices < 3 || numVertices > 10000) return null;

            int offVerts = reader.ReadInt32();
            int offNormals = reader.ReadInt32();
            int offMaterials = reader.ReadInt32();
            reader.ReadInt32(); // skip
            uint numElements = reader.ReadUInt32();

            if (numElements == 0 || numElements > 1000) return null;

            int offElementTypes = reader.ReadInt32();
            int offElements = reader.ReadInt32();

            // Read vertices
            var vertices = ReadVertices(offVerts, numVertices);
            if (vertices == null) return null;

            // Read normals
            var normals = ReadNormals(offNormals, numVertices);

            // Read element types
            var elementTypes = ReadElementTypes(offElementTypes, numElements);
            if (elementTypes == null) return null;

            // Read elements and build submeshes
            var subMeshes = new List<SubMeshData>();
            var allTriangles = new List<int>();

            for (int i = 0; i < numElements; i++)
            {
                if (elementTypes[i] == 1) // Material/Triangles element
                {
                    int elemPtrOffset = offElements + (i * 4);
                    var elemReader = _memory.GetReaderAt(elemPtrOffset);
                    if (elemReader == null) continue;

                    int elemAddr = elemReader.ReadInt32();
                    var element = ReadElementTriangles(elemAddr, numVertices);
                    if (element != null)
                    {
                        var subMesh = new SubMeshData
                        {
                            Triangles = element.Triangles,
                            TextureName = element.TextureName,
                            MaterialFlags = element.MaterialFlags,
                            IsLight = element.IsLight
                        };

                        if (element.UVs != null && element.UVMapping != null)
                        {
                            subMesh.UVs = element.UVs;
                            subMesh.UVIndices = element.UVMapping;
                        }

                        subMeshes.Add(subMesh);
                        allTriangles.AddRange(element.Triangles);
                    }
                }
            }

            return new MeshData
            {
                Name = $"Mesh_{address:X8}",
                Vertices = vertices,
                Normals = normals,
                SubMeshes = subMeshes,
                Indices = allTriangles.ToArray()
            };
        }
        catch
        {
            return null;
        }
    }

    private Vector3[]? ReadVertices(int address, uint count)
    {
        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var vertices = new Vector3[count];
            for (int i = 0; i < count; i++)
            {
                float x = reader.ReadSingle();
                float z = reader.ReadSingle();
                float y = reader.ReadSingle();

                if (!float.IsFinite(x) || !float.IsFinite(y) || !float.IsFinite(z))
                    return null;

                vertices[i] = new Vector3(x, y, z);
            }
            return vertices;
        }
        catch { return null; }
    }

    private Vector3[]? ReadNormals(int address, uint count)
    {
        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var normals = new Vector3[count];
            for (int i = 0; i < count; i++)
            {
                float x = reader.ReadSingle();
                float z = reader.ReadSingle();
                float y = reader.ReadSingle();
                normals[i] = new Vector3(x, y, z);
            }
            return normals;
        }
        catch { return null; }
    }

    private ushort[]? ReadElementTypes(int address, uint count)
    {
        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var types = new ushort[count];
            for (int i = 0; i < count; i++)
            {
                types[i] = reader.ReadUInt16();
            }
            return types;
        }
        catch { return null; }
    }

    private static int _elemDebugCount = 0;

    private ElementData? ReadElementTriangles(int address, uint numVertices)
    {
        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            int offMaterial = reader.ReadInt32();

            if (_elemDebugCount < 5)
            {
                Console.WriteLine($"      Element at 0x{address:X8}: offMaterial=0x{offMaterial:X8}, textureTable={(_textureTable != null ? "OK" : "NULL")}");
                _elemDebugCount++;
            }
            ushort numTriangles = reader.ReadUInt16();
            ushort numUvs = reader.ReadUInt16();
            int offTriangles = reader.ReadInt32();
            int offMappingUvs = reader.ReadInt32();
            int offNormals = reader.ReadInt32();
            int offUvs = reader.ReadInt32();

            if (numTriangles == 0 || numTriangles > 10000)
                return null;

            // Read triangles
            var triReader = _memory.GetReaderAt(offTriangles);
            if (triReader == null) return null;

            var triangles = new int[numTriangles * 3];
            for (int i = 0; i < numTriangles * 3; i++)
            {
                short index = triReader.ReadInt16();
                if (index < 0 || index >= numVertices)
                    return null;
                triangles[i] = index;
            }

            var element = new ElementData { Triangles = triangles };

            // Read UVs
            if (numUvs > 0 && offUvs != 0)
            {
                var uvReader = _memory.GetReaderAt(offUvs);
                if (uvReader != null)
                {
                    element.UVs = new Vector2[numUvs];
                    for (int i = 0; i < numUvs; i++)
                    {
                        float u = uvReader.ReadSingle();
                        float v = uvReader.ReadSingle();
                        element.UVs[i] = new Vector2(u, 1f - v);
                    }
                }
            }

            // Read UV mapping
            if (offMappingUvs != 0 && numTriangles > 0)
            {
                var uvMapReader = _memory.GetReaderAt(offMappingUvs);
                if (uvMapReader != null)
                {
                    element.UVMapping = new int[numTriangles * 3];
                    for (int i = 0; i < numTriangles * 3; i++)
                    {
                        element.UVMapping[i] = uvMapReader.ReadInt16();
                    }
                }
            }

            // Try to get texture name from material using proper material reader
            if (_textureTable != null && offMaterial != 0)
            {
                var gameMaterial = _gameMaterialReader.Read(offMaterial);
                if (_elemDebugCount <= 5)
                {
                    bool hasVisMat = gameMaterial?.VisualMaterial != null;
                    int offTex = gameMaterial?.VisualMaterial?.OffTexture ?? 0;
                    Console.WriteLine($"        gameMaterial: hasVisMat={hasVisMat}, offTexture=0x{offTex:X8}");
                }
                if (gameMaterial?.VisualMaterial != null)
                {
                    element.MaterialFlags = gameMaterial.VisualMaterial.Flags;
                    if (gameMaterial.VisualMaterial.OffTexture != 0)
                    {
                        var textureEntry = _textureTable.GetTextureEntry(gameMaterial.VisualMaterial.OffTexture);
                        if (_elemDebugCount <= 5)
                        {
                            Console.WriteLine($"        textureEntry: {(textureEntry?.Name ?? "NULL")}");
                        }
                        if (textureEntry != null)
                        {
                            element.TextureName = textureEntry.Name;
                            element.IsLight = textureEntry.IsLight;
                        }
                    }
                }
            }

            return element;
        }
        catch { return null; }
    }

    /// <summary>
    /// Creates a GLTF mesh builder from mesh data.
    /// </summary>
    private static bool _loggedTextureDebug = false;

    private MeshBuilder<VertexPositionNormal, VertexTexture1, VertexEmpty> CreateMeshBuilder(
        MeshData mesh, string name, Func<string?, string?>? textureLookup)
    {
        var meshBuilder = new MeshBuilder<VertexPositionNormal, VertexTexture1, VertexEmpty>(name);
        bool hasNormals = mesh.Normals != null && mesh.Normals.Length == mesh.Vertices.Length;

        var materialCache = new Dictionary<string, MaterialBuilder>();

        int subMeshCount = 0;
        foreach (var subMesh in mesh.SubMeshes)
        {
            subMeshCount++;
            string? resolvedTexture = textureLookup?.Invoke(subMesh.TextureName);

            // Debug: log texture resolution
            if (subMeshCount <= 3)
            {
                Console.WriteLine($"    SubMesh {subMeshCount}: texName='{subMesh.TextureName ?? "NULL"}' -> resolved='{resolvedTexture ?? "NULL"}'");
            }

            bool isTransparent = (subMesh.MaterialFlags & 0x8) != 0;
            bool isLight = subMesh.IsLight;
            string materialKey = $"{resolvedTexture ?? "__default__"}_{isTransparent}_{isLight}";

            if (!materialCache.TryGetValue(materialKey, out var material))
            {
                material = CreateMaterial(resolvedTexture, isTransparent, isLight);
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

                var n0 = hasNormals ? SanitizeNormal(mesh.Normals![i0]) : CalculateNormal(v0, v1, v2);
                var n1 = hasNormals ? SanitizeNormal(mesh.Normals![i1]) : n0;
                var n2 = hasNormals ? SanitizeNormal(mesh.Normals![i2]) : n0;

                var uv0 = GetUV(subMesh, i, hasUVs);
                var uv1 = GetUV(subMesh, i + 1, hasUVs);
                var uv2 = GetUV(subMesh, i + 2, hasUVs);

                // Original engine data uses CW winding (DirectX convention)
                // glTF expects CCW winding, so swap v1↔v2 to reverse winding order
                primitive.AddTriangle(
                    (new VertexPositionNormal(v0, n0), new VertexTexture1(uv0)),
                    (new VertexPositionNormal(v2, n2), new VertexTexture1(uv2)),
                    (new VertexPositionNormal(v1, n1), new VertexTexture1(uv1)));
            }
        }

        return meshBuilder;
    }

    private static int _matDebugCount = 0;

    private static int _materialCounter = 0;

    private static MaterialBuilder CreateMaterial(string? texturePath, bool isTransparent = false, bool isLight = false)
    {
        string matName = !string.IsNullOrEmpty(texturePath)
            ? $"mat_{Path.GetFileNameWithoutExtension(texturePath)}"
            : $"mat_default_{_materialCounter++}";

        var material = new MaterialBuilder(matName)
            .WithMetallicRoughnessShader()
            .WithMetallicRoughness(0f, 1f);

        if (!string.IsNullOrEmpty(texturePath))
        {
            bool exists = File.Exists(texturePath);
            if (_matDebugCount < 5)
            {
                Console.WriteLine($"    Material: path='{texturePath}', exists={exists}");
                _matDebugCount++;
            }

            if (exists)
            {
                try
                {
                    byte[] imageBytes = File.ReadAllBytes(texturePath);
                    Console.WriteLine($"      Loaded {imageBytes.Length} bytes");

                    var image = new MemoryImage(imageBytes);
                    Console.WriteLine($"      Created MemoryImage, applying to material '{matName}'");

                    if (isLight)
                    {
                        material.WithBaseColor(new Vector4(0f, 0f, 0f, 1f));
                        material.WithEmissive(image, Vector3.One);
                    }
                    else
                    {
                        material.WithBaseColor(image);
                    }

                    if (texturePath.EndsWith(".png", StringComparison.OrdinalIgnoreCase))
                    {
                        material.WithAlpha(SharpGLTF.Materials.AlphaMode.MASK, 0.5f);
                    }

                    Console.WriteLine($"      Material configured successfully");
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"      ERROR: {ex.GetType().Name}: {ex.Message}");
                    material.WithBaseColor(new Vector4(0.8f, 0.8f, 0.8f, 1f));
                }
            }
            else
            {
                material.WithBaseColor(new Vector4(0.8f, 0.8f, 0.8f, 1f));
            }
        }
        else
        {
            material.WithBaseColor(new Vector4(0.8f, 0.8f, 0.8f, 1f));
        }

        return material;
    }

    private static Vector2 GetUV(SubMeshData subMesh, int triangleVertexIndex, bool hasUVs)
    {
        if (!hasUVs || triangleVertexIndex >= subMesh.UVIndices.Length)
            return Vector2.Zero;

        int uvIndex = subMesh.UVIndices[triangleVertexIndex];
        if (uvIndex < 0 || uvIndex >= subMesh.UVs.Length)
            return Vector2.Zero;

        return subMesh.UVs[uvIndex];
    }

    private static Vector3 CalculateNormal(Vector3 v0, Vector3 v1, Vector3 v2)
    {
        var edge1 = v1 - v0;
        var edge2 = v2 - v0;
        var normal = Vector3.Cross(edge1, edge2);

        if (normal.LengthSquared() > 0.0001f)
            return Vector3.Normalize(normal);
        return Vector3.UnitY;
    }

    private static Vector3 SanitizeNormal(Vector3 v)
    {
        if (!float.IsFinite(v.X) || !float.IsFinite(v.Y) || !float.IsFinite(v.Z))
            return Vector3.UnitY;

        if (v.LengthSquared() > 0.0001f)
            return Vector3.Normalize(v);
        return Vector3.UnitY;
    }
}
