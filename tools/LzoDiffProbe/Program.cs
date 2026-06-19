using System.IO.Compression;
using Astrolabe.Core.FileFormats;
using Astrolabe.Core.Rete.OpenSpace;
using lzo.net;

var path = args.Length > 0
    ? args[0]
    : Path.Combine(FindRepoRoot(), "disc", "Gamedata", "World", "Levels", "astrolabe", "astrolabe.rtp");

var reader = new RelocationTableReader(path);
var block = reader.PointerBlocks.Single(b => b.Module == 0x05 && b.Id == 0x01);
var original = block.CompressedData;
var plain = block.PointerData;
var recomp = OpenSpaceLzo.Compress(plain);

Console.WriteLine($"file={path}");
Console.WriteLine($"block=05:01 isCompressed={block.IsCompressed} count={block.Count} entrySize={block.EntrySize}");
Console.WriteLine($"plain={plain.Length} decompressedSize={block.DecompressedSize} checksum=0x{block.DecompressedChecksum:X8}");
Console.WriteLine($"original={original.Length} compressedChecksum=0x{block.CompressedChecksum:X8}");
Console.WriteLine($"recomp={recomp.Length} recompChecksum=0x{OpenSpaceChecksum.Calculate(recomp):X8}");
Console.WriteLine($"plain checksum=0x{OpenSpaceChecksum.Calculate(plain):X8}");
Console.WriteLine();

var min = Math.Min(original.Length, recomp.Length);
var firstDiff = -1;
for (var i = 0; i < min; i++)
{
    if (original[i] != recomp[i])
    {
        firstDiff = i;
        break;
    }
}

if (firstDiff < 0 && original.Length != recomp.Length)
{
    firstDiff = min;
}

if (firstDiff >= 0)
{
    Console.WriteLine($"First differing byte index: 0x{firstDiff:X} ({firstDiff})");
    var start = Math.Max(0, firstDiff - 8);
    var end = Math.Min(Math.Max(original.Length, recomp.Length), firstDiff + 24);
    Console.WriteLine("idx  orig recomp");
    for (var i = start; i < end; i++)
    {
        var o = i < original.Length ? original[i] : (byte?)null;
        var r = i < recomp.Length ? recomp[i] : (byte?)null;
        var mark = i == firstDiff ? " <--" : string.Empty;
        Console.WriteLine(
            $"0x{i:X3} {(o.HasValue ? $"0x{o.Value:X2}" : "  --")}   {(r.HasValue ? $"0x{r.Value:X2}" : "  --")}{mark}");
    }
}

var diffs = 0;
for (var i = 0; i < Math.Max(original.Length, recomp.Length); i++)
{
    byte? o = i < original.Length ? original[i] : null;
    byte? r = i < recomp.Length ? recomp[i] : null;
    if (o != r)
    {
        diffs++;
    }
}

Console.WriteLine($"Total byte positions differing (incl length): {diffs}");

Console.WriteLine("\nOriginal first 64:");
Dump(original, 0, 64);
Console.WriteLine("Recomp first 64:");
Dump(recomp, 0, 64);

Console.WriteLine("\nOriginal last 32:");
Dump(original, Math.Max(0, original.Length - 32), 32);
Console.WriteLine("Recomp last 32:");
Dump(recomp, Math.Max(0, recomp.Length - 32), 32);

if (firstDiff >= 0)
{
    Console.WriteLine("\nStream bytes around first diff:");
    Console.WriteLine("Original:");
    Dump(original, Math.Max(0, firstDiff - 16), Math.Min(64, original.Length - Math.Max(0, firstDiff - 16)));
    Console.WriteLine("Recomp:");
    Dump(recomp, Math.Max(0, firstDiff - 16), Math.Min(64, recomp.Length - Math.Max(0, firstDiff - 16)));
}

Console.WriteLine("\nPlaintext first 96 bytes:");
Dump(plain, 0, Math.Min(96, plain.Length));

Console.WriteLine("\nPointer entries (first 8):");
for (var i = 0; i < Math.Min(8, block.Count); i++)
{
    var p = block.Pointers[i];
    Console.WriteLine(
        $"  [{i}] off=0x{p.OffsetInMemory:X8} -> {p.TargetModule:X2}:{p.TargetId:X2} " +
        $"b6=0x{p.Byte6:X2} b7=0x{p.Byte7:X2}");
}

Console.WriteLine("\nLZO instruction trace (first 12 steps each):");
TraceLzo("original", original, 12);
TraceLzo("recomp", recomp, 12);

