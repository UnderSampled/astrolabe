using System.Buffers.Binary;

namespace Astrolabe.Core.FileFormats.AI;

/// <summary>
/// Represents a parsed OpenSpace AI script containing a list of nodes.
/// Scripts are stored as flat arrays of nodes with an indent field encoding tree structure.
/// </summary>
public class Script
{
    /// <summary>Memory offset where the script starts.</summary>
    public int Offset { get; set; }

    /// <summary>List of script nodes in order.</summary>
    public List<ScriptNode> Nodes { get; } = new();

    /// <summary>
    /// True when <see cref="Nodes"/> is a real ScriptNode stream (not a 4-byte off_script shell).
    /// </summary>
    public bool IsNodeStream { get; set; } = true;

    /// <summary>
    /// Reads a script from a memory address.
    /// Nodes are read until a node with indent=0 is encountered (end marker).
    /// </summary>
    public static Script? Read(MemoryContext memory, int address, AITypes aiTypes)
    {
        var reader = memory.GetReaderAt(address);
        if (reader == null) return null;

        var script = new Script { Offset = address };

        while (true)
        {
            int nodeOffset = address + (script.Nodes.Count * ScriptNode.Size);
            var node = ScriptNode.Read(reader, nodeOffset, aiTypes);
            script.Nodes.Add(node);

            // Indent 0 marks end of script
            if (node.Indent == 0)
                break;

            // Safety: cap runaway reads on corrupt data
            if (script.Nodes.Count > 100_000)
                break;
        }

        script.IsNodeStream = true;
        return script;
    }

    /// <summary>
    /// Reads a script from raw byte data at a given offset.
    /// Stops at indent=0 end marker or when remaining bytes cannot hold a node.
    /// </summary>
    public static Script Read(byte[] data, int offset, AITypes aiTypes)
    {
        var script = new Script { Offset = offset };

        if (data == null || offset < 0 || offset >= data.Length)
        {
            return script;
        }

        using var ms = new MemoryStream(data);
        ms.Position = offset;
        using var reader = new BinaryReader(ms);

        while (ms.Position + ScriptNode.Size <= data.Length)
        {
            int nodeOffset = (int)ms.Position;
            var node = ScriptNode.Read(reader, nodeOffset, aiTypes);
            script.Nodes.Add(node);

            if (node.Indent == 0)
                break;

            if (script.Nodes.Count > 100_000)
                break;
        }

        script.IsNodeStream = LooksLikeNodeStream(data.AsSpan(offset));
        return script;
    }

    /// <summary>
    /// Attempts to parse a ScriptNode stream. Returns false for pointer shells
    /// (off_script headers) or truncated/non-aligned blobs so callers can fall back.
    /// </summary>
    public static bool TryRead(ReadOnlySpan<byte> data, AITypes aiTypes, out Script script)
    {
        script = new Script();
        if (!LooksLikeNodeStream(data))
        {
            return false;
        }

        var bytes = data.ToArray();
        script = Read(bytes, 0, aiTypes);
        return script.Nodes.Count > 0;
    }

    /// <summary>
    /// Heuristic: real ScriptNode arrays have 8-byte alignment, zero padding on each node,
    /// a final indent=0 end marker, and no premature end markers.
    /// Four-byte <c>off_script</c> pointer shells fail this check (non-zero "padding" bytes).
    /// </summary>
    public static bool LooksLikeNodeStream(ReadOnlySpan<byte> data)
    {
        if (data.Length < ScriptNode.Size || data.Length % ScriptNode.Size != 0)
        {
            return false;
        }

        var nodeCount = data.Length / ScriptNode.Size;
        for (var i = 0; i < nodeCount; i++)
        {
            var baseOffset = i * ScriptNode.Size;
            var padding = BinaryPrimitives.ReadUInt16LittleEndian(data.Slice(baseOffset + 4, 2));
            if (padding != 0)
            {
                return false;
            }

            var indent = data[baseOffset + 6];
            if (i < nodeCount - 1)
            {
                // Intermediate nodes must not be end markers.
                if (indent == 0)
                {
                    return false;
                }
            }
            else
            {
                // Final node must be the end marker.
                if (indent != 0)
                {
                    return false;
                }
            }
        }

        // Empty script (sole end marker) is a valid stream.
        if (nodeCount == 1)
        {
            return true;
        }

        // First real statement typically starts at indent 1.
        return data[6] >= 1;
    }
}
