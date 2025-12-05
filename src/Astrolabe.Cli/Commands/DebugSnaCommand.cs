using Astrolabe.Core.FileFormats;

namespace Astrolabe.Cli.Commands;

public static class DebugSnaCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Level directory path required");
            Console.Error.WriteLine("Usage: astrolabe debug-sna <level-dir> <level-name>");
            return 1;
        }

        var levelDir = args[0];
        var levelName = args.Length > 1 ? args[1] : Path.GetFileName(levelDir.TrimEnd('/', '\\'));

        try
        {
            Console.WriteLine($"Loading level: {levelName}");
            Console.WriteLine($"Directory: {levelDir}");
            Console.WriteLine();

            var loader = new LevelLoader(levelDir, levelName);
            loader.PrintDebugInfo(Console.Out);

            // Try to find and parse some structures
            Console.WriteLine();
            Console.WriteLine("Attempting to locate geometry data...");

            // Look for blocks that might contain geometry
            // In Montreal engine, geometry is typically in specific module/id combinations
            foreach (var block in loader.Sna.Blocks.Where(b => b.Data != null && b.Data.Length > 100))
            {
                // Try to identify potential GeometricObject structures by looking for patterns
                var data = block.Data!;
                using var reader = new BinaryReader(new MemoryStream(data));

                // Skip very small blocks
                if (data.Length < 50) continue;

                // Look for potential vertex count patterns (reasonable values)
                for (int offset = 0; offset < Math.Min(data.Length - 20, 100); offset += 4)
                {
                    reader.BaseStream.Position = offset;

                    // Montreal GeometricObject starts with num_vertices (uint32)
                    uint potentialVertCount = reader.ReadUInt32();
                    if (potentialVertCount > 0 && potentialVertCount < 10000)
                    {
                        // Read potential off_vertices pointer
                        int potentialPtr = reader.ReadInt32();

                        // Check if this pointer could be valid
                        if (potentialPtr > 0 && potentialPtr < 0x10000000)
                        {
                            // This might be a GeometricObject, note it
                            // Console.WriteLine($"  Potential mesh in [{block.Module:X2}:{block.Id:X2}] at offset {offset}: {potentialVertCount} verts, ptr=0x{potentialPtr:X8}");
                        }
                    }
                }
            }

            // Dump some raw hex from the first few blocks
            Console.WriteLine();
            Console.WriteLine("First 64 bytes of first 3 data blocks:");
            int shown = 0;
            foreach (var block in loader.Sna.Blocks.Where(b => b.Data != null).Take(5))
            {
                if (block.Data!.Length < 64) continue;
                if (shown++ >= 3) break;

                Console.WriteLine($"\nBlock [{block.Module:X2}:{block.Id:X2}] (Base=0x{block.BaseInMemory:X8}, {block.Data.Length} bytes):");
                for (int i = 0; i < 64; i++)
                {
                    Console.Write($"{block.Data[i]:X2} ");
                    if ((i + 1) % 16 == 0) Console.WriteLine();
                }
            }

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
