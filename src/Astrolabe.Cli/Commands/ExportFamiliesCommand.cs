using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.Animation;
using Astrolabe.Core.FileFormats.Geometry;

namespace Astrolabe.Cli.Commands;

public static class ExportFamiliesCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Level directory path required");
            Console.Error.WriteLine("Usage: astrolabe export-families <level-dir> [output-dir]");
            Console.Error.WriteLine();
            Console.Error.WriteLine("Exports all character Families (meshes + animations) found in a level to GLTF files.");
            Console.Error.WriteLine();
            Console.Error.WriteLine("Examples:");
            Console.Error.WriteLine("  astrolabe export-families ./disc/Gamedata/World/Levels/castle_village");
            Console.Error.WriteLine("  astrolabe export-families ./disc/Gamedata/World/Levels/castle_village ./output/families");
            return 1;
        }

        var levelDir = args[0];
        var levelName = Path.GetFileName(levelDir.TrimEnd('/', '\\'));
        var outputDir = args.Length > 1 ? args[1] : $"output/{levelName}_families";

        try
        {
            Console.WriteLine($"Loading level: {levelName}");
            var loader = new LevelLoader(levelDir, levelName);
            Console.WriteLine($"Loaded {loader.Sna.Blocks.Count} level SNA blocks");

            // Try to load Fix data from parent Levels directory
            var levelsDir = Path.GetDirectoryName(levelDir.TrimEnd('/', '\\'));
            var fixSnaPath = levelsDir != null ? Path.Combine(levelsDir, "Fix.sna") : null;
            var fixRtbPath = levelsDir != null ? Path.Combine(levelsDir, "Fix.rtb") : null;
            var fixLvlRtbPath = Path.Combine(levelDir, "fixlvl.rtb");

            if (fixSnaPath != null && File.Exists(fixSnaPath))
            {
                Console.WriteLine($"Loading Fix data from: {fixSnaPath}");
                var fixSna = new SnaReader(fixSnaPath);
                Console.WriteLine($"  fixSna loaded with {fixSna.Blocks.Count} blocks");

                // Debug: show address range of Fix blocks
                foreach (var block in fixSna.Blocks.Take(5))
                {
                    int endAddr = block.BaseInMemory + (block.Data?.Length ?? 0);
                    Console.WriteLine($"  Block mod={block.Module} id={block.Id}: 0x{block.BaseInMemory:X8} - 0x{endAddr:X8} ({block.Data?.Length ?? 0} bytes)");
                }
                loader.Sna.Merge(fixSna);
                Console.WriteLine($"  Merged {fixSna.Blocks.Count} Fix SNA blocks (total: {loader.Sna.Blocks.Count})");

                // Merge Fix.rtb
                if (fixRtbPath != null && File.Exists(fixRtbPath) && loader.Rtb != null)
                {
                    var fixRtb = new RelocationTableReader(fixRtbPath);
                    loader.Rtb.Merge(fixRtb);
                    Console.WriteLine($"  Merged Fix.rtb relocation table");
                }

                // Merge fixlvl.rtb (cross-references between Fix and level)
                if (File.Exists(fixLvlRtbPath) && loader.Rtb != null)
                {
                    var fixLvlRtb = new RelocationTableReader(fixLvlRtbPath);
                    loader.Rtb.Merge(fixLvlRtb);
                    Console.WriteLine($"  Merged fixlvl.rtb cross-reference table");
                }

                // Rebuild memory map to include merged Fix blocks
                loader.RebuildMemoryMap();
                Console.WriteLine($"  Rebuilt memory map with {loader.Sna.Blocks.Count} blocks");
            }
            else
            {
                Console.WriteLine("Note: Fix.sna not found - shared character meshes may be missing");
            }

            // Load texture table from PTX
            TextureTable? textureTable = null;
            var ptxPath = Path.Combine(levelDir, $"{levelName}.ptx");
            if (!File.Exists(ptxPath))
            {
                ptxPath = Directory.GetFiles(levelDir, $"{levelName}.ptx*").FirstOrDefault() ?? "";
            }
            if (File.Exists(ptxPath))
            {
                textureTable = new TextureTable(loader, ptxPath);
                Console.WriteLine($"Loaded {textureTable.TextureNames.Count} texture references from level PTX");

                // Also load Fix.ptx for character textures
                var fixPtxPath = levelsDir != null ? Path.Combine(levelsDir, "Fix.ptx") : null;
                if (fixPtxPath != null && File.Exists(fixPtxPath))
                {
                    Console.WriteLine($"Loading Fix.ptx from: {fixPtxPath}");
                    textureTable.MergeFromPtx(fixPtxPath);
                }
            }

            // Load GPT and scene graph
            var gptPath = Path.Combine(levelDir, $"{levelName}.gpt");
            if (!File.Exists(gptPath))
            {
                gptPath = Directory.GetFiles(levelDir, $"{levelName}.gpt*").FirstOrDefault() ?? "";
            }

            if (!File.Exists(gptPath))
            {
                Console.Error.WriteLine("Error: Could not find GPT file for scene graph");
                return 1;
            }

            var memory = new MemoryContext(loader.Sna, loader.Rtb);
            var gpt = new GptReader(gptPath);
            var soReader = new SuperObjectReader(memory);
            var sceneGraph = soReader.ReadSceneGraph(gpt);

            Console.WriteLine($"Scene graph has {sceneGraph.AllNodes.Count} nodes");
            var persoNodes = sceneGraph.AllNodes.Where(n => n.Type == SuperObjectType.Perso).ToList();
            Console.WriteLine($"Found {persoNodes.Count} Perso nodes");

            if (persoNodes.Count == 0)
            {
                Console.WriteLine("No Persos found in this level. Try a different level.");
                return 0;
            }

            // Try to find family names from the object types table
            var objectTypeReader = new ObjectTypeReader(memory, loader.Sna);
            var familyNames = objectTypeReader.TryFindFamilyNames();
            if (familyNames.Count > 0)
            {
                Console.WriteLine($"Found {familyNames.Count} family names from object types table");
            }

            // Read Families from Persos
            var familyReader = new FamilyReader(memory);
            var persos = familyReader.FindPersosInSceneGraph(sceneGraph);

            // Also check for spawnable persos (like the main player character)
            if (gpt.SpawnableCount > 0 && gpt.OffSpawnableHead != 0)
            {
                Console.WriteLine($"Checking {gpt.SpawnableCount} spawnable persos...");
                var spawnablePersos = familyReader.FindSpawnablePersos(gpt.OffSpawnableHead, gpt.SpawnableCount);
                persos.AddRange(spawnablePersos);
                Console.WriteLine($"  Found {spawnablePersos.Count} spawnable persos");
            }

            // Try to find Hype1's ObjectList at Fix|0x89E
            // Raymap says: "Hype1 has an Object List @ Fix|0x0000089E"
            var fixBlock = loader.Sna.Blocks.FirstOrDefault(b => b.Module == 5 && b.Id == 0);
            Family? hypeFamily = null;

            if (fixBlock?.Data != null && fixBlock.Data.Length > 0x900)
            {
                Console.WriteLine($"Fix block: base=0x{fixBlock.BaseInMemory:X8}, data length={fixBlock.Data.Length}");

                // Dump raw bytes at 0x89E to understand the structure
                int hypeOffset = 0x89E;
                Console.WriteLine($"Raw bytes at Fix offset 0x{hypeOffset:X}:");
                using (var ms = new MemoryStream(fixBlock.Data, hypeOffset, 32))
                using (var br = new BinaryReader(ms))
                {
                    for (int i = 0; i < 32; i += 4)
                    {
                        int val = br.ReadInt32();
                        Console.WriteLine($"  +0x{i:X2}: 0x{val:X8} ({val})");
                    }
                }

                // ObjectList structure (Montreal):
                // +0x00: Pointer off_objList_next
                // +0x04: Pointer off_objList_prev
                // +0x08: Pointer off_objList_hdr
                // +0x0C: Pointer off_objList_start (entry array)
                // +0x10: Pointer off_objList_2
                // +0x14: uint16 num_entries

                // Try reading as ObjectList directly
                int hypeObjListAddr = fixBlock.BaseInMemory + hypeOffset;
                Console.WriteLine($"Trying to read ObjectList at virtual 0x{hypeObjListAddr:X8}...");

                var hypeObjList = familyReader.ReadObjectListAt(hypeObjListAddr);
                if (hypeObjList != null && hypeObjList.Entries.Count > 0)
                {
                    Console.WriteLine($"  Found Hype1 ObjectList with {hypeObjList.Entries.Count} entries!");
                    hypeFamily = new Family
                    {
                        Address = hypeObjListAddr,
                        Name = "Hype1",
                        FamilyIndex = 9999
                    };
                    hypeFamily.ObjectLists.Add(hypeObjList);
                }
                else
                {
                    // Maybe 0x89E is a pointer TO an ObjectList, not the ObjectList itself
                    using (var ms = new MemoryStream(fixBlock.Data, hypeOffset, 4))
                    using (var br = new BinaryReader(ms))
                    {
                        int ptrValue = br.ReadInt32();
                        Console.WriteLine($"  Value at 0x89E = 0x{ptrValue:X8}, trying as pointer...");

                        if (ptrValue >= 0x01000000 && ptrValue < 0x10000000)
                        {
                            hypeObjList = familyReader.ReadObjectListAt(ptrValue);
                            if (hypeObjList != null && hypeObjList.Entries.Count > 0)
                            {
                                Console.WriteLine($"  Found Hype1 ObjectList (via pointer) with {hypeObjList.Entries.Count} entries!");
                                hypeFamily = new Family
                                {
                                    Address = ptrValue,
                                    Name = "Hype1",
                                    FamilyIndex = 9999
                                };
                                hypeFamily.ObjectLists.Add(hypeObjList);
                            }
                        }
                    }

                    // Also scan for ObjectLists near 0x89E with correct structure
                    if (hypeFamily == null)
                    {
                        Console.WriteLine("  Scanning near 0x89E for valid ObjectList structures...");
                        for (int scanOffset = hypeOffset - 0x20; scanOffset < hypeOffset + 0x40; scanOffset += 4)
                        {
                            if (scanOffset < 0 || scanOffset + 0x18 > fixBlock.Data.Length) continue;

                            using var scanMs = new MemoryStream(fixBlock.Data, scanOffset, 0x18);
                            using var scanBr = new BinaryReader(scanMs);

                            int offNext = scanBr.ReadInt32();
                            int offPrev = scanBr.ReadInt32();
                            int offHdr = scanBr.ReadInt32();
                            int offStart = scanBr.ReadInt32();
                            int off2 = scanBr.ReadInt32();
                            ushort numEntries = scanBr.ReadUInt16();

                            // Check for valid ObjectList: reasonable numEntries, valid start pointer
                            if (numEntries >= 50 && numEntries <= 500 &&
                                offStart >= 0x01000000 && offStart < 0x10000000)
                            {
                                int virtAddr = fixBlock.BaseInMemory + scanOffset;
                                Console.WriteLine($"    Candidate at 0x{scanOffset:X} (virt 0x{virtAddr:X8}): numEntries={numEntries}, offStart=0x{offStart:X8}");

                                var testObjList = familyReader.ReadObjectListAt(virtAddr);
                                if (testObjList != null && testObjList.Entries.Count > 0)
                                {
                                    Console.WriteLine($"    -> Valid! {testObjList.Entries.Count} entries with geometry");
                                    if (hypeFamily == null)
                                    {
                                        hypeFamily = new Family
                                        {
                                            Address = virtAddr,
                                            Name = "Hype1",
                                            FamilyIndex = 9999
                                        };
                                        hypeFamily.ObjectLists.Add(testObjList);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            else
            {
                Console.WriteLine("Warning: Fix block not found or has no data");
            }
            Console.WriteLine($"Successfully read {persos.Count} Persos");

            var families = familyReader.GetUniqueFamilies(persos);
            Console.WriteLine($"Found {families.Count} unique Families");

            // Add Hype1 family if we found it
            if (hypeFamily != null)
            {
                families.Add(hypeFamily);
                Console.WriteLine($"Added Hype1 family with {hypeFamily.ObjectLists.FirstOrDefault()?.Entries.Count ?? 0} mesh entries");
            }

            // Apply names from object types table using ObjectTypeIndex
            foreach (var family in families)
            {
                if (familyNames.TryGetValue(family.ObjectTypeIndex, out var name))
                {
                    family.Name = name;
                }
            }

            if (families.Count == 0)
            {
                Console.WriteLine("No Families found. The Family data may not be loading correctly.");
                return 0;
            }

            // Print family info
            foreach (var family in families)
            {
                Console.WriteLine($"  - {family.Name ?? $"Family_{family.FamilyIndex}"}: " +
                    $"{family.States.Count} states, {family.ObjectLists.Count} object lists");

                foreach (var state in family.States.Take(5))
                {
                    string animInfo = state.Animation != null
                        ? $"{state.Animation.NumFrames} frames, {state.Animation.NumChannels} channels"
                        : "no animation";
                    Console.WriteLine($"      State {state.Index}: {state.Name ?? "unnamed"} ({animInfo})");
                }

                if (family.States.Count > 5)
                {
                    Console.WriteLine($"      ... and {family.States.Count - 5} more states");
                }
            }

            // Build texture lookup - search common texture directories
            var textureLookup = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            foreach (var textureBaseDir in new[] { "output/Gamedata/Textures", "output/textures", "textures" })
            {
                if (Directory.Exists(textureBaseDir))
                {
                    foreach (var file in Directory.EnumerateFiles(textureBaseDir, "*.*", SearchOption.AllDirectories)
                        .Where(f => f.EndsWith(".tga", StringComparison.OrdinalIgnoreCase) ||
                                   f.EndsWith(".png", StringComparison.OrdinalIgnoreCase)))
                    {
                        var fileName = Path.GetFileName(file);
                        if (!textureLookup.ContainsKey(fileName))
                            textureLookup[fileName] = file;
                    }
                }
            }
            Console.WriteLine($"Indexed {textureLookup.Count} textures for lookup");

            // Texture lookup function
            Func<string?, string?> lookupTexture = (texName) =>
            {
                if (string.IsNullOrEmpty(texName))
                    return null;

                string fileName = Path.GetFileName(texName);
                if (!fileName.EndsWith(".tga", StringComparison.OrdinalIgnoreCase) &&
                    !fileName.EndsWith(".png", StringComparison.OrdinalIgnoreCase))
                {
                    fileName += ".png";
                }

                if (textureLookup.TryGetValue(fileName, out var foundPath))
                    return foundPath;

                var pngName = Path.ChangeExtension(fileName, ".png");
                if (textureLookup.TryGetValue(pngName, out foundPath))
                    return foundPath;

                return null;
            };

            // Export families
            Directory.CreateDirectory(outputDir);
            var exporter = new FamilyExporter(loader, textureTable);

            int exported = 0;
            foreach (var family in families)
            {
                string safeName = family.Name ?? $"Family_{family.FamilyIndex}";
                safeName = string.Join("_", safeName.Split(Path.GetInvalidFileNameChars()));
                string outputPath = Path.Combine(outputDir, $"{safeName}.glb");

                try
                {
                    exporter.ExportFamily(family, outputPath, lookupTexture);
                    exported++;
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"Failed to export {safeName}: {ex.Message}");
                }
            }

            Console.WriteLine($"Exported {exported} Families to: {outputDir}/");
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error: {ex.Message}");
            Console.Error.WriteLine(ex.StackTrace);
            return 1;
        }
    }
}
