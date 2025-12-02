#r "src/Astrolabe.Core/bin/Debug/net9.0/Astrolabe.Core.dll"
using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Geometry;

var levelDir = "extracted/Gamedata/World/Levels/astrolabe";
var levelName = "astrolabe";
var loader = new LevelLoader(levelDir, levelName);
var scanner = new MeshScanner(loader);
var meshes = scanner.ScanForMeshes();

// Get first mesh with triangles
var mesh = meshes.FirstOrDefault(m => m.Indices != null && m.Indices.Length >= 3);
if (mesh != null)
{
    var texturePath = "textures/astrolabe/2reliefs5txy.tga";
    GltfExporter.ExportMesh(mesh, "output/test_textured.glb", texturePath);
    Console.WriteLine($"Exported {mesh.Name} with texture");
}
