namespace Astrolabe.Core.Intermediate;

public sealed class LevelIntermediateManifest
{
    public string Schema { get; set; } = "astrolabe.level-intermediate.v1";
    public string LevelName { get; set; } = "";
    public string SourceDirectoryName { get; set; } = "";
    public List<SnaFileManifest> SnaFiles { get; set; } = new();
    public List<RelocationTableFileManifest> RelocationTables { get; set; } = new();
    public List<LooseFileManifest> LooseFiles { get; set; } = new();
    public SemanticManifest? Semantic { get; set; }
}

public sealed class SnaFileManifest
{
    public string FileName { get; set; } = "";
    public List<SnaBlockManifest> Blocks { get; set; } = new();
}

public sealed class SnaBlockManifest
{
    public int Order { get; set; }
    public string Key { get; set; } = "";
    public byte Module { get; set; }
    public byte Id { get; set; }
    public int BaseInMemory { get; set; }
    public uint Unk2 { get; set; }
    public uint Unk3 { get; set; }
    public uint MaxPosMinus9 { get; set; }
    public bool HasPayload { get; set; }
    public string? DataPath { get; set; }
    public string? DataSha256 { get; set; }
    public string? ContentPath { get; set; }
    public SnaStorageManifest? OriginalStorage { get; set; }
}

public sealed class SnaBlockContentDocument
{
    public string Schema { get; set; } = "astrolabe.sna-block-content.v1";
    public string FileName { get; set; } = "";
    public int BlockOrder { get; set; }
    public string BlockKey { get; set; } = "";
    public byte Module { get; set; }
    public byte Id { get; set; }
    public int BaseInMemory { get; set; }
    public string BaseInMemoryHex { get; set; } = "";
    public string OriginalDataSha256 { get; set; } = "";
    public List<SnaBlockContentElement> Elements { get; set; } = new();
}

public sealed class SnaBlockContentElement
{
    public int Order { get; set; }
    public string Kind { get; set; } = "";
    public string DataPath { get; set; } = "";
    public string Sha256 { get; set; } = "";
    public List<string> Labels { get; set; } = new();
}

public class IntermediateSuperObject
{
    public string Schema { get; set; } = "astrolabe.super-object.v1";
    public uint TypeCode { get; set; }
    public string Type { get; set; } = "";
    public int OffData { get; set; }
    public int ChildrenHead { get; set; }
    public int ChildrenTail { get; set; }
    public uint ChildrenCount { get; set; }
    public int BrotherNext { get; set; }
    public int BrotherPrev { get; set; }
    public int Parent { get; set; }
    public int Matrix { get; set; }
    public int StaticMatrix { get; set; }
    public int GlobalMatrix { get; set; }
    public uint DrawFlags { get; set; }
    public uint Flags { get; set; }
    public int BoundingVolume { get; set; }
}

public sealed class IntermediateSceneNode : IntermediateSuperObject
{
    public new string Schema { get; set; } = "astrolabe.scene-node.v1";
    public string Id { get; set; } = "";
    public string Path { get; set; } = "";
    public string Root { get; set; } = "";
    public string? Name { get; set; }
    public int GeometricObjectAddress { get; set; }
    public string? MatrixPath { get; set; }
    public string? StaticMatrixPath { get; set; }
    public List<string> Children { get; set; } = new();
}

public sealed class IntermediateMatrix
{
    public string Schema { get; set; } = "astrolabe.matrix.v1";
    public uint Type { get; set; }
    public float[] Translation { get; set; } = [];
    public float[] BasisX { get; set; } = [];
    public float[] BasisY { get; set; } = [];
    public float[] BasisZ { get; set; } = [];
    public string ExtraBase64 { get; set; } = "";
}

public sealed class IntermediateGeometricObject
{
    public string Schema { get; set; } = "astrolabe.geometric-object.v1";
    public uint NumVertices { get; set; }
    public int Vertices { get; set; }
    public int Normals { get; set; }
    public int Materials { get; set; }
    public int Unknown0 { get; set; }
    public uint NumElements { get; set; }
    public int ElementTypes { get; set; }
    public int Elements { get; set; }
    public int[] Unknowns { get; set; } = [];
    public float SphereRadius { get; set; }
    public float[] SphereCenterRaw { get; set; } = [];
}

public sealed class IntermediatePhysicalObject
{
    public string Schema { get; set; } = "astrolabe.physical-object.v1";
    public int VisualSet { get; set; }
    public int CollideSet { get; set; }
    public int VisualBoundingVolume { get; set; }
    public int Unknown0 { get; set; }
}

public sealed class IntermediateIpo
{
    public string Schema { get; set; } = "astrolabe.ipo.v1";
    public int PhysicalObject { get; set; }
    public int Radiosity { get; set; }
}

public sealed class IntermediateGameMaterial
{
    public string Schema { get; set; } = "astrolabe.game-material.v1";
    public int VisualMaterial { get; set; }
    public int MechanicsMaterial { get; set; }
    public uint SoundMaterial { get; set; }
    public int CollideMaterial { get; set; }
}

