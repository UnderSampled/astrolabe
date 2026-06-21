using System.Buffers.Binary;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Text.Json;
using Astrolabe.Core;
using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Geometry;
using Astrolabe.Core.Hub;
using Astrolabe.Core.Rete.OpenSpace;
using Astrolabe.Core.Serialization;
using Astrolabe.Core.Serialization.Codecs;

namespace Astrolabe.Core.Rete;

internal static class OpenSpacePackageCodec
{
    public const string ManifestFileName = "manifest.json";
    public const string ReteManifestSchema = "astrolabe.rete.v1";
    private const string FixFamilyPrefix = "Fix.";

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    private static readonly HashSet<string> RelocationExtensions = new(StringComparer.OrdinalIgnoreCase)
    {
        ".rtb",
        ".rtp",
        ".rts",
        ".rtt",
        ".rtl",
        ".rtd",
        ".rtg",
        ".rtv"
    };

    public static RetePackageManifest ImportLevel(string levelDir, string outputDir)
    {
        if (!Directory.Exists(levelDir))
        {
            throw new DirectoryNotFoundException($"Level directory not found: {levelDir}");
        }

        var levelName = Path.GetFileName(levelDir.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
        var fixPackageDir = ImportSiblingFixPackageIfAvailable(levelDir, outputDir);
        var manifest = ImportPackage(levelDir, levelName, outputDir, "level", _ => true);
        RewritePointerReferences(outputDir, fixPackageDir);
        AnnotateOpaquePointersFromSourceRelocations(outputDir, fixPackageDir, levelDir);
        if (!string.IsNullOrWhiteSpace(fixPackageDir))
        {
            var levelsDir = Directory.GetParent(Path.GetFullPath(levelDir))?.FullName;
            if (levelsDir != null)
            {
                RemoveNullOpaquePointerLutEntries(fixPackageDir);
                AnnotateOpaquePointersFromSourceRelocations(fixPackageDir, outputDir, levelsDir);
            }

            AnnotateOpaquePointersFromFixLevelRelocations(fixPackageDir, outputDir, levelDir);
        }

        return manifest;
    }

    private static RetePackageManifest ImportPackage(
        string sourceDir,
        string packageName,
        string outputDir,
        string packageRole,
        Func<string, bool> includeFile)
    {
        Directory.CreateDirectory(outputDir);
        PruneLegacyRelocationArtifacts(outputDir);

        var manifest = new RetePackageManifest
        {
            Schema = ReteManifestSchema,
            PackageRole = packageRole,
            LevelName = packageName,
            SourceDirectoryName = Path.GetFileName(sourceDir.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar))
        };

        var handledFiles = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var files = Directory.GetFiles(sourceDir)
            .Where(includeFile)
            .OrderBy(Path.GetFileName, StringComparer.OrdinalIgnoreCase)
            .ToList();
        var semanticContext = TryBuildSemanticContext(sourceDir, packageName);
        var sceneNodePaths = WriteSceneSourceTree(outputDir, semanticContext);

        foreach (var snaPath in files.Where(f => Path.GetExtension(f).Equals(".sna", StringComparison.OrdinalIgnoreCase)))
        {
            manifest.SnaFiles.Add(ExtractSnaFile(snaPath, outputDir, semanticContext?.Tracker, sceneNodePaths));
            handledFiles.Add(snaPath);
        }

        var importErrors = new List<string>();
        foreach (var relocationPath in files.Where(f => RelocationExtensions.Contains(Path.GetExtension(f))))
        {
            try
            {
                manifest.RelocationTables.Add(ExtractRelocationTable(relocationPath, outputDir));
                handledFiles.Add(relocationPath);
                if (IsUnsupportedRelocationTable(Path.GetFileName(relocationPath)))
                {
                    manifest.LooseFiles.Add(CopyLooseFile(relocationPath, outputDir));
                }
            }
            catch (Exception ex)
            {
                // Some RT* files can be tiny placeholders in shipped data. Keep
                // unsupported tables as exact loose leaves rather than dropping them.
                importErrors.Add($"Relocation table {Path.GetFileName(relocationPath)}: {ex.Message}");
            }
        }

        foreach (var file in files.Where(f => !handledFiles.Contains(f)))
        {
            manifest.LooseFiles.Add(CopyLooseFile(file, outputDir));
        }

        manifest.Semantic = WriteSemanticMetadata(packageName, outputDir, semanticContext);
        manifest.Semantic.Errors.AddRange(importErrors);

        WriteJson(Path.Combine(outputDir, ManifestFileName), manifest);
        return manifest;
    }

    private static string? ImportSiblingFixPackageIfAvailable(string levelDir, string levelOutputDir)
    {
        var levelsDir = Directory.GetParent(Path.GetFullPath(levelDir))?.FullName;
        if (levelsDir == null || FindFile(levelsDir, "Fix.sna") == null)
        {
            return null;
        }

        var outputParent = Directory.GetParent(Path.GetFullPath(levelOutputDir))?.FullName;
        if (outputParent == null)
        {
            return null;
        }

        var fixOutputDir = Path.Combine(outputParent, "fix");
        if (Path.GetFullPath(fixOutputDir).Equals(Path.GetFullPath(levelOutputDir), StringComparison.OrdinalIgnoreCase))
        {
            return null;
        }

        var fixManifestPath = Path.Combine(fixOutputDir, ManifestFileName);
        if (!File.Exists(fixManifestPath))
        {
            ImportPackage(levelsDir, "Fix", fixOutputDir, "fix", IsFixFile);
        }

        RewritePointerReferences(fixOutputDir, null);
        AnnotateOpaquePointersFromSourceRelocations(fixOutputDir, null, levelsDir);
        return fixOutputDir;
    }

    private static bool IsFixFile(string filePath) =>
        Path.GetFileName(filePath).StartsWith(FixFamilyPrefix, StringComparison.Ordinal);

    private static void RewritePointerReferences(string packageDir, string? extraPackageDir)
    {
        var manifestPath = Path.Combine(packageDir, ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            return;
        }

        var resolver = new ReferenceAddressResolver(packageDir);
        if (!string.IsNullOrWhiteSpace(extraPackageDir))
        {
            resolver.LoadPackage(extraPackageDir);
        }

        var manifest = ReadJson<RetePackageManifest>(manifestPath);
        foreach (var snaFile in manifest.SnaFiles)
        {
            foreach (var block in snaFile.Blocks)
            {
                if (block.ContentPath == null)
                {
                    continue;
                }

                var content = ReadJson<SnaBlockContentDocument>(ResolvePath(packageDir, block.ContentPath));
                foreach (var element in content.Elements)
                {
                    if (!StructCodecRegistry.TryGet(element.Kind, out var codec) ||
                        (codec.PointerFields.Count == 0 && !codec.IsPointerArray))
                    {
                        continue;
                    }

                    var elementPath = ReferenceUri.Resolve(packageDir, element.DataPath).FilePath;
                    if (!elementPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase) ||
                        !File.Exists(elementPath))
                    {
                        continue;
                    }

                    ReferenceJson.RewritePointersToUris(
                        elementPath,
                        packageDir,
                        codec,
                        resolver);
                }
            }
        }