static void TraceLzo(string label, byte[] stream, int maxSteps)
{
    Console.WriteLine(label + ":");
    var pos = 0;
    var outPos = 0;
    var state = 0; // ZeroCopy
    var step = 0;

    if (pos >= stream.Length)
    {
        return;
    }

    var instruction = stream[pos++];
    if (instruction is > 15 and <= 17)
    {
        Console.WriteLine("  invalid first opcode");
        return;
    }

    if (instruction >= 18)
    {
        var numLiterals = instruction - 17;
        Console.WriteLine(
            $"  step {step++}: FIRST literals={numLiterals} stream@0x{pos - 1:X3} out@0x{outPos:X3}");
        pos += numLiterals;
        outPos += numLiterals;
        state = instruction <= 21 ? numLiterals : 4;
        if (pos >= stream.Length)
        {
            return;
        }

        instruction = stream[pos++];
    }

    while (step < maxSteps && pos < stream.Length)
    {
        var startPos = pos - 1;
        if (instruction <= 15)
        {
            if (state == 0)
            {
                var length = 3 + (instruction == 0 ? 15 + ReadLength(stream, ref pos) : instruction);
                Console.WriteLine(
                    $"  step {step++}: LONG_LITERAL len={length} op=0x{instruction:X2} stream@0x{startPos:X3} out@0x{outPos:X3}");
                outPos += length;
                state = 4;
            }
            else
            {
                Console.WriteLine(
                    $"  step {step++}: SMALL_LITERAL op=0x{instruction:X2} state={state} stream@0x{startPos:X3} out@0x{outPos:X3}");
                outPos += 1;
                state = Math.Max(0, state - 1);
            }
        }
        else if (instruction < 32)
        {
            var l = instruction & 0x7;
            var length = 2 + (l == 0 ? 7 + ReadLength(stream, ref pos) : l);
            if (pos + 1 >= stream.Length)
            {
                break;
            }

            var s = stream[pos++];
            var d = stream[pos++];
            var distance = 16384 + ((instruction & 0x8) << 11) + (((d << 8) | s) >> 2);
            var copyState = s & 0x3;
            Console.WriteLine(
                $"  step {step++}: FAR_COPY len={length} dist={distance} trail={copyState} op=0x{instruction:X2} stream@0x{startPos:X3} out@0x{outPos:X3}");
            outPos += length + copyState;
            state = copyState;
        }
        else if (instruction < 64)
        {
            var l = instruction & 0x1f;
            var length = 2 + (l == 0 ? 31 + ReadLength(stream, ref pos) : l);
            if (pos + 1 >= stream.Length)
            {
                break;
            }

            var s = stream[pos++];
            var d = stream[pos++];
            var distance = (((d << 8) | s) >> 2) + 1;
            var copyState = s & 0x3;
            Console.WriteLine(
                $"  step {step++}: MID_COPY len={length} dist={distance} trail={copyState} op=0x{instruction:X2} stream@0x{startPos:X3} out@0x{outPos:X3}");
            outPos += length + copyState;
            state = copyState;
        }
        else if (instruction < 128)
        {
            if (pos >= stream.Length)
            {
                break;
            }

            var length = 3 + ((instruction >> 5) & 0x1);
            var h = stream[pos++];
            var distance = (h << 3) + ((instruction >> 2) & 0x7) + 1;
            var copyState = instruction & 0x3;
            Console.WriteLine(
                $"  step {step++}: NEAR_COPY len={length} dist={distance} trail={copyState} op=0x{instruction:X2} stream@0x{startPos:X3} out@0x{outPos:X3}");
            outPos += length + copyState;
            state = copyState;
        }
        else
        {
            if (pos >= stream.Length)
            {
                break;
            }

            var length = 5 + ((instruction >> 5) & 0x3);
            var h = stream[pos++];
            var distance = (h << 3) + ((instruction & 0x1c) >> 2) + 1;
            var copyState = instruction & 0x3;
            Console.WriteLine(
                $"  step {step++}: NEAR_COPY2 len={length} dist={distance} trail={copyState} op=0x{instruction:X2} stream@0x{startPos:X3} out@0x{outPos:X3}");
            outPos += length + copyState;
            state = copyState;
        }

        if (pos >= stream.Length)
        {
            break;
        }

        instruction = stream[pos++];
        if (distanceCheck(instruction, stream, pos))
        {
            Console.WriteLine($"  step {step++}: END marker op=0x{instruction:X2} stream@0x{pos - 1:X3}");
            break;
        }
    }
}

static bool distanceCheck(int instruction, byte[] stream, int pos) =>
    instruction is >= 16 and < 32 && pos + 1 < stream.Length &&
    16384 + ((instruction & 0x8) << 11) + (((stream[pos + 1] << 8) | stream[pos]) >> 2) == 16384;

static int ReadLength(byte[] stream, ref int pos)
{
    var length = 0;
    int value;
    do
    {
        if (pos >= stream.Length)
        {
            return length;
        }

        value = stream[pos++];
        length += value;
    }
    while (value == 0);

    return length;
}

static void Dump(byte[] data, int offset, int count)
{
    for (var row = 0; row < count; row += 16)
    {
        var line = data.AsSpan(offset + row, Math.Min(16, count - row));
        Console.WriteLine($"  0x{offset + row:X4}: {BitConverter.ToString(line.ToArray()).Replace('-', ' ')}");
    }
}

static string FindRepoRoot()
{
    var dir = new DirectoryInfo(AppContext.BaseDirectory);
    while (dir != null)
    {
        if (Directory.Exists(Path.Combine(dir.FullName, "disc", "Gamedata", "World", "Levels")))
        {
            return dir.FullName;
        }

        dir = dir.Parent;
    }

    throw new InvalidOperationException("Could not find repository root.");
}