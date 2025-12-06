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
        }

        return script;
    }

    /// <summary>
    /// Reads a script from raw byte data at a given offset.
    /// </summary>
    public static Script Read(byte[] data, int offset, AITypes aiTypes)
    {
        var script = new Script { Offset = offset };

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
        }

        return script;
    }
}
