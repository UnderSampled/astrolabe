using System.Buffers.Binary;
using System.IO.Compression;
using System.Text.Json;
using Astrolabe.Core.Serialization;
using Astrolabe.Core.Serialization.Codecs;
using lzo.net;

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
        bool includeSourcePackageAsTarget = true,
        RelocationPackageContext? context = null)
    {
        context ??= new RelocationPackageContext();
        var sourceLayout = context.GetLayout(sourcePackageRoot);
        var targetRoots = includeSourcePackageAsTarget
            ? targetPackageRoots.Prepend(sourcePackageRoot)
            : targetPackageRoots;
        var targetLayouts = targetRoots
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .Select(context.GetLayout)
            .ToList();
        var targetResolver = new TargetBlockResolver(sourcePackageRoot, targetLayouts);

        var resolver = new ReferenceAddressResolver(sourcePackageRoot);
        foreach (var targetPackageRoot in targetPackageRoots)
        {
            resolver.LoadPackage(targetPackageRoot);
        }

        var document = new RelocationTableDocument { FileName = fileName };
        foreach (var sourceBlock in sourceLayout.Blocks)
        {
            var pointers = GenerateBlockPointers(sourcePackageRoot, sourceBlock, targetResolver, resolver);
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
        IReadOnlyList<string> targetPackageRoots,
        RelocationPackageContext? context = null)
    {
        context ??= new RelocationPackageContext();
        var sourceLayout = context.GetLayout(sourcePackageRoot);
        var sourceBlock = sourceLayout.Blocks.FirstOrDefault(b => b.Elements.Count > 0)
            ?? sourceLayout.Blocks.FirstOrDefault()
            ?? throw new InvalidDataException($"Rete package has no payload blocks: {sourcePackageRoot}");
        var targetLayouts = targetPackageRoots
            .Prepend(sourcePackageRoot)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .Select(context.GetLayout)
            .ToList();
        var targetResolver = new TargetBlockResolver(sourcePackageRoot, targetLayouts);

        var pointers = GeneratePointerFileEntries(pointerFilePath, targetResolver);
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

    internal static string? FindTargetBlockKey(string packageRoot, int address)
    {
        var layout = PackageLayout.Load(packageRoot);
        return layout.TryFindBlock(address, out var block)
            ? $"{block.Module:X2}:{block.Id:X2}"
            : null;
    }

    public static RelocationTableDocument GenerateFixLevelRtb(
        string fixPackageRoot,
        string levelPackageRoot,
        string fileName,
        RelocationPackageContext? context = null)
    {
        context ??= new RelocationPackageContext();
        var fixLayout = context.GetLayout(fixPackageRoot);
        var levelLayout = context.GetLayout(levelPackageRoot);
        var resolver = new ReferenceAddressResolver(fixPackageRoot);
        resolver.LoadPackage(levelPackageRoot);
        var importedSites = LoadImportedFixLevelSites(levelPackageRoot);
        var sitesByBlock = importedSites.Sites
            .GroupBy(site => (site.SourceModule, site.SourceId))
            .ToDictionary(group => group.Key, group => group.ToList());
        var fixBlocksByKey = fixLayout.Blocks.ToDictionary(block => (block.Module, block.Id));

        var document = new RelocationTableDocument { FileName = fileName };
        foreach (var importedBlock in importedSites.Blocks.OrderBy(block => block.Order))
        {
            if (!fixBlocksByKey.TryGetValue((importedBlock.SourceModule, importedBlock.SourceId), out var sourceBlock))
            {
                continue;
            }

            var pointers = GenerateImportedFixLevelPointers(
                fixPackageRoot,
                levelPackageRoot,
                sourceBlock,
                levelLayout,
                resolver,
                sitesByBlock);
            var block = new RelocationPointerBlockManifest
            {
                Order = document.Blocks.Count,
                Key = ToKey(sourceBlock.Module, sourceBlock.Id),
                Module = sourceBlock.Module,
                Id = sourceBlock.Id,
                EntrySize = 8,
                Pointers = pointers
            };
            block.PointerDataSha256 = HashBytes(BuildPointerData(block));
            document.Blocks.Add(block);
        }

        return document;
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
        TargetBlockResolver targetResolver,
        ReferenceAddressResolver resolver)
    {
        var pointers = new List<RelocationPointerManifest>();
        var seenOffsets = new HashSet<uint>();

        foreach (var element in sourceBlock.Elements)
        {
            if (!StructCodecRegistry.TryGet(element.Kind, out var codec) ||
                (!codec.UsesExternalBinaryPayload &&
                    codec.PointerFields.Count == 0 &&
                    !codec.IsPointerArray))
            {
                continue;
            }

            if (TryEmitOpaquePointerMap(
                    pointers,
                    seenOffsets,
                    sourceBlock,
                    element,
                    codec,
                    sourcePackageRoot,
                    targetResolver))
            {
                continue;
            }

            if (!TryLoadElementData(sourcePackageRoot, element, resolver, out var data))
            {
                continue;
            }

            if (codec.PointerFields.Count == 0 && !codec.IsPointerArray)
            {
                continue;
            }

            var pointerLength = data.Length;
            if (codec.IsPointerArray)
            {
                var stride = codec.PointerEntryStride;
                if (pointerLength % stride != 0)
                {
                    pointerLength -= pointerLength % stride;
                    if (pointerLength == 0)
                    {
                        continue;
                    }
                }
            }

            var pointerData = data.AsSpan(0, pointerLength);
            var pointerFields = codec.EnumeratePointerFields(pointerData);
            EmitPointerFields(
                pointers,
                seenOffsets,
                sourceBlock,
                element,
                pointerData,
                pointerFields,
                targetResolver);
        }

        return pointers
            .OrderBy(p => p.OffsetInMemory)
            .ThenBy(p => p.TargetModule)
            .ThenBy(p => p.TargetId)
            .ToList();
    }

    private static List<RelocationPointerManifest> GenerateImportedFixLevelPointers(
        string sourcePackageRoot,
        string targetPackageRoot,
        BlockLayout sourceBlock,
        PackageLayout targetLayout,
        ReferenceAddressResolver resolver,
        IReadOnlyDictionary<(byte Module, byte Id), List<FixLevelSiteEntry>> sitesByBlock)
    {
        var pointers = new List<RelocationPointerManifest>();
        if (!sitesByBlock.TryGetValue((sourceBlock.Module, sourceBlock.Id), out var sites))
        {
            return pointers;
        }

        foreach (var site in sites.OrderBy(site => site.OffsetInMemory))
        {
            var targetModule = site.TargetModule;
            var targetId = site.TargetId;
            if (!string.IsNullOrWhiteSpace(site.TargetUri) &&
                TryResolveExplicitTargetBlock(
                    sourcePackageRoot,
                    targetPackageRoot,
                    targetLayout,
                    resolver,
                    site.TargetUri,
                    out var targetBlock))
            {
                targetModule = targetBlock.Module;
                targetId = targetBlock.Id;
            }

            pointers.Add(new RelocationPointerManifest
            {
                OffsetInMemory = site.OffsetInMemory,
                TargetModule = targetModule,
                TargetId = targetId,
                Byte6 = 0,
                Byte7 = 0
            });
        }

        return pointers;
    }

    private static bool TryResolveExplicitTargetBlock(
        string sourcePackageRoot,
        string targetPackageRoot,
        PackageLayout targetLayout,
        ReferenceAddressResolver resolver,
        string? uri,
        out BlockLayout targetBlock)
    {
        targetBlock = null!;
        if (string.IsNullOrWhiteSpace(uri) ||
            !ReferenceUri.TryResolve(
                sourcePackageRoot,
                uri,
                out var targetPath,
                out _,
                levelPackageRoot: targetPackageRoot) ||
            !IsPathWithinPackageRoot(targetPath, targetPackageRoot))
        {
            return false;
        }

        try
        {
            var address = resolver.ResolveAddress(sourcePackageRoot, uri);
            return targetLayout.TryFindBlock(address, out targetBlock);
        }
        catch (InvalidDataException)
        {
            return false;
        }
    }

    private static FixLevelSitesDocument LoadImportedFixLevelSites(string levelPackageRoot)
    {
        var manifestPath = Path.Combine(levelPackageRoot, OpenSpacePackageCodec.ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            return new FixLevelSitesDocument();
        }

        var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
            File.ReadAllText(manifestPath),
            JsonOptions);
        var sitesPath = manifest?.Semantic?.FixLevelSitesPath;
        if (string.IsNullOrWhiteSpace(sitesPath))
        {
            return new FixLevelSitesDocument();
        }

        var fullPath = Path.Combine(levelPackageRoot, sitesPath.Replace('/', Path.DirectorySeparatorChar));
        if (!File.Exists(fullPath))
        {
            return new FixLevelSitesDocument();
        }

        return JsonSerializer.Deserialize<FixLevelSitesDocument>(
            File.ReadAllText(fullPath),
            JsonOptions) ?? new FixLevelSitesDocument();
    }

    private static bool TryEmitOpaquePointerMap(
        List<RelocationPointerManifest> pointers,
        HashSet<uint> seenOffsets,
        BlockLayout sourceBlock,
        ElementLayout element,
        IStructCodecBinding codec,
        string sourcePackageRoot,
        TargetBlockResolver targetResolver)
    {
        if (!codec.UsesExternalBinaryPayload)
        {
            return false;
        }

        var elementPath = ReferenceUri.Resolve(sourcePackageRoot, element.DataPath).FilePath;
        if (!elementPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase) ||
            !File.Exists(elementPath))
        {
            return false;
        }

        var record = (OpaqueBinaryRecord)codec.ReadFromJsonPath(sourcePackageRoot, elementPath);
        if (record.Pointers.Count == 0)
        {
            return false;
        }

        foreach (var entry in record.Pointers.OrderBy(e => ParsePointerOffset(e.Key)))
        {
            if (string.IsNullOrWhiteSpace(entry.Value))
            {
                continue;
            }

            var offset = ParsePointerOffset(entry.Key);
            if (offset < 0 || offset + sizeof(int) > record.Data.Length || offset % sizeof(int) != 0)
            {
                throw new InvalidDataException(
                    $"Opaque pointer offset '{entry.Key}' is out of bounds for {elementPath}.");
            }

            var value = BinaryPrimitives.ReadInt32LittleEndian(record.Data.AsSpan(offset, sizeof(int)));
            if (!ShouldEmitRelocation(pointerField: null, value))
            {
                continue;
            }

            var offsetInMemory = checked((uint)(sourceBlock.BaseInMemory + element.OffsetInBlock + offset));
            if (!seenOffsets.Add(offsetInMemory))
            {
                continue;
            }

            var target = targetResolver.FindOpaqueTargetBlock(entry.Value, value);
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

        return true;
    }

    private static bool TryLoadElementData(
        string sourcePackageRoot,
        ElementLayout element,
        ReferenceAddressResolver resolver,
        out byte[] data)
    {
        data = [];
        var elementPath = ReferenceUri.Resolve(sourcePackageRoot, element.DataPath).FilePath;
        if (!File.Exists(elementPath))
        {
            return false;
        }

        if (elementPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
        {
            data = ReferenceJson.WriteElementBytesForExport(
                sourcePackageRoot,
                element.Kind,
                element.DataPath,
                resolver);
            return true;
        }

        data = File.ReadAllBytes(elementPath);
        return true;
    }

    private static void EmitPointerFields(
        List<RelocationPointerManifest> pointers,
        HashSet<uint> seenOffsets,
        BlockLayout sourceBlock,
        ElementLayout element,
        ReadOnlySpan<byte> data,
        IReadOnlyList<PointerField> pointerFields,
        TargetBlockResolver targetResolver)
    {
        foreach (var pointerField in pointerFields.OrderBy(f => f.Offset))
        {
            if (pointerField.Offset < 0 || pointerField.Offset + 4 > data.Length)
            {
                continue;
            }

            var value = BinaryPrimitives.ReadInt32LittleEndian(
                data.Slice(pointerField.Offset, sizeof(int)));
            if (!ShouldEmitRelocation(pointerField, value))
            {
                continue;
            }

            var offsetInMemory = checked((uint)(sourceBlock.BaseInMemory + element.OffsetInBlock + pointerField.Offset));
            if (!seenOffsets.Add(offsetInMemory))
            {
                continue;
            }

            var target = targetResolver.FindTargetBlock(value, pointerField.Target);
            if (target == null)
            {
                continue;
            }

            var targetRank = target.GetMatchRank(value);
            if (!ShouldEmitTargetMatch(pointerField, targetRank, sourceBlock, element))
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

    private static bool ShouldEmitRelocation(PointerField? pointerField, int value)
    {
        if (value == 0)
        {
            return false;
        }

        if (pointerField is not { } field)
        {
            return true;
        }

        if (IsIgnoredPointerValue(field, value))
        {
            return false;
        }

        if (field.RequiresVmRange && !VmPointerScanning.IsLikelyVirtualAddress(value))
        {
            return false;
        }

        return true;
    }

    private static bool IsIgnoredPointerValue(PointerField pointerField, int value) =>
        pointerField.IgnoreValues?.Contains(value) == true;

    private static bool AllowsPlaceholderTarget(BlockLayout sourceBlock, ElementLayout element) =>
        sourceBlock.Elements.Count == 0 ||
        element.Kind.Equals("raw", StringComparison.OrdinalIgnoreCase) ||
        element.Kind.Equals("padding", StringComparison.OrdinalIgnoreCase);

    private static bool ShouldEmitTargetMatch(
        PointerField pointerField,
        int targetRank,
        BlockLayout sourceBlock,
        ElementLayout element)
    {
        if (pointerField.RequiresDecompressedTarget && targetRank > 0)
        {
            return false;
        }

        return true;
    }

    private static bool IsPathWithinPackageRoot(string path, string packageRoot)
    {
        var relative = Path.GetRelativePath(packageRoot, path);
        return relative != ".." &&
            !relative.StartsWith($"..{Path.DirectorySeparatorChar}", StringComparison.Ordinal) &&
            !relative.StartsWith($"..{Path.AltDirectorySeparatorChar}", StringComparison.Ordinal) &&
            !Path.IsPathRooted(relative);
    }

    private static int ParsePointerOffset(string value)
    {
        if (TryParsePointerOffset(value, out var offset))
        {
            return offset;
        }

        throw new InvalidDataException($"Invalid opaque pointer offset '{value}'.");
    }

    private static bool TryParsePointerOffset(string value, out int offset)
    {
        offset = 0;
        if (string.IsNullOrWhiteSpace(value))
        {
            return false;
        }

        if (value.StartsWith("0x", StringComparison.OrdinalIgnoreCase))
        {
            return int.TryParse(
                value.AsSpan(2),
                System.Globalization.NumberStyles.HexNumber,
                System.Globalization.CultureInfo.InvariantCulture,
                out offset);
        }

        return int.TryParse(value, out offset);
    }

    private static BlockLayout? FindBestTargetBlock(
        IEnumerable<PackageLayout> layouts,
        int address,
        string? sourcePackageRoot)
    {
        BlockLayout? bestBlock = null;
        PackageLayout? bestLayout = null;
        var bestRank = int.MaxValue;
        var bestSpan = int.MaxValue;

        foreach (var layout in layouts)
        {
            if (!layout.TryFindBlock(address, out var block))
            {
                continue;
            }

            var rank = block.GetMatchRank(address);
            var span = block.GetMatchSpan(address);
            if (bestBlock == null || rank < bestRank || (rank == bestRank && span < bestSpan))
            {
                bestRank = rank;
                bestSpan = span;
                bestBlock = block;
                bestLayout = layout;
            }
            else if (rank == bestRank && span == bestSpan && bestLayout != null)
            {
                if (PreferLayoutOnTie(layout, bestLayout, sourcePackageRoot))
                {
                    bestBlock = block;
                    bestLayout = layout;
                }
            }
        }

        return bestBlock;
    }

    private static bool PreferLayoutOnTie(
        PackageLayout candidate,
        PackageLayout current,
        string? sourcePackageRoot)
    {
        var candidateIsLevel = candidate.PackageRole.Equals("level", StringComparison.OrdinalIgnoreCase);
        var currentIsLevel = current.PackageRole.Equals("level", StringComparison.OrdinalIgnoreCase);
        if (candidateIsLevel != currentIsLevel)
        {
            return candidateIsLevel;
        }

        if (sourcePackageRoot == null)
        {
            return false;
        }

        var candidateIsSource = candidate.PackageRoot
            .Equals(sourcePackageRoot, StringComparison.OrdinalIgnoreCase);
        var currentIsSource = current.PackageRoot
            .Equals(sourcePackageRoot, StringComparison.OrdinalIgnoreCase);
        return candidateIsSource && !currentIsSource;
    }

    private static List<RelocationPointerManifest> GeneratePointerFileEntries(
        string pointerFilePath,
        TargetBlockResolver targetResolver)
    {
        var data = File.ReadAllBytes(pointerFilePath);
        var pointers = new List<RelocationPointerManifest>();
        var seenEntries = new HashSet<(int FileOffset, uint Value)>();

        for (var offset = 0; offset <= data.Length - sizeof(uint); offset += sizeof(uint))
        {
            var value = BinaryPrimitives.ReadUInt32LittleEndian(data.AsSpan(offset, sizeof(uint)));
            if (!seenEntries.Add((offset, value)) ||
                !ShouldEmitRelocation(pointerField: null, unchecked((int)value)))
            {
                continue;
            }

            var target = targetResolver.FindTargetBlock(unchecked((int)value), PointerTarget.Any);
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
        var preservedBlocks = preserved.Blocks.ToDictionary(b => (b.Module, b.Id));
        var generatedBlocks = generated.Blocks.ToDictionary(b => (b.Module, b.Id));
        if (preservedBlocks.Count != generatedBlocks.Count)
        {
            return false;
        }

        foreach (var (key, preservedBlock) in preservedBlocks)
        {
            if (!generatedBlocks.TryGetValue(key, out var generatedBlock) ||
                preservedBlock.Pointers.Count != generatedBlock.Pointers.Count)
            {
                return false;
            }

            if (!BuildOrderedPointerData(preservedBlock).AsSpan()
                    .SequenceEqual(BuildOrderedPointerData(generatedBlock)))
            {
                return false;
            }
        }

        return true;
    }

    private static byte[] BuildOrderedPointerData(RelocationPointerBlockManifest block)
    {
        var entrySize = block.EntrySize >= 8 ? 8 : 6;
        using var stream = new MemoryStream();
        using var writer = new BinaryWriter(stream);
        foreach (var pointer in block.Pointers
                     .OrderBy(p => p.OffsetInMemory)
                     .ThenBy(p => p.TargetModule)
                     .ThenBy(p => p.TargetId)
                     .ThenBy(p => p.Byte6)
                     .ThenBy(p => p.Byte7))
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

    internal sealed class PackageLayout
    {
        internal string PackageRoot { get; private init; } = "";
        internal string PackageRole { get; private init; } = "level";
        internal List<BlockLayout> Blocks { get; } = new();
        private ReferenceAddressResolver Resolver { get; init; } = null!;

        internal static PackageLayout Load(string packageRoot)
        {
            var manifestPath = Path.Combine(packageRoot, OpenSpacePackageCodec.ManifestFileName);
            var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                File.ReadAllText(manifestPath),
                JsonOptions) ?? throw new InvalidDataException($"Could not read Rete manifest: {manifestPath}");
            var layout = new PackageLayout
            {
                PackageRoot = Path.GetFullPath(packageRoot),
                PackageRole = manifest.PackageRole,
                Resolver = ReferenceAddressResolver.CreateForExport(packageRoot)
            };

            foreach (var snaFile in manifest.SnaFiles)
            {
                foreach (var block in snaFile.Blocks.OrderBy(b => b.Order))
                {
                    if (block.BaseInMemory < 0)
                    {
                        continue;
                    }

                    if (block.ContentPath == null)
                    {
                        layout.Blocks.Add(new BlockLayout(
                            block.Order,
                            block.Module,
                            block.Id,
                            block.BaseInMemory,
                            unchecked((int)block.Unk2),
                            GetDecompressedSize(block),
                            [],
                            block.OriginalStorage));
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
                            : DetermineElementLength(packageRoot, element, layout.Resolver);

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
                        unchecked((int)block.Unk2),
                        GetDecompressedSize(block),
                        elements,
                        block.OriginalStorage));
                }
            }

            return layout;
        }

        private static int GetDecompressedSize(SnaBlockManifest block) =>
            block.OriginalStorage?.DecompressedSize is { } size
                ? unchecked((int)size)
                : 0;

        internal bool TryFindBlock(int address, out BlockLayout block)
        {
            BlockLayout? best = null;
            var bestRank = int.MaxValue;
            var bestSpan = int.MaxValue;

            foreach (var candidate in Blocks)
            {
                if (!candidate.ContainsAddress(address))
                {
                    continue;
                }

                var rank = candidate.GetMatchRank(address);
                var span = candidate.GetMatchSpan(address);
                if (rank < bestRank || (rank == bestRank && span < bestSpan))
                {
                    bestRank = rank;
                    bestSpan = span;
                    best = candidate;
                }
            }

            if (best != null)
            {
                block = best;
                return true;
            }

            block = null!;
            return false;
        }

        internal bool ContainsAddress(int address) => TryFindBlock(address, out _);

        internal bool ContainsAllocatedAddress(int address) =>
            TryFindBlock(address, out var block) && block.GetMatchRank(address) <= 1;

        internal void WarmBlockByteCache()
        {
            foreach (var block in Blocks)
            {
                if (block.DecompressedSize > 0)
                {
                    TryReadBlockBytes(block, out _);
                }
            }
        }

        internal bool TryReadInt32(int virtualAddress, out int value)
        {
            value = 0;
            if (!TryFindBlock(virtualAddress, out var block))
            {
                return false;
            }

            var offsetInBlock = virtualAddress - block.BaseInMemory;

            // Prefer preserved decompressed SNA bytes. Fix.rtb walks hundreds of
            // thousands of sites; re-serializing JSON elements per lookup is prohibitive.
            if (block.DecompressedSize > 0 &&
                offsetInBlock >= 0 &&
                offsetInBlock + sizeof(int) <= block.DecompressedSize &&
                TryReadBlockBytes(block, out var blockBytes))
            {
                value = BinaryPrimitives.ReadInt32LittleEndian(
                    blockBytes.AsSpan(offsetInBlock, sizeof(int)));
                return true;
            }

            foreach (var element in block.Elements)
            {
                if (offsetInBlock < element.OffsetInBlock ||
                    offsetInBlock + sizeof(int) > element.OffsetInBlock + element.Length)
                {
                    continue;
                }

                var elementOffset = offsetInBlock - element.OffsetInBlock;
                var bytes = ReadElementBytes(element);
                value = BinaryPrimitives.ReadInt32LittleEndian(bytes.AsSpan(elementOffset, sizeof(int)));
                return true;
            }

            return false;
        }

        private readonly Dictionary<(byte Module, byte Id), byte[]> _blockByteCache = new();

        private bool TryReadBlockBytes(BlockLayout block, out byte[] bytes)
        {
            var key = (block.Module, block.Id);
            if (_blockByteCache.TryGetValue(key, out bytes!))
            {
                return true;
            }

            if (TryLoadOriginalDecompressedBlock(block, out bytes) ||
                TryReconstructBlockBytes(block, out bytes))
            {
                _blockByteCache[key] = bytes;
                return true;
            }

            bytes = [];
            return false;
        }

        private bool TryLoadOriginalDecompressedBlock(BlockLayout block, out byte[] bytes)
        {
            bytes = [];
            var storage = block.OriginalStorage;
            if (storage?.EncodedPath == null || block.DecompressedSize <= 0)
            {
                return false;
            }

            var encodedPath = ResolvePackagePath(PackageRoot, storage.EncodedPath);
            if (!File.Exists(encodedPath))
            {
                return false;
            }

            var encoded = File.ReadAllBytes(encodedPath);
            bytes = storage.IsCompressed
                ? DecompressLzo(encoded, block.DecompressedSize)
                : encoded;
            return bytes.Length >= sizeof(int);
        }

        private bool TryReconstructBlockBytes(BlockLayout block, out byte[] bytes)
        {
            bytes = [];
            if (block.DecompressedSize <= 0)
            {
                return false;
            }

            bytes = new byte[block.DecompressedSize];
            foreach (var element in block.Elements)
            {
                if (element.OffsetInBlock < 0 ||
                    element.OffsetInBlock + element.Length > bytes.Length)
                {
                    continue;
                }

                var elementBytes = ReadElementBytes(element);
                var copyLength = Math.Min(elementBytes.Length, element.Length);
                if (copyLength > 0)
                {
                    elementBytes.AsSpan(0, copyLength).CopyTo(
                        bytes.AsSpan(element.OffsetInBlock, copyLength));
                }
            }

            return true;
        }

        private static byte[] DecompressLzo(byte[] compressedData, int decompressedSize)
        {
            using var inputStream = new MemoryStream(compressedData);
            using var lzoStream = new LzoStream(inputStream, CompressionMode.Decompress);
            using var outputStream = new MemoryStream();

            var buffer = new byte[4096];
            int bytesRead;
            while ((bytesRead = lzoStream.Read(buffer, 0, buffer.Length)) > 0)
            {
                outputStream.Write(buffer, 0, bytesRead);
            }

            var data = outputStream.ToArray();
            if (data.Length > decompressedSize)
            {
                Array.Resize(ref data, decompressedSize);
            }

            return data;
        }

        private byte[] ReadElementBytes(ElementLayout element) =>
            ReferenceJson.WriteElementBytesForExport(
                PackageRoot,
                element.Kind,
                element.DataPath,
                Resolver);

        private static int DetermineElementLength(
            string packageRoot,
            SnaBlockContentElement element,
            ReferenceAddressResolver resolver)
        {
            if (StructCodecRegistry.TryGet(element.Kind, out var codec) && codec.FixedSize is { } fixedSize)
            {
                return fixedSize;
            }

            var dataPath = ReferenceUri.Resolve(packageRoot, element.DataPath).FilePath;
            if (StructCodecRegistry.TryGet(element.Kind, out codec) &&
                dataPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
            {
                return ReferenceJson.WriteElementBytesForExport(
                    packageRoot,
                    element.Kind,
                    element.DataPath,
                    resolver).Length;
            }

            return checked((int)new FileInfo(dataPath).Length);
        }

        private static string ResolvePackagePath(string packageRoot, string relativePath) =>
            Path.Combine(relativePath.Split('/').Prepend(packageRoot).ToArray());
    }

    private sealed class TargetBlockResolver
    {
        private readonly string _sourcePackageRoot;
        private readonly IReadOnlyList<PackageLayout> _allLayouts;
        private readonly IReadOnlyList<PackageLayout> _sourceLayouts;
        private readonly IReadOnlyList<PackageLayout> _fixLayouts;
        private readonly IReadOnlyList<PackageLayout> _blockRelativeFallbackLayouts;
        private readonly ReferenceAddressResolver _referenceResolver;
        private readonly Dictionary<(string TargetUri, int Address), BlockLayout?> _opaqueTargetCache =
            new();
        private readonly Dictionary<string, PackageLayout?> _opaqueLayoutCache =
            new(StringComparer.Ordinal);
        private readonly Dictionary<string, OpaqueTargetAddressResolution> _opaqueAddressCache =
            new(StringComparer.Ordinal);

        public TargetBlockResolver(string sourcePackageRoot, IReadOnlyList<PackageLayout> layouts)
        {
            _sourcePackageRoot = Path.GetFullPath(sourcePackageRoot);
            _allLayouts = layouts;
            _sourceLayouts = layouts
                .Where(layout => layout.PackageRoot.Equals(_sourcePackageRoot, StringComparison.OrdinalIgnoreCase))
                .ToList();
            _fixLayouts = layouts
                .Where(layout => layout.PackageRole.Equals("fix", StringComparison.OrdinalIgnoreCase))
                .ToList();
            _blockRelativeFallbackLayouts = _fixLayouts
                .Where(layout => !layout.PackageRoot.Equals(_sourcePackageRoot, StringComparison.OrdinalIgnoreCase))
                .ToList();
            _referenceResolver = new ReferenceAddressResolver(_sourcePackageRoot);
            foreach (var layout in layouts)
            {
                _referenceResolver.LoadPackage(layout.PackageRoot);
            }
        }

        public BlockLayout? FindTargetBlock(int address, PointerTarget pointerTarget)
        {
            var best = FindBestTargetBlock(GetTargetLayouts(pointerTarget), address, _sourcePackageRoot);
            if (best != null)
            {
                return best;
            }

            if (pointerTarget == PointerTarget.BlockRelative)
            {
                return FindBestTargetBlock(_blockRelativeFallbackLayouts, address, _sourcePackageRoot);
            }

            return null;
        }

        public BlockLayout? FindOpaqueTargetBlock(string targetUri, int address)
        {
            var cacheKey = (targetUri, address);
            if (_opaqueTargetCache.TryGetValue(cacheKey, out var cached))
            {
                return cached;
            }

            var lookupAddress = address;
            if (TryResolveOpaqueTargetAddress(targetUri, out var resolvedAddress))
            {
                lookupAddress = resolvedAddress;
            }

            var targetBlock = FindExplicitOpaqueTargetBlock(targetUri, lookupAddress)
                ?? FindTargetBlock(lookupAddress, PointerTarget.Any);
            _opaqueTargetCache[cacheKey] = targetBlock;
            return targetBlock;
        }

        private IReadOnlyList<PackageLayout> GetTargetLayouts(PointerTarget pointerTarget) =>
            pointerTarget switch
            {
                PointerTarget.BlockRelative => _sourceLayouts,
                PointerTarget.Fix => _fixLayouts,
                _ => _allLayouts
            };

        private PackageLayout? GetOpaqueTargetLayout(string targetUri)
        {
            if (_opaqueLayoutCache.TryGetValue(targetUri, out var cached))
            {
                return cached;
            }

            PackageLayout? targetLayout = null;
            if (ReferenceUri.TryResolve(_sourcePackageRoot, targetUri, out var targetPath, out _))
            {
                targetLayout = _allLayouts
                    .Where(layout => IsPathWithinPackageRoot(targetPath, layout.PackageRoot))
                    .OrderByDescending(layout => layout.PackageRoot.Length)
                    .FirstOrDefault();
            }

            _opaqueLayoutCache[targetUri] = targetLayout;
            return targetLayout;
        }

        private bool TryResolveOpaqueTargetAddress(string targetUri, out int address)
        {
            if (_opaqueAddressCache.TryGetValue(targetUri, out var cached))
            {
                address = cached.Address;
                return cached.Success;
            }

            try
            {
                address = _referenceResolver.ResolveAddress(_sourcePackageRoot, targetUri);
                _opaqueAddressCache[targetUri] = new OpaqueTargetAddressResolution(true, address);
                return true;
            }
            catch (InvalidDataException)
            {
            }

            address = 0;
            _opaqueAddressCache[targetUri] = new OpaqueTargetAddressResolution(false, 0);
            return false;
        }

        private BlockLayout? FindExplicitOpaqueTargetBlock(string targetUri, int address)
        {
            var targetLayout = GetOpaqueTargetLayout(targetUri);
            if (targetLayout != null && targetLayout.TryFindBlock(address, out var explicitTarget))
            {
                return explicitTarget;
            }

            return null;
        }
    }

    private readonly record struct OpaqueTargetAddressResolution(bool Success, int Address);

    internal sealed record BlockLayout(
        int Order,
        byte Module,
        byte Id,
        int BaseInMemory,
        int MaxVmAddress,
        int DecompressedSize,
        IReadOnlyList<ElementLayout> Elements,
        SnaStorageManifest? OriginalStorage = null)
    {
        public int Length => Elements.Count == 0
            ? 0
            : Elements.Max(e => e.OffsetInBlock + e.Length);

        public int EndInMemory => checked(BaseInMemory + Length);

        public bool ContainsAddress(int address)
        {
            if (address < BaseInMemory)
            {
                return false;
            }

            if (DecompressedSize > 0 && address < checked(BaseInMemory + DecompressedSize))
            {
                return true;
            }

            if (Length > 0 && address < EndInMemory)
            {
                return true;
            }

            return MaxVmAddress > BaseInMemory && address <= MaxVmAddress;
        }

        public int GetMatchRank(int address)
        {
            if (DecompressedSize > 0 && address < checked(BaseInMemory + DecompressedSize))
            {
                return 0;
            }

            if (Length > 0 && address < EndInMemory)
            {
                return 1;
            }

            return 2;
        }

        public int GetMatchSpan(int address) =>
            GetMatchRank(address) switch
            {
                0 => DecompressedSize,
                1 => Length,
                _ => MaxVmAddress > BaseInMemory ? MaxVmAddress - BaseInMemory : 0
            };
    }

    internal sealed record ElementLayout(
        int Order,
        string Kind,
        string DataPath,
        int OffsetInBlock,
        int Length);

    internal sealed class RelocationPackageContext
    {
        private readonly Dictionary<string, PackageLayout> _layouts =
            new(StringComparer.OrdinalIgnoreCase);

        public void EnsureLayout(string packageRoot) => _ = GetLayout(packageRoot);

        internal PackageLayout GetLayout(string packageRoot)
        {
            var normalized = Path.GetFullPath(packageRoot);
            if (!_layouts.TryGetValue(normalized, out var layout))
            {
                layout = PackageLayout.Load(normalized);
                _layouts[normalized] = layout;
            }

            return layout;
        }
    }
}