        RewriteScenePointerReferences(packageDir, resolver);
    }

    private static void RewriteScenePointerReferences(string packageDir, ReferenceAddressResolver resolver)
    {
        var sceneDir = Path.Combine(packageDir, "scene");
        if (!Directory.Exists(sceneDir))
        {
            return;
        }

        if (!StructCodecRegistry.TryGet("superObject", out var sceneCodec))
        {
            return;
        }

        foreach (var nodePath in Directory.EnumerateFiles(sceneDir, "node.json", SearchOption.AllDirectories))
        {
            ReferenceJson.RewritePointersToUris(nodePath, packageDir, sceneCodec, resolver);
        }
    }

    private static void AnnotateOpaquePointersFromSourceRelocations(
        string packageDir,
        string? extraPackageDir,
        string? sourceDir)
    {
        var manifestPath = Path.Combine(packageDir, ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            return;
        }

        var resolver = new ReferenceAddressResolver(packageDir);
        if (!string.IsNullOrWhiteSpace(extraPackageDir))
        {
            resolver.LoadPackage(extraPackageDir);
        }

        var manifest = ReadJson<RetePackageManifest>(manifestPath);
        var isFixPackage = manifest.PackageRole.Equals("fix", StringComparison.OrdinalIgnoreCase);
        var sourceElements = LoadElementIndex(packageDir, manifest);
        var blockByteCache = BuildImportBlockByteCache(packageDir, manifest);
        var pendingRecords = new Dictionary<string, PendingOpaquePointerRecord>(StringComparer.OrdinalIgnoreCase);
        var pendingStructPointers =
            new Dictionary<string, Dictionary<string, string?>>(StringComparer.OrdinalIgnoreCase);

        foreach (var table in manifest.RelocationTables)
        {
            if (!Path.GetExtension(table.FileName).Equals(".rtb", StringComparison.OrdinalIgnoreCase) ||
                table.FileName.Equals("fixlvl.rtb", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            if (!TryLoadTransientSourceRelocationTable(
                    packageDir,
                    manifest,
                    table,
                    sourceDir,
                    out var relocationTable))
            {
                continue;
            }

            foreach (var block in relocationTable.Blocks)
            {
                if (!sourceElements.TryGetValue((block.Module, block.Id), out var elements))
                {
                    continue;
                }

                foreach (var pointer in block.Pointers)
                {
                    var sourceAddress = checked((int)pointer.OffsetInMemory);
                    var element = FindElementAt(elements, sourceAddress);
                    if (element == null || !StructCodecRegistry.TryGet(element.Kind, out var codec))
                    {
                        continue;
                    }

                    var elementPath = ReferenceUri.Resolve(packageDir, element.DataPath).FilePath;
                    if (!elementPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase) ||
                        !File.Exists(elementPath))
                    {
                        continue;
                    }

                    var offset = sourceAddress - element.VirtualAddress;
                    var key = ReferenceJson.FormatPointerOffset(offset);
                    var isDiscSentinel =
                        pointer.TargetModule == 0xFF && pointer.TargetId == 0xFF;
                    if (codec.UsesExternalBinaryPayload)
                    {
                        if (isDiscSentinel && isFixPackage)
                        {
                            continue;
                        }

                        if (!pendingRecords.TryGetValue(elementPath, out var pending))
                        {
                            pending = new PendingOpaquePointerRecord(
                                codec,
                                (OpaqueBinaryRecord)codec.ReadFromJsonPath(packageDir, elementPath));
                            pendingRecords[elementPath] = pending;
                        }

                        blockByteCache.TryGetValue((block.Module, block.Id), out var blockBytes);
                        var spanLength = GetImportPointerSpanLength(element, blockBytes);
                        if (!ReferenceJson.TryValidateRelocationPointerOffset(offset, spanLength, out _))
                        {
                            continue;
                        }

                        string? uri = null;
                        if (!isDiscSentinel)
                        {
                            if (!TryReadOpaqueImportPointerValue(
                                    element,
                                    offset,
                                    blockBytes,
                                    pending.Record.Data,
                                    out var value))
                            {
                                continue;
                            }

                            if (value != 0 &&
                                resolver.TryGetReferenceUri(value, packageDir, out var resolvedUri))
                            {
                                uri = resolvedUri;
                            }
                        }

                        if (ReferenceJson.MergePointerLut(pending.Record.Pointers, key, uri))
                        {
                            pending.Changed = true;
                        }
                    }
                    else
                    {
                        blockByteCache.TryGetValue((block.Module, block.Id), out var blockBytes);
                        var spanLength = GetImportPointerSpanLength(element, blockBytes);
                        if (!ReferenceJson.TryValidateRelocationPointerOffset(offset, spanLength, out _))
                        {
                            continue;
                        }

                        string? uri = null;
                        if (!isDiscSentinel &&
                            TryReadImportPointerValue(
                                element,
                                offset,
                                blockBytes,
                                packageDir,
                                resolver,
                                out var value) &&
                            value != 0 &&
                            resolver.TryGetReferenceUri(value, packageDir, out var resolvedUri))
                        {
                            uri = resolvedUri;
                        }

                        if (!pendingStructPointers.TryGetValue(elementPath, out var structPointers))
                        {
                            structPointers = new Dictionary<string, string?>(StringComparer.OrdinalIgnoreCase);
                            pendingStructPointers[elementPath] = structPointers;
                        }

                        ReferenceJson.MergePointerLut(structPointers, key, uri);
                    }
                }
            }
        }

        foreach (var (elementPath, pending) in pendingRecords)
        {
            if (pending.Changed)
            {
                pending.Codec.WriteJson(packageDir, elementPath, pending.Record);
            }
        }

        foreach (var (elementPath, structPointers) in pendingStructPointers)
        {
            ReferenceJson.ApplyStructPointerLut(elementPath, structPointers);
        }
    }

    private static bool TryLoadTransientSourceRelocationTable(
        string packageDir,
        RetePackageManifest manifest,
        RelocationTableFileManifest table,
        string? sourceDir,
        out RelocationTableDocument document)
    {
        document = new RelocationTableDocument { FileName = table.FileName };
        if (!string.IsNullOrWhiteSpace(sourceDir))
        {
            var sourcePath = ResolveTransientRelocationSourcePath(sourceDir, manifest, table.FileName);
            if (sourcePath != null && File.Exists(sourcePath))
            {
                document = ReadRelocationTableFromDisc(sourcePath);
                return true;
            }
        }

        return TryLoadSourceRelocationTable(packageDir, manifest, table, out document, out _);
    }

    private static string? ResolveTransientRelocationSourcePath(
        string sourceDir,
        RetePackageManifest manifest,
        string fileName)
    {
        var directPath = Path.Combine(sourceDir, fileName);
        if (File.Exists(directPath))
        {
            return directPath;
        }

        if (manifest.PackageRole.Equals("fix", StringComparison.OrdinalIgnoreCase))
        {
            var sharedPath = Path.Combine(sourceDir, fileName);
            if (File.Exists(sharedPath))
            {
                return sharedPath;
            }
        }

        var levelsParent = Directory.GetParent(sourceDir)?.FullName;
        if (levelsParent == null)
        {
            return null;
        }

        var sharedLevelPath = Path.Combine(levelsParent, fileName);
        return File.Exists(sharedLevelPath) ? sharedLevelPath : null;
    }

    private static void RemoveNullOpaquePointerLutEntries(string packageDir)
    {
        var manifestPath = Path.Combine(packageDir, ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            return;
        }

        var manifest = ReadJson<RetePackageManifest>(manifestPath);
        foreach (var element in EnumeratePackageElements(packageDir, manifest))
        {
            if (!StructCodecRegistry.TryGet(element.Kind, out var codec) ||
                !codec.UsesExternalBinaryPayload)
            {
                continue;
            }

            var elementPath = ReferenceUri.Resolve(packageDir, element.DataPath).FilePath;
            if (!elementPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase) ||
                !File.Exists(elementPath))
            {
                continue;
            }

            var record = (OpaqueBinaryRecord)codec.ReadFromJsonPath(packageDir, elementPath);
            var nullKeys = record.Pointers
                .Where(pair => string.IsNullOrWhiteSpace(pair.Value))
                .Select(pair => pair.Key)
                .ToList();
            if (nullKeys.Count == 0)
            {
                continue;
            }

            foreach (var key in nullKeys)
            {
                record.Pointers.Remove(key);
            }

            codec.WriteJson(packageDir, elementPath, record);
        }
    }

    private static IEnumerable<SnaBlockContentElement> EnumeratePackageElements(
        string packageDir,
        RetePackageManifest manifest)
    {
        foreach (var snaFile in manifest.SnaFiles)
        {
            foreach (var block in snaFile.Blocks)
            {
                if (block.ContentPath == null)
                {
                    continue;
                }

                var contentPath = ResolvePath(packageDir, block.ContentPath);
                if (!File.Exists(contentPath))
                {
                    continue;
                }

                var content = ReadJson<SnaBlockContentDocument>(contentPath);
                foreach (var element in content.Elements)
                {
                    yield return element;
                }
            }
        }
    }

    private static void AnnotateOpaquePointersFromFixLevelRelocations(
        string fixPackageDir,
        string levelPackageDir,
        string levelSourceDir)
    {
        var fixManifestPath = Path.Combine(fixPackageDir, ManifestFileName);
        var levelManifestPath = Path.Combine(levelPackageDir, ManifestFileName);
        if (!File.Exists(fixManifestPath) || !File.Exists(levelManifestPath))
        {
            return;
        }

        PruneLegacyRelocationArtifacts(fixPackageDir);

        var levelManifest = ReadJson<RetePackageManifest>(levelManifestPath);
        if (levelManifest.RelocationTables.All(table =>
                !table.FileName.Equals("fixlvl.rtb", StringComparison.OrdinalIgnoreCase)))
        {
            return;
        }

        var fixLevelSourcePath = Path.Combine(levelSourceDir, "fixlvl.rtb");
        if (!File.Exists(fixLevelSourcePath))
        {
            levelManifest.Semantic ??= new SemanticManifest();
            levelManifest.Semantic.Errors.Add(
                $"fixlvl.rtb is listed in relocationTables but was not found at {fixLevelSourcePath}.");
            WriteJson(levelManifestPath, levelManifest);
            return;
        }

        var resolver = new ReferenceAddressResolver(fixPackageDir);
        resolver.LoadPackage(levelPackageDir);

        var fixManifest = ReadJson<RetePackageManifest>(fixManifestPath);
        var fixElements = LoadElementIndex(fixPackageDir, fixManifest);
        var blockByteCache = BuildImportBlockByteCache(fixPackageDir, fixManifest);
        var relocationTable = ReadRelocationTableFromDisc(fixLevelSourcePath);
        levelManifest.FixlvlBlockKeys = relocationTable.Blocks
            .Select(block => $"{block.Module:X2}:{block.Id:X2}")
            .ToList();
        WriteJson(levelManifestPath, levelManifest);
        var pendingRecords = new Dictionary<string, PendingOpaquePointerRecord>(StringComparer.OrdinalIgnoreCase);

        foreach (var block in relocationTable.Blocks)
        {
            if (!fixElements.TryGetValue((block.Module, block.Id), out var elements))
            {
                continue;
            }

            foreach (var pointer in block.Pointers)
            {
                var sourceAddress = checked((int)pointer.OffsetInMemory);
                var element = FindElementAt(elements, sourceAddress);
                if (element == null ||
                    !StructCodecRegistry.TryGet(element.Kind, out var codec) ||
                    !codec.UsesExternalBinaryPayload)
                {
                    continue;
                }

                var elementPath = ReferenceUri.Resolve(fixPackageDir, element.DataPath).FilePath;
                if (!elementPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase) ||
                    !File.Exists(elementPath))
                {
                    continue;
                }

                if (!pendingRecords.TryGetValue(elementPath, out var pending))
                {
                    pending = new PendingOpaquePointerRecord(
                        codec,
                        (OpaqueBinaryRecord)codec.ReadFromJsonPath(fixPackageDir, elementPath));
                    pendingRecords[elementPath] = pending;
                }

                var record = pending.Record;
                var offset = sourceAddress - element.VirtualAddress;
                var key = ReferenceJson.FormatPointerOffset(offset);
                blockByteCache.TryGetValue((block.Module, block.Id), out var blockBytes);
                var spanLength = GetImportPointerSpanLength(element, blockBytes);
                if (!ReferenceJson.TryValidateRelocationPointerOffset(offset, spanLength, out _))
                {
                    continue;
                }

                if (pointer.TargetModule == 0xFF && pointer.TargetId == 0xFF)
                {
                    if (ReferenceJson.MergePointerLut(record.Pointers, key, null))
                    {
                        pending.Changed = true;
                    }

                    continue;
                }

                if (!TryReadOpaqueImportPointerValue(
                        element,
                        offset,
                        blockBytes,
                        record.Data,
                        out var value))
                {
                    continue;
                }
                string? lutUri = null;
                if (resolver.TryGetReferenceUri(value, levelPackageDir, out var resolvedUri) ||
                    resolver.TryGetReferenceUri(value, fixPackageDir, out resolvedUri))
                {
                    lutUri = WriteLevelSlotForFixSite(
                        levelPackageDir,
                        fixPackageDir,
                        sourceAddress,
                        resolvedUri,
                        resolver);
                }

                if (ReferenceJson.MergePointerLut(record.Pointers, key, lutUri))
                {
                    pending.Changed = true;
                }
            }
        }

        foreach (var (elementPath, pending) in pendingRecords)
        {
            if (pending.Changed)
            {
                pending.Codec.WriteJson(fixPackageDir, elementPath, pending.Record);
            }
        }
    }

    private static string ToPackageRelativePath(string packageDir, string absolutePath)
    {
        var relative = Path.GetRelativePath(Path.GetFullPath(packageDir), Path.GetFullPath(absolutePath));
        return ReferenceUri.ToUriPath(relative);
    }

    /// <summary>
    /// Loads preserved RT* from the original game directory for <c>debug-relocations</c> comparison only.
    /// Export (<see cref="CompileRelocationTable"/>) must not call this.
    /// </summary>
    private static bool TryLoadSourceRelocationTable(
        string packageDir,
        RetePackageManifest manifest,
        RelocationTableFileManifest table,
        out RelocationTableDocument document,
        out string failureNote)
    {
        document = new RelocationTableDocument { FileName = table.FileName };
        var (sourcePath, attemptedPaths) = TryResolveSourceRelocationPath(packageDir, manifest, table.FileName);
        if (sourcePath == null || !File.Exists(sourcePath))
        {
            failureNote = BuildSourceRelocationNotFoundMessage(manifest, table.FileName, attemptedPaths, sourcePath);
            return false;
        }

        document = ReadRelocationTableFromDisc(sourcePath);
        failureNote = "";
        return true;
    }

    private static (string? Path, IReadOnlyList<string> AttemptedPaths) TryResolveSourceRelocationPath(
        string packageDir,
        RetePackageManifest manifest,
        string fileName)
    {
        var attemptedPaths = new List<string>();
        var (sourceDirectory, directoryAttempts) = ResolveSourceDirectory(packageDir, manifest);
        attemptedPaths.AddRange(directoryAttempts);
        if (sourceDirectory == null)
        {
            return (null, attemptedPaths);
        }

        if (fileName.Equals("fixlvl.rtb", StringComparison.OrdinalIgnoreCase))
        {
            var fixLevelPath = Path.Combine(sourceDirectory, "fixlvl.rtb");
            attemptedPaths.Add(fixLevelPath);
            return (fixLevelPath, attemptedPaths);
        }

        var levelSourcePath = Path.Combine(sourceDirectory, fileName);
        attemptedPaths.Add(levelSourcePath);
        if (File.Exists(levelSourcePath))
        {
            return (levelSourcePath, attemptedPaths);
        }

        var levelsParent = Directory.GetParent(sourceDirectory)?.FullName;
        if (levelsParent == null)
        {
            return (null, attemptedPaths);
        }

        var sharedSourcePath = Path.Combine(levelsParent, fileName);
        attemptedPaths.Add(sharedSourcePath);
        return File.Exists(sharedSourcePath) ? (sharedSourcePath, attemptedPaths) : (null, attemptedPaths);
    }

    private static (string? Directory, IReadOnlyList<string> AttemptedPaths) ResolveSourceDirectory(
        string packageDir,
        RetePackageManifest manifest)
    {
        var attemptedPaths = new List<string>();
        if (string.IsNullOrWhiteSpace(manifest.SourceDirectoryName))
        {
            return (null, attemptedPaths);
        }

        var envSource = Environment.GetEnvironmentVariable("ASTROLABE_SOURCE_DIR");
        if (!string.IsNullOrWhiteSpace(envSource))
        {
            var directCandidate = Path.GetFullPath(envSource);
            attemptedPaths.Add(directCandidate);
            if (Directory.Exists(directCandidate) &&
                Path.GetFileName(directCandidate.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar))
                    .Equals(manifest.SourceDirectoryName, StringComparison.OrdinalIgnoreCase))
            {
                return (directCandidate, attemptedPaths);
            }

            var envCandidate = Path.Combine(envSource, manifest.SourceDirectoryName);
            attemptedPaths.Add(envCandidate);
            if (Directory.Exists(envCandidate))
            {
                return (envCandidate, attemptedPaths);
            }
        }

        var packageFullPath = Path.GetFullPath(packageDir);
        var maxDepth = ResolveSourceWalkDepth();
        var current = packageFullPath;
        for (var depth = 0; depth < maxDepth && current != null; depth++)
        {
            foreach (var candidate in EnumerateSourceDirectoryCandidates(current, manifest))
            {
                attemptedPaths.Add(candidate);
                if (Directory.Exists(candidate) &&
                    !Path.GetFullPath(candidate).Equals(packageFullPath, StringComparison.OrdinalIgnoreCase))
                {
                    return (candidate, attemptedPaths);
                }
            }

            current = Directory.GetParent(current)?.FullName;
        }

        return (null, attemptedPaths);
    }

    private static IEnumerable<string> EnumerateSourceDirectoryCandidates(
        string walkRoot,
        RetePackageManifest manifest)
    {
        var levelsRoot = Path.Combine(walkRoot, "disc", "Gamedata", "World", "Levels");
        if (manifest.PackageRole.Equals("fix", StringComparison.OrdinalIgnoreCase) &&
            manifest.SourceDirectoryName.Equals("Levels", StringComparison.OrdinalIgnoreCase))
        {
            yield return levelsRoot;
            yield break;
        }

        yield return Path.Combine(levelsRoot, manifest.SourceDirectoryName);
    }

    private static int ResolveSourceWalkDepth()
    {
        var configured = Environment.GetEnvironmentVariable("ASTROLABE_SOURCE_WALK_DEPTH");
        return int.TryParse(configured, out var depth) && depth > 0 ? depth : 32;
    }

    private static string BuildSourceRelocationNotFoundMessage(
        RetePackageManifest manifest,
        string fileName,
        IReadOnlyList<string> attemptedPaths,
        string? resolvedPath)
    {
        var attempts = attemptedPaths.Count > 0
            ? string.Join("; ", attemptedPaths.Take(12))
            : "(no paths attempted)";
        if (!string.IsNullOrWhiteSpace(resolvedPath))
        {
            return
                $"Source RT* '{fileName}' not found at {resolvedPath}. " +
                "Set ASTROLABE_SOURCE_DIR or place the package under a discoverable disc/Gamedata/World/Levels tree. " +
                $"Attempted: {attempts}";
        }

        return
            $"Source RT* '{fileName}' not found for sourceDirectoryName '{manifest.SourceDirectoryName}'. " +
            "Set ASTROLABE_SOURCE_DIR or place the package under a discoverable disc/Gamedata/World/Levels tree. " +
            $"Attempted: {attempts}";
    }

    private static RelocationTableDocument ReadRelocationTableFromDisc(string tablePath)
    {
        var reader = new RelocationTableReader(tablePath);
        var fileName = Path.GetFileName(tablePath);
        var document = new RelocationTableDocument { FileName = fileName };

        for (var i = 0; i < reader.PointerBlocks.Count; i++)
        {
            var block = reader.PointerBlocks[i];
            document.Blocks.Add(new RelocationPointerBlockManifest
            {
                Order = i,
                Key = ToKey(block.Module, block.Id),
                Module = block.Module,
                Id = block.Id,
                EntrySize = block.EntrySize,
                PointerDataSha256 = HashBytes(block.PointerData),
                Pointers = block.Pointers.Select(p => new RelocationPointerManifest
                {
                    OffsetInMemory = p.OffsetInMemory,
                    TargetModule = p.TargetModule,
                    TargetId = p.TargetId,
                    Byte6 = p.Byte6,
                    Byte7 = p.Byte7
                }).ToList(),
                TrailingDataBase64 = block.TrailingData.Length > 0
                    ? Convert.ToBase64String(block.TrailingData)
                    : null
            });
        }

        return document;
    }

    private static bool TryReadElementPointerValue(
        string packageDir,
        IndexedContentElement element,
        int virtualAddress,
        out int value,
        ReferenceAddressResolver? resolver = null)
    {
        value = 0;
        if (!StructCodecRegistry.TryGet(element.Kind, out var codec))
        {
            return false;
        }

        var elementPath = ReferenceUri.Resolve(packageDir, element.DataPath).FilePath;
        if (!File.Exists(elementPath))
        {
            return false;
        }

        var elementOffset = virtualAddress - element.VirtualAddress;
        byte[] bytes;
        if (elementPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
        {
            bytes = resolver != null
                ? ReferenceJson.WriteElementBytesForExport(
                    packageDir,
                    element.Kind,
                    element.DataPath,
                    resolver)
                : codec.WriteFromJsonPath(packageDir, elementPath);
        }
        else
        {
            bytes = File.ReadAllBytes(elementPath);
        }

        if (elementOffset < 0 || elementOffset + sizeof(int) > bytes.Length || elementOffset % sizeof(int) != 0)
        {
            return false;
        }

        value = BinaryPrimitives.ReadInt32LittleEndian(bytes.AsSpan(elementOffset, sizeof(int)));
        return true;
    }

    private static Dictionary<(byte Module, byte Id), List<IndexedContentElement>> LoadElementIndex(
        string packageDir,
        RetePackageManifest manifest)
    {
        var result = new Dictionary<(byte Module, byte Id), List<IndexedContentElement>>();
        foreach (var snaFile in manifest.SnaFiles)
        {
            foreach (var block in snaFile.Blocks)
            {
                if (block.ContentPath == null)
                {
                    continue;
                }

                var content = ReadJson<SnaBlockContentDocument>(ResolvePath(packageDir, block.ContentPath));
                var elements = content.Elements
                    .Select(e => new IndexedContentElement(
                        e.Kind,
                        e.DataPath,
                        e.VirtualAddress,
                        e.Length,
                        e.OffsetInBlock))
                    .OrderBy(e => e.VirtualAddress)
                    .ToList();
                result[(block.Module, block.Id)] = elements;
            }
        }

        return result;
    }

    private static IndexedContentElement? FindElementAt(
        IReadOnlyList<IndexedContentElement> elements,
        int virtualAddress)
    {
        IndexedContentElement? best = null;
        var bestSpan = int.MaxValue;

        foreach (var element in elements)
        {
            if (virtualAddress < element.VirtualAddress ||
                virtualAddress >= checked(element.VirtualAddress + element.Length))
            {
                continue;
            }

            if (element.Length < bestSpan)
            {
                best = element;
                bestSpan = element.Length;
            }
        }

        return best;
    }

    public static void ExportLevel(string packageDir, string outputDir)
    {
        var level = Level.Load(packageDir);
        ExportLevelFromHub(level, outputDir);
    }

    public static void ExportLevelFromHub(Level level, string outputDir)
    {
        if (level.SourceKind != LevelSourceKind.Rete || level.Catalog == null || level.Manifest == null)
        {
            throw new InvalidOperationException("OpenSpace export requires a Rete-loaded Level hub.");
        }

        ExportHubPackage(level.SourcePath, level.Manifest, level.Catalog, outputDir);
    }

    public static void ExportFixFromHub(
        string packageDir,
        RetePackageManifest manifest,
        HubCatalog catalog,
        string outputDir) =>
        ExportHubPackage(packageDir, manifest, catalog, outputDir);

    private static void ExportHubPackage(
        string packageDir,
        RetePackageManifest manifest,
        HubCatalog catalog,
        string outputDir)
    {
        ValidateReteManifestSchema(manifest.Schema);
        Directory.CreateDirectory(outputDir);
        var targetPackageRoots = FindTargetPackageRoots(packageDir, manifest).ToList();
        GuardAgainstLegacyRelocationArtifacts(packageDir, targetPackageRoots);
        ValidateExportRelocationTables(packageDir, manifest, targetPackageRoots);
        var referenceResolver = CreateExportResolver(packageDir);

        foreach (var snaFile in manifest.SnaFiles)
        {
            CompileSnaFileFromHub(
                packageDir,
                catalog,
                snaFile,
                Path.Combine(outputDir, snaFile.FileName),
                referenceResolver);
        }

        ExportGeneratedRelocationTables(packageDir, manifest, outputDir, targetPackageRoots);
        CopyLooseFiles(packageDir, manifest, outputDir);
    }

    private static void CopyLooseFiles(string packageDir, RetePackageManifest manifest, string outputDir)
    {
        foreach (var looseFile in manifest.LooseFiles)
        {
            var sourcePath = ResolvePath(packageDir, looseFile.Path);
            var outputPath = Path.Combine(outputDir, looseFile.FileName);
            Directory.CreateDirectory(Path.GetDirectoryName(outputPath)!);
            File.Copy(sourcePath, outputPath, overwrite: true);
        }
    }

    public static IReadOnlyList<RelocationComparisonResult> CompareGeneratedRelocations(string packageDir)
    {
        var manifestPath = Path.Combine(packageDir, ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            throw new FileNotFoundException($"Rete manifest not found: {manifestPath}");
        }

        var manifest = ReadJson<RetePackageManifest>(manifestPath);
        ValidateReteManifestSchema(manifest.Schema);

        var results = new List<RelocationComparisonResult>();
        var targetPackageRoots = FindTargetPackageRoots(packageDir, manifest).ToList();
        var legacyArtifactWarning = BuildLegacyRelocationArtifactWarning(packageDir, targetPackageRoots);
        var staleFixlvlBlockKeysWarning = BuildStaleFixlvlBlockKeysWarning(manifest);
        var relocationContext = new RelocationGenerator.RelocationPackageContext();
        relocationContext.EnsureLayout(packageDir);
        foreach (var targetPackageRoot in targetPackageRoots)
        {
            relocationContext.EnsureLayout(targetPackageRoot);
        }

        foreach (var table in manifest.RelocationTables)
        {
            if (IsUnsupportedRelocationTable(table.FileName))
            {
                var passThrough = ComparePassThroughRelocationTable(packageDir, manifest, table);
                results.Add(passThrough with
                {
                    Note = AppendWarnings(
                        passThrough.Note,
                        legacyArtifactWarning,
                        staleFixlvlBlockKeysWarning)
                });
                continue;
            }

            if (!TryLoadSourceRelocationTable(packageDir, manifest, table, out var preserved, out var loadNote))
            {
                results.Add(UnsupportedRelocationComparison(
                    table.FileName,
                    new RelocationTableDocument { FileName = table.FileName },
                    AppendWarnings(loadNote, legacyArtifactWarning, staleFixlvlBlockKeysWarning) ?? loadNote));
                continue;
            }

            var extension = Path.GetExtension(table.FileName);
            if (extension.Equals(".rtb", StringComparison.OrdinalIgnoreCase))
            {
                if (table.FileName.Equals("fixlvl.rtb", StringComparison.OrdinalIgnoreCase))
                {
                    var fixPackageDir = targetPackageRoots.FirstOrDefault();
                    if (fixPackageDir == null)
                    {
                        results.Add(UnsupportedRelocationComparison(
                            table.FileName,
                            preserved,
                            AppendWarnings(
                                "fixlvl.rtb requires a sibling Fix Rete package.",
                                legacyArtifactWarning,
                                staleFixlvlBlockKeysWarning) ?? "fixlvl.rtb requires a sibling Fix Rete package."));
                        continue;
                    }

                    var generatedFixLevel = RelocationGenerator.GenerateFixLevelRtb(
                        fixPackageDir,
                        packageDir,
                        table.FileName,
                        relocationContext);
                    var fixLevelResult = RelocationGenerator.Compare(
                        preserved,
                        generatedFixLevel,
                        ParseFixlvlBlockKeys(manifest.FixlvlBlockKeys));
                    results.Add(fixLevelResult with
                    {
                        Note = AppendWarnings(fixLevelResult.Note, legacyArtifactWarning, staleFixlvlBlockKeysWarning)
                    });
                    continue;
                }

                var generated = RelocationGenerator.GenerateRtb(
                    packageDir,
                    table.FileName,
                    targetPackageRoots,
                    context: relocationContext);
                var rtbResult = RelocationGenerator.Compare(preserved, generated);
                results.Add(rtbResult with
                {
                    Note = AppendWarnings(rtbResult.Note, legacyArtifactWarning, staleFixlvlBlockKeysWarning)
                });
                continue;
            }

            if (TryGetPointerFilePath(packageDir, manifest, table.FileName, out var pointerFilePath))
            {
                var generated = RelocationGenerator.GeneratePointerFileTable(
                    packageDir,
                    table.FileName,
                    pointerFilePath,
                    targetPackageRoots,
                    relocationContext);
                var pointerFileResult = RelocationGenerator.Compare(preserved, generated);
                results.Add(pointerFileResult with
                {
                    Note = AppendWarnings(pointerFileResult.Note, legacyArtifactWarning, staleFixlvlBlockKeysWarning)
                });
                continue;
            }

            results.Add(UnsupportedRelocationComparison(
                table.FileName,
                preserved,
                AppendWarnings(
                    "Relocation generation is not implemented for this table type.",
                    legacyArtifactWarning,
                    staleFixlvlBlockKeysWarning) ?? "Relocation generation is not implemented for this table type."));
        }

        return results;
    }

    private static bool TryGetPointerFilePath(
        string packageDir,
        RetePackageManifest manifest,
        string relocationFileName,
        out string pointerFilePath)
    {
        pointerFilePath = "";
        var extension = Path.GetExtension(relocationFileName);
        var pointerExtension = extension.ToLowerInvariant() switch
        {
            ".rtp" => ".gpt",
            ".rtt" => ".ptx",
            _ => null
        };
        if (pointerExtension == null)
        {
            return false;
        }

        var expectedName = Path.ChangeExtension(relocationFileName, pointerExtension);
        var looseFile = manifest.LooseFiles.FirstOrDefault(f =>
            f.FileName.Equals(expectedName, StringComparison.OrdinalIgnoreCase));
        if (looseFile == null)
        {
            return false;
        }

        pointerFilePath = ResolvePath(packageDir, looseFile.Path);
        return File.Exists(pointerFilePath);
    }

    private static IEnumerable<string> FindTargetPackageRoots(string packageDir, RetePackageManifest manifest)
    {
        var packageParent = Directory.GetParent(Path.GetFullPath(packageDir))?.FullName;
        if (packageParent == null)
        {
            yield break;
        }

        if (manifest.PackageRole.Equals("level", StringComparison.OrdinalIgnoreCase))
        {
            var fixPackageDir = Path.Combine(packageParent, "fix");
            if (File.Exists(Path.Combine(fixPackageDir, ManifestFileName)))
            {
                yield return fixPackageDir;
            }

            yield break;
        }

        if (!manifest.PackageRole.Equals("fix", StringComparison.OrdinalIgnoreCase))
        {
            yield break;
        }

        foreach (var siblingDir in Directory.EnumerateDirectories(packageParent))
        {
            if (siblingDir.Equals(packageDir, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            var siblingManifestPath = Path.Combine(siblingDir, ManifestFileName);
            if (!File.Exists(siblingManifestPath))
            {
                continue;
            }

            var siblingManifest = ReadJson<RetePackageManifest>(siblingManifestPath);
            if (siblingManifest.PackageRole.Equals("level", StringComparison.OrdinalIgnoreCase))
            {
                yield return siblingDir;
            }
        }
    }

    internal static ReferenceAddressResolver CreateExportResolver(string packageDir)
    {
        var resolver = ReferenceAddressResolver.CreateForExport(packageDir);
        var manifestPath = Path.Combine(packageDir, ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            return resolver;
        }

        var manifest = ReadJson<RetePackageManifest>(manifestPath);
        foreach (var targetPackageRoot in FindTargetPackageRoots(packageDir, manifest))
        {
            resolver.LoadPackage(targetPackageRoot);
        }

        return resolver;
    }

    private static RelocationComparisonResult ComparePassThroughRelocationTable(
        string packageDir,
        RetePackageManifest manifest,
        RelocationTableFileManifest table)
    {
        var looseFile = manifest.LooseFiles.FirstOrDefault(file =>
            file.FileName.Equals(table.FileName, StringComparison.OrdinalIgnoreCase));
        if (looseFile == null)
        {
            return UnsupportedRelocationComparison(
                table.FileName,
                new RelocationTableDocument { FileName = table.FileName },
                "Pass-through relocation table is missing a files/ loose copy.");
        }

        var sourcePath = ResolvePath(packageDir, looseFile.Path);
        if (!File.Exists(sourcePath))
        {
            return UnsupportedRelocationComparison(
                table.FileName,
                new RelocationTableDocument { FileName = table.FileName },
                $"Pass-through relocation table loose file not found: {looseFile.Path}");
        }

        var actualHash = HashBytes(File.ReadAllBytes(sourcePath));
        var hashMatches = actualHash.Equals(looseFile.Sha256, StringComparison.OrdinalIgnoreCase);
        return new RelocationComparisonResult(
            table.FileName,
            Supported: true,
            PreservedPointerCount: 0,
            GeneratedPointerCount: 0,
            MatchingPointerCount: 0,
            MissingPointerCount: 0,
            ExtraPointerCount: 0,
            PointerDataMatches: hashMatches,
            Note: hashMatches
                ? "Pass-through package fidelity check (files/ loose copy SHA256; not source-disc RT* parity)."
                : $"Loose file SHA256 mismatch: expected {looseFile.Sha256}, actual {actualHash}.",
            MissingSamples: [],
            ExtraSamples: []);
    }

    private static RelocationComparisonResult UnsupportedRelocationComparison(
        string fileName,
        RelocationTableDocument preserved,
        string note)
    {
        return new RelocationComparisonResult(
            fileName,
            Supported: false,
            PreservedPointerCount: preserved.Blocks.Sum(b => b.Pointers.Count),
            GeneratedPointerCount: 0,
            MatchingPointerCount: 0,
            MissingPointerCount: preserved.Blocks.Sum(b => b.Pointers.Count),
            ExtraPointerCount: 0,
            PointerDataMatches: false,
            Note: note,
            MissingSamples: [],
            ExtraSamples: []);
    }

    private static SnaFileManifest ExtractSnaFile(
        string snaPath,
        string outputDir,
        ByteRangeTracker? tracker,
        IReadOnlyDictionary<int, string> sceneNodePaths)
    {
        var reader = new SnaReader(snaPath);
        var fileName = Path.GetFileName(snaPath);
        var stem = Path.GetFileNameWithoutExtension(fileName);
        var manifest = new SnaFileManifest { FileName = fileName };

        for (var i = 0; i < reader.Blocks.Count; i++)
        {
            var block = reader.Blocks[i];
            var blockStem = $"{i:D4}_{block.Module:X2}_{block.Id:X2}";
            var blockDir = $"sna/{stem}/blocks";
            var blockManifest = new SnaBlockManifest
            {
                Order = i,
                Key = ToKey(block.Module, block.Id),
                Module = block.Module,
                Id = block.Id,
                BaseInMemory = block.BaseInMemory,
                Unk2 = block.Unk2,
                Unk3 = block.Unk3,
                MaxPosMinus9 = block.MaxPosMinus9,
                HasPayload = block.Size > 0
            };

            if (block.Size > 0)
            {
                var data = block.Data ?? [];
                blockManifest.DataSha256 = HashBytes(data);
                blockManifest.ContentPath = WriteSnaBlockContent(
                    outputDir,
                    fileName,
                    stem,
                    blockStem,
                    block,
                    i,
                    data,
                    tracker,
                    sceneNodePaths);

                blockManifest.OriginalStorage = new SnaStorageManifest
                {
                    IsCompressed = block.IsCompressed,
                    CompressedSize = block.CompressedSize,
                    CompressedChecksum = block.CompressedChecksum,
                    DecompressedSize = block.DecompressedSize,
                    DecompressedChecksum = block.DecompressedChecksum
                };
            }

            manifest.Blocks.Add(blockManifest);
        }

        return manifest;
    }

    private static string WriteSnaBlockContent(
        string outputDir,
        string fileName,
        string stem,
        string blockStem,
        SnaBlock block,
        int blockOrder,
        byte[] data,
        ByteRangeTracker? tracker,
        IReadOnlyDictionary<int, string> sceneNodePaths)
    {
        var blockRoot = $"sna/{stem}/blocks/{blockStem}";
        var contentPath = $"{blockRoot}/content.json";
        var document = new SnaBlockContentDocument
        {
            FileName = fileName,
            BlockOrder = blockOrder,
            BlockKey = ToKey(block.Module, block.Id),
            Module = block.Module,
            Id = block.Id,
            BaseInMemory = block.BaseInMemory,
            BaseInMemoryHex = ToHex(block.BaseInMemory),
            OriginalDataSha256 = HashBytes(data)
        };

        var ranges = tracker?.Ranges ?? Array.Empty<ByteRange>();
        var elementPlans = BuildContentPlans(block, data, ranges);

        for (var i = 0; i < elementPlans.Count; i++)
        {
            var plan = elementPlans[i];
            string dataPath;
            byte[] emittedBytes;

            if (StructCodecRegistry.TryGet(plan.Kind, out var codec))
            {
                var value = codec.ReadFromBytes(data, plan.Start, plan.Length);
                emittedBytes = codec.WriteFromObject(value);

                if (plan.Kind is "superObject" or "matrix")
                {
                    var virtualAddress = block.BaseInMemory + plan.Start;
                    if (sceneNodePaths.TryGetValue(virtualAddress, out var existingPath))
                    {
                        dataPath = existingPath;
                    }
                    else
                    {
                        dataPath = GetTypedDataPath(plan.Kind, blockStem, i, "json");
                        codec.WriteJson(outputDir, ResolvePath(outputDir, dataPath), value);
                    }
                }
                else
                {
                    dataPath = GetTypedDataPath(plan.Kind, blockStem, i, "json");
                    codec.WriteJson(outputDir, ResolvePath(outputDir, dataPath), value);
                }
            }
            else
            {
                emittedBytes = data.AsSpan(plan.Start, plan.Length).ToArray();
                dataPath = plan.Kind is "raw" or "padding"
                    ? $"{blockRoot}/elements/{i:D4}_{plan.Kind}.bin"
                    : GetTypedDataPath(plan.Kind, blockStem, i, "bin");
                WriteBytes(outputDir, dataPath, emittedBytes);
            }

            document.Elements.Add(new SnaBlockContentElement
            {
                Order = document.Elements.Count,
                Kind = plan.Kind,
                DataPath = dataPath,
                OffsetInBlock = plan.Start,
                Length = plan.Length,
                VirtualAddress = block.BaseInMemory + plan.Start,
                VirtualAddressHex = ToHex(block.BaseInMemory + plan.Start),
                Sha256 = HashBytes(emittedBytes),
                Labels = plan.Labels
            });
        }

        WriteJson(ResolvePath(outputDir, contentPath), document);
        return contentPath;
    }

    private static List<SnaContentPlan> BuildContentPlans(SnaBlock block, byte[] data, IReadOnlyList<ByteRange> ranges)
    {
        const int MaxSegmentLength = 256 * 1024;

        var length = data.Length;
        var blockStart = block.BaseInMemory;
        var blockEnd = blockStart + length;
        var typedRangesBySpan = new Dictionary<(int Start, int End), List<string>>();

        foreach (var range in ranges)
        {
            if (range.End <= blockStart || range.Start >= blockEnd)
            {
                continue;
            }

            var start = Math.Max(range.Start, blockStart) - blockStart;
            var end = Math.Min(range.End, blockEnd) - blockStart;
            if (end <= start)
            {
                continue;
            }

            var key = (start, end);
            if (!typedRangesBySpan.TryGetValue(key, out var labels))
            {
                labels = [];
                typedRangesBySpan[key] = labels;
            }

            if (!labels.Contains(range.Label, StringComparer.Ordinal))
            {
                labels.Add(range.Label);
            }
        }

        var typedRanges = typedRangesBySpan
            .Select(kvp => (
                kvp.Key.Start,
                kvp.Key.End,
                Kind: GetContentKind(kvp.Value),
                Labels: kvp.Value.Order(StringComparer.Ordinal).ToList()))
            .ToList();

        // Registered fixed-size struct ranges (e.g. geometricobject 0x40) must win over
        // smaller overlapping interior/sub-field tracker ranges (elementtypes at +0x18, etc.).
        var selectedRanges = new List<(int Start, int End, string Kind, List<string> Labels)>();
        foreach (var range in typedRanges
                     .OrderByDescending(r => IsFixedSizeStructKind(r.Kind) ? 1 : 0)
                     .ThenByDescending(r => r.End - r.Start)
                     .ThenBy(r => r.Start))
        {
            if (selectedRanges.Any(r => r.Start < range.End && r.End > range.Start))
            {
                continue;
            }

            selectedRanges.Add(range);
        }

        // Carve promoted interior ranges (e.g. inline elementtypes inside a geo header) back out
        // of the parent fixed-size span so they become first-class manifest entries.
        foreach (var parent in selectedRanges.Where(r => IsFixedSizeStructKind(r.Kind)).ToList())
        {
            foreach (var interior in typedRanges)
            {
                if (selectedRanges.Any(r =>
                        r.Start == interior.Start && r.End == interior.End && r.Kind == interior.Kind))
                {
                    continue;
                }

                if (IsFixedSizeStructKind(interior.Kind))
                {
                    continue;
                }

                if (interior.Start >= parent.Start &&
                    interior.End <= parent.End &&
                    StructCodecRegistry.TryGet(interior.Kind, out _))
                {
                    selectedRanges.Add(interior);
                }
            }
        }

        CarveInlineGeometricObjectFields(block, data, selectedRanges);

        if (selectedRanges.Count == 0)
        {
            return RefineDynamPlans(
                SplitRawContent(data, 0, length, MaxSegmentLength),
                data,
                block.Module,
                block.Id);
        }

        selectedRanges.Sort((a, b) => a.Start != b.Start ? a.Start.CompareTo(b.Start) : a.End.CompareTo(b.End));

        var plans = new List<SnaContentPlan>();
        var cursor = 0;
        foreach (var range in selectedRanges)
        {
            if (range.Start < cursor)
            {
                // Interior carve-outs (e.g. inline elementtypes inside a geo header) start before
                // the cursor advanced past the parent span — still emit them once.
                if (!plans.Any(plan =>
                        plan.Start == range.Start &&
                        plan.Length == range.End - range.Start &&
                        plan.Kind == range.Kind))
                {
                    plans.Add(new SnaContentPlan(range.Start, range.End - range.Start, range.Kind, range.Labels));
                }

                continue;
            }

            if (cursor < range.Start)
            {
                plans.AddRange(SplitRawContent(data, cursor, range.Start, MaxSegmentLength));
            }

            plans.Add(new SnaContentPlan(range.Start, range.End - range.Start, range.Kind, range.Labels));
            cursor = range.End;
        }

        if (cursor < length)
        {
            plans.AddRange(SplitRawContent(data, cursor, length, MaxSegmentLength));
        }

        return RefineDynamPlans(plans, data, block.Module, block.Id);
    }

    private static bool IsFixedSizeStructKind(string kind) =>
        StructCodecRegistry.TryGet(kind, out var codec) && codec.FixedSize is > 0;

    private static void CarveInlineGeometricObjectFields(
        SnaBlock block,
        byte[] data,
        List<(int Start, int End, string Kind, List<string> Labels)> selectedRanges)
    {
        if (!StructCodecRegistry.TryGet("geometricobject", out var codec))
        {
            return;
        }

        foreach (var plan in selectedRanges.Where(r => r.Kind == "geometricobject").ToList())
        {
            var length = plan.End - plan.Start;
            if (length <= 0)
            {
                continue;
            }

            var geo = (GeometricObjectRecord)codec.ReadFromBytes(data, plan.Start, length);
            var geoAddress = block.BaseInMemory + plan.Start;
            TryCarveInlineField(
                selectedRanges,
                plan,
                geoAddress,
                geo.ElementTypes,
                checked((int)(geo.NumElements * 2)),
                "elementtypes",
                "InlineElementTypes");
        }
    }

    private static void TryCarveInlineField(
        List<(int Start, int End, string Kind, List<string> Labels)> selectedRanges,
        (int Start, int End, string Kind, List<string> Labels) parent,
        int parentAddress,
        HubReference pointer,
        int length,
        string kind,
        string label)
    {
        if (length <= 0 || pointer.IsNull)
        {
            return;
        }

        var targetAddress = HubReferenceIO.Materialize(pointer);
        if (targetAddress == 0)
        {
            return;
        }

        var relativeStart = targetAddress - parentAddress;
        var parentLength = parent.End - parent.Start;
        if (relativeStart < 0 || relativeStart + length > parentLength)
        {
            return;
        }

        var start = parent.Start + relativeStart;
        var end = start + length;
        if (selectedRanges.Any(r => r.Kind == kind && r.Start == start && r.End == end))
        {
            return;
        }

        selectedRanges.Add((start, end, kind, [label]));
    }

    private static string GetContentKind(IReadOnlyList<string> labels)
    {
        if (labels.Contains("SuperObject", StringComparer.Ordinal))
        {
            return "superObject";
        }

        if (labels.Contains("Matrix", StringComparer.Ordinal))
        {
            return "matrix";
        }

        if (labels.Contains("Dynam", StringComparer.Ordinal))
        {
            return "dynam";
        }

        return NormalizeKind(labels[0]);
    }

    private static string GetTypedDataPath(string kind, string blockStem, int order, string extension)
    {
        return $"types/{kind}/{blockStem}_{order:D4}.{extension}";
    }

    private static List<SnaContentPlan> SplitRawContent(
        byte[] data,
        int start,
        int end,
        int maxSegmentLength)
    {
        var plans = new List<SnaContentPlan>();
        var cursor = start;
        while (cursor < end)
        {
            var chunkEnd = Math.Min(end, cursor + maxSegmentLength);
            var kind = IsPadding(data, cursor, chunkEnd - cursor) ? "padding" : "raw";
            plans.Add(new SnaContentPlan(cursor, chunkEnd - cursor, kind, []));
            cursor = chunkEnd;
        }

        return plans;
    }

    private static List<SnaContentPlan> RefineDynamPlans(
        List<SnaContentPlan> plans,
        byte[] data,
        byte module,
        byte id) =>
        plans;

    private static bool IsPadding(byte[] data, int start, int length)
    {
        if (length == 0)
        {
            return true;
        }

        for (var i = 0; i < length; i++)
        {
            if (data[start + i] != 0)
            {
                return false;
            }
        }

        return true;
    }

    private static Dictionary<int, string> WriteSceneSourceTree(string outputDir, SemanticContext? context)
    {
        var paths = new Dictionary<int, string>();
        if (context?.SceneGraph == null || context.Memory == null)
        {
            return paths;
        }

        WriteSceneRoot(outputDir, "actual_world", context.SceneGraph.ActualWorld, context.Memory, paths);
        WriteSceneRoot(outputDir, "dynamic_world", context.SceneGraph.DynamicWorld, context.Memory, paths);
        WriteSceneRoot(outputDir, "father_sector", context.SceneGraph.FatherSector, context.Memory, paths);
        return paths;
    }

    private static void WriteSceneRoot(
        string outputDir,
        string rootName,
        SceneNode? root,
        MemoryContext memory,
        Dictionary<int, string> paths)
    {
        if (root == null)
        {
            return;
        }

        WriteSceneNode(outputDir, $"scene/{rootName}", rootName, root, memory, paths);
    }

    private static void WriteSceneNode(
        string outputDir,
        string parentDir,
        string rootName,
        SceneNode node,
        MemoryContext memory,
        Dictionary<int, string> paths)
    {
        var nodeDir = $"{parentDir}/{GetSceneFolderName(node)}";
        var nodePath = $"{nodeDir}/node.json";
        var data = memory.ReadBytes(node.Address, 0x38);
        if (data == null)
        {
            return;
        }

        var superObject = SuperObjectCodec.Instance.Read(data, 0, SuperObjectCodec.Size);
        var sceneNode = new IntermediateSceneNode
        {
            Id = GetSceneNodeId(node),
            Path = nodePath,
            Root = rootName,
            Name = node.Name,
            TypeCode = superObject.TypeCode,
            Type = superObject.Type,
            OffData = superObject.OffData,
            ChildrenHead = superObject.ChildrenHead,
            ChildrenTail = superObject.ChildrenTail,
            ChildrenCount = superObject.ChildrenCount,
            BrotherNext = superObject.BrotherNext,
            BrotherPrev = superObject.BrotherPrev,
            Parent = superObject.Parent,
            Matrix = superObject.Matrix,
            StaticMatrix = superObject.StaticMatrix,
            GlobalMatrix = superObject.GlobalMatrix,
            DrawFlags = superObject.DrawFlags,
            Flags = superObject.Flags,
            BoundingVolume = superObject.BoundingVolume,
            GeometricObjectAddress = node.GeometricObjectAddress
        };

        foreach (var child in node.Children)
        {
            sceneNode.Children.Add($"{nodeDir}/{GetSceneFolderName(child)}/node.json");
        }

        sceneNode.MatrixPath = WriteSceneMatrix(outputDir, nodeDir, "matrix", HubReferenceIO.Materialize(superObject.Matrix), memory, paths);
        sceneNode.StaticMatrixPath = WriteSceneMatrix(outputDir, nodeDir, "static_matrix", HubReferenceIO.Materialize(superObject.StaticMatrix), memory, paths);

        WriteJson(ResolvePath(outputDir, nodePath), sceneNode);
        paths.TryAdd(node.Address, nodePath);

        foreach (var child in node.Children)
        {
            WriteSceneNode(outputDir, nodeDir, rootName, child, memory, paths);
        }
    }

    private static string? WriteSceneMatrix(
        string outputDir,
        string nodeDir,
        string name,
        int address,
        MemoryContext memory,
        Dictionary<int, string> paths)
    {
        if (address == 0)
        {
            return null;
        }

        var data = memory.ReadBytes(address, 88);
        if (data == null)
        {
            return null;
        }

        var matrix = MatrixCodec.Instance.Read(data, 0, MatrixCodec.Size);
        var matrixPath = $"{nodeDir}/{name}.json";
        WriteJson(ResolvePath(outputDir, matrixPath), matrix);
        paths.TryAdd(address, matrixPath);
        return matrixPath;
    }

    private static string GetSceneFolderName(SceneNode node)
    {
        var name = string.IsNullOrWhiteSpace(node.Name) ? node.Type.ToString() : node.Name!;
        return $"{SanitizePathPart(name)}_{node.Address:X8}";
    }

    private static string GetSceneNodeId(SceneNode node)
    {
        var name = string.IsNullOrWhiteSpace(node.Name) ? node.Type.ToString() : node.Name!;
        return $"{SanitizePathPart(name)}_{node.Address:X8}";
    }

    private static string SanitizePathPart(string value)
    {
        var invalid = Path.GetInvalidFileNameChars();
        var chars = value.Select(c => invalid.Contains(c) || c is '/' or '\\' or ':' ? '_' : c).ToArray();
        var sanitized = new string(chars).Trim();
        return string.IsNullOrWhiteSpace(sanitized) ? "node" : sanitized;
    }

    private static string NormalizeKind(string label)
    {
        var chars = label.Select(c => char.IsLetterOrDigit(c) ? char.ToLowerInvariant(c) : '_').ToArray();
        var normalized = new string(chars).Trim('_');
        while (normalized.Contains("__", StringComparison.Ordinal))
        {
            normalized = normalized.Replace("__", "_", StringComparison.Ordinal);
        }

        return string.IsNullOrWhiteSpace(normalized) ? "typed" : normalized;
    }

    private static RelocationTableFileManifest ExtractRelocationTable(string tablePath, string outputDir)
    {
        _ = outputDir;
        var fileName = Path.GetFileName(tablePath);
        var extension = Path.GetExtension(fileName);
        if (extension is ".rtb" or ".rtp" or ".rtt")
        {
            _ = new RelocationTableReader(tablePath);
        }

        return new RelocationTableFileManifest
        {
            FileName = fileName
        };
    }

    private static void ValidateExportRelocationTables(
        string packageDir,
        RetePackageManifest manifest,
        IReadOnlyList<string> targetPackageRoots)
    {
        foreach (var table in manifest.RelocationTables)
        {
            if (!table.FileName.Equals("fixlvl.rtb", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            _ = targetPackageRoots.FirstOrDefault()
                ?? throw new InvalidDataException("fixlvl.rtb requires a sibling Fix Rete package.");
        }

        var staleFixlvlBlockKeysWarning = BuildStaleFixlvlBlockKeysWarning(manifest);
        if (!string.IsNullOrWhiteSpace(staleFixlvlBlockKeysWarning))
        {
            Trace.TraceWarning(staleFixlvlBlockKeysWarning);
        }
    }

    private static void GuardAgainstLegacyRelocationArtifacts(
        string packageDir,
        IReadOnlyList<string>? siblingPackageRoots = null)
    {
        var artifacts = CollectLegacyRelocationArtifacts(packageDir, siblingPackageRoots);
        if (artifacts.Count == 0)
        {
            return;
        }

        throw new InvalidDataException(
            "Package contains legacy relocation bridge artifacts and must be re-imported before export. " +
            $"Found: {string.Join(", ", artifacts.Take(8))}" +
            (artifacts.Count > 8 ? $" (+{artifacts.Count - 8} more)" : ""));
    }

    private static string? BuildLegacyRelocationArtifactWarning(
        string packageDir,
        IReadOnlyList<string>? siblingPackageRoots = null)
    {
        var artifacts = CollectLegacyRelocationArtifacts(packageDir, siblingPackageRoots);
        if (artifacts.Count == 0)
        {
            return null;
        }

        return
            "Legacy relocation bridge artifacts detected; re-import required for reliable compare/export. " +
            $"Found: {string.Join(", ", artifacts.Take(8))}" +
            (artifacts.Count > 8 ? $" (+{artifacts.Count - 8} more)" : "");
    }

    private static List<string> CollectLegacyRelocationArtifacts(
        string packageDir,
        IReadOnlyList<string>? siblingPackageRoots)
    {
        var artifacts = FindLegacyRelocationArtifacts(packageDir).ToList();
        if (siblingPackageRoots != null)
        {
            foreach (var siblingPackageRoot in siblingPackageRoots
                         .Distinct(StringComparer.OrdinalIgnoreCase)
                         .Where(root => !root.Equals(packageDir, StringComparison.OrdinalIgnoreCase)))
            {
                artifacts.AddRange(FindLegacyRelocationArtifacts(siblingPackageRoot));
            }
        }

        return artifacts
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .OrderBy(path => path, StringComparer.OrdinalIgnoreCase)
            .ToList();
    }

    private static string? AppendLegacyArtifactWarning(string? note, string? warning)
    {
        if (string.IsNullOrWhiteSpace(warning))
        {
            return note;
        }

        return string.IsNullOrWhiteSpace(note)
            ? warning
            : $"{note} {warning}";
    }

    private static string? AppendWarnings(string? note, params string?[] warnings)
    {
        var appended = note;
        foreach (var warning in warnings)
        {
            appended = AppendLegacyArtifactWarning(appended, warning);
        }

        return appended;
    }

    private static string? BuildStaleFixlvlBlockKeysWarning(RetePackageManifest manifest)
    {
        if (manifest.RelocationTables.All(table =>
                !table.FileName.Equals("fixlvl.rtb", StringComparison.OrdinalIgnoreCase)))
        {
            return null;
        }

        if (manifest.FixlvlBlockKeys is { Count: > 0 })
        {
            return null;
        }

        return
            "fixlvl.rtb is listed but FixlvlBlockKeys is empty; re-import required to emit disc empty blocks " +
            "(e.g. 07:00, 13:01).";
    }

    private static HashSet<(byte Module, byte Id)> ParseFixlvlBlockKeys(IReadOnlyList<string> blockKeys)
    {
        var parsed = new HashSet<(byte Module, byte Id)>();
        foreach (var blockKey in blockKeys)
        {
            var parts = blockKey.Split(':');
            if (parts.Length != 2 ||
                !byte.TryParse(parts[0], System.Globalization.NumberStyles.HexNumber, null, out var module) ||
                !byte.TryParse(parts[1], System.Globalization.NumberStyles.HexNumber, null, out var id))
            {
                continue;
            }

            parsed.Add((module, id));
        }

        return parsed;
    }

    private static void PruneLegacyRelocationArtifacts(string packageDir)
    {
        var semanticDir = Path.Combine(packageDir, "semantic");
        if (Directory.Exists(semanticDir))
        {
            foreach (var file in Directory.EnumerateFiles(semanticDir))
            {
                var fileName = Path.GetFileName(file);
                if (IsLegacySemanticArtifactFileName(fileName))
                {
                    File.Delete(file);
                }
            }
        }

        var relocationsDir = Path.Combine(packageDir, "relocations");
        if (Directory.Exists(relocationsDir))
        {
            Directory.Delete(relocationsDir, recursive: true);
        }

        foreach (var relocJson in Directory.EnumerateFiles(packageDir, "*.reloc.json", SearchOption.AllDirectories))
        {
            File.Delete(relocJson);
        }

        foreach (var encodedBin in Directory.EnumerateFiles(packageDir, "*.encoded.bin", SearchOption.AllDirectories))
        {
            File.Delete(encodedBin);
        }

        PruneLegacyRelocationManifestFields(packageDir);
        PruneLegacySnaEncodedManifestFields(packageDir);
    }

#pragma warning disable CS0618
    private static void PruneLegacySnaEncodedManifestFields(string packageDir)
    {
        var manifestPath = Path.Combine(packageDir, ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            return;
        }

        var manifest = ReadJson<RetePackageManifest>(manifestPath);
        var changed = false;
        foreach (var snaFile in manifest.SnaFiles)
        {
            foreach (var block in snaFile.Blocks)
            {
                if (block.OriginalStorage?.EncodedPath == null &&
                    block.OriginalStorage?.EncodedSha256 == null)
                {
                    continue;
                }

                block.OriginalStorage.EncodedPath = null;
                block.OriginalStorage.EncodedSha256 = null;
                changed = true;
            }
        }

        if (changed)
        {
            WriteJson(manifestPath, manifest);
        }
    }
#pragma warning restore CS0618

    private static void PruneLegacyRelocationManifestFields(string packageDir)
    {
        var manifestPath = Path.Combine(packageDir, ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            return;
        }

        var manifest = ReadJson<RetePackageManifest>(manifestPath);
        WriteJson(manifestPath, manifest);
    }

    private static bool IsLegacySemanticArtifactFileName(string fileName) =>
        fileName.EndsWith("-sites.json", StringComparison.OrdinalIgnoreCase) ||
        fileName.EndsWith("-encoding.json", StringComparison.OrdinalIgnoreCase) ||
        fileName.Equals("rtb-sites.json", StringComparison.OrdinalIgnoreCase) ||
        fileName.Equals("fix-level-sites.json", StringComparison.OrdinalIgnoreCase) ||
        fileName.Equals("fixlvl-sites.json", StringComparison.OrdinalIgnoreCase);

    private static IReadOnlyList<string> FindLegacyRelocationArtifacts(string packageDir)
    {
        var artifacts = new List<string>();
        var semanticDir = Path.Combine(packageDir, "semantic");
        if (Directory.Exists(semanticDir))
        {
            foreach (var file in Directory.EnumerateFiles(semanticDir))
            {
                var fileName = Path.GetFileName(file);
                if (IsLegacySemanticArtifactFileName(fileName))
                {
                    artifacts.Add(Path.GetRelativePath(packageDir, file).Replace('\\', '/'));
                }
            }
        }

        var relocationsDir = Path.Combine(packageDir, "relocations");
        if (Directory.Exists(relocationsDir))
        {
            artifacts.Add("relocations/");
        }

        foreach (var relocJson in Directory.EnumerateFiles(packageDir, "*.reloc.json", SearchOption.AllDirectories))
        {
            artifacts.Add(Path.GetRelativePath(packageDir, relocJson).Replace('\\', '/'));
        }

        var manifestPath = Path.Combine(packageDir, ManifestFileName);
        if (File.Exists(manifestPath))
        {
            artifacts.AddRange(FindLegacyManifestRelocationProperties(manifestPath));
        }

        return artifacts
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .OrderBy(path => path, StringComparer.OrdinalIgnoreCase)
            .ToList();
    }

    private static IEnumerable<string> FindLegacyManifestRelocationProperties(string manifestPath)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(manifestPath));
        var root = document.RootElement;
        if (!root.TryGetProperty("relocationTables", out var relocationTables) ||
            relocationTables.ValueKind != JsonValueKind.Array)
        {
            yield break;
        }

        for (var index = 0; index < relocationTables.GetArrayLength(); index++)
        {
            var table = relocationTables[index];
            if (table.TryGetProperty("jsonPath", out var jsonPath) &&
                jsonPath.ValueKind == JsonValueKind.String &&
                !string.IsNullOrWhiteSpace(jsonPath.GetString()))
            {
                yield return $"manifest.json: relocationTables[{index}].jsonPath";
            }

            if (table.TryGetProperty("encodingPath", out var encodingPath) &&
                encodingPath.ValueKind == JsonValueKind.String &&
                !string.IsNullOrWhiteSpace(encodingPath.GetString()))
            {
                yield return $"manifest.json: relocationTables[{index}].encodingPath";
            }
        }

        if (!root.TryGetProperty("semantic", out var semantic) ||
            semantic.ValueKind != JsonValueKind.Object)
        {
            yield break;
        }

        if (semantic.TryGetProperty("rtbSitesPath", out var rtbSitesPath) &&
            rtbSitesPath.ValueKind == JsonValueKind.String &&
            !string.IsNullOrWhiteSpace(rtbSitesPath.GetString()))
        {
            yield return "manifest.json: semantic.rtbSitesPath";
        }

        if (semantic.TryGetProperty("fixLevelSitesPath", out var fixLevelSitesPath) &&
            fixLevelSitesPath.ValueKind == JsonValueKind.String &&
            !string.IsNullOrWhiteSpace(fixLevelSitesPath.GetString()))
        {
            yield return "manifest.json: semantic.fixLevelSitesPath";
        }

        if (semantic.TryGetProperty("pointerFileSitesPaths", out var pointerFileSitesPaths) &&
            pointerFileSitesPaths.ValueKind == JsonValueKind.Object &&
            pointerFileSitesPaths.EnumerateObject().Any())
        {
            yield return "manifest.json: semantic.pointerFileSitesPaths";
        }
    }

    private static LooseFileManifest CopyLooseFile(string filePath, string outputDir)
    {
        var fileName = Path.GetFileName(filePath);
        var data = File.ReadAllBytes(filePath);
        var relativePath = $"files/{fileName}";
        WriteBytes(outputDir, relativePath, data);

        return new LooseFileManifest
        {
            FileName = fileName,
            Path = relativePath,
            Size = data.Length,
            Sha256 = HashBytes(data)
        };
    }

    private static void CompileSnaFileFromHub(
        string packageDir,
        HubCatalog catalog,
        SnaFileManifest manifest,
        string outputPath,
        ReferenceAddressResolver referenceResolver)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(outputPath)!);
        File.WriteAllBytes(
            outputPath,
            BuildSnaFileBytesFromHub(packageDir, catalog, manifest, referenceResolver));
    }

    internal static byte[] BuildSnaFileBytes(
        string packageDir,
        SnaFileManifest manifest,
        ReferenceAddressResolver referenceResolver)
    {
        using var stream = new MemoryStream();
        using var writer = new BinaryWriter(stream);

        foreach (var block in manifest.Blocks.OrderBy(b => b.Order))
        {
            writer.Write(block.Module);
            writer.Write(block.Id);
            writer.Write(block.BaseInMemory);

            if (block.BaseInMemory == -1)
            {
                continue;
            }

            byte[] data = [];
            if (block.HasPayload)
            {
                data = ReadSnaBlockData(packageDir, block, referenceResolver);
            }

            var size = block.HasPayload ? checked((uint)data.Length) : 0u;
            var maxPosMinus9 = GetMaxPosMinus9(block, size);
            writer.Write(block.Unk2);
            writer.Write(block.Unk3);
            writer.Write(maxPosMinus9);
            writer.Write(size);

            if (!block.HasPayload)
            {
                continue;
            }

            var checksum = OpenSpaceChecksum.Calculate(data);
            if (OpenSpaceLzo.TryCompress(data, out var compressed) &&
                compressed.Length < data.Length)
            {
                writer.Write(1u);
                writer.Write((uint)compressed.Length);
                writer.Write(OpenSpaceChecksum.Calculate(compressed));
                writer.Write(size);
                writer.Write(checksum);
                writer.Write(compressed);
            }
            else
            {
                writer.Write(0u);
                writer.Write(size);
                writer.Write(checksum);
                writer.Write(size);
                writer.Write(checksum);
                writer.Write(data);
            }
        }

        writer.Flush();
        return stream.ToArray();
    }

    private static byte[] ReadSnaBlockData(
        string intermediateDir,
        SnaBlockManifest block,
        ReferenceAddressResolver referenceResolver)
    {
        if (block.ContentPath != null)
        {
            var document = ReadJson<SnaBlockContentDocument>(ResolvePath(intermediateDir, block.ContentPath));
            if (document.Schema != "astrolabe.sna-block-content.v1")
            {
                throw new InvalidDataException($"Unsupported SNA block content schema: {document.Schema}");
            }

            if (document.BlockKey != block.Key)
            {
                throw new InvalidDataException($"SNA block content manifest does not match block {block.Key}.");
            }

            using var stream = new MemoryStream();

            foreach (var element in document.Elements.OrderBy(s => s.Order))
            {
                var data = StructCodecRegistry.TryGet(element.Kind, out _)
                    ? ReadStructuredElementBytes(intermediateDir, element, referenceResolver)
                    : File.ReadAllBytes(ResolvePath(intermediateDir, element.DataPath));

                stream.Write(data);
            }

            return stream.ToArray();
        }

        if (block.DataPath != null)
        {
            return File.ReadAllBytes(ResolvePath(intermediateDir, block.DataPath));
        }

        throw new InvalidDataException($"SNA block {block.Key} is missing content and data paths.");
    }

    private static byte[] ReadStructuredElementBytes(
        string packageDir,
        SnaBlockContentElement element,
        ReferenceAddressResolver referenceResolver) =>
        ReferenceJson.WriteElementBytesForExport(
            packageDir,
            element.Kind,
            element.DataPath,
            referenceResolver);

    private static byte[] BuildSnaFileBytesFromHub(
        string packageDir,
        HubCatalog catalog,
        SnaFileManifest manifest,
        ReferenceAddressResolver referenceResolver)
    {
        using var stream = new MemoryStream();
        using var writer = new BinaryWriter(stream);

        foreach (var block in manifest.Blocks.OrderBy(b => b.Order))
        {
            writer.Write(block.Module);
            writer.Write(block.Id);
            writer.Write(block.BaseInMemory);

            if (block.BaseInMemory == -1)
            {
                continue;
            }

            byte[] data = [];
            if (block.HasPayload)
            {
                data = ReadSnaBlockDataFromHub(packageDir, catalog, block, referenceResolver);
            }

            var size = block.HasPayload ? checked((uint)data.Length) : 0u;
            var maxPosMinus9 = GetMaxPosMinus9(block, size);
            writer.Write(block.Unk2);
            writer.Write(block.Unk3);
            writer.Write(maxPosMinus9);
            writer.Write(size);

            if (!block.HasPayload)
            {
                continue;
            }

            var checksum = OpenSpaceChecksum.Calculate(data);
            if (OpenSpaceLzo.TryCompress(data, out var compressed) &&
                compressed.Length < data.Length)
            {
                writer.Write(1u);
                writer.Write((uint)compressed.Length);
                writer.Write(OpenSpaceChecksum.Calculate(compressed));
                writer.Write(size);
                writer.Write(checksum);
                writer.Write(compressed);
            }
            else
            {
                writer.Write(0u);
                writer.Write(size);
                writer.Write(checksum);
                writer.Write(size);
                writer.Write(checksum);
                writer.Write(data);
            }
        }

        writer.Flush();
        return stream.ToArray();
    }

    private static byte[] ReadSnaBlockDataFromHub(
        string packageDir,
        HubCatalog catalog,
        SnaBlockManifest block,
        ReferenceAddressResolver referenceResolver)
    {
        if (block.ContentPath == null)
        {
            if (block.DataPath != null)
            {
                return File.ReadAllBytes(ResolvePath(packageDir, block.DataPath));
            }

            throw new InvalidDataException($"SNA block {block.Key} is missing content and data paths.");
        }

        var document = ReadJson<SnaBlockContentDocument>(ResolvePath(packageDir, block.ContentPath));
        using var stream = new MemoryStream();
        foreach (var element in document.Elements.OrderBy(s => s.Order))
        {
            var data = ReadHubElementBytes(catalog, element, referenceResolver, packageDir);
            stream.Write(data);
        }

        return stream.ToArray();
    }

    private static byte[] ReadHubElementBytes(
        HubCatalog catalog,
        SnaBlockContentElement element,
        ReferenceAddressResolver referenceResolver,
        string packageDir)
    {
        var dataPath = NormalizeRetePath(element.DataPath);
        if (catalog.TryGetByPath(dataPath, out var hubElement) &&
            catalog.TryHydrate(hubElement) &&
            hubElement.Value != null &&
            StructCodecRegistry.TryGet(element.Kind, out var codec))
        {
            HubReferenceMaterializer.Materialize(hubElement.Value, referenceResolver, packageDir);
            return codec.WriteFromObject(hubElement.Value);
        }

        if (StructCodecRegistry.TryGet(element.Kind, out _))
        {
            return ReadStructuredElementBytes(packageDir, element, referenceResolver);
        }

        return File.ReadAllBytes(ResolvePath(packageDir, element.DataPath));
    }

    private static string NormalizeRetePath(string path) =>
        path.Replace('\\', '/').TrimStart('/');

    private static uint GetMaxPosMinus9(SnaBlockManifest block, uint size)
    {
        var originalSize = block.OriginalStorage?.DecompressedSize ?? 0;
        if (block.BaseInMemory >= 0 &&
            originalSize <= int.MaxValue &&
            block.MaxPosMinus9 == unchecked((uint)(block.BaseInMemory + (int)originalSize - 9)))
        {
            return unchecked((uint)(block.BaseInMemory + (int)size - 9));
        }

        return block.MaxPosMinus9;
    }

    private static void ExportGeneratedRelocationTables(
        string packageDir,
        RetePackageManifest manifest,
        string outputDir,
        IReadOnlyList<string> targetPackageRoots)
    {
        var relocationContext = CreateRelocationPackageContext(packageDir, targetPackageRoots);

        foreach (var table in manifest.RelocationTables)
        {
            if (!TryGenerateRelocationTableDocument(
                    packageDir,
                    manifest,
                    table,
                    targetPackageRoots,
                    relocationContext,
                    out var generated))
            {
                if (TryExportUnsupportedRelocationTable(packageDir, manifest, table, outputDir))
                {
                    continue;
                }

                throw new InvalidDataException(
                    $"Relocation generation is not implemented for table {table.FileName}.");
            }

            CompileRelocationTable(packageDir, generated, Path.Combine(outputDir, table.FileName));
        }
    }

    private static bool IsUnsupportedRelocationTable(string fileName)
    {
        var extension = Path.GetExtension(fileName);
        return extension is ".rtv" or ".rts" or ".rtl" or ".rtd" or ".rtg";
    }

    private static bool TryExportUnsupportedRelocationTable(
        string packageDir,
        RetePackageManifest manifest,
        RelocationTableFileManifest table,
        string outputDir)
    {
        var extension = Path.GetExtension(table.FileName);
        if (extension.Equals(".rtb", StringComparison.OrdinalIgnoreCase) ||
            extension is ".rtp" or ".rtt")
        {
            return false;
        }

        var looseFile = manifest.LooseFiles.FirstOrDefault(file =>
            file.FileName.Equals(table.FileName, StringComparison.OrdinalIgnoreCase));
        if (looseFile == null)
        {
            return false;
        }

        var sourcePath = ResolvePath(packageDir, looseFile.Path);
        if (!File.Exists(sourcePath))
        {
            return false;
        }

        var outputPath = Path.Combine(outputDir, table.FileName);
        Directory.CreateDirectory(Path.GetDirectoryName(outputPath)!);
        File.Copy(sourcePath, outputPath, overwrite: true);
        return true;
    }

    private static void CompileRelocationTable(string intermediateDir, RelocationTableDocument document, string outputPath)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(outputPath)!);
        File.WriteAllBytes(outputPath, BuildRelocationTableBytes(document));
    }

    internal static byte[] BuildRelocationTableBytes(RelocationTableDocument document)
    {
        if (document.Blocks.Count > byte.MaxValue)
        {
            throw new InvalidDataException($"Relocation table {document.FileName} has too many pointer blocks.");
        }

        using var stream = new MemoryStream();
        using var writer = new BinaryWriter(stream);
        writer.Write((byte)document.Blocks.Count);

        foreach (var block in document.Blocks.OrderBy(b => b.Order))
        {
            writer.Write(block.Module);
            writer.Write(block.Id);
            writer.Write((uint)block.Pointers.Count);

            if (block.Pointers.Count == 0)
            {
                continue;
            }

            WriteRelocationPointerBlock(writer, block);
        }

        writer.Flush();
        return stream.ToArray();
    }

    private static void WriteRelocationPointerBlock(BinaryWriter writer, RelocationPointerBlockManifest block)
    {
        var pointerData = BuildPointerData(block);
        var decompressedSize = (uint)pointerData.Length;
        var decompressedChecksum = OpenSpaceChecksum.Calculate(pointerData);

        if (OpenSpaceLzo.TryCompress(pointerData, out var compressed) &&
            compressed.Length < pointerData.Length)
        {
            writer.Write(1u);
            writer.Write((uint)compressed.Length);
            writer.Write(OpenSpaceChecksum.Calculate(compressed));
            writer.Write(decompressedSize);
            writer.Write(decompressedChecksum);
            writer.Write(compressed);
            return;
        }

        writer.Write(0u);
        writer.Write(decompressedSize);
        writer.Write(decompressedChecksum);
        writer.Write(decompressedSize);
        writer.Write(decompressedChecksum);
        writer.Write(pointerData);
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

        if (!string.IsNullOrWhiteSpace(block.TrailingDataBase64))
        {
            writer.Write(Convert.FromBase64String(block.TrailingDataBase64));
        }

        writer.Flush();
        return stream.ToArray();
    }

    private static SemanticContext TryBuildSemanticContext(string levelDir, string levelName)
    {
        var context = new SemanticContext();

        try
        {
            var gptPath = FindFile(levelDir, $"{levelName}.gpt");
            if (gptPath == null)
            {
                context.Errors.Add($"GPT file not found for {levelName}.");
                return context;
            }

            var loader = new LevelLoader(levelDir, levelName);
            var gpt = new GptReader(gptPath);
            var memory = new MemoryContext(loader.Sna, loader.Rtb);
            var tracker = new ByteRangeTracker();
            var sceneReader = new TrackingSuperObjectReader(memory, tracker);
            var sceneGraph = sceneReader.ReadSceneGraph(gpt);
            var coverage = tracker.ComputeCoverage(loader.Sna.Blocks);

            context.Tracker = tracker;
            context.SceneGraph = sceneGraph;
            context.Coverage = coverage;
            context.Memory = memory;
        }
        catch (Exception ex)
        {
            context.Errors.Add(ex.Message);
        }

        return context;
    }

    private static SemanticManifest WriteSemanticMetadata(
        string levelName,
        string outputDir,
        SemanticContext? context)
    {
        var semantic = new SemanticManifest();
        if (context == null)
        {
            return semantic;
        }

        semantic.Errors.AddRange(context.Errors);

        if (context.SceneGraph == null || context.Coverage == null || context.Tracker == null)
        {
            return semantic;
        }

        var sceneGraph = context.SceneGraph;
        var coverage = context.Coverage;
        var tracker = context.Tracker;

        var sceneDocument = new SemanticSceneDocument
        {
            LevelName = levelName,
            TotalNodes = sceneGraph.AllNodes.Count,
            Roots =
            {
                ["actualWorld"] = ToSemanticNode(sceneGraph.ActualWorld),
                ["dynamicWorld"] = ToSemanticNode(sceneGraph.DynamicWorld),
                ["fatherSector"] = ToSemanticNode(sceneGraph.FatherSector)
            }
        };

        var coverageDocument = new SemanticCoverageDocument
        {
            LevelName = levelName,
            TotalBytes = coverage.TotalBytes,
            CoveredBytes = coverage.CoveredBytes,
            UncoveredBytes = coverage.UncoveredBytes,
            CoveragePercent = coverage.CoveragePercent,
            Ranges = tracker.Ranges.Select(ToSemanticRange).ToList(),
            UncoveredRegions = coverage.UncoveredRegions.Select(ToSemanticRange).ToList(),
            Blocks = coverage.BlockStats.Select(b => new SemanticBlockCoverage
            {
                Key = ToKey(b.Block.Module, b.Block.Id),
                Module = b.Block.Module,
                Id = b.Block.Id,
                BaseInMemory = b.Block.BaseInMemory,
                BaseInMemoryHex = ToHex(b.Block.BaseInMemory),
                TotalBytes = b.TotalBytes,
                CoveredBytes = b.CoveredBytes,
                UncoveredBytes = b.UncoveredBytes,
                CoveragePercent = b.CoveragePercent
            }).ToList()
        };

        semantic.SceneTreePath = "semantic/scene-tree.json";
        semantic.CoveragePath = "semantic/coverage.json";
        WriteJson(ResolvePath(outputDir, semantic.SceneTreePath), sceneDocument);
        WriteJson(ResolvePath(outputDir, semantic.CoveragePath), coverageDocument);

        return semantic;
    }

    private static SemanticSceneNode? ToSemanticNode(SceneNode? node)
    {
        if (node == null)
        {
            return null;
        }

        return new SemanticSceneNode
        {
            Address = node.Address,
            AddressHex = ToHex(node.Address),
            Type = node.Type.ToString(),
            TypeCode = node.TypeCode,
            Name = node.Name,
            OffData = node.OffData,
            GeometricObjectAddress = node.GeometricObjectAddress,
            OffMatrix = node.OffMatrix,
            OffStaticMatrix = node.OffStaticMatrix,
            OffBoundingVolume = node.OffBoundingVolume,
            DrawFlags = node.DrawFlags,
            Flags = node.Flags,
            FamilyIndex = node.FamilyIndex,
            ModelIndex = node.ModelIndex,
            InstanceIndex = node.InstanceIndex,
            Children = node.Children.Select(ToSemanticNode).Where(n => n != null).Select(n => n!).ToList()
        };
    }

    private static SemanticByteRange ToSemanticRange(ByteRange range)
    {
        return new SemanticByteRange
        {
            Start = range.Start,
            StartHex = ToHex(range.Start),
            End = range.End,
            EndHex = ToHex(range.End),
            Length = range.Length,
            Label = range.Label
        };
    }

    private static string? FindFile(string dir, string fileName)
    {
        var exact = Path.Combine(dir, fileName);
        if (File.Exists(exact))
        {
            return exact;
        }

        return Directory.GetFiles(dir, fileName + "*", SearchOption.TopDirectoryOnly).FirstOrDefault();
    }

    private static void WriteBytes(string rootDir, string relativePath, byte[] data)
    {
        var path = ResolvePath(rootDir, relativePath);
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllBytes(path, data);
    }

    private static void WriteJson<T>(string path, T value)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllText(path, JsonSerializer.Serialize(value, JsonOptions));
    }

    private static T ReadJson<T>(string path)
    {
        var value = JsonSerializer.Deserialize<T>(File.ReadAllText(path), JsonOptions);
        return value ?? throw new InvalidDataException($"Could not read JSON document: {path}");
    }

    private static string ResolvePath(string rootDir, string relativePath)
    {
        return Path.Combine(relativePath.Split('/').Prepend(rootDir).ToArray());
    }

    private static string HashBytes(byte[] data)
    {
        return Convert.ToHexString(SHA256.HashData(data)).ToLowerInvariant();
    }

    private sealed class SemanticContext
    {
        public ByteRangeTracker? Tracker { get; set; }
        public SceneGraph? SceneGraph { get; set; }
        public CoverageStats? Coverage { get; set; }
        public MemoryContext? Memory { get; set; }
        public List<string> Errors { get; } = new();
    }

    private sealed record SnaContentPlan(int Start, int Length, string Kind, List<string> Labels);

    private sealed record IndexedContentElement(
        string Kind,
        string DataPath,
        int VirtualAddress,
        int Length,
        int OffsetInBlock);

    private static Dictionary<(byte Module, byte Id), byte[]?> BuildImportBlockByteCache(
        string packageDir,
        RetePackageManifest manifest)
    {
        var cache = new Dictionary<(byte Module, byte Id), byte[]?>();
        foreach (var snaFile in manifest.SnaFiles)
        {
            foreach (var block in snaFile.Blocks)
            {
                cache[(block.Module, block.Id)] = TryLoadImportBlockBytes(packageDir, block, out var bytes)
                    ? bytes
                    : null;
            }
        }

        return cache;
    }

    private static bool TryLoadImportBlockBytes(
        string packageDir,
        SnaBlockManifest block,
        out byte[] bytes)
    {
        bytes = [];
        if (!block.HasPayload)
        {
            return false;
        }

        if (block.ContentPath == null)
        {
            return false;
        }

        try
        {
            var resolver = new ReferenceAddressResolver(packageDir);
            bytes = ReadSnaBlockData(packageDir, block, resolver);
            return bytes.Length >= sizeof(int);
        }
        catch (InvalidDataException)
        {
            return false;
        }
    }

    private static int GetImportPointerSpanLength(IndexedContentElement element, byte[]? blockBytes)
    {
        var spanLength = element.Length;
        if (blockBytes != null &&
            element.OffsetInBlock >= 0 &&
            element.OffsetInBlock < blockBytes.Length)
        {
            spanLength = Math.Max(spanLength, blockBytes.Length - element.OffsetInBlock);
        }

        return spanLength;
    }

    private static bool TryReadOpaqueImportPointerValue(
        IndexedContentElement element,
        int offsetInElement,
        byte[]? blockBytes,
        ReadOnlySpan<byte> opaqueData,
        out int value)
    {
        value = 0;
        if (blockBytes != null &&
            element.OffsetInBlock + offsetInElement + sizeof(int) <= blockBytes.Length)
        {
            value = BinaryPrimitives.ReadInt32LittleEndian(
                blockBytes.AsSpan(element.OffsetInBlock + offsetInElement, sizeof(int)));
            return true;
        }

        if (offsetInElement + sizeof(int) <= opaqueData.Length)
        {
            value = BinaryPrimitives.ReadInt32LittleEndian(opaqueData.Slice(offsetInElement, sizeof(int)));
            return true;
        }

        return false;
    }

    private static bool TryReadImportPointerValue(
        IndexedContentElement element,
        int offsetInElement,
        byte[]? blockBytes,
        string packageDir,
        ReferenceAddressResolver resolver,
        out int value)
    {
        value = 0;
        if (blockBytes != null &&
            element.OffsetInBlock + offsetInElement + sizeof(int) <= blockBytes.Length)
        {
            value = BinaryPrimitives.ReadInt32LittleEndian(
                blockBytes.AsSpan(element.OffsetInBlock + offsetInElement, sizeof(int)));
            return true;
        }

        if (!StructCodecRegistry.TryGet(element.Kind, out _))
        {
            return false;
        }

        var elementBytes = ReferenceJson.WriteElementBytesForExport(
            packageDir,
            element.Kind,
            element.DataPath,
            resolver);
        if (offsetInElement + sizeof(int) > elementBytes.Length)
        {
            return false;
        }

        value = BinaryPrimitives.ReadInt32LittleEndian(elementBytes.AsSpan(offsetInElement, sizeof(int)));
        return true;
    }

    private static bool TryResolveLevelTargetPath(
        string levelPackageDir,
        string fixPackageDir,
        string resolvedLevelUri,
        out string sourcePath)
    {
        if (ReferenceUri.TryResolve(levelPackageDir, resolvedLevelUri, out sourcePath, out _))
        {
            return true;
        }

        return ReferenceUri.TryResolve(
            fixPackageDir,
            resolvedLevelUri,
            out sourcePath,
            out _,
            levelPackageRoot: levelPackageDir);
    }

    private static string MakeLevelSlotUri(int fixSiteAddress) =>
        $"{ReferenceUri.LevelPrefix}slots/0x{fixSiteAddress:X8}.json";

    private static string MakeLevelSlotRelativePath(int fixSiteAddress) =>
        $"slots/0x{fixSiteAddress:X8}.json";

    private static string? WriteLevelSlotForFixSite(
        string levelPackageDir,
        string fixPackageDir,
        int fixSiteAddress,
        string resolvedLevelUri,
        ReferenceAddressResolver resolver)
    {
        if (!TryResolveLevelTargetPath(
                levelPackageDir,
                fixPackageDir,
                resolvedLevelUri,
                out var sourcePath) ||
            !File.Exists(sourcePath))
        {
            return null;
        }

        var slotRelativePath = MakeLevelSlotRelativePath(fixSiteAddress);
        var slotPath = Path.Combine(levelPackageDir, slotRelativePath.Replace('/', Path.DirectorySeparatorChar));
        Directory.CreateDirectory(Path.GetDirectoryName(slotPath)!);
        File.Copy(sourcePath, slotPath, overwrite: true);

        resolver.LoadPackage(levelPackageDir);
        return MakeLevelSlotUri(fixSiteAddress);
    }

    private sealed class PendingOpaquePointerRecord
    {
        public PendingOpaquePointerRecord(IStructCodecBinding codec, OpaqueBinaryRecord record)
        {
            Codec = codec;
            Record = record;
        }

        public IStructCodecBinding Codec { get; }
        public OpaqueBinaryRecord Record { get; }
        public bool Changed { get; set; }
    }

    private static string ToKey(byte module, byte id)
    {
        return $"{module:X2}:{id:X2}";
    }

    private static string ToHex(int value)
    {
        return $"0x{value:X8}";
    }

    internal static bool IsRetePackageDirectory(string path) =>
        File.Exists(Path.Combine(path, ManifestFileName));

    internal static RetePackageManifest ReadReteManifest(string packageDir)
    {
        var manifestPath = Path.Combine(packageDir, ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            throw new FileNotFoundException($"Rete manifest not found: {manifestPath}");
        }

        var manifest = ReadJson<RetePackageManifest>(manifestPath);
        ValidateReteManifestSchema(manifest.Schema);
        return manifest;
    }

    internal static void ValidateReteManifestSchema(string schema)
    {
        if (!string.Equals(schema, ReteManifestSchema, StringComparison.Ordinal))
        {
            throw new InvalidDataException($"Unsupported Rete manifest schema: {schema}");
        }
    }

    private static RelocationGenerator.RelocationPackageContext CreateRelocationPackageContext(
        string packageDir,
        IReadOnlyList<string> targetPackageRoots)
    {
        var relocationContext = new RelocationGenerator.RelocationPackageContext();
        relocationContext.EnsureLayout(packageDir);
        foreach (var targetPackageRoot in targetPackageRoots)
        {
            relocationContext.EnsureLayout(targetPackageRoot);
        }

        return relocationContext;
    }

    private static bool TryGenerateRelocationTableDocument(
        string packageDir,
        RetePackageManifest manifest,
        RelocationTableFileManifest table,
        IReadOnlyList<string> targetPackageRoots,
        RelocationGenerator.RelocationPackageContext relocationContext,
        out RelocationTableDocument document)
    {
        var extension = Path.GetExtension(table.FileName);
        if (extension.Equals(".rtb", StringComparison.OrdinalIgnoreCase))
        {
            if (table.FileName.Equals("fixlvl.rtb", StringComparison.OrdinalIgnoreCase))
            {
                var fixPackageDir = targetPackageRoots.FirstOrDefault()
                    ?? throw new InvalidDataException("fixlvl.rtb requires a sibling Fix Rete package.");
                document = RelocationGenerator.GenerateFixLevelRtb(
                    fixPackageDir,
                    packageDir,
                    table.FileName,
                    relocationContext);
                return true;
            }

            document = RelocationGenerator.GenerateRtb(
                packageDir,
                table.FileName,
                targetPackageRoots,
                context: relocationContext);
            return true;
        }

        if (TryGetPointerFilePath(packageDir, manifest, table.FileName, out var pointerFilePath))
        {
            document = RelocationGenerator.GeneratePointerFileTable(
                packageDir,
                table.FileName,
                pointerFilePath,
                targetPackageRoots,
                relocationContext);
            return true;
        }

        if (IsUnsupportedRelocationTable(table.FileName))
        {
            document = null!;
            return false;
        }

        throw new InvalidDataException(
            $"Relocation generation is not implemented for table {table.FileName}.");
    }

    private static void AssignLevelRelocationReader(
        string fileName,
        string levelName,
        RelocationTableReader reader,
        ref RelocationTableReader? rtb,
        ref RelocationTableReader? rtp,
        ref RelocationTableReader? rtt)
    {
        if (fileName.Equals($"{levelName}.rtb", StringComparison.OrdinalIgnoreCase))
        {
            rtb = reader;
            return;
        }

        if (fileName.Equals($"{levelName}.rtp", StringComparison.OrdinalIgnoreCase))
        {
            rtp = reader;
            return;
        }

        if (fileName.Equals($"{levelName}.rtt", StringComparison.OrdinalIgnoreCase))
        {
            rtt = reader;
        }
    }

    internal static string? FindLooseFilePath(RetePackageManifest manifest, string packageDir, string fileName)
    {
        var looseFile = manifest.LooseFiles.FirstOrDefault(file =>
            file.FileName.Equals(fileName, StringComparison.OrdinalIgnoreCase));
        return looseFile == null ? null : ResolvePath(packageDir, looseFile.Path);
    }

}
