using System.Buffers.Binary;
using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;
using Astrolabe.Core.Hub;

namespace Astrolabe.Core.Serialization.Codecs;

public sealed class ScriptBlobCodec : IStructCodec<ScriptRecord>, IPointerArrayCodec
{
    public static ScriptBlobCodec Instance { get; } = new();

    public string Kind => "script";
    public string Schema => "astrolabe.script.v1";
    public int? FixedSize => null;
    public IReadOnlyList<PointerField> PointerFields { get; } =
    [
        new PointerField(0x00, "offScript", PointerTarget.BlockRelative)
    ];
    public string PointerArrayPropertyName => "nodes";

    public IReadOnlyList<PointerField> GetPointerFieldsForLength(int byteLength) =>
        [];

    public IReadOnlyList<PointerField> EnumeratePointerFields(ReadOnlySpan<byte> data)
    {
        var fields = new List<PointerField>();
        if (data.Length < sizeof(int))
        {
            return fields;
        }

        fields.Add(new PointerField(0x00, "offScript", PointerTarget.BlockRelative));

        for (var offset = sizeof(int); offset <= data.Length - sizeof(int); offset += sizeof(int))
        {
            var value = BinaryPrimitives.ReadInt32LittleEndian(data.Slice(offset, sizeof(int)));
            if (!VmPointerScanning.IsLikelyVirtualAddress(value))
            {
                continue;
            }

            fields.Add(new PointerField(
                offset,
                $"ptr_{offset:X}",
                PointerTarget.BlockRelative,
                RequiresVmRange: true,
                RequiresDecompressedTarget: true));
        }

        return fields;
    }

    public ScriptRecord Read(ReadOnlySpan<byte> data, int offset, int length)
    {
        var slice = data.Slice(offset, length);
        if (slice.Length == 0)
        {
            return new ScriptRecord { Form = "raw", WireBytes = [] };
        }

        // 8-byte shell: off_script + trailing dword (often a count / pad).
        if (slice.Length == 8 && !Script.LooksLikeNodeStream(slice))
        {
            return new ScriptRecord
            {
                Form = "shell",
                OffScript = HubReferenceIO.Read(slice, 0x00),
                Unknown04 = StructBinaryIO.ReadUInt32(slice, 0x04),
                Nodes = []
            };
        }

        if (Script.LooksLikeNodeStream(slice) && slice.Length % ScriptNode.Size == 0)
        {
            var nodeCount = slice.Length / ScriptNode.Size;
            var nodes = new ScriptNodeRecord[nodeCount];
            for (var i = 0; i < nodeCount; i++)
            {
                var noff = i * ScriptNode.Size;
                nodes[i] = new ScriptNodeRecord
                {
                    Param = StructBinaryIO.ReadUInt32(slice, noff),
                    Padding = StructBinaryIO.ReadUInt16(slice, noff + 4),
                    Indent = StructBinaryIO.ReadByte(slice, noff + 6),
                    Type = StructBinaryIO.ReadByte(slice, noff + 7)
                };
            }

            return new ScriptRecord
            {
                Form = "nodes",
                Nodes = nodes
            };
        }

        return new ScriptRecord
        {
            Form = "raw",
            WireBytes = slice.ToArray(),
            Nodes = []
        };
    }

    public byte[] Write(ScriptRecord value)
    {
        return value.Form switch
        {
            "shell" => WriteShell(value),
            "nodes" => WriteNodes(value),
            _ => value.WireBytes?.ToArray()
                 ?? throw new InvalidDataException($"{Schema} raw form requires wireBytes.")
        };
    }

    public ScriptRecord FromJson(JsonElement json) =>
        JsonStructCodec.Deserialize<ScriptRecord>(json, Schema);

    public void ToJson(ScriptRecord value, Utf8JsonWriter writer) =>
        JsonStructCodec.Serialize(writer, value);

    private static byte[] WriteShell(ScriptRecord value)
    {
        var bytes = new byte[8];
        HubReferenceIO.Write(bytes, 0x00, value.OffScript);
        StructBinaryIO.WriteUInt32(bytes, 0x04, value.Unknown04);
        return bytes;
    }

    private static byte[] WriteNodes(ScriptRecord value)
    {
        if (value.Nodes == null)
        {
            throw new InvalidDataException($"{value.Schema} nodes form requires a non-null nodes array.");
        }

        var bytes = new byte[value.Nodes.Length * ScriptNode.Size];
        for (var i = 0; i < value.Nodes.Length; i++)
        {
            var n = value.Nodes[i];
            var noff = i * ScriptNode.Size;
            StructBinaryIO.WriteUInt32(bytes, noff, n.Param);
            StructBinaryIO.WriteUInt16(bytes, noff + 4, n.Padding);
            StructBinaryIO.WriteByte(bytes, noff + 6, n.Indent);
            StructBinaryIO.WriteByte(bytes, noff + 7, n.Type);
        }

        return bytes;
    }
}
