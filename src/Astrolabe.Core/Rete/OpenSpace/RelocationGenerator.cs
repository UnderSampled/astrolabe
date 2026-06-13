using System.Buffers.Binary;
using System.Text.Json;
using Astrolabe.Core.Serialization;

namespace Astrolabe.Core.Rete.OpenSpace;

public sealed record RelocationComparisonResult(
    string FileName,
    bool Supported,
    int PreservedPointerCount,
    int GeneratedPointerCount,
    int MatchingPointerCount,
    int MissingPointerCount,
    int ExtraPointerCount,
    bool PointerDataMatches,
    string? Note,
    IReadOnlyList<string> MissingSamples,
    IReadOnlyList<string> ExtraSamples);

internal static class RelocationGenerator
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    public static RelocationTableDocument GenerateRtb(
        string sourcePackageRoot,
        string fileName,
        IReadOnlyList<string> targetPackageRoots,
        bool includeEmptyBlocks = false,
        bool includeSourcePackageAsTarget = true)
    {
        var sourceLayout = PackageLayout.Load(sourcePackageRoot);
        var targetRoots = includeSourcePackageAsTarget
            ? targetPackageRoots.Prepend(sourcePackageRoot)
            : targetPackageRoots;
        var targetLayouts = targetRoots
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .Select(PackageLayout.Load)
            .ToList();

        var resolver = new ReferenceAddressResolver(sourcePackageRoot);
        foreach (var targetPackageRoot in targetPackageRoots)
        {
            resolver.LoadPackage(targetPackageRoot);
        }

        var document = new RelocationTableDocument { FileName = fileName };
        foreach (var sourceBlock in sourceLayout.Blocks)
        {
            var pointers = GenerateBlockPointers(sourcePackageRoot, sourceBlock, targetLayouts, resolver);
            if (pointers.Count == 0 && !includeEmptyBlocks)
            {
                continue;
            }

            var block = new RelocationPointerBlockManifest
            {
                Order = document.Blocks.Count,
                Key = ToKey(sourceBlock.Module, sourceBlock.Id),
                Module = sourceBlock.Module,
                Id = sourceBlock.Id,
                EntrySize = 8,
                Pointers = pointers,
            };
            block.PointerDataSha256 = HashBytes(BuildPointerData(block));
            document.Blocks.Add(block);
        }

        return document;
    }

    public static RelocationTableDocument GeneratePointerFileTable(
        string sourcePackageRoot,
        string fileName,
        string pointerFilePath,
        IReadOnlyList<string> targetPackageRoots)
    {
        var sourceLayout = PackageLayout.Load(sourcePackageRoot);
        var sourceBlock = sourceLayout.Blocks.FirstOrDefault()
            ?? throw new InvalidDataException($"Rete package has no payload blocks: {sourcePackageRoot}");
        var targetLayouts = targetPackageRoots
            .Prepend(sourcePackageRoot)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .Select(PackageLayout.Load)
            .ToList();

        var pointers = GeneratePointerFileEntries(pointerFilePath, targetLayouts);
        var block = new RelocationPointerBlockManifest
        {
            Order = 0,
            Key = ToKey(sourceBlock.Module, sourceBlock.Id),
            Module = sourceBlock.Module,
            Id = sourceBlock.Id,
            EntrySize = 8,
            Pointers = pointers
        };
        block.PointerDataSha256 = HashBytes(BuildPointerData(block));

        return new RelocationTableDocument
        {
            FileName = fileName,
            Blocks = [block]
        };
    }

    public static RelocationComparisonResult Compare(RelocationTableDocument preserved, RelocationTableDocument generated)
    {
        var preservedPointers = Flatten(preserved).ToHashSet();
        var generatedPointers = Flatten(generated).ToHashSet();
        var matching = preservedPointers.Intersect(generatedPointers).Count();
        var missingPointers = preservedPointers.Except(generatedPointers).Order().ToList();
        var extraPointers = generatedPointers.Except(preservedPointers).Order().ToList();

        return new RelocationComparisonResult(
            preserved.FileName,
            Supported: true,
            PreservedPointerCount: preservedPointers.Count,
            GeneratedPointerCount: generatedPointers.Count,
            MatchingPointerCount: matching,
            MissingPointerCount: missingPointers.Count,
            ExtraPointerCount: extraPointers.Count,
            PointerDataMatches: PointerDataMatches(preserved, generated),
            Note: null,
            MissingSamples: missingPointers.Take(10).Select(p => p.ToDisplayString()).ToList(),
            ExtraSamples: extraPointers.Take(10).Select(p => p.ToDisplayString()).ToList());
    }

    private static List<RelocationPointerManifest> GenerateBlockPointers(
        string sourcePackageRoot,
        BlockLayout sourceBlock,
        IReadOnlyList<PackageLayout> targetLayouts,
        ReferenceAddressResolver resolver)
    {
        var pointers = new List<RelocationPointerManifest>();
        var seenOffsets = new HashSet<uint>();

        foreach (var element in sourceBlock.Elements)
        {
            if (!StructCodecRegistry.TryGet(element.Kind, out var codec) ||
                codec.PointerFields.Count == 0)
            {
                continue;
            }

            var elementPath = ReferenceUri.Resolve(sourcePackageRoot, element.DataPath).FilePath;
            if (!elementPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase) ||
                !File.Exists(elementPath))
            {
                continue;
            }

            using var json = JsonDocument.Parse(File.ReadAllText(elementPath));
            using var resolvedJson = ReferenceJson.ResolvePointersForExport(
                json.RootElement,
                sourcePackageRoot,
                codec.PointerFields,
                resolver);
            var data = codec.WriteFromJsonElement(resolvedJson.RootElement);

            foreach (var pointerField in codec.PointerFields.OrderBy(f => f.Offset))
            {
                if (pointerField.Offset < 0 || pointerField.Offset + 4 > data.Length)
                {
                    continue;
                }

                var value = BinaryPrimitives.ReadInt32LittleEndian(
                    data.AsSpan(pointerField.Offset, sizeof(int)));
                if (value == 0)
                {
                    continue;
                }

                var offsetInMemory = checked((uint)(sourceBlock.BaseInMemory + element.OffsetInBlock + pointerField.Offset));
                if (!seenOffsets.Add(offsetInMemory))
                {
                    continue;
                }

                var target = FindTargetBlock(value, targetLayouts);
                if (target == null)
                {
                    continue;
                }

                pointers.Add(new RelocationPointerManifest
                {
                    OffsetInMemory = offsetInMemory,
                    TargetModule = target.Module,
                    TargetId = target.Id,
                    Byte6 = 0,
                    Byte7 = 0
                });
            }
        }

        return pointers
            .OrderBy(p => p.OffsetInMemory)
            .ThenBy(p => p.TargetModule)
            .ThenBy(p => p.TargetId)
            .ToList();
    }

    private static BlockLayout? FindTargetBlock(int address, IReadOnlyList<PackageLayout> layouts)
    {
        foreach (var layout in layouts)
        {
            if (layout.TryFindBlock(address, out var block))
            {
                return block;
            }
        }

        return null;
    }

    private static List<RelocationPointerManifest> GeneratePointerFileEntries(
        string pointerFilePath,
        IReadOnlyList<PackageLayout> targetLayouts)
    {
        var data = File.ReadAllBytes(pointerFilePath);
        var pointers = new List<RelocationPointerManifest>();
        var seenValues = new HashSet<uint>();

        for (var offset = 0; offset <= data.Length - sizeof(uint); offset += sizeof(uint))
        {
            var value = BinaryPrimitives.ReadUInt32LittleEndian(data.AsSpan(offset, sizeof(uint)));
            if (value == 0 || !seenValues.Add(value))
            {
                continue;
            }

            var target = FindTargetBlock(unchecked((int)value), targetLayouts);
            if (target == null)
            {
                continue;
            }

            pointers.Add(new RelocationPointerManifest
            {
                OffsetInMemory = value,
                TargetModule = target.Module,
                TargetId = target.Id,
                Byte6 = 0,
                Byte7 = 0
            });
        }

        return pointers
            .OrderBy(p => p.OffsetInMemory)
            .ThenBy(p => p.TargetModule)
            .ThenBy(p => p.TargetId)
            .ToList();
    }

    private static IEnumerable<ComparablePointer> Flatten(RelocationTableDocument document)
    {
        foreach (var block in document.Blocks)
        {
            foreach (var pointer in block.Pointers)
            {
                yield return new ComparablePointer(
                    block.Module,
                    block.Id,
                    pointer.OffsetInMemory,
                    pointer.TargetModule,
                    pointer.TargetId,
                    pointer.Byte6,
                    pointer.Byte7);
            }
        }
    }

    private static byte[] BuildPointerData(RelocationPointerBlockManifest block)
    {
        var entrySize = block.EntrySize >= 8 ? 8 : 6;
        using var stream = new MemoryStream();
        using var writer = new BinaryWriter(stream);
        foreach (var pointer in block.Pointers)
        {
            writer.Write(pointer.OffsetInMemory);
            writer.Write(pointer.TargetModule);
            writer.Write(pointer.TargetId);
            if (entrySize >= 8)
            {
                writer.Write(pointer.Byte6);
                writer.Write(pointer.Byte7);
            }
        }

        writer.Flush();
        return stream.ToArray();
    }

    private static bool PointerDataMatches(RelocationTableDocument preserved, RelocationTableDocument generated)
    {
        var preservedBlocks = preserved.Blocks.OrderBy(b => b.Order).ToList();
        var generatedBlocks = generated.Blocks.OrderBy(b => b.Order).ToList();
        if (preservedBlocks.Count != generatedBlocks.Count)
        {
            return false;
        }

        for (var i = 0; i < preservedBlocks.Count; i++)
        {
            var preservedBlock = preservedBlocks[i];
            var generatedBlock = generatedBlocks[i];
            if (preservedBlock.Module != generatedBlock.Module ||
                preservedBlock.Id != generatedBlock.Id ||
                preservedBlock.Pointers.Count != generatedBlock.Pointers.Count)
            {
                return false;
            }

            if (!BuildPointerData(preservedBlock).AsSpan().SequenceEqual(BuildPointerData(generatedBlock)))
            {
                return false;
            }
        }

        return true;
    }

    private static string HashBytes(byte[] data)
    {
        var hash = System.Security.Cryptography.SHA256.HashData(data);
        return Convert.ToHexString(hash).ToLowerInvariant();
    }

    private static string ToKey(byte module, byte id) => $"{module:X2}:{id:X2}";

    private readonly record struct ComparablePointer(
        byte SourceModule,
        byte SourceId,
        uint OffsetInMemory,
        byte TargetModule,
        byte TargetId,
        byte Byte6,
        byte Byte7) : IComparable<ComparablePointer>
    {
        public int CompareTo(ComparablePointer other)
        {
            var compare = SourceModule.CompareTo(other.SourceModule);
            if (compare != 0) return compare;
            compare = SourceId.CompareTo(other.SourceId);
            if (compare != 0) return compare;
            compare = OffsetInMemory.CompareTo(other.OffsetInMemory);
            if (compare != 0) return compare;
            compare = TargetModule.CompareTo(other.TargetModule);
            if (compare != 0) return compare;
            compare = TargetId.CompareTo(other.TargetId);
            if (compare != 0) return compare;
            compare = Byte6.CompareTo(other.Byte6);
            if (compare != 0) return compare;
            return Byte7.CompareTo(other.Byte7);
        }

        public string ToDisplayString() =>
            $"[{SourceModule:X2}:{SourceId:X2}] @0x{OffsetInMemory:X8} -> [{TargetModule:X2}:{TargetId:X2}] ({Byte6:X2},{Byte7:X2})";
    }

    private sealed class PackageLayout
    {
        public List<BlockLayout> Blocks { get; } = new();

        public static PackageLayout Load(string packageRoot)
        {
            var layout = new PackageLayout();
            var manifestPath = Path.Combine(packageRoot, OpenSpacePackageCodec.ManifestFileName);
            var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(manifestPath),
                JsonOptions) ?? throw new InvalidDataException($"Could not read Rete manifest: {manifestPath}");

            foreach (var snaFile in manifest.SnaFiles)
            {
                foreach (var block in snaFile.Blocks.OrderBy(b => b.Order))
                {
                    if (block.ContentPath == null)
                    {
                        continue;
                    }

                    var contentPath = ResolvePackagePath(packageRoot, block.ContentPath);
                    var content = JsonSerializer.Deserialize<SnaBlockContentDocument>(
                        File.ReadAllText(contentPath),
                        JsonOptions) ?? throw new InvalidDataException($"Could not read SNA block content: {contentPath}");

                    var elements = new List<ElementLayout>();
                    var cursor = 0;
                    foreach (var element in content.Elements.OrderBy(e => e.Order))
                    {
                        var offset = element.Length > 0 ? element.OffsetInBlock : cursor;
                        var length = element.Length > 0
                            ? element.Length
                            : DetermineElementLength(packageRoot, element);

                        elements.Add(new ElementLayout(
                            element.Order,
                            element.Kind,
                            element.DataPath,
                            offset,
                            length));
                        cursor = checked(offset + length);
                    }

                    layout.Blocks.Add(new BlockLayout(
                        block.Order,
                        block.Module,
                        block.Id,
                        content.BaseInMemory,
                        elements));
                }
            }

            return layout;
        }

        public bool TryFindBlock(int address, out BlockLayout block)
        {
            foreach (var candidate in Blocks)
            {
                if (address >= candidate.BaseInMemory && address < candidate.EndInMemory)
                {
                    block = candidate;
                    return true;
                }
            }

            block = null!;
            return false;
        }

        private static int DetermineElementLength(string packageRoot, SnaBlockContentElement element)
        {
            var dataPath = ReferenceUri.Resolve(packageRoot, element.DataPath).FilePath;
            if (StructCodecRegistry.TryGet(element.Kind, out var codec))
            {
                if (codec.FixedSize is { } fixedSize)
                {
                    return fixedSize;
                }

                if (dataPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
                {
                    using var document = JsonDocument.Parse(File.ReadAllText(dataPath));
                    return codec.WriteFromJsonElement(document.RootElement).Length;
                }
            }

            return checked((int)new FileInfo(dataPath).Length);
        }

        private static string ResolvePackagePath(string packageRoot, string relativePath) =>
            Path.Combine(relativePath.Split('/').Prepend(packageRoot).ToArray());
    }

    private sealed record BlockLayout(
        int Order,
        byte Module,
        byte Id,
        int BaseInMemory,
        IReadOnlyList<ElementLayout> Elements)
    {
        public int Length => Elements.Count == 0
            ? 0
            : Elements.Max(e => e.OffsetInBlock + e.Length);

        public int EndInMemory => checked(BaseInMemory + Length);
    }

    private sealed record ElementLayout(
        int Order,
        string Kind,
        string DataPath,
        int OffsetInBlock,
        int Length);
}
