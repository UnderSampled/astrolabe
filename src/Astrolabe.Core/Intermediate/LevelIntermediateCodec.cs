using System.Security.Cryptography;
using System.Text.Json;
using Astrolabe.Core.FileFormats;
using Astrolabe.Core.Serialization;

namespace Astrolabe.Core.Intermediate;

public static class LevelIntermediateCodec
{
    public const string ManifestFileName = "manifest.json";

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

    public static LevelIntermediateManifest ExtractLevel(string levelDir, string outputDir)
    {
        if (!Directory.Exists(levelDir))
        {
            throw new DirectoryNotFoundException($"Level directory not found: {levelDir}");
        }

        Directory.CreateDirectory(outputDir);

        var levelName = Path.GetFileName(levelDir.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar));
        var manifest = new LevelIntermediateManifest
        {
            LevelName = levelName,
            SourceDirectoryName = Path.GetFileName(levelDir.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar))
        };

        var handledFiles = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var files = Directory.GetFiles(levelDir).OrderBy(Path.GetFileName, StringComparer.OrdinalIgnoreCase).ToList();
        var semanticContext = TryBuildSemanticContext(levelDir, levelName);
        var sceneNodePaths = WriteSceneSourceTree(outputDir, semanticContext);

        foreach (var snaPath in files.Where(f => Path.GetExtension(f).Equals(".sna", StringComparison.OrdinalIgnoreCase)))
        {
            manifest.SnaFiles.Add(ExtractSnaFile(snaPath, outputDir, semanticContext?.Tracker, sceneNodePaths));
            handledFiles.Add(snaPath);
        }

        foreach (var relocationPath in files.Where(f => RelocationExtensions.Contains(Path.GetExtension(f))))
        {
            try
            {
                manifest.RelocationTables.Add(ExtractRelocationTable(relocationPath, outputDir));
                handledFiles.Add(relocationPath);
            }
            catch
            {
                // Some RT* files can be tiny placeholders in shipped data. Keep
                // unsupported tables as exact loose leaves rather than dropping them.
            }
        }

        foreach (var file in files.Where(f => !handledFiles.Contains(f)))
        {
            manifest.LooseFiles.Add(CopyLooseFile(file, outputDir));
        }

        manifest.Semantic = WriteSemanticMetadata(levelName, outputDir, semanticContext);

        WriteJson(Path.Combine(outputDir, ManifestFileName), manifest);
        return manifest;
    }

    public static void CompileLevel(string intermediateDir, string outputDir)
    {
        var manifestPath = Path.Combine(intermediateDir, ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            throw new FileNotFoundException($"Intermediate manifest not found: {manifestPath}");
        }

        var manifest = ReadJson<LevelIntermediateManifest>(manifestPath);
        if (manifest.Schema != "astrolabe.level-intermediate.v1")
        {
            throw new InvalidDataException($"Unsupported intermediate schema: {manifest.Schema}");
        }

        Directory.CreateDirectory(outputDir);

        foreach (var snaFile in manifest.SnaFiles)
        {
            CompileSnaFile(intermediateDir, snaFile, Path.Combine(outputDir, snaFile.FileName));
        }

        foreach (var table in manifest.RelocationTables)
        {
            var tableDocument = ReadJson<RelocationTableDocument>(ResolvePath(intermediateDir, table.JsonPath));
            CompileRelocationTable(intermediateDir, tableDocument, Path.Combine(outputDir, table.FileName));
        }

        foreach (var looseFile in manifest.LooseFiles)
        {
            var sourcePath = ResolvePath(intermediateDir, looseFile.Path);
            var outputPath = Path.Combine(outputDir, looseFile.FileName);
            Directory.CreateDirectory(Path.GetDirectoryName(outputPath)!);
            File.Copy(sourcePath, outputPath, overwrite: true);
        }
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

                var storage = new SnaStorageManifest
                {
                    IsCompressed = block.IsCompressed,
                    CompressedSize = block.CompressedSize,
                    CompressedChecksum = block.CompressedChecksum,
                    DecompressedSize = block.DecompressedSize,
                    DecompressedChecksum = block.DecompressedChecksum
                };

                if (block.CompressedData is { Length: > 0 })
                {
                    var encodedPath = $"{blockDir}/{blockStem}.encoded.bin";
                    WriteBytes(outputDir, encodedPath, block.CompressedData);
                    storage.EncodedPath = encodedPath;
                    storage.EncodedSha256 = HashBytes(block.CompressedData);
                }

                blockManifest.OriginalStorage = storage;
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

            if (plan.Kind == "superObject")
            {
                var superObject = ReadIntermediateSuperObject(data, plan.Start);
                var virtualAddress = block.BaseInMemory + plan.Start;
                if (sceneNodePaths.TryGetValue(virtualAddress, out var sceneNodePath))
                {
                    dataPath = sceneNodePath;
                }
                else
                {
                    dataPath = GetTypedDataPath(plan.Kind, blockStem, i, "json");
                    WriteJson(ResolvePath(outputDir, dataPath), superObject);
                }

                emittedBytes = WriteIntermediateSuperObject(superObject);
            }
            else if (plan.Kind == "matrix")
            {
                var matrix = ReadIntermediateMatrix(data, plan.Start);
                var virtualAddress = block.BaseInMemory + plan.Start;
                if (sceneNodePaths.TryGetValue(virtualAddress, out var matrixPath))
                {
                    dataPath = matrixPath;
                }
                else
                {
                    dataPath = GetTypedDataPath(plan.Kind, blockStem, i, "json");
                    WriteJson(ResolvePath(outputDir, dataPath), matrix);
                }

                emittedBytes = WriteIntermediateMatrix(matrix);
            }
            else if (plan.Kind == "geometricobject")
            {
                var value = ReadIntermediateGeometricObject(data, plan.Start);
                dataPath = GetTypedDataPath(plan.Kind, blockStem, i, "json");
                WriteJson(ResolvePath(outputDir, dataPath), value);
                emittedBytes = WriteIntermediateGeometricObject(value);
            }
            else if (plan.Kind == "physicalobject")
            {
                var value = ReadIntermediatePhysicalObject(data, plan.Start);
                dataPath = GetTypedDataPath(plan.Kind, blockStem, i, "json");
                WriteJson(ResolvePath(outputDir, dataPath), value);
                emittedBytes = WriteIntermediatePhysicalObject(value);
            }
            else if (plan.Kind == "ipo")
            {
                var value = ReadIntermediateIpo(data, plan.Start);
                dataPath = GetTypedDataPath(plan.Kind, blockStem, i, "json");
                WriteJson(ResolvePath(outputDir, dataPath), value);
                emittedBytes = WriteIntermediateIpo(value);
            }
            else if (plan.Kind == "gamematerial")
            {
                var value = ReadIntermediateGameMaterial(data, plan.Start);
                dataPath = GetTypedDataPath(plan.Kind, blockStem, i, "json");
                WriteJson(ResolvePath(outputDir, dataPath), value);
                emittedBytes = WriteIntermediateGameMaterial(value);
            }
            else if (plan.Kind is "boundingvolume" or "collidematerial")
            {
                var value = ReadIntermediateUInt32Record(data, plan.Start, plan.Length, plan.Kind);
                dataPath = GetTypedDataPath(plan.Kind, blockStem, i, "json");
                WriteJson(ResolvePath(outputDir, dataPath), value);
                emittedBytes = WriteIntermediateUInt32Record(value);
            }
            else if (plan.Kind is "vertices" or "normals" or "trianglenormals")
            {
                var value = ReadIntermediateFloat3Array(data, plan.Start, plan.Length, plan.Kind);
                dataPath = GetTypedDataPath(plan.Kind, blockStem, i, "json");
                WriteJson(ResolvePath(outputDir, dataPath), value);
                emittedBytes = WriteIntermediateFloat3Array(value);
            }
            else if (StructCodecRegistry.TryGet(plan.Kind, out var codec))
            {
                var value = codec.ReadFromBytes(data, plan.Start, plan.Length);
                dataPath = GetTypedDataPath(plan.Kind, blockStem, i, "json");
                codec.WriteJson(ResolvePath(outputDir, dataPath), value);
                emittedBytes = codec.WriteFromObject(value);
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
            .OrderBy(r => r.End - r.Start)
            .ThenBy(r => r.Start)
            .ToList();

        var selectedRanges = new List<(int Start, int End, string Kind, List<string> Labels)>();
        foreach (var range in typedRanges)
        {
            if (selectedRanges.Any(r => r.Start < range.End && r.End > range.Start))
            {
                continue;
            }

            selectedRanges.Add(range);
        }

        if (selectedRanges.Count == 0)
        {
            return SplitRawContent(data, 0, length, MaxSegmentLength);
        }

        selectedRanges.Sort((a, b) => a.Start != b.Start ? a.Start.CompareTo(b.Start) : a.End.CompareTo(b.End));

        var plans = new List<SnaContentPlan>();
        var cursor = 0;
        foreach (var range in selectedRanges)
        {
            if (range.Start < cursor)
            {
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

        return plans;
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

        var superObject = ReadIntermediateSuperObject(data, 0);
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

        sceneNode.MatrixPath = WriteSceneMatrix(outputDir, nodeDir, "matrix", superObject.Matrix, memory, paths);
        sceneNode.StaticMatrixPath = WriteSceneMatrix(outputDir, nodeDir, "static_matrix", superObject.StaticMatrix, memory, paths);

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

        var matrix = ReadIntermediateMatrix(data, 0);
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

    private static IntermediateSuperObject ReadIntermediateSuperObject(byte[] data, int start)
    {
        using var reader = new BinaryReader(new MemoryStream(data, start, 0x38));
        var typeCode = reader.ReadUInt32();

        return new IntermediateSuperObject
        {
            TypeCode = typeCode,
            Type = TrackingSuperObjectReader.GetSuperObjectType(typeCode).ToString(),
            OffData = reader.ReadInt32(),
            ChildrenHead = reader.ReadInt32(),
            ChildrenTail = reader.ReadInt32(),
            ChildrenCount = reader.ReadUInt32(),
            BrotherNext = reader.ReadInt32(),
            BrotherPrev = reader.ReadInt32(),
            Parent = reader.ReadInt32(),
            Matrix = reader.ReadInt32(),
            StaticMatrix = reader.ReadInt32(),
            GlobalMatrix = reader.ReadInt32(),
            DrawFlags = reader.ReadUInt32(),
            Flags = reader.ReadUInt32(),
            BoundingVolume = reader.ReadInt32()
        };
    }

    private static byte[] WriteIntermediateSuperObject(IntermediateSuperObject superObject)
    {
        using var stream = new MemoryStream(0x38);
        using var writer = new BinaryWriter(stream);

        writer.Write(superObject.TypeCode);
        writer.Write(superObject.OffData);
        writer.Write(superObject.ChildrenHead);
        writer.Write(superObject.ChildrenTail);
        writer.Write(superObject.ChildrenCount);
        writer.Write(superObject.BrotherNext);
        writer.Write(superObject.BrotherPrev);
        writer.Write(superObject.Parent);
        writer.Write(superObject.Matrix);
        writer.Write(superObject.StaticMatrix);
        writer.Write(superObject.GlobalMatrix);
        writer.Write(superObject.DrawFlags);
        writer.Write(superObject.Flags);
        writer.Write(superObject.BoundingVolume);
        writer.Flush();

        return stream.ToArray();
    }

    private static IntermediateMatrix ReadIntermediateMatrix(byte[] data, int start)
    {
        using var reader = new BinaryReader(new MemoryStream(data, start, 88));
        var matrix = new IntermediateMatrix
        {
            Type = reader.ReadUInt32(),
            Translation = [reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle()],
            BasisX = [reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle()],
            BasisY = [reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle()],
            BasisZ = [reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle()]
        };

        var remaining = 88 - (int)reader.BaseStream.Position;
        if (remaining > 0)
        {
            matrix.ExtraBase64 = Convert.ToBase64String(reader.ReadBytes(remaining));
        }

        return matrix;
    }

    private static byte[] WriteIntermediateMatrix(IntermediateMatrix matrix)
    {
        using var stream = new MemoryStream(88);
        using var writer = new BinaryWriter(stream);

        writer.Write(matrix.Type);
        WriteFloat3(writer, matrix.Translation, nameof(matrix.Translation));
        WriteFloat3(writer, matrix.BasisX, nameof(matrix.BasisX));
        WriteFloat3(writer, matrix.BasisY, nameof(matrix.BasisY));
        WriteFloat3(writer, matrix.BasisZ, nameof(matrix.BasisZ));

        if (!string.IsNullOrWhiteSpace(matrix.ExtraBase64))
        {
            writer.Write(Convert.FromBase64String(matrix.ExtraBase64));
        }

        writer.Flush();
        var bytes = stream.ToArray();
        if (bytes.Length != 88)
        {
            throw new InvalidDataException($"Matrix serialized to {bytes.Length} bytes, expected 88 bytes.");
        }

        return bytes;
    }

    private static void WriteFloat3(BinaryWriter writer, float[] values, string fieldName)
    {
        if (values.Length != 3)
        {
            throw new InvalidDataException($"{fieldName} must contain exactly 3 floats.");
        }

        writer.Write(values[0]);
        writer.Write(values[1]);
        writer.Write(values[2]);
    }

    private static IntermediateGeometricObject ReadIntermediateGeometricObject(byte[] data, int start)
    {
        using var reader = new BinaryReader(new MemoryStream(data, start, 0x40));
        return new IntermediateGeometricObject
        {
            NumVertices = reader.ReadUInt32(),
            Vertices = reader.ReadInt32(),
            Normals = reader.ReadInt32(),
            Materials = reader.ReadInt32(),
            Unknown0 = reader.ReadInt32(),
            NumElements = reader.ReadUInt32(),
            ElementTypes = reader.ReadInt32(),
            Elements = reader.ReadInt32(),
            Unknowns = [reader.ReadInt32(), reader.ReadInt32(), reader.ReadInt32(), reader.ReadInt32()],
            SphereRadius = reader.ReadSingle(),
            SphereCenterRaw = [reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle()]
        };
    }

    private static byte[] WriteIntermediateGeometricObject(IntermediateGeometricObject value)
    {
        using var stream = new MemoryStream(0x40);
        using var writer = new BinaryWriter(stream);
        writer.Write(value.NumVertices);
        writer.Write(value.Vertices);
        writer.Write(value.Normals);
        writer.Write(value.Materials);
        writer.Write(value.Unknown0);
        writer.Write(value.NumElements);
        writer.Write(value.ElementTypes);
        writer.Write(value.Elements);
        WriteIntArray(writer, value.Unknowns, 4, nameof(value.Unknowns));
        writer.Write(value.SphereRadius);
        WriteFloat3(writer, value.SphereCenterRaw, nameof(value.SphereCenterRaw));
        writer.Flush();
        return RequireLength(stream.ToArray(), 0x40, nameof(IntermediateGeometricObject));
    }

    private static IntermediatePhysicalObject ReadIntermediatePhysicalObject(byte[] data, int start)
    {
        using var reader = new BinaryReader(new MemoryStream(data, start, 0x10));
        return new IntermediatePhysicalObject
        {
            VisualSet = reader.ReadInt32(),
            CollideSet = reader.ReadInt32(),
            VisualBoundingVolume = reader.ReadInt32(),
            Unknown0 = reader.ReadInt32()
        };
    }

    private static byte[] WriteIntermediatePhysicalObject(IntermediatePhysicalObject value)
    {
        using var stream = new MemoryStream(0x10);
        using var writer = new BinaryWriter(stream);
        writer.Write(value.VisualSet);
        writer.Write(value.CollideSet);
        writer.Write(value.VisualBoundingVolume);
        writer.Write(value.Unknown0);
        writer.Flush();
        return RequireLength(stream.ToArray(), 0x10, nameof(IntermediatePhysicalObject));
    }

    private static IntermediateIpo ReadIntermediateIpo(byte[] data, int start)
    {
        using var reader = new BinaryReader(new MemoryStream(data, start, 8));
        return new IntermediateIpo
        {
            PhysicalObject = reader.ReadInt32(),
            Radiosity = reader.ReadInt32()
        };
    }

    private static byte[] WriteIntermediateIpo(IntermediateIpo value)
    {
        using var stream = new MemoryStream(8);
        using var writer = new BinaryWriter(stream);
        writer.Write(value.PhysicalObject);
        writer.Write(value.Radiosity);
        writer.Flush();
        return RequireLength(stream.ToArray(), 8, nameof(IntermediateIpo));
    }

    private static IntermediateGameMaterial ReadIntermediateGameMaterial(byte[] data, int start)
    {
        using var reader = new BinaryReader(new MemoryStream(data, start, 0x10));
        return new IntermediateGameMaterial
        {
            VisualMaterial = reader.ReadInt32(),
            MechanicsMaterial = reader.ReadInt32(),
            SoundMaterial = reader.ReadUInt32(),
            CollideMaterial = reader.ReadInt32()
        };
    }

    private static byte[] WriteIntermediateGameMaterial(IntermediateGameMaterial value)
    {
        using var stream = new MemoryStream(0x10);
        using var writer = new BinaryWriter(stream);
        writer.Write(value.VisualMaterial);
        writer.Write(value.MechanicsMaterial);
        writer.Write(value.SoundMaterial);
        writer.Write(value.CollideMaterial);
        writer.Flush();
        return RequireLength(stream.ToArray(), 0x10, nameof(IntermediateGameMaterial));
    }

    private static IntermediateUInt32Record ReadIntermediateUInt32Record(byte[] data, int start, int length, string type)
    {
        if (length % 4 != 0)
        {
            throw new InvalidDataException($"{type} length {length} is not a multiple of 4.");
        }

        using var reader = new BinaryReader(new MemoryStream(data, start, length));
        var values = new uint[length / 4];
        for (var i = 0; i < values.Length; i++)
        {
            values[i] = reader.ReadUInt32();
        }

        return new IntermediateUInt32Record { Type = type, Values = values };
    }

    private static byte[] WriteIntermediateUInt32Record(IntermediateUInt32Record value)
    {
        using var stream = new MemoryStream(value.Values.Length * 4);
        using var writer = new BinaryWriter(stream);
        foreach (var item in value.Values)
        {
            writer.Write(item);
        }

        writer.Flush();
        return stream.ToArray();
    }

    private static IntermediateFloat3Array ReadIntermediateFloat3Array(byte[] data, int start, int length, string type)
    {
        if (length % 12 != 0)
        {
            throw new InvalidDataException($"{type} length {length} is not a multiple of 12.");
        }

        using var reader = new BinaryReader(new MemoryStream(data, start, length));
        var values = new float[length / 12][];
        for (var i = 0; i < values.Length; i++)
        {
            values[i] = [reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle()];
        }

        return new IntermediateFloat3Array { Type = type, Values = values };
    }

    private static byte[] WriteIntermediateFloat3Array(IntermediateFloat3Array value)
    {
        using var stream = new MemoryStream(value.Values.Length * 12);
        using var writer = new BinaryWriter(stream);
        for (var i = 0; i < value.Values.Length; i++)
        {
            WriteFloat3(writer, value.Values[i], $"{value.Type}[{i}]");
        }

        writer.Flush();
        return stream.ToArray();
    }

    private static void WriteIntArray(BinaryWriter writer, int[] values, int expectedLength, string fieldName)
    {
        if (values.Length != expectedLength)
        {
            throw new InvalidDataException($"{fieldName} must contain exactly {expectedLength} integers.");
        }

        foreach (var value in values)
        {
            writer.Write(value);
        }
    }

    private static byte[] RequireLength(byte[] data, int expectedLength, string typeName)
    {
        if (data.Length != expectedLength)
        {
            throw new InvalidDataException($"{typeName} serialized to {data.Length} bytes, expected {expectedLength} bytes.");
        }

        return data;
    }

    private static RelocationTableFileManifest ExtractRelocationTable(string tablePath, string outputDir)
    {
        var reader = new RelocationTableReader(tablePath);
        var fileName = Path.GetFileName(tablePath);
        var stem = fileName;
        var document = new RelocationTableDocument { FileName = fileName };

        for (var i = 0; i < reader.PointerBlocks.Count; i++)
        {
            var block = reader.PointerBlocks[i];
            var blockStem = $"{i:D4}_{block.Module:X2}_{block.Id:X2}";
            var blockDir = $"relocations/{stem}/blocks";

            var blockManifest = new RelocationPointerBlockManifest
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
                }).ToList()
            };

            if (block.TrailingData.Length > 0)
            {
                blockManifest.TrailingDataBase64 = Convert.ToBase64String(block.TrailingData);
            }

            if (block.Count > 0)
            {
                var storage = new RelocationStorageManifest
                {
                    IsCompressed = block.IsCompressed,
                    CompressedSize = block.CompressedSize,
                    CompressedChecksum = block.CompressedChecksum,
                    DecompressedSize = block.DecompressedSize,
                    DecompressedChecksum = block.DecompressedChecksum
                };

                if (block.CompressedData.Length > 0)
                {
                    var encodedPath = $"{blockDir}/{blockStem}.encoded.bin";
                    WriteBytes(outputDir, encodedPath, block.CompressedData);
                    storage.EncodedPath = encodedPath;
                    storage.EncodedSha256 = HashBytes(block.CompressedData);
                }

                blockManifest.OriginalStorage = storage;
            }

            document.Blocks.Add(blockManifest);
        }

        var jsonPath = $"relocations/{stem}.json";
        WriteJson(ResolvePath(outputDir, jsonPath), document);

        return new RelocationTableFileManifest
        {
            FileName = fileName,
            JsonPath = jsonPath
        };
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

    private static void CompileSnaFile(string intermediateDir, SnaFileManifest manifest, string outputPath)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(outputPath)!);

        using var writer = new BinaryWriter(File.Create(outputPath));
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
                data = ReadSnaBlockData(intermediateDir, block);
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

            if (CanReuseSnaStorage(intermediateDir, block, data, out var encodedData))
            {
                var storage = block.OriginalStorage!;
                writer.Write(storage.IsCompressed ? 1u : 0u);
                writer.Write(storage.CompressedSize);
                writer.Write(storage.CompressedChecksum);
                writer.Write(storage.DecompressedSize);
                writer.Write(storage.DecompressedChecksum);
                writer.Write(encodedData);
            }
            else
            {
                var checksum = OpenSpaceChecksum.Calculate(data);
                writer.Write(0u);
                writer.Write(size);
                writer.Write(checksum);
                writer.Write(size);
                writer.Write(checksum);
                writer.Write(data);
            }
        }
    }

    private static byte[] ReadSnaBlockData(string intermediateDir, SnaBlockManifest block)
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
                var data = element.Kind switch
                {
                    "superObject" => WriteIntermediateSuperObject(
                        ReadJson<IntermediateSuperObject>(ResolvePath(intermediateDir, element.DataPath))),
                    "matrix" => WriteIntermediateMatrix(
                        ReadJson<IntermediateMatrix>(ResolvePath(intermediateDir, element.DataPath))),
                    "geometricobject" => WriteIntermediateGeometricObject(
                        ReadJson<IntermediateGeometricObject>(ResolvePath(intermediateDir, element.DataPath))),
                    "physicalobject" => WriteIntermediatePhysicalObject(
                        ReadJson<IntermediatePhysicalObject>(ResolvePath(intermediateDir, element.DataPath))),
                    "ipo" => WriteIntermediateIpo(
                        ReadJson<IntermediateIpo>(ResolvePath(intermediateDir, element.DataPath))),
                    "gamematerial" => WriteIntermediateGameMaterial(
                        ReadJson<IntermediateGameMaterial>(ResolvePath(intermediateDir, element.DataPath))),
                    "boundingvolume" or "collidematerial" => WriteIntermediateUInt32Record(
                        ReadJson<IntermediateUInt32Record>(ResolvePath(intermediateDir, element.DataPath))),
                    "vertices" or "normals" or "trianglenormals" => WriteIntermediateFloat3Array(
                        ReadJson<IntermediateFloat3Array>(ResolvePath(intermediateDir, element.DataPath))),
                    _ when StructCodecRegistry.TryGet(element.Kind, out _) =>
                        StructCodecRegistry.ReadElementBytes(intermediateDir, element.DataPath, element.Kind),
                    _ => File.ReadAllBytes(ResolvePath(intermediateDir, element.DataPath))
                };

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

    private static void CompileRelocationTable(string intermediateDir, RelocationTableDocument document, string outputPath)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(outputPath)!);

        if (document.Blocks.Count > byte.MaxValue)
        {
            throw new InvalidDataException($"Relocation table {document.FileName} has too many pointer blocks.");
        }

        using var writer = new BinaryWriter(File.Create(outputPath));
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

            var pointerData = BuildPointerData(block);

            if (CanReuseRelocationStorage(intermediateDir, block, pointerData, out var encodedData))
            {
                var storage = block.OriginalStorage!;
                writer.Write(storage.IsCompressed ? 1u : 0u);
                writer.Write(storage.CompressedSize);
                writer.Write(storage.CompressedChecksum);
                writer.Write(storage.DecompressedSize);
                writer.Write(storage.DecompressedChecksum);
                writer.Write(encodedData);
            }
            else
            {
                var checksum = OpenSpaceChecksum.Calculate(pointerData);
                writer.Write(0u);
                writer.Write((uint)pointerData.Length);
                writer.Write(checksum);
                writer.Write((uint)pointerData.Length);
                writer.Write(checksum);
                writer.Write(pointerData);
            }
        }
    }

    private static bool CanReuseSnaStorage(string intermediateDir, SnaBlockManifest block, byte[] data, out byte[] encodedData)
    {
        encodedData = [];

        var storage = block.OriginalStorage;
        if (storage?.EncodedPath == null || storage.EncodedSha256 == null || block.DataSha256 == null)
        {
            return false;
        }

        if (!HashBytes(data).Equals(block.DataSha256, StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        if (data.Length != storage.DecompressedSize || OpenSpaceChecksum.Calculate(data) != storage.DecompressedChecksum)
        {
            return false;
        }

        var encodedPath = ResolvePath(intermediateDir, storage.EncodedPath);
        if (!File.Exists(encodedPath))
        {
            return false;
        }

        encodedData = File.ReadAllBytes(encodedPath);
        return HashBytes(encodedData).Equals(storage.EncodedSha256, StringComparison.OrdinalIgnoreCase);
    }

    private static bool CanReuseRelocationStorage(
        string intermediateDir,
        RelocationPointerBlockManifest block,
        byte[] pointerData,
        out byte[] encodedData)
    {
        encodedData = [];

        var storage = block.OriginalStorage;
        if (storage?.EncodedPath == null || storage.EncodedSha256 == null)
        {
            return false;
        }

        if (!HashBytes(pointerData).Equals(block.PointerDataSha256, StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        if (pointerData.Length != storage.DecompressedSize ||
            OpenSpaceChecksum.Calculate(pointerData) != storage.DecompressedChecksum)
        {
            return false;
        }

        var encodedPath = ResolvePath(intermediateDir, storage.EncodedPath);
        if (!File.Exists(encodedPath))
        {
            return false;
        }

        encodedData = File.ReadAllBytes(encodedPath);
        return HashBytes(encodedData).Equals(storage.EncodedSha256, StringComparison.OrdinalIgnoreCase);
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

    private static string ToKey(byte module, byte id)
    {
        return $"{module:X2}:{id:X2}";
    }

    private static string ToHex(int value)
    {
        return $"0x{value:X8}";
    }
}
