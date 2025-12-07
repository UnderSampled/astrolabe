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
            Console.WriteLine($"Loaded {loader.Sna.Blocks.Count} SNA blocks");

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
                Console.WriteLine($"Loaded {textureTable.TextureNames.Count} texture references from PTX");
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
            Console.WriteLine($"Successfully read {persos.Count} Persos");

            var families = familyReader.GetUniqueFamilies(persos);
            Console.WriteLine($"Found {families.Count} unique Families");

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
