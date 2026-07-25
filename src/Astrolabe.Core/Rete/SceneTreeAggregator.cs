using System.Globalization;
using System.Text.Json;
using Astrolabe.Core.FileFormats.Semantic;

namespace Astrolabe.Core.Rete;

/// <summary>
/// Collapses the deep <c>scene/&lt;root&gt;/&lt;node&gt;/…</c> folder forest into a single
/// nested <c>scene/tree.json</c> document with byId + runs (dual-layer).
/// Authoring: named roots → byId nodes with URI child links; stream: content expand of runs.
/// </summary>
internal static class SceneTreeAggregator
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = false,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    private static readonly string[] RootNames = ["actual_world", "dynamic_world", "father_sector"];

    public static void Aggregate(string packageDir, RetePackageManifest manifest)
    {
        var sceneRoot = Path.Combine(packageDir, "scene");
        if (!Directory.Exists(sceneRoot))
        {
            return;
        }

        var doc = new SceneTreeDocument
        {
            Schema = SceneTreeDocument.SchemaValue
        };
        var pathToUri = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        var counter = 0;

        foreach (var rootName in RootNames)
        {
            var rootDir = Path.Combine(sceneRoot, rootName);
            if (!Directory.Exists(rootDir))
            {
                doc.Roots[rootName] = null;
                continue;
            }

            // Shallowest node.json under this root is the tree root.
            var nodeJson = Directory.EnumerateFiles(rootDir, "node.json", SearchOption.AllDirectories)
                .OrderBy(path => path.Count(c => c == Path.DirectorySeparatorChar || c == '/'))
                .ThenBy(path => path, StringComparer.OrdinalIgnoreCase)
                .FirstOrDefault();
            if (nodeJson == null)
            {
                doc.Roots[rootName] = null;
                continue;
            }

            var rootId = IngestNode(packageDir, nodeJson, doc, pathToUri, ref counter);
            doc.Roots[rootName] = rootId;
        }

        // Pool any remaining superobject/matrix leaves that live under types/ or scene
        // and appear in content.json but weren't in the folder forest.
        AbsorbContentSceneLeaves(packageDir, manifest, doc, pathToUri, ref counter);

        if (doc.ById.Count == 0)
        {
            return;
        }

        // Rewrite content.json paths that pointed at scene/.../node.json or matrix.json
        RewriteContentPaths(packageDir, manifest, pathToUri, doc);

        var treePath = Path.Combine(packageDir, SceneTreeDocument.RelativePath.Replace('/', Path.DirectorySeparatorChar));
        Directory.CreateDirectory(Path.GetDirectoryName(treePath)!);
        File.WriteAllText(treePath, JsonSerializer.Serialize(doc, JsonOptions));

        RewritePackageJsonStrings(packageDir, pathToUri);
        DeleteAbsorbedLeaves(packageDir, pathToUri);
        DeleteSceneFolderForest(sceneRoot);
    }

    private static string IngestNode(
        string packageDir,
        string nodeJsonPath,
        SceneTreeDocument doc,
        Dictionary<string, string> pathToUri,
        ref int counter)
    {
        var relNode = Normalize(Path.GetRelativePath(packageDir, nodeJsonPath));
        if (pathToUri.TryGetValue(relNode, out var existingUri) &&
            SemanticPoolPaths.TryParseByIdField(
                existingUri.Contains('#') ? existingUri[(existingUri.IndexOf('#') + 1)..] : null,
                out var existingId,
                out _))
        {
            return existingId;
        }

        using var document = JsonDocument.Parse(File.ReadAllText(nodeJsonPath));
        var json = document.RootElement;
        var id = $"scene_{counter:D5}";
        counter++;

        var node = new SemanticPoolNode
        {
            Id = id,
            Kind = "superObject",
            ProvenanceVirtualAddress = ParseAddressFromPath(nodeJsonPath)
        };

        var nodeDir = Path.GetDirectoryName(nodeJsonPath)!;
        var matrixPath = Path.Combine(nodeDir, "matrix.json");
        if (File.Exists(matrixPath))
        {
            using var mdoc = JsonDocument.Parse(File.ReadAllText(matrixPath));
            node.Matrix = mdoc.RootElement.Clone();
            pathToUri[Normalize(Path.GetRelativePath(packageDir, matrixPath))] =
                $"{SceneTreeDocument.RelativePath}#/byId/{id}/matrix";
        }

        var staticMatrixPath = Path.Combine(nodeDir, "static_matrix.json");
        if (File.Exists(staticMatrixPath))
        {
            using var sdoc = JsonDocument.Parse(File.ReadAllText(staticMatrixPath));
            node.StaticMatrix = sdoc.RootElement.Clone();
            pathToUri[Normalize(Path.GetRelativePath(packageDir, staticMatrixPath))] =
                $"{SceneTreeDocument.RelativePath}#/byId/{id}/staticMatrix";
        }

        if (json.TryGetProperty("children", out var children) &&
            children.ValueKind == JsonValueKind.Array)
        {
            foreach (var child in children.EnumerateArray())
            {
                if (child.ValueKind != JsonValueKind.String)
                {
                    continue;
                }

                var childRel = child.GetString();
                if (string.IsNullOrWhiteSpace(childRel))
                {
                    continue;
                }

                var childFull = Path.Combine(packageDir, childRel.Replace('/', Path.DirectorySeparatorChar));
                if (!File.Exists(childFull))
                {
                    continue;
                }

                var childId = IngestNode(packageDir, childFull, doc, pathToUri, ref counter);
                node.Children.Add(childId);
            }
        }

        // Rewrite children to URI refs in the stored record (authoring edges).
        node.Record = RewriteRecordChildren(json, node.Children);

        doc.ById[id] = node;
        var uri = SemanticPoolPaths.SceneNodeUri(id);
        pathToUri[relNode] = uri;
        return id;
    }

    private static JsonElement RewriteRecordChildren(JsonElement original, List<string> childIds)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream))
        {
            writer.WriteStartObject();
            foreach (var prop in original.EnumerateObject())
            {
                if (prop.NameEquals("children"))
                {
                    writer.WritePropertyName("children");
                    writer.WriteStartArray();
                    foreach (var childId in childIds)
                    {
                        writer.WriteStringValue(SemanticPoolPaths.SceneNodeUri(childId));
                    }

                    writer.WriteEndArray();
                    continue;
                }

                if (prop.NameEquals("path") ||
                    prop.NameEquals("matrixPath") ||
                    prop.NameEquals("staticMatrixPath"))
                {
                    // Drop folder-forest paths; matrix lives on SemanticPoolNode.
                    continue;
                }

                prop.WriteTo(writer);
            }

            // Ensure children array exists even when original omitted it.
            if (!original.TryGetProperty("children", out _))
            {
                writer.WritePropertyName("children");
                writer.WriteStartArray();
                foreach (var childId in childIds)
                {
                    writer.WriteStringValue(SemanticPoolPaths.SceneNodeUri(childId));
                }

                writer.WriteEndArray();
            }

            writer.WriteEndObject();
        }

        using var doc = JsonDocument.Parse(stream.ToArray());
        return doc.RootElement.Clone();
    }

    private static void AbsorbContentSceneLeaves(
        string packageDir,
        RetePackageManifest manifest,
        SceneTreeDocument doc,
        Dictionary<string, string> pathToUri,
        ref int counter)
    {
        foreach (var snaFile in manifest.SnaFiles)
        {
            foreach (var block in snaFile.Blocks)
            {
                if (block.ContentPath == null)
                {
                    continue;
                }

                var contentPath = Path.Combine(packageDir, block.ContentPath.Replace('/', Path.DirectorySeparatorChar));
                if (!File.Exists(contentPath))
                {
                    continue;
                }

                var content = JsonSerializer.Deserialize<SnaBlockContentDocument>(
                    File.ReadAllText(contentPath), JsonOptions);
                if (content == null)
                {
                    continue;
                }

                var leaves = SnaBlockContentLinearizer.Linearize(packageDir, content)
                    .Select((leaf, i) => new SnaBlockContentElement
                    {
                        Order = i,
                        Kind = leaf.Kind,
                        DataPath = leaf.DataPath
                    });

                foreach (var element in leaves)
                {
                    if (!SemanticDomainKinds.Scene.Contains(element.Kind))
                    {
                        continue;
                    }

                    var rel = Normalize(element.DataPath);
                    if (pathToUri.ContainsKey(rel) || rel.Contains('#'))
                    {
                        continue;
                    }

                    // Already scene/tree or animation — skip.
                    if (rel.StartsWith("scene/tree.json", StringComparison.OrdinalIgnoreCase) ||
                        rel.StartsWith("animation/", StringComparison.OrdinalIgnoreCase))
                    {
                        continue;
                    }

                    var full = Path.Combine(packageDir, rel.Replace('/', Path.DirectorySeparatorChar));
                    if (!File.Exists(full))
                    {
                        continue;
                    }

                    var id = $"scene_{counter:D5}";
                    counter++;
                    JsonElement? record = null;
                    if (full.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
                    {
                        using var jdoc = JsonDocument.Parse(File.ReadAllText(full));
                        record = jdoc.RootElement.Clone();
                    }

                    var kind = element.Kind.Equals("matrix", StringComparison.OrdinalIgnoreCase)
                        ? "matrix"
                        : "superObject";

                    doc.ById[id] = new SemanticPoolNode
                    {
                        Id = id,
                        Kind = kind,
                        Record = record,
                        ProvenanceVirtualAddress = element.VirtualAddress != 0 ? element.VirtualAddress : null
                    };
                    pathToUri[rel] = SemanticPoolPaths.SceneNodeUri(id);
                }
            }
        }
    }

    private static void RewriteContentPaths(
        string packageDir,
        RetePackageManifest manifest,
        Dictionary<string, string> pathToUri,
        SceneTreeDocument doc)
    {
        var runCounter = 0;
        foreach (var snaFile in manifest.SnaFiles)
        {
            foreach (var block in snaFile.Blocks)
            {
                if (block.ContentPath == null)
                {
                    continue;
                }

                var contentPath = Path.Combine(packageDir, block.ContentPath.Replace('/', Path.DirectorySeparatorChar));
                if (!File.Exists(contentPath))
                {
                    continue;
                }

                var content = JsonSerializer.Deserialize<SnaBlockContentDocument>(
                    File.ReadAllText(contentPath), JsonOptions);
                if (content == null)
                {
                    continue;
                }

                // Import always writes v2 segments; remap scene leaves into pool URIs / expand runs.
                content.Segments = RewriteSceneSegments(
                    content.Segments,
                    pathToUri,
                    doc,
                    ref runCounter,
                    block.Key);
                content.Schema = SnaBlockContentDocument.SchemaValue;
                File.WriteAllText(contentPath, JsonSerializer.Serialize(content, JsonOptions));
            }
        }
    }

    private static List<SnaBlockContentSegment> RewriteSceneSegments(
        List<SnaBlockContentSegment> input,
        Dictionary<string, string> pathToUri,
        SceneTreeDocument doc,
        ref int runCounter,
        string blockKey)
    {
        var output = new List<SnaBlockContentSegment>();
        var i = 0;
        while (i < input.Count)
        {
            var seg = input[i];
            if (seg.Kind.Equals(SnaBlockContentSegment.ExpandKind, StringComparison.OrdinalIgnoreCase) ||
                seg.Children is { Count: > 0 })
            {
                // Keep non-scene expands (animation, etc.); remap leaf dataPaths inside groups.
                if (seg.Children is { Count: > 0 })
                {
                    seg.Children = RewriteSceneSegments(seg.Children, pathToUri, doc, ref runCounter, blockKey);
                }

                output.Add(seg);
                i++;
                continue;
            }

            var rel = Normalize(seg.DataPath.Split('#')[0]);
            if (pathToUri.TryGetValue(Normalize(seg.DataPath), out var uri) ||
                pathToUri.TryGetValue(rel, out uri))
            {
                var runKeys = new List<string>();
                var firstKind = seg.Kind;
                while (i < input.Count)
                {
                    var cur = input[i];
                    if (cur.Kind.Equals(SnaBlockContentSegment.ExpandKind, StringComparison.OrdinalIgnoreCase) ||
                        cur.Children is { Count: > 0 })
                    {
                        break;
                    }

                    var r = Normalize(cur.DataPath);
                    var r0 = Normalize(cur.DataPath.Split('#')[0]);
                    if (!pathToUri.TryGetValue(r, out var u) && !pathToUri.TryGetValue(r0, out u))
                    {
                        break;
                    }

                    if (!u.StartsWith(SceneTreeDocument.RelativePath, StringComparison.OrdinalIgnoreCase))
                    {
                        break;
                    }

                    if (TryStreamLeafKeyFromUri(u, out var key))
                    {
                        runKeys.Add(key);
                    }

                    i++;
                }

                AppendSceneRun(output, runKeys, firstKind, doc, ref runCounter, blockKey);
                continue;
            }

            output.Add(seg);
            i++;
        }

        return output;
    }

    private static bool TryStreamLeafKeyFromUri(string uri, out string key)
    {
        key = "";
        var hash = uri.IndexOf('#');
        if (hash < 0)
        {
            return false;
        }

        return SemanticPoolPaths.TryParseByIdField(uri[(hash + 1)..], out var id, out var field) &&
               !string.IsNullOrEmpty(id) &&
               (key = string.IsNullOrEmpty(field) ? id : $"{id}/{field}").Length > 0;
    }

    private static bool TryStreamLeafKey(string packageDir, string uri, out string key)
    {
        key = "";
        try
        {
            var pointer = ReferenceUri.Resolve(packageDir, uri).JsonPointer;
            if (!SemanticPoolPaths.TryParseByIdField(pointer, out var id, out var field))
            {
                return false;
            }

            key = SemanticPoolPaths.SceneStreamLeafKey(id, field);
            return true;
        }
        catch
        {
            // URI may be relative-only without a real package root; parse fragment manually.
            var hash = uri.IndexOf('#');
            if (hash < 0)
            {
                return false;
            }

            if (!SemanticPoolPaths.TryParseByIdField(uri[(hash + 1)..], out var id, out var field))
            {
                return false;
            }

            key = SemanticPoolPaths.SceneStreamLeafKey(id, field);
            return true;
        }
    }

    private static void AppendSceneRun(
        List<SnaBlockContentSegment> segments,
        List<string> runKeys,
        string firstKind,
        SceneTreeDocument doc,
        ref int runCounter,
        string blockKey)
    {
        if (runKeys.Count == 0)
        {
            return;
        }

        if (runKeys.Count == 1)
        {
            var key = runKeys[0];
            var kind = key.Contains('/') ? "matrix" : firstKind;
            segments.Add(new SnaBlockContentSegment
            {
                Kind = kind,
                DataPath = SemanticPoolPaths.SceneNodeUri(key)
            });
            return;
        }

        var runId = $"scene_run_{blockKey.Replace(':', '_')}_{runCounter:D3}";
        runCounter++;
        doc.Runs[runId] = runKeys;
        segments.Add(new SnaBlockContentSegment
        {
            Kind = SnaBlockContentSegment.ExpandKind,
            DataPath = SemanticPoolPaths.SceneRunUri(runId)
        });
    }

    private static void RemapSegments(List<SnaBlockContentSegment> segments, Dictionary<string, string> pathToUri)
    {
        foreach (var seg in segments)
        {
            if (seg.Children is { Count: > 0 })
            {
                RemapSegments(seg.Children, pathToUri);
            }

            if (string.IsNullOrWhiteSpace(seg.DataPath) ||
                seg.Kind.Equals(SnaBlockContentSegment.ExpandKind, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            var rel = Normalize(seg.DataPath.Split('#')[0]);
            if (pathToUri.TryGetValue(rel, out var uri))
            {
                seg.DataPath = uri;
            }
        }
    }

    private static void RewritePackageJsonStrings(string packageDir, Dictionary<string, string> pathToUri)
    {
        // Longest paths first so nested scene/.../node.json rewrites before parent prefixes.
        var ordered = pathToUri
            .OrderByDescending(kv => kv.Key.Length)
            .ToList();

        foreach (var file in Directory.EnumerateFiles(packageDir, "*.json", SearchOption.AllDirectories))
        {
            var rel = Path.GetRelativePath(packageDir, file).Replace('\\', '/');
            // Include scene/tree.json so SuperObject pointer fields inside records
            // (parent/brother/matrix) remap to tree byId URIs before types/ leaves are deleted.
            if (rel.StartsWith("geometry/", StringComparison.OrdinalIgnoreCase) ||
                rel.StartsWith("ai/", StringComparison.OrdinalIgnoreCase) ||
                rel.StartsWith("animation/", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            // content.json + types/* still hold scene/ and types/superObject paths that move into
            // scene/tree.json. Must rewrite types/ before absorbed leaves are deleted.
            var text = File.ReadAllText(file);
            if (text.IndexOf("scene/", StringComparison.OrdinalIgnoreCase) < 0 &&
                text.IndexOf("types/superObject", StringComparison.OrdinalIgnoreCase) < 0 &&
                text.IndexOf("types/superobject", StringComparison.OrdinalIgnoreCase) < 0 &&
                text.IndexOf("types/matrix", StringComparison.OrdinalIgnoreCase) < 0)
            {
                continue;
            }

            var changed = false;
            foreach (var (oldPath, newUri) in ordered)
            {
                if (text.IndexOf(oldPath, StringComparison.OrdinalIgnoreCase) >= 0)
                {
                    text = text.Replace(oldPath, newUri, StringComparison.OrdinalIgnoreCase);
                    changed = true;
                }
            }

            // types/matrix/foo#byteOffset=54 → scene/tree.json#/byId/x/staticMatrix#byteOffset=54
            // is invalid (double #). Normalize to semicolon form for JSON Pointer URIs.
            if (text.Contains("#/byId/", StringComparison.OrdinalIgnoreCase) &&
                text.Contains("#byteOffset=", StringComparison.OrdinalIgnoreCase))
            {
                var fixedText = NormalizeDoubleHashByteOffsets(text);
                if (!ReferenceEquals(fixedText, text) && fixedText != text)
                {
                    text = fixedText;
                    changed = true;
                }
            }

            if (changed)
            {
                File.WriteAllText(file, text);
            }
        }
    }

    /// <summary>
    /// After path rewrite, URIs may look like <c>doc#/byId/id#byteOffset=N</c>. Convert the
    /// second hash to <c>;byteOffset=N</c> (same convention as AnimationTreeImporter).
    /// </summary>
    private static string NormalizeDoubleHashByteOffsets(string text)
    {
        // Only fix when #byteOffset appears after a JSON pointer fragment.
        return System.Text.RegularExpressions.Regex.Replace(
            text,
            @"(#/byId/[^""#]+)#byteOffset=",
            "$1;byteOffset=",
            System.Text.RegularExpressions.RegexOptions.IgnoreCase);
    }

    private static void DeleteAbsorbedLeaves(string packageDir, Dictionary<string, string> pathToUri)
    {
        foreach (var oldPath in pathToUri.Keys)
        {
            if (oldPath.StartsWith("scene/tree.json", StringComparison.OrdinalIgnoreCase) ||
                oldPath.Contains('#'))
            {
                continue;
            }

            var full = Path.Combine(packageDir, oldPath.Replace('/', Path.DirectorySeparatorChar));
            if (File.Exists(full))
            {
                try { File.Delete(full); } catch { /* ignore */ }
            }
        }

        foreach (var kind in SemanticDomainKinds.Scene)
        {
            var dir = Path.Combine(packageDir, "types", kind);
            if (Directory.Exists(dir) && !Directory.EnumerateFileSystemEntries(dir).Any())
            {
                try { Directory.Delete(dir); } catch { /* ignore */ }
            }
        }
    }

    private static void DeleteSceneFolderForest(string sceneRoot)
    {
        foreach (var rootName in RootNames)
        {
            var dir = Path.Combine(sceneRoot, rootName);
            if (Directory.Exists(dir))
            {
                try { Directory.Delete(dir, recursive: true); } catch { /* ignore */ }
            }
        }
    }

    private static int? ParseAddressFromPath(string nodeJsonPath)
    {
        var fileName = Path.GetFileName(Path.GetDirectoryName(nodeJsonPath)) ?? "";
        var underscore = fileName.LastIndexOf('_');
        if (underscore < 0 || underscore == fileName.Length - 1)
        {
            return null;
        }

        var hex = fileName[(underscore + 1)..];
        // OpenSpace VM addresses are often > int.MaxValue as unsigned; store as bit pattern int.
        if (uint.TryParse(hex, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out var uaddr))
        {
            return unchecked((int)uaddr);
        }

        return null;
    }

    private static string Normalize(string path) => path.Replace('\\', '/');
}
