using System.Buffers.Binary;
using System.Text.Json;
using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Audio;
using Astrolabe.Core.FileFormats.Semantic;
using Astrolabe.Core.Hub;

namespace Astrolabe.Core.Rete;

/// <summary>
/// Promotes GPT/PTX/SDA/SND loose sidecars into URI-backed semantic records
/// (<c>sidecars/level.json</c>) with wire-lossless Base64 payloads for export parity.
/// When a disc <c>Textures.cnt</c> (or pre-decoded PNG tree) is discoverable, resolves
/// PTX-referenced GF names into mirrored <c>texture:/Gamedata/Textures/…</c> URIs and
/// writes PNGs under the conversion output root. Sound is best-effort
/// <c>sound:/Gamedata/World/Sound/…</c> inventory (existing WAV / light BNM-APM decode).
/// </summary>
/// <remarks>
/// <b>Residual:</b> RTP/RTT (and RTS) generation still may use heuristic <c>uint32</c>
/// scans of sidecar wire bytes until promoted sidecar types expose codec
/// <c>PointerFields</c> metadata. Full PNG corpus requires disc-backed CNT (or a prior
/// <c>extract</c> into the output tree); without CNT only already-decoded PNGs and
/// PTX name placeholders are inventoried. SDA/SND event→bank mapping is incomplete
/// without full sound resource table promotion — bank-level <c>sound:/</c> URIs are
/// emitted when Sound/ is discoverable.
/// </remarks>
internal static class SidecarAggregator
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    private const string TextureScheme = "texture:/";
    private const string SoundScheme = "sound:/";
    private const string TexturesMirror = "Gamedata/Textures";
    private const string VignetteMirror = "Gamedata/Vignette";
    private const string SoundMirror = "Gamedata/World/Sound";

    public static void Aggregate(string packageDir, RetePackageManifest manifest)
    {
        var levelName = manifest.LevelName;
        if (string.IsNullOrWhiteSpace(levelName))
        {
            return;
        }

        var filesDir = Path.Combine(packageDir, "files");
        if (!Directory.Exists(filesDir))
        {
            return;
        }

        var doc = new SidecarDocument();
        var removedLoose = new List<LooseFileManifest>();
        var outputRoot = ResolveOutputRoot(packageDir);

        foreach (var loose in manifest.LooseFiles.ToList())
        {
            var name = loose.FileName;
            var ext = Path.GetExtension(name).ToLowerInvariant();
            if (ext is not (".gpt" or ".ptx" or ".sda" or ".snd"))
            {
                continue;
            }

            var full = Path.Combine(packageDir, loose.Path.Replace('/', Path.DirectorySeparatorChar));
            if (!File.Exists(full))
            {
                full = Path.Combine(filesDir, name);
            }

            if (!File.Exists(full))
            {
                continue;
            }

            var bytes = File.ReadAllBytes(full);
            var entry = new SidecarPointerFile
            {
                Kind = ext.TrimStart('.'),
                SourceFileName = name,
                WireBase64 = Convert.ToBase64String(bytes),
                Pointers = ExtractPointerSlots(bytes)
            };

            switch (ext)
            {
                case ".gpt":
                    doc.Gpt = entry;
                    break;
                case ".ptx":
                    entry.TextureUris = ResolveTextureUris(
                        packageDir, outputRoot, levelName, bytes, full);
                    doc.Ptx = entry;
                    break;
                case ".sda":
                    entry.SoundUris = ResolveSoundUris(packageDir, outputRoot, bytes);
                    doc.Sda = entry;
                    break;
                case ".snd":
                    entry.SoundUris = ResolveSoundUris(packageDir, outputRoot, bytes);
                    doc.Snd = entry;
                    break;
            }

            removedLoose.Add(loose);
        }

        if (doc.Gpt == null && doc.Ptx == null && doc.Sda == null && doc.Snd == null)
        {
            return;
        }

        var outPath = Path.Combine(packageDir, SidecarDocument.RelativePath.Replace('/', Path.DirectorySeparatorChar));
        Directory.CreateDirectory(Path.GetDirectoryName(outPath)!);
        File.WriteAllText(outPath, JsonSerializer.Serialize(doc, JsonOptions));

        // Remove opaque pass-through as long-term source of truth (keep bytes in sidecar doc).
        foreach (var loose in removedLoose)
        {
            manifest.LooseFiles.Remove(loose);
            var full = Path.Combine(packageDir, loose.Path.Replace('/', Path.DirectorySeparatorChar));
            if (File.Exists(full))
            {
                try { File.Delete(full); } catch { /* ignore */ }
            }
        }

        // Persist updated manifest without those loose files.
        var manifestPath = Path.Combine(packageDir, OpenSpacePackageCodec.ManifestFileName);
        File.WriteAllText(manifestPath, JsonSerializer.Serialize(manifest, JsonOptions));
    }

    private static List<string?> ExtractPointerSlots(byte[] bytes)
    {
        var pointers = new List<string?>();
        for (var offset = 0; offset + 4 <= bytes.Length; offset += 4)
        {
            var value = BinaryPrimitives.ReadInt32LittleEndian(bytes.AsSpan(offset, 4));
            // Preserve slot inventory; URIs filled when address map is available at export.
            if (value == 0)
            {
                pointers.Add(null);
            }
            else
            {
                pointers.Add($"0x{value:X8}");
            }
        }

        return pointers;
    }

    private static List<string> ResolveTextureUris(
        string packageDir,
        string outputRoot,
        string levelName,
        byte[] ptxBytes,
        string ptxPath)
    {
        var uris = new List<string>();
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        void AddUri(string uri)
        {
            if (seen.Add(uri))
            {
                uris.Add(uri);
            }
        }

        try
        {
            var names = CollectTextureNames(packageDir, ptxBytes, ptxPath);
            var cntPaths = DiscoverCntArchives(packageDir).ToList();
            var decodedAny = false;

            if (cntPaths.Count > 0)
            {
                decodedAny = TryDecodeReferencedTextures(
                    cntPaths, names, levelName, outputRoot, AddUri);
            }

            // Prefer already-decoded PNGs under the output root (extract or prior import).
            CollectExistingTextureUris(outputRoot, levelName, names, AddUri);

            // Parent-chain search for a sibling Gamedata/Textures tree (flat layouts).
            if (uris.Count == 0)
            {
                CollectExistingTextureUrisFromAncestors(packageDir, levelName, names, AddUri);
            }

            // Name-only placeholders when we know GF stems but have no PNG yet.
            if (!decodedAny && names.Count > 0 && uris.Count == 0)
            {
                foreach (var name in names)
                {
                    var stem = NormalizeTextureStem(name);
                    if (string.IsNullOrEmpty(stem))
                    {
                        continue;
                    }

                    AddUri($"{TextureScheme}{TexturesMirror}/{stem}.png");
                }
            }

            WriteTextureProvenance(outputRoot, levelName, names.Count, uris.Count, cntPaths);
        }
        catch
        {
            // optional inventory — never fail import aggregate
        }

        return uris;
    }

    private static List<string> CollectTextureNames(
        string packageDir,
        byte[] ptxBytes,
        string ptxPath)
    {
        var names = new List<string>();
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        void AddName(string? name)
        {
            if (string.IsNullOrWhiteSpace(name))
            {
                return;
            }

            var trimmed = name.Trim();
            if (seen.Add(trimmed))
            {
                names.Add(trimmed);
            }
        }

        // Hub path: resolve TextureInfo at PTX pointers after SNA import.
        try
        {
            if (File.Exists(Path.Combine(packageDir, OpenSpacePackageCodec.ManifestFileName)))
            {
                var catalog = HubCatalog.Load(packageDir);
                var tempPtx = ptxPath;
                var ownsTemp = false;
                if (!File.Exists(tempPtx))
                {
                    tempPtx = Path.Combine(Path.GetTempPath(), "astrolabe-ptx-" + Guid.NewGuid().ToString("N") + ".ptx");
                    File.WriteAllBytes(tempPtx, ptxBytes);
                    ownsTemp = true;
                }

                try
                {
                    var table = new TextureTable(catalog, tempPtx);
                    foreach (var entry in table.TextureEntries.Values)
                    {
                        AddName(entry.Name);
                    }
                }
                finally
                {
                    if (ownsTemp)
                    {
                        try { File.Delete(tempPtx); } catch { /* ignore */ }
                    }
                }
            }
        }
        catch
        {
            // Hub may be incomplete during isolated unit tests.
        }

        return names;
    }

    private static bool TryDecodeReferencedTextures(
        IReadOnlyList<string> cntPaths,
        IReadOnlyList<string> names,
        string levelName,
        string outputRoot,
        Action<string> addUri)
    {
        var decoded = false;
        var nameStems = new HashSet<string>(
            names.Select(NormalizeTextureStem).Where(s => s.Length > 0),
            StringComparer.OrdinalIgnoreCase);

        foreach (var cntPath in cntPaths)
        {
            if (!File.Exists(cntPath))
            {
                continue;
            }

            CntReader cnt;
            try
            {
                cnt = new CntReader(cntPath);
            }
            catch
            {
                continue;
            }

            var isVignette = Path.GetFileName(cntPath)
                .Equals("Vignette.cnt", StringComparison.OrdinalIgnoreCase);
            var mirrorRoot = isVignette ? VignetteMirror : TexturesMirror;
            var archiveStem = Path.GetFileNameWithoutExtension(cntPath);

            // Build stem → entry index (filename and full path stems).
            var byStem = new Dictionary<string, CntFileEntry>(StringComparer.OrdinalIgnoreCase);
            foreach (var entry in cnt.Files)
            {
                var fileStem = Path.GetFileNameWithoutExtension(entry.Filename);
                if (!string.IsNullOrEmpty(fileStem))
                {
                    byStem.TryAdd(fileStem, entry);
                }

                var fullStem = Path.ChangeExtension(entry.FullPath.Replace('\\', '/'), null);
                if (!string.IsNullOrEmpty(fullStem))
                {
                    byStem.TryAdd(fullStem!, entry);
                    byStem.TryAdd(Path.GetFileName(fullStem)!, entry);
                }
            }

            IEnumerable<CntFileEntry> targets;
            if (nameStems.Count > 0)
            {
                var matched = new List<CntFileEntry>();
                var used = new HashSet<CntFileEntry>();
                foreach (var stem in nameStems)
                {
                    if (byStem.TryGetValue(stem, out var entry) && used.Add(entry))
                    {
                        matched.Add(entry);
                        continue;
                    }

                    // Partial suffix match (TextureInfo names sometimes omit path).
                    foreach (var (key, entry2) in byStem)
                    {
                        if (key.Equals(stem, StringComparison.OrdinalIgnoreCase) ||
                            key.EndsWith(stem, StringComparison.OrdinalIgnoreCase) ||
                            stem.EndsWith(key, StringComparison.OrdinalIgnoreCase))
                        {
                            if (used.Add(entry2))
                            {
                                matched.Add(entry2);
                            }

                            break;
                        }
                    }
                }

                // Level-folder heuristic when few PTX names resolved.
                if (matched.Count == 0 && !string.IsNullOrWhiteSpace(levelName))
                {
                    matched.AddRange(cnt.Files.Where(f =>
                        f.FullPath.StartsWith(levelName + "\\", StringComparison.OrdinalIgnoreCase) ||
                        f.FullPath.StartsWith(levelName + "/", StringComparison.OrdinalIgnoreCase) ||
                        f.FullPath.StartsWith(levelName, StringComparison.OrdinalIgnoreCase)));
                }

                targets = matched;
            }
            else if (!string.IsNullOrWhiteSpace(levelName))
            {
                // No PTX names: still pull level-scoped CNT directory if present.
                targets = cnt.Files.Where(f =>
                    f.FullPath.StartsWith(levelName + "\\", StringComparison.OrdinalIgnoreCase) ||
                    f.FullPath.StartsWith(levelName + "/", StringComparison.OrdinalIgnoreCase));
            }
            else
            {
                targets = [];
            }

            foreach (var entry in targets)
            {
                try
                {
                    var relInside = Path.ChangeExtension(entry.FullPath.Replace('\\', '/'), ".png")!;
                    var pngRel = $"{mirrorRoot}/{relInside}".Replace("//", "/");
                    var pngAbs = Path.Combine(
                        outputRoot,
                        pngRel.Replace('/', Path.DirectorySeparatorChar));

                    if (!File.Exists(pngAbs))
                    {
                        var data = cnt.ExtractFile(entry);
                        var gf = new GfReader(data)
                        {
                            IsVignette = isVignette || (/* width check after parse */ false)
                        };
                        // GfReader sets dimensions in ctor; re-check vignette heuristic.
                        if (!isVignette && gf.Width == 640 && gf.Height == 480)
                        {
                            gf.IsVignette = true;
                        }

                        var dir = Path.GetDirectoryName(pngAbs);
                        if (!string.IsNullOrEmpty(dir))
                        {
                            Directory.CreateDirectory(dir);
                        }

                        gf.SaveAsPng(pngAbs);
                        decoded = true;
                    }

                    addUri(TextureScheme + pngRel);
                }
                catch
                {
                    // skip individual GF failures
                }
            }

            _ = archiveStem; // reserved for provenance
        }

        return decoded;
    }

    private static void CollectExistingTextureUris(
        string outputRoot,
        string levelName,
        IReadOnlyList<string> names,
        Action<string> addUri)
    {
        foreach (var mirror in new[] { TexturesMirror, VignetteMirror })
        {
            var root = Path.Combine(outputRoot, mirror.Replace('/', Path.DirectorySeparatorChar));
            if (!Directory.Exists(root))
            {
                continue;
            }

            var nameStems = new HashSet<string>(
                names.Select(NormalizeTextureStem).Where(s => s.Length > 0),
                StringComparer.OrdinalIgnoreCase);

            IEnumerable<string> pngs = Directory.EnumerateFiles(root, "*.png", SearchOption.AllDirectories);
            if (nameStems.Count > 0)
            {
                pngs = pngs.Where(p =>
                {
                    var stem = Path.GetFileNameWithoutExtension(p);
                    return nameStems.Contains(stem) ||
                           nameStems.Any(n => stem.Equals(n, StringComparison.OrdinalIgnoreCase) ||
                                              stem.EndsWith(n, StringComparison.OrdinalIgnoreCase) ||
                                              n.EndsWith(stem, StringComparison.OrdinalIgnoreCase));
                });
            }
            else if (!string.IsNullOrWhiteSpace(levelName))
            {
                var levelDir = Path.Combine(root, levelName);
                if (Directory.Exists(levelDir))
                {
                    pngs = Directory.EnumerateFiles(levelDir, "*.png", SearchOption.AllDirectories);
                }
                else
                {
                    // Cap unscoped listing so we do not dump entire corpus into the sidecar.
                    pngs = pngs.Take(0);
                }
            }
            else
            {
                pngs = pngs.Take(0);
            }

            foreach (var png in pngs.Take(2048))
            {
                var rel = Path.GetRelativePath(outputRoot, png).Replace('\\', '/');
                if (!rel.StartsWith("Gamedata/", StringComparison.OrdinalIgnoreCase))
                {
                    rel = mirror + "/" + Path.GetFileName(png);
                }

                addUri(TextureScheme + rel);
            }
        }
    }

    private static void CollectExistingTextureUrisFromAncestors(
        string packageDir,
        string levelName,
        IReadOnlyList<string> names,
        Action<string> addUri)
    {
        var dir = new DirectoryInfo(packageDir);
        for (var i = 0; i < 8 && dir != null; i++, dir = dir.Parent)
        {
            var gamedata = Path.Combine(dir.FullName, "Gamedata", "Textures");
            if (!Directory.Exists(gamedata))
            {
                continue;
            }

            CollectExistingTextureUris(dir.FullName, levelName, names, addUri);
            break;
        }
    }

    private static void WriteTextureProvenance(
        string outputRoot,
        string levelName,
        int nameCount,
        int uriCount,
        IReadOnlyList<string> cntPaths)
    {
        try
        {
            var mirrored = Path.Combine(outputRoot, "Gamedata", "Textures");
            Directory.CreateDirectory(mirrored);
            File.WriteAllText(
                Path.Combine(mirrored, "_provenance.json"),
                JsonSerializer.Serialize(new
                {
                    schema = "astrolabe.texture-provenance.v1",
                    level = levelName,
                    textureNamesResolved = nameCount,
                    textureUrisEmitted = uriCount,
                    cntSources = cntPaths,
                    note =
                        "Referenced textures decode to PNG under Gamedata/Textures/ when CNT is discoverable; " +
                        "wire PTX lives in sidecars/level.json. Full corpus still needs disc Textures.cnt / Vignette.cnt " +
                        "(or a prior extract). RTP/RTT remain heuristic until sidecar PointerFields metadata is complete."
                }, JsonOptions));
        }
        catch
        {
            // optional
        }
    }

    private static List<string> ResolveSoundUris(
        string packageDir,
        string outputRoot,
        byte[] soundBytes)
    {
        var uris = new List<string>();
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        void AddUri(string uri)
        {
            if (seen.Add(uri))
            {
                uris.Add(uri);
            }
        }

        try
        {
            // Existing WAV tree (from extract or prior decode).
            var soundRoot = Path.Combine(outputRoot, SoundMirror.Replace('/', Path.DirectorySeparatorChar));
            if (Directory.Exists(soundRoot))
            {
                foreach (var wav in Directory.EnumerateFiles(soundRoot, "*.wav", SearchOption.AllDirectories)
                             .Take(512))
                {
                    var rel = Path.GetRelativePath(outputRoot, wav).Replace('\\', '/');
                    // sound:/ paths are under Gamedata/World/Sound/ — keep full mirrored path after scheme
                    // Spec examples use sound:/Bnk_foo/... but also sound:/Gamedata/World/Sound/...
                    // Prefer full mirrored path for resolver consistency with texture:/.
                    AddUri(SoundScheme + rel);
                }
            }

            // Discover disc Sound/ for bank placeholders + light APM decode.
            foreach (var discSoundDir in DiscoverSoundDirectories(packageDir))
            {
                // Bank placeholders from BNM names (no full BNM extract by default).
                foreach (var bnm in Directory.EnumerateFiles(discSoundDir, "Bnk_*.bnm", SearchOption.TopDirectoryOnly)
                             .Take(256))
                {
                    var bank = Path.GetFileNameWithoutExtension(bnm);
                    var placeholderRel = $"{SoundMirror}/{bank}/";
                    // Only emit a bank marker URI if no WAVs were found under it yet.
                    var bankWavDir = Path.Combine(outputRoot, SoundMirror.Replace('/', Path.DirectorySeparatorChar), bank);
                    if (!Directory.Exists(bankWavDir) ||
                        !Directory.EnumerateFiles(bankWavDir, "*.wav", SearchOption.AllDirectories).Any())
                    {
                        AddUri($"{SoundScheme}{SoundMirror}/{bank}/");
                    }
                }

                // Best-effort: convert loose APM streams (small set) to WAV.
                foreach (var apm in Directory.EnumerateFiles(discSoundDir, "*.apm", SearchOption.TopDirectoryOnly)
                             .Take(64))
                {
                    try
                    {
                        var wavRel = $"{SoundMirror}/{Path.GetFileNameWithoutExtension(apm)}.wav";
                        var wavAbs = Path.Combine(outputRoot, wavRel.Replace('/', Path.DirectorySeparatorChar));
                        if (!File.Exists(wavAbs))
                        {
                            Directory.CreateDirectory(Path.GetDirectoryName(wavAbs)!);
                            WavWriter.ConvertApmToWav(apm, wavAbs);
                        }

                        AddUri(SoundScheme + wavRel);
                    }
                    catch
                    {
                        // skip failed APM
                    }
                }
            }

            // If nothing discovered, still record non-zero wire slots as opaque sound refs.
            if (uris.Count == 0 && soundBytes.Length >= 4)
            {
                var nonZero = 0;
                for (var i = 0; i + 4 <= soundBytes.Length; i += 4)
                {
                    if (BinaryPrimitives.ReadInt32LittleEndian(soundBytes.AsSpan(i, 4)) != 0)
                    {
                        nonZero++;
                    }
                }

                if (nonZero > 0)
                {
                    AddUri($"{SoundScheme}{SoundMirror}/_unresolved/{nonZero}_slots");
                }
            }
        }
        catch
        {
            // optional
        }

        return uris;
    }

    /// <summary>
    /// Output root holds the mirrored Gamedata tree. Flat layout: parent of package dir.
    /// Mirrored layout: ancestor above Gamedata/World/Levels.
    /// </summary>
    internal static string ResolveOutputRoot(string packageDir)
    {
        var full = Path.GetFullPath(packageDir);
        var normalized = full.Replace('\\', '/');
        const string marker = "/Gamedata/World/Levels";
        var idx = normalized.IndexOf(marker, StringComparison.OrdinalIgnoreCase);
        if (idx > 0)
        {
            return Path.GetFullPath(normalized[..idx].Replace('/', Path.DirectorySeparatorChar));
        }

        var parent = Directory.GetParent(full)?.FullName;
        return parent ?? full;
    }

    private static IEnumerable<string> DiscoverCntArchives(string packageDir)
    {
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        foreach (var gamedata in DiscoverGamedataRoots(packageDir))
        {
            foreach (var name in new[] { "Textures.cnt", "Vignette.cnt" })
            {
                var path = Path.Combine(gamedata, name);
                if (File.Exists(path) && seen.Add(path))
                {
                    yield return path;
                }
            }
        }
    }

    private static IEnumerable<string> DiscoverSoundDirectories(string packageDir)
    {
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var gamedata in DiscoverGamedataRoots(packageDir))
        {
            var sound = Path.Combine(gamedata, "World", "Sound");
            if (Directory.Exists(sound) && seen.Add(sound))
            {
                yield return sound;
            }
        }
    }

    private static IEnumerable<string> DiscoverGamedataRoots(string packageDir)
    {
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var candidates = new List<string>();

        void AddCandidate(string? gamedata)
        {
            if (string.IsNullOrWhiteSpace(gamedata))
            {
                return;
            }

            var full = Path.GetFullPath(gamedata);
            if (Directory.Exists(full) && seen.Add(full))
            {
                candidates.Add(full);
            }
        }

        var envSource = Environment.GetEnvironmentVariable("ASTROLABE_SOURCE_DIR");
        if (!string.IsNullOrWhiteSpace(envSource))
        {
            AddCandidate(Path.Combine(envSource, "Gamedata"));
            // env may point at Levels or a level dir
            var parent = Directory.GetParent(envSource)?.FullName;
            if (parent != null)
            {
                AddCandidate(Path.Combine(parent, "Gamedata"));
                var gp = Directory.GetParent(parent)?.FullName;
                if (gp != null)
                {
                    AddCandidate(Path.Combine(gp, "Gamedata"));
                }
            }

            // env = disc root
            AddCandidate(Path.Combine(envSource, "disc", "Gamedata"));
        }

        var current = Path.GetFullPath(packageDir);
        for (var depth = 0; depth < 32 && current != null; depth++)
        {
            AddCandidate(Path.Combine(current, "Gamedata"));
            AddCandidate(Path.Combine(current, "disc", "Gamedata"));
            current = Directory.GetParent(current)?.FullName!;
        }

        // Source level directory discovery (same walk as RT* resolve).
        try
        {
            var manifestPath = Path.Combine(packageDir, OpenSpacePackageCodec.ManifestFileName);
            if (File.Exists(manifestPath))
            {
                var manifest = JsonSerializer.Deserialize<RetePackageManifest>(
                    File.ReadAllText(manifestPath), JsonOptions);
                if (manifest != null && !string.IsNullOrWhiteSpace(manifest.SourceDirectoryName))
                {
                    current = Path.GetFullPath(packageDir);
                    for (var depth = 0; depth < 32 && current != null; depth++)
                    {
                        var levels = Path.Combine(current, "disc", "Gamedata", "World", "Levels");
                        if (Directory.Exists(levels))
                        {
                            AddCandidate(Path.Combine(current, "disc", "Gamedata"));
                        }

                        var gamedataWorld = Path.Combine(current, "Gamedata", "World", "Levels");
                        if (Directory.Exists(gamedataWorld))
                        {
                            AddCandidate(Path.Combine(current, "Gamedata"));
                        }

                        current = Directory.GetParent(current)?.FullName!;
                    }
                }
            }
        }
        catch
        {
            // ignore
        }

        return candidates;
    }

    private static string NormalizeTextureStem(string name)
    {
        var n = name.Replace('\\', '/').Trim();
        if (n.EndsWith(".gf", StringComparison.OrdinalIgnoreCase) ||
            n.EndsWith(".png", StringComparison.OrdinalIgnoreCase))
        {
            n = n[..n.LastIndexOf('.')];
        }

        // CNT paths may appear; keep leaf stem for matching.
        var slash = n.LastIndexOf('/');
        if (slash >= 0)
        {
            n = n[(slash + 1)..];
        }

        return n;
    }

    /// <summary>Regenerate sidecar wire bytes from the semantic document for export.</summary>
    public static bool TryWriteLooseFile(
        string packageDir,
        string fileName,
        out byte[] bytes)
    {
        bytes = [];
        var path = Path.Combine(packageDir, SidecarDocument.RelativePath.Replace('/', Path.DirectorySeparatorChar));
        if (!File.Exists(path))
        {
            return false;
        }

        var doc = JsonSerializer.Deserialize<SidecarDocument>(File.ReadAllText(path), JsonOptions);
        if (doc == null)
        {
            return false;
        }

        var ext = Path.GetExtension(fileName).ToLowerInvariant();
        SidecarPointerFile? entry = ext switch
        {
            ".gpt" => doc.Gpt,
            ".ptx" => doc.Ptx,
            ".sda" => doc.Sda,
            ".snd" => doc.Snd,
            _ => null
        };

        if (entry?.WireBase64 == null)
        {
            return false;
        }

        if (!entry.SourceFileName.Equals(fileName, StringComparison.OrdinalIgnoreCase) &&
            !fileName.EndsWith(ext, StringComparison.OrdinalIgnoreCase))
        {
            // Accept either exact name or any matching extension for the level.
        }

        bytes = Convert.FromBase64String(entry.WireBase64);
        return true;
    }

    /// <summary>Load sidecars/level.json if present.</summary>
    public static bool TryLoadDocument(string packageDir, out SidecarDocument document)
    {
        document = null!;
        var path = Path.Combine(packageDir, SidecarDocument.RelativePath.Replace('/', Path.DirectorySeparatorChar));
        if (!File.Exists(path))
        {
            return false;
        }

        var doc = JsonSerializer.Deserialize<SidecarDocument>(File.ReadAllText(path), JsonOptions);
        if (doc == null)
        {
            return false;
        }

        document = doc;
        return true;
    }
}
