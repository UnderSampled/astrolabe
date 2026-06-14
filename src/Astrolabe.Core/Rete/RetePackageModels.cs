using Astrolabe.Core.FileFormats;

namespace Astrolabe.Core.Rete;

public sealed class RetePackageManifest
{
    public string Schema { get; set; } = "astrolabe.rete.v1";
    public string PackageRole { get; set; } = "level";
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
    public int OffsetInBlock { get; set; }
    public int Length { get; set; }
    public int VirtualAddress { get; set; }
    public string VirtualAddressHex { get; set; } = "";
    public string Sha256 { get; set; } = "";
    public List<string> Labels { get; set; } = new();
}

public sealed class IntermediateSceneNode : SuperObjectRecord
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
    public string? FixLevelSitesPath { get; set; }
    public List<string> Errors { get; set; } = new();
}

public sealed class FixLevelSitesDocument
{
    public string Schema { get; set; } = "astrolabe.fix-level-sites.v1";
    public string LevelName { get; set; } = "";
    public List<FixLevelSiteBlock> Blocks { get; set; } = new();
    public List<FixLevelSiteEntry> Sites { get; set; } = new();
}

public sealed class FixLevelSiteBlock
{
    public int Order { get; set; }
    public byte SourceModule { get; set; }
    public byte SourceId { get; set; }
}

public sealed class FixLevelSiteEntry
{
    public byte SourceModule { get; set; }
    public byte SourceId { get; set; }
    public uint OffsetInMemory { get; set; }
    public byte TargetModule { get; set; }
    public byte TargetId { get; set; }
    public string? TargetUri { get; set; }
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