public sealed class IntermediateUInt32Record
{
    public string Schema { get; set; } = "astrolabe.uint32-record.v1";
    public string Type { get; set; } = "";
    public uint[] Values { get; set; } = [];
}

public sealed class IntermediateFloat3Array
{
    public string Schema { get; set; } = "astrolabe.float3-array.v1";
    public string Type { get; set; } = "";
    public float[][] Values { get; set; } = [];
}

public sealed class SnaStorageManifest
{
    public bool IsCompressed { get; set; }
    public uint CompressedSize { get; set; }
    public uint CompressedChecksum { get; set; }
    public uint DecompressedSize { get; set; }
    public uint DecompressedChecksum { get; set; }
    public string? EncodedPath { get; set; }
    public string? EncodedSha256 { get; set; }
}

public sealed class RelocationTableFileManifest
{
    public string FileName { get; set; } = "";
    public string JsonPath { get; set; } = "";
}

public sealed class RelocationTableDocument
{
    public string Schema { get; set; } = "astrolabe.relocation-table.v1";
    public string FileName { get; set; } = "";
    public List<RelocationPointerBlockManifest> Blocks { get; set; } = new();
}

public sealed class RelocationPointerBlockManifest
{
    public int Order { get; set; }
    public string Key { get; set; } = "";
    public byte Module { get; set; }
    public byte Id { get; set; }
    public int EntrySize { get; set; }
    public string PointerDataSha256 { get; set; } = "";
    public RelocationStorageManifest? OriginalStorage { get; set; }
    public List<RelocationPointerManifest> Pointers { get; set; } = new();
    public string? TrailingDataBase64 { get; set; }
}

public sealed class RelocationPointerManifest
{
    public uint OffsetInMemory { get; set; }
    public byte TargetModule { get; set; }
    public byte TargetId { get; set; }
    public byte Byte6 { get; set; }
    public byte Byte7 { get; set; }
}

public sealed class RelocationStorageManifest
{
    public bool IsCompressed { get; set; }
    public uint CompressedSize { get; set; }
    public uint CompressedChecksum { get; set; }
    public uint DecompressedSize { get; set; }
    public uint DecompressedChecksum { get; set; }
    public string? EncodedPath { get; set; }
    public string? EncodedSha256 { get; set; }
}

public sealed class LooseFileManifest
{
    public string FileName { get; set; } = "";
    public string Path { get; set; } = "";
    public long Size { get; set; }
    public string Sha256 { get; set; } = "";
}

public sealed class SemanticManifest
{
    public string? SceneTreePath { get; set; }
    public string? CoveragePath { get; set; }
    public List<string> Errors { get; set; } = new();
}

public sealed class SemanticSceneDocument
{
    public string Schema { get; set; } = "astrolabe.scene-tree.v1";
    public string LevelName { get; set; } = "";
    public int TotalNodes { get; set; }
    public Dictionary<string, SemanticSceneNode?> Roots { get; set; } = new();
}

public sealed class SemanticSceneNode
{
    public int Address { get; set; }
    public string AddressHex { get; set; } = "";
    public string Type { get; set; } = "";
    public uint TypeCode { get; set; }
    public string? Name { get; set; }
    public int OffData { get; set; }
    public int GeometricObjectAddress { get; set; }
    public int OffMatrix { get; set; }
    public int OffStaticMatrix { get; set; }
    public int OffBoundingVolume { get; set; }
    public uint DrawFlags { get; set; }
    public uint Flags { get; set; }
    public uint FamilyIndex { get; set; }
    public uint ModelIndex { get; set; }
    public uint InstanceIndex { get; set; }
    public List<SemanticSceneNode> Children { get; set; } = new();
}

public sealed class SemanticCoverageDocument
{
    public string Schema { get; set; } = "astrolabe.byte-coverage.v1";
    public string LevelName { get; set; } = "";
    public int TotalBytes { get; set; }
    public int CoveredBytes { get; set; }
    public int UncoveredBytes { get; set; }
    public double CoveragePercent { get; set; }
    public List<SemanticByteRange> Ranges { get; set; } = new();
    public List<SemanticByteRange> UncoveredRegions { get; set; } = new();
    public List<SemanticBlockCoverage> Blocks { get; set; } = new();
}

public sealed class SemanticByteRange
{
    public int Start { get; set; }
    public string StartHex { get; set; } = "";
    public int Length { get; set; }
    public int End { get; set; }
    public string EndHex { get; set; } = "";
    public string Label { get; set; } = "";
}

public sealed class SemanticBlockCoverage
{
    public string Key { get; set; } = "";
    public byte Module { get; set; }
    public byte Id { get; set; }
    public int BaseInMemory { get; set; }
    public string BaseInMemoryHex { get; set; } = "";
    public int TotalBytes { get; set; }
    public int CoveredBytes { get; set; }
    public int UncoveredBytes { get; set; }
    public double CoveragePercent { get; set; }
}
