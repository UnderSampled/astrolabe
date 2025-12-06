using Astrolabe.Core.FileFormats;
using Astrolabe.Core.FileFormats.AI;

namespace Astrolabe.Cli.Commands;

/// <summary>
/// Command to find and display AI scripts from level data.
/// </summary>
public static class ScriptsCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Level directory path required");
            Console.Error.WriteLine("Usage: astrolabe scripts <level-dir> [--limit N] [--raw]");
            return 1;
        }

        string levelDir = args[0];
        int limit = 10;
        bool raw = false;

        // Parse options
        for (int i = 1; i < args.Length; i++)
        {
            if (args[i] == "--limit" && i + 1 < args.Length)
            {
                limit = int.Parse(args[++i]);
            }
            else if (args[i] == "--raw")
            {
                raw = true;
            }
        }

        if (!Directory.Exists(levelDir))
        {
            Console.Error.WriteLine($"Directory not found: {levelDir}");
            return 1;
        }

        // Find level name from directory
        string levelName = Path.GetFileName(levelDir.TrimEnd(Path.DirectorySeparatorChar));

        string snaPath = Path.Combine(levelDir, $"{levelName}.sna");
        string rtbPath = Path.Combine(levelDir, $"{levelName}.rtb");
        string gptPath = Path.Combine(levelDir, $"{levelName}.gpt");
        string? outputDir = null;

        // Parse output directory option
        for (int i = 1; i < args.Length; i++)
        {
            if ((args[i] == "--output" || args[i] == "-o") && i + 1 < args.Length)
            {
                outputDir = args[++i];
            }
        }

        if (!File.Exists(snaPath))
        {
            Console.Error.WriteLine($"SNA file not found: {snaPath}");
            return 1;
        }

        Console.WriteLine($"Loading level: {levelName}");

        var sna = new SnaReader(snaPath);
        RelocationTableReader? rtb = File.Exists(rtbPath) ? new RelocationTableReader(rtbPath) : null;
        var memory = new MemoryContext(sna, rtb);

        Console.WriteLine($"Loaded {sna.Blocks.Count} blocks");

        // Find Persos through the scene graph
        List<PersoInfo> persos = new();

        if (File.Exists(gptPath))
        {
            Console.WriteLine("Reading scene graph to find Persos...");
            var gpt = new GptReader(gptPath);
            var soReader = new SuperObjectReader(memory);
            var sceneGraph = soReader.ReadSceneGraph(gpt);

            // Find all Perso nodes
            foreach (var node in sceneGraph.AllNodes)
            {
                if (node.Type == SuperObjectType.Perso && node.OffData != 0)
                {
                    var persoInfo = ReadPersoInfo(memory, node.OffData, node.Address);
                    if (persoInfo != null)
                    {
                        persos.Add(persoInfo);
                    }
                }
            }
            Console.WriteLine($"Found {persos.Count} Persos with AI data");
        }

        Console.WriteLine();

        var converter = new SExpressionConverter(AITypes.Hype, new SExpressionOptions
        {
            ShowPointers = true
        });

        int shown = 0;

        // If outputDir specified, save scripts by AIModel (not per-perso, since AIModels are shared)
        if (outputDir != null && persos.Count > 0)
        {
            Directory.CreateDirectory(outputDir);
            Console.WriteLine($"Saving scripts to {outputDir}/");

            // Group persos by AIModel
            var aiModelGroups = persos
                .Where(p => p.AIModelAddress != 0)
                .GroupBy(p => p.AIModelAddress)
                .ToList();

            int aiModelIndex = 0;
            foreach (var group in aiModelGroups)
            {
                int aiModelAddr = group.Key;
                var persosUsingThis = group.ToList();

                var behaviors = ReadBehaviors(memory, aiModelAddr, AITypes.Hype);
                if (behaviors.Count == 0) continue;

                string aiModelDir = Path.Combine(outputDir, $"aimodel_{aiModelIndex:D3}_0x{aiModelAddr:X8}");
                Directory.CreateDirectory(aiModelDir);

                // Write AIModel info including which persos use it
                var infoLines = new List<string>
                {
                    $"AIModel: 0x{aiModelAddr:X8}",
                    $"Used by {persosUsingThis.Count} Perso(s):",
                };
                foreach (var p in persosUsingThis)
                {
                    infoLines.Add($"  - Perso 0x{p.Address:X8} (SO: 0x{p.SuperObjectAddress:X8})");
                }
                File.WriteAllLines(Path.Combine(aiModelDir, "info.txt"), infoLines);

                foreach (var (name, scripts) in behaviors)
                {
                    for (int scriptIdx = 0; scriptIdx < scripts.Count; scriptIdx++)
                    {
                        var script = scripts[scriptIdx];
                        if (script.Nodes.Count < 2) continue;

                        string safeName = name.Replace("[", "_").Replace("]", "");
                        string filename = $"{safeName}_script{scriptIdx}.lisp";

                        string sexpr = converter.Convert(script);
                        string header = $";; {name} script {scriptIdx}\n" +
                                       $";; AIModel: 0x{aiModelAddr:X8}\n" +
                                       $";; Script offset: 0x{script.Offset:X8}\n\n";

                        File.WriteAllText(Path.Combine(aiModelDir, filename), header + sexpr);
                    }
                }

                aiModelIndex++;
                shown++;
            }

            Console.WriteLine($"Saved scripts for {shown} AIModels (used by {persos.Count(p => p.AIModelAddress != 0)} Persos)");
            return 0;
        }

        // If we found persos, show their scripts
        if (persos.Count > 0)
        {
            foreach (var perso in persos.Take(limit))
            {
                Console.WriteLine($"=== Perso @ 0x{perso.Address:X8} ===");
                Console.WriteLine($"    Brain: 0x{perso.BrainAddress:X8}");

                if (perso.AIModelAddress != 0)
                {
                    Console.WriteLine($"    AIModel: 0x{perso.AIModelAddress:X8}");

                    // Try to read behaviors
                    var behaviors = ReadBehaviors(memory, perso.AIModelAddress, AITypes.Hype);

                    foreach (var (name, scripts) in behaviors.Take(5))
                    {
                        Console.WriteLine($"\n  Behavior: {name}");
                        foreach (var script in scripts.Take(3))
                        {
                            if (script.Nodes.Count < 2) continue;

                            if (raw)
                            {
                                PrintRawNodes(script, AITypes.Hype);
                            }
                            else
                            {
                                string sexpr = converter.Convert(script);
                                // Indent the output
                                foreach (var line in sexpr.Split('\n'))
                                {
                                    Console.WriteLine($"    {line}");
                                }
                            }
                        }
                    }
                }

                Console.WriteLine();
                shown++;
            }
        }

        // Fallback to scanning if no persos found
        if (shown == 0)
        {
            Console.WriteLine("Scanning memory for script patterns...");
            var scripts = ScanForScripts(memory, AITypes.Hype, limit * 2);

            Console.WriteLine($"Found {scripts.Count} potential scripts");
            Console.WriteLine();

            foreach (var (address, script) in scripts)
            {
                if (shown >= limit) break;
                if (script.Nodes.Count < 3) continue;

                Console.WriteLine($"=== Script @ 0x{address:X8} ({script.Nodes.Count} nodes) ===");

                if (raw)
                {
                    PrintRawNodes(script, AITypes.Hype);
                }
                else
                {
                    string sexpr = converter.Convert(script);
                    Console.WriteLine(sexpr);
                }

                Console.WriteLine();
                shown++;
            }
        }

        if (shown == 0)
        {
            Console.WriteLine("No valid scripts found.");
        }

        return 0;
    }

    /// <summary>
    /// Reads basic Perso info to find Brain/AIModel.
    /// </summary>
    private static PersoInfo? ReadPersoInfo(MemoryContext memory, int address, int soAddress)
    {
        var reader = memory.GetReaderAt(address);
        if (reader == null) return null;

        // Perso structure (Montreal):
        // +0x00: off_3dData
        // +0x04: off_stdGame
        // +0x08: off_dynam
        // +0x0C: uint32 (Montreal padding)
        // +0x10: off_brain
        // ...

        reader.ReadInt32(); // off_3dData
        reader.ReadInt32(); // off_stdGame
        reader.ReadInt32(); // off_dynam
        reader.ReadInt32(); // Montreal padding
        int offBrain = reader.ReadInt32();

        if (offBrain == 0) return null;

        // Read Brain to get AIModel
        var brainReader = memory.GetReaderAt(offBrain);
        if (brainReader == null) return null;

        // Brain structure (Montreal):
        // +0x00: off_mind
        int offMind = brainReader.ReadInt32();
        if (offMind == 0) return null;

        // Mind structure:
        // +0x00: off_AIModel
        var mindReader = memory.GetReaderAt(offMind);
        if (mindReader == null) return null;

        int offAIModel = mindReader.ReadInt32();

        return new PersoInfo
        {
            Address = address,
            SuperObjectAddress = soAddress,
            BrainAddress = offBrain,
            MindAddress = offMind,
            AIModelAddress = offAIModel
        };
    }

    /// <summary>
    /// Reads behaviors and macros from an AIModel.
    /// </summary>
    private static List<(string name, List<Script> scripts)> ReadBehaviors(MemoryContext memory, int aiModelAddress, AITypes aiTypes)
    {
        var results = new List<(string name, List<Script> scripts)>();

        var reader = memory.GetReaderAt(aiModelAddress);
        if (reader == null) return results;

        // AIModel structure (R2/Montreal):
        // +0x00: off_behaviors_normal
        // +0x04: off_behaviors_reflex
        // +0x08: off_dsgVar
        // +0x0C: off_macros
        // +0x10: flags

        int offBehaviorsNormal = reader.ReadInt32();
        int offBehaviorsReflex = reader.ReadInt32();
        int offDsgVar = reader.ReadInt32();
        int offMacros = reader.ReadInt32();

        // Read normal behaviors
        if (offBehaviorsNormal != 0)
        {
            var normalBehaviors = ReadBehaviorList(memory, offBehaviorsNormal, "Normal", aiTypes);
            results.AddRange(normalBehaviors);
        }

        // Read reflex behaviors
        if (offBehaviorsReflex != 0)
        {
            var reflexBehaviors = ReadBehaviorList(memory, offBehaviorsReflex, "Reflex", aiTypes);
            results.AddRange(reflexBehaviors);
        }

        // Read macros
        if (offMacros != 0)
        {
            var macros = ReadMacroList(memory, offMacros, aiTypes);
            results.AddRange(macros);
        }

        return results;
    }

    /// <summary>
    /// Reads macros from an AIModel.
    /// </summary>
    private static List<(string name, List<Script> scripts)> ReadMacroList(MemoryContext memory, int listAddress, AITypes aiTypes)
    {
        var results = new List<(string name, List<Script> scripts)>();

        var reader = memory.GetReaderAt(listAddress);
        if (reader == null) return results;

        // Macro list:
        // +0x00: off_entries
        // +0x04: num_entries (byte) + 3 padding bytes

        int offEntries = reader.ReadInt32();
        byte numEntries = reader.ReadByte();

        if (offEntries == 0 || numEntries == 0 || numEntries > 100) return results;

        for (int i = 0; i < numEntries; i++)
        {
            // Macro structure (no names in Hype):
            // +0x00: off_script
            // +0x04: off_script2 (unused?)
            int macroOffset = offEntries + i * 8; // 2 pointers = 8 bytes
            var macroReader = memory.GetReaderAt(macroOffset);
            if (macroReader == null) continue;

            int offScript = macroReader.ReadInt32();
            // int offScript2 = macroReader.ReadInt32(); // Usually null

            if (offScript != 0)
            {
                var script = TryReadScript(memory, offScript, aiTypes);
                if (script != null && script.Nodes.Count >= 2)
                {
                    results.Add(($"Macro[{i}]", new List<Script> { script }));
                }
            }
        }

        return results;
    }

    /// <summary>
    /// Reads a list of behaviors.
    /// </summary>
    private static List<(string name, List<Script> scripts)> ReadBehaviorList(MemoryContext memory, int listAddress, string prefix, AITypes aiTypes)
    {
        var results = new List<(string name, List<Script> scripts)>();

        var reader = memory.GetReaderAt(listAddress);
        if (reader == null) return results;

        // Behavior list:
        // +0x00: off_entries (pointer to array of Behavior pointers)
        // +0x04: num_entries

        int offEntries = reader.ReadInt32();
        uint numEntries = reader.ReadUInt32();

        if (offEntries == 0 || numEntries == 0 || numEntries > 100) return results;

        var entriesReader = memory.GetReaderAt(offEntries);
        if (entriesReader == null) return results;

        for (int i = 0; i < numEntries; i++)
        {
            // In Montreal, behaviors are inline (not pointers)
            int behaviorOffset = offEntries + i * GetBehaviorSize();
            var behaviorReader = memory.GetReaderAt(behaviorOffset);
            if (behaviorReader == null) continue;

            var scripts = ReadBehaviorScripts(memory, behaviorOffset, aiTypes);
            if (scripts.Count > 0)
            {
                results.Add(($"{prefix}[{i}]", scripts));
            }
        }

        return results;
    }

    private static int GetBehaviorSize()
    {
        // Behavior structure size in Montreal:
        // 0x100 bytes name + 4 off_scripts + 4 off_scheduleScript + 1 num_scripts + 3 padding = 0x10C
        // But for Hype it might have names... let's use the minimum without names
        return 16; // off_scripts (4) + off_scheduleScript (4) + num_scripts (1) + padding (3) + 4
    }

    /// <summary>
    /// Reads scripts from a Behavior.
    /// </summary>
    private static List<Script> ReadBehaviorScripts(MemoryContext memory, int behaviorAddress, AITypes aiTypes)
    {
        var results = new List<Script>();

        var reader = memory.GetReaderAt(behaviorAddress);
        if (reader == null) return results;

        // Try reading without name first (simpler structure)
        // Behavior structure (no names):
        // +0x00: off_scripts
        // +0x04: off_scheduleScript
        // +0x08: num_scripts (byte)

        int offScripts = reader.ReadInt32();
        int offScheduleScript = reader.ReadInt32();
        byte numScripts = reader.ReadByte();

        // Validate
        if (numScripts > 20) return results;

        // Read scripts array
        if (offScripts != 0 && numScripts > 0)
        {
            var scriptsReader = memory.GetReaderAt(offScripts);
            if (scriptsReader != null)
            {
                for (int i = 0; i < numScripts; i++)
                {
                    // Each entry is a pointer to a script
                    int scriptPtr = scriptsReader.ReadInt32();
                    if (scriptPtr != 0)
                    {
                        var script = TryReadScript(memory, scriptPtr, aiTypes);
                        if (script != null && script.Nodes.Count >= 2)
                        {
                            results.Add(script);
                        }
                    }
                }
            }
        }

        // Read schedule script
        if (offScheduleScript != 0)
        {
            var script = TryReadScript(memory, offScheduleScript, aiTypes);
            if (script != null && script.Nodes.Count >= 2)
            {
                results.Add(script);
            }
        }

        return results;
    }

    /// <summary>
    /// Tries to read a script from an address.
    /// </summary>
    private static Script? TryReadScript(MemoryContext memory, int address, AITypes aiTypes)
    {
        var reader = memory.GetReaderAt(address);
        if (reader == null) return null;

        var script = new Script { Offset = address };
        int maxNodes = 200;

        for (int i = 0; i < maxNodes; i++)
        {
            try
            {
                uint param = reader.ReadUInt32();
                reader.ReadUInt16(); // padding
                byte indent = reader.ReadByte();
                byte type = reader.ReadByte();

                // Validate
                if (type > 50) return null;
                if (indent > 20) return null;

                var nodeType = aiTypes.GetNodeType(type);

                script.Nodes.Add(new ScriptNode
                {
                    Offset = address + i * 8,
                    Param = param,
                    Indent = indent,
                    Type = type,
                    NodeType = nodeType
                });

                if (indent == 0) break;
            }
            catch
            {
                return null;
            }
        }

        // Validate structure
        if (script.Nodes.Count == 0) return null;
        if (script.Nodes[^1].Indent != 0) return null;
        if (!ValidateIndents(script.Nodes)) return null;

        return script;
    }

    private class PersoInfo
    {
        public int Address { get; set; }
        public int SuperObjectAddress { get; set; }
        public int BrainAddress { get; set; }
        public int MindAddress { get; set; }
        public int AIModelAddress { get; set; }
    }

    /// <summary>
    /// Scans memory blocks for script-like patterns.
    /// </summary>
    private static List<(int address, Script script)> ScanForScripts(MemoryContext memory, AITypes aiTypes, int maxScripts)
    {
        var results = new List<(int address, Script script)>();

        foreach (var block in memory.Sna.Blocks)
        {
            if (block.Data == null || block.Data.Length < 16) continue;

            // Scan for script patterns
            // Scripts have nodes with: valid type byte, sensible indent (1-15), and end with indent=0
            for (int offset = 0; offset < block.Data.Length - 16; offset += 4)
            {
                if (results.Count >= maxScripts) break;

                // Check if this looks like the start of a script
                // First node should have indent >= 1 and valid type
                if (offset + 8 > block.Data.Length) continue;

                byte indent = block.Data[offset + 6];
                byte type = block.Data[offset + 7];

                // Script should start with indent 1
                if (indent != 1) continue;

                // Type should be in valid range for Hype (0-41)
                if (type > 41) continue;

                // Check if it's a control flow keyword (If, Then, etc.) - common script starts
                var nodeType = aiTypes.GetNodeType(type);
                if (nodeType != NodeType.KeyWord && nodeType != NodeType.Procedure &&
                    nodeType != NodeType.Operator && nodeType != NodeType.MetaAction)
                    continue;

                // Try to parse as a script
                var script = TryParseScript(block.Data, offset, aiTypes);
                if (script != null && script.Nodes.Count >= 2)
                {
                    int address = block.BaseInMemory + offset;
                    results.Add((address, script));

                    // Skip past this script to avoid overlapping matches
                    offset += (script.Nodes.Count * ScriptNode.Size) - 4;
                }
            }
        }

        return results;
    }

    /// <summary>
    /// Tries to parse a script at a given offset.
    /// </summary>
    private static Script? TryParseScript(byte[] data, int offset, AITypes aiTypes)
    {
        var script = new Script { Offset = offset };

        int pos = offset;
        int maxNodes = 200; // Reasonable limit
        int nodeCount = 0;

        while (pos + 8 <= data.Length && nodeCount < maxNodes)
        {
            uint param = BitConverter.ToUInt32(data, pos);
            byte indent = data[pos + 6];
            byte type = data[pos + 7];

            // Validate
            if (type > 50) return null; // Invalid type
            if (indent > 20) return null; // Unreasonably deep

            var nodeType = aiTypes.GetNodeType(type);

            var node = new ScriptNode
            {
                Offset = pos,
                Param = param,
                Indent = indent,
                Type = type,
                NodeType = nodeType
            };

            script.Nodes.Add(node);
            nodeCount++;
            pos += 8;

            // End marker
            if (indent == 0)
                break;
        }

        // Validate: script must end with indent 0
        if (script.Nodes.Count > 0 && script.Nodes[^1].Indent != 0)
            return null;

        // Validate: check for reasonable indent progression
        if (!ValidateIndents(script.Nodes))
            return null;

        return script;
    }

    /// <summary>
    /// Validates that indent levels follow proper tree structure.
    /// </summary>
    private static bool ValidateIndents(List<ScriptNode> nodes)
    {
        if (nodes.Count == 0) return false;

        int prevIndent = 0;
        foreach (var node in nodes)
        {
            // Indent can increase by at most 1
            if (node.Indent > prevIndent + 1)
                return false;

            prevIndent = node.Indent;
        }

        return true;
    }

    private static void PrintRawNodes(Script script, AITypes aiTypes)
    {
        foreach (var node in script.Nodes)
        {
            string typeName = node.NodeType.ToString();
            string valueName = GetNodeValueName(node, aiTypes);

            string indentStr = new string(' ', node.Indent * 2);
            Console.WriteLine($"{indentStr}[{node.Indent}] {typeName}: {valueName} (param=0x{node.Param:X8})");
        }
    }

    private static string GetNodeValueName(ScriptNode node, AITypes aiTypes)
    {
        return node.NodeType switch
        {
            NodeType.KeyWord => aiTypes.GetKeyword(node.Param),
            NodeType.Condition => aiTypes.GetCondition(node.Param),
            NodeType.Operator => aiTypes.GetOperator(node.Param),
            NodeType.Function => aiTypes.GetFunction(node.Param),
            NodeType.Procedure => aiTypes.GetProcedure(node.Param),
            NodeType.MetaAction => aiTypes.GetMetaAction(node.Param),
            NodeType.Field => aiTypes.GetField(node.Param),
            NodeType.Constant => BitConverter.ToInt32(BitConverter.GetBytes(node.Param), 0).ToString(),
            NodeType.Real => BitConverter.ToSingle(BitConverter.GetBytes(node.Param), 0).ToString("G"),
            NodeType.DsgVarRef => $"dsgVar_{node.Param}",
            NodeType.TextRef => $"text_{node.Param}",
            _ => $"0x{node.Param:X8}"
        };
    }
}
