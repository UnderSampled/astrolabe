using System.Text;
using Astrolabe.Core.FileFormats;

namespace Astrolabe.Cli.Commands;

public static class DebugNamesCommand
{
    public static int Run(string[] args)
    {
        if (args.Length < 1)
        {
            Console.WriteLine("Usage: astrolabe debug-names <level-dir> [search-string]");
            Console.WriteLine("       astrolabe debug-names <level-dir> --all   (dump all names)");
            Console.WriteLine("       astrolabe debug-names <level-dir> --hex <address>  (hex dump around address)");
            return 1;
        }

        var levelDir = args[0];
        var levelName = Path.GetFileName(levelDir.TrimEnd('/', '\\'));

        var loader = new LevelLoader(levelDir, levelName);
        var memory = new MemoryContext(loader.Sna, loader.Rtb);
        Console.WriteLine($"Loaded {loader.Sna.Blocks.Count} SNA blocks");

        if (args.Length > 2 && args[1] == "--hex")
        {
            // Hex dump around a memory address
            int addr = Convert.ToInt32(args[2], 16);
            Console.WriteLine($"Hex dump around 0x{addr:X8}:");
            var reader = memory.GetReaderAt(addr - 64);
            if (reader != null)
            {
                for (int i = 0; i < 256; i += 16)
                {
                    int lineAddr = addr - 64 + i;
                    Console.Write($"{lineAddr:X8}: ");
                    byte[] bytes = reader.ReadBytes(16);
                    foreach (var b in bytes)
                        Console.Write($"{b:X2} ");
                    Console.Write(" ");
                    foreach (var b in bytes)
                        Console.Write(b >= 0x20 && b < 0x7F ? (char)b : '.');
                    Console.WriteLine();
                }
            }
            return 0;
        }

        var searchStr = args.Length > 1 ? args[1] : null;
        bool dumpAll = searchStr == "--all";

        foreach (var block in loader.Sna.Blocks)
        {
            if (block.Data == null) continue;

            Console.WriteLine($"Block module={block.Module} id={block.Id} baseAddr=0x{block.BaseInMemory:X8} size={block.Data.Length}");

            if (dumpAll)
            {
                // Look for object type entries: they have a structure with pointer to name followed by the name
                // Pattern: [prev][index][next][ptr1][ptr2][namePtr][01 00 00 00][id][name string]
                Console.WriteLine("  Looking for object type entries:");
                for (int i = 0; i < block.Data.Length - 32; i++)
                {
                    // Check for pattern: 01 00 00 00 followed by a byte id, then a name string
                    if (block.Data[i] == 0x01 && block.Data[i+1] == 0x00 &&
                        block.Data[i+2] == 0x00 && block.Data[i+3] == 0x00)
                    {
                        // Read the id (next 4 bytes)
                        int id = BitConverter.ToInt32(block.Data, i + 4);
                        if (id >= 0 && id < 100)
                        {
                            // Check if there's a valid name string following
                            int nameStart = i + 8;
                            if (nameStart < block.Data.Length &&
                                ((block.Data[nameStart] >= 'A' && block.Data[nameStart] <= 'Z') ||
                                 (block.Data[nameStart] >= 'a' && block.Data[nameStart] <= 'z')))
                            {
                                int j = nameStart;
                                while (j < block.Data.Length && block.Data[j] >= 0x20 && block.Data[j] < 0x7F && j - nameStart < 64)
                                    j++;
                                if (j - nameStart >= 2 && (j >= block.Data.Length || block.Data[j] == 0))
                                {
                                    string name = Encoding.ASCII.GetString(block.Data, nameStart, j - nameStart);
                                    if (name.All(c => char.IsLetterOrDigit(c) || c == '_' || c == '-'))
                                    {
                                        // Determine type by prefix
                                        string typeName = "Family";
                                        if (name.StartsWith("I") && name.Length > 1 && char.IsUpper(name[1]))
                                            typeName = "Instance";
                                        else if (name.StartsWith("M") && name.Length > 1 && char.IsUpper(name[1]))
                                            typeName = "Model";

                                        int memAddr = block.BaseInMemory + nameStart;
                                        Console.WriteLine($"    [{typeName}] id={id} 0x{memAddr:X8}: \"{name}\"");
                                        i = j;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            else if (searchStr != null)
            {
                string blockStr = Encoding.ASCII.GetString(block.Data);
                int idx = blockStr.IndexOf(searchStr, StringComparison.OrdinalIgnoreCase);
                if (idx >= 0)
                {
                    Console.WriteLine($"  *** Found '{searchStr}' at offset 0x{idx:X} (memAddr 0x{block.BaseInMemory + idx:X8})");
                    Console.WriteLine("  Names found nearby:");
                    for (int i = Math.Max(0, idx - 2000); i < Math.Min(block.Data.Length - 4, idx + 2000); i++)
                    {
                        if (block.Data[i] >= 'A' && block.Data[i] <= 'Z')
                        {
                            int j = i;
                            while (j < block.Data.Length && ((block.Data[j] >= 0x20 && block.Data[j] < 0x7F) || block.Data[j] == 0) && j - i < 64)
                            {
                                if (block.Data[j] == 0) break;
                                j++;
                            }
                            if (j - i >= 3 && block.Data[j] == 0)
                            {
                                string name = Encoding.ASCII.GetString(block.Data, i, j - i);
                                if (name.Length >= 3 && name.All(c => char.IsLetterOrDigit(c) || c == '_' || c == '-'))
                                {
                                    int memAddr = block.BaseInMemory + i;
                                    Console.WriteLine($"    0x{memAddr:X8} (+0x{i:X}): \"{name}\"");
                                    i = j;
                                }
                            }
                        }
                    }
                }
            }
        }

        return 0;
    }
}
