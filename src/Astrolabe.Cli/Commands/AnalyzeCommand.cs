using Astrolabe.Core.FileFormats;

namespace Astrolabe.Cli.Commands;

public static class AnalyzeCommand
{
    public static int Run(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine("Error: Level directory path required");
            Console.Error.WriteLine("Usage: astrolabe analyze <level-dir> [level-name]");
            return 1;
        }

        var levelDir = args[0];
        var levelName = args.Length > 1 ? args[1] : Path.GetFileName(levelDir.TrimEnd('/', '\\'));

        try
        {
            Console.WriteLine($"Loading level: {levelName}");
            var loader = new LevelLoader(levelDir, levelName);

            Console.WriteLine("\n=== SNA Block Summary ===");
            foreach (var block in loader.Sna.Blocks.OrderBy(b => b.BaseInMemory))
            {
                Console.WriteLine($"[{block.Module:X2}:{block.Id:X2}] Base=0x{block.BaseInMemory:X8} Size={block.Data?.Length ?? 0}");
            }

            // Block 05:01 is the "Fix" block in Montreal - contains the globals struct
            var fixBlock = loader.Sna.Blocks.FirstOrDefault(b => b.Module == 0x05 && b.Id == 0x01);
            if (fixBlock?.Data == null)
            {
                Console.WriteLine("Fix block not found!");
                return 1;
            }

            Console.WriteLine($"\n=== Fix Block [05:01] Analysis ===");
            Console.WriteLine($"Base: 0x{fixBlock.BaseInMemory:X8}");
            Console.WriteLine($"Size: {fixBlock.Data.Length} bytes");

            // Get pointer info from RTB
            var rtbBlock = loader.Rtb?.GetBlock(0x05, 0x01);
            if (rtbBlock != null)
            {
                Console.WriteLine($"RTB pointers: {rtbBlock.Count}");

                // Show first 30 pointers to understand the structure
                Console.WriteLine("\nFirst 30 RTB pointers from Fix block:");
                for (int i = 0; i < Math.Min(30, rtbBlock.Pointers.Length); i++)
                {
                    var ptr = rtbBlock.Pointers[i];
                    int offsetInBlock = (int)ptr.OffsetInMemory - fixBlock.BaseInMemory;

                    // Read the actual pointer value at this location
                    if (offsetInBlock >= 0 && offsetInBlock + 4 <= fixBlock.Data.Length)
                    {
                        int ptrValue = BitConverter.ToInt32(fixBlock.Data, offsetInBlock);
                        Console.WriteLine($"  [{i:D4}] Offset 0x{ptr.OffsetInMemory:X8} (block +0x{offsetInBlock:X}) -> [{ptr.TargetModule:X2}:{ptr.TargetId:X2}] value=0x{ptrValue:X8}");
                    }
                }
            }

            // Now let's look for GeometricObject structures in ALL blocks
            Console.WriteLine("\n=== Scanning ALL blocks for GeometricObject headers ===");
            int totalFound = 0;

            foreach (var block in loader.Sna.Blocks.Where(b => b.Data != null && b.Data.Length > 100))
            {
                int baseAddr = block.BaseInMemory;
                int endAddr = baseAddr + block.Data!.Length;
                int found = 0;

                using var ms = new MemoryStream(block.Data);
                using var reader = new BinaryReader(ms);

                for (int offset = 0; offset < block.Data.Length - 64; offset += 4)
                {
                    ms.Position = offset;

                    uint numVertices = reader.ReadUInt32();
                    if (numVertices < 3 || numVertices > 10000) continue;

                    int offVerts = reader.ReadInt32();
                    int offNormals = reader.ReadInt32();
                    int offMaterials = reader.ReadInt32();
                    reader.ReadInt32(); // skip
                    uint numElements = reader.ReadUInt32();

                    if (numElements == 0 || numElements > 1000) continue;

                    // Check if the vertex/normal pointers look valid (pointing within this block)
                    bool vertsValid = offVerts >= baseAddr && offVerts < endAddr;
                    bool normalsValid = offNormals >= baseAddr && offNormals < endAddr;

                    if (vertsValid && normalsValid)
                    {
                        // Validate vertex data at the pointer location
                        int vertOffset = offVerts - baseAddr;
                        if (vertOffset >= 0 && vertOffset + numVertices * 12 <= block.Data.Length)
                        {
                            ms.Position = vertOffset;
                            float x = reader.ReadSingle();
                            float z = reader.ReadSingle();
                            float y = reader.ReadSingle();

                            // Only count meshes with non-trivial vertex data
                            if (!float.IsNaN(x) && !float.IsInfinity(x) &&
                                Math.Abs(x) < 100000 && Math.Abs(y) < 100000 && Math.Abs(z) < 100000)
                            {
                                // Calculate bounding box to filter out all-zero meshes
                                ms.Position = vertOffset;
                                float minX = float.MaxValue, maxX = float.MinValue;
                                float minY = float.MaxValue, maxY = float.MinValue;
                                float minZ = float.MaxValue, maxZ = float.MinValue;
                                bool hasVariation = false;

                                for (int v = 0; v < numVertices; v++)
                                {
                                    float vx = reader.ReadSingle();
                                    float vz = reader.ReadSingle();
                                    float vy = reader.ReadSingle();

                                    if (!float.IsNaN(vx) && !float.IsInfinity(vx))
                                    {
                                        minX = Math.Min(minX, vx); maxX = Math.Max(maxX, vx);
                                        minY = Math.Min(minY, vy); maxY = Math.Max(maxY, vy);
                                        minZ = Math.Min(minZ, vz); maxZ = Math.Max(maxZ, vz);
                                    }
                                }

                                float sizeX = maxX - minX;
                                float sizeY = maxY - minY;
                                float sizeZ = maxZ - minZ;
                                hasVariation = sizeX > 0.01f || sizeY > 0.01f || sizeZ > 0.01f;

                                if (hasVariation)
                                {
                                    if (found == 0)
                                    {
                                        Console.WriteLine($"\n[{block.Module:X2}:{block.Id:X2}] GeometricObjects:");
                                    }
                                    Console.WriteLine($"  +0x{offset:X}: {numVertices} verts, {numElements} elems, size=({sizeX:F2}, {sizeY:F2}, {sizeZ:F2})");
                                    found++;
                                    totalFound++;
                                    if (found >= 10) break; // Limit per block
                                }
                            }
                        }
                    }
                }
            }

            Console.WriteLine($"\nTotal GeometricObjects with variation: {totalFound}");

            // Debug: show pointer stats
            Console.WriteLine("\n=== Pointer validation stats ===");
            int vertPtrCount = 0, normPtrCount = 0, elemTypePtrCount = 0, elemsPtrCount = 0;
            foreach (var block in loader.Sna.Blocks.Where(b => b.Data != null && b.Data.Length > 100))
            {
                int baseAddr = block.BaseInMemory;
                for (int offset = 0; offset < block.Data!.Length - 64; offset += 4)
                {
                    int memAddr = baseAddr + offset;
                    var memory = new MemoryContext(loader.Sna, loader.Rtb);
                    if (memory.GetPointerAt(memAddr + 4) != null) vertPtrCount++;
                    if (memory.GetPointerAt(memAddr + 8) != null) normPtrCount++;
                    if (memory.GetPointerAt(memAddr + 24) != null) elemTypePtrCount++;
                    if (memory.GetPointerAt(memAddr + 28) != null) elemsPtrCount++;
                }
            }
            Console.WriteLine($"Potential vert ptrs: {vertPtrCount}");
            Console.WriteLine($"Potential norm ptrs: {normPtrCount}");
            Console.WriteLine($"Potential elemType ptrs: {elemTypePtrCount}");
            Console.WriteLine($"Potential elems ptrs: {elemsPtrCount}");

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
