using Astrolabe.Core.Hub;

namespace Astrolabe.Core.FileFormats.AI;

public sealed class ActionTableRecord
{
    public string Schema { get; set; } = "astrolabe.action-table.v1";
    public HubReference Header { get; set; } = HubReference.Null;
    public uint Unknown04 { get; set; }
    public uint Unknown08 { get; set; }
    public byte[] Unknown0C { get; set; } = [];
}

public sealed class ActionTreeRecord
{
    public string Schema { get; set; } = "astrolabe.action-tree.v1";
    public uint Unknown00 { get; set; }
    public uint Unknown04 { get; set; }
    public HubReference Child { get; set; } = HubReference.Null;
    public HubReference Sibling { get; set; } = HubReference.Null;
    public HubReference Parent { get; set; } = HubReference.Null;
}

public sealed class BehaviorListRecord
{
    public string Schema { get; set; } = "astrolabe.behavior-list.v1";
    public string Type { get; set; } = "";
    public HubReference Entries { get; set; } = HubReference.Null;
    public uint Count { get; set; }
}

public sealed class BehaviorEntryRecord
{
    public HubReference Scripts { get; set; } = HubReference.Null;
    public HubReference ScheduleScript { get; set; } = HubReference.Null;
    public uint Unknown08 { get; set; }
    public uint Unknown0C { get; set; }
}

public sealed class BehaviorArrayRecord
{
    public string Schema { get; set; } = "astrolabe.behaviors-normal.v1";
    public string Type { get; set; } = "";
    public BehaviorEntryRecord[] Entries { get; set; } = [];
}

public sealed class DsgVarRecord
{
    public string Schema { get; set; } = "astrolabe.dsg-var.v1";
    public HubReference MemBuffer { get; set; } = HubReference.Null;
    public HubReference Infos { get; set; } = HubReference.Null;
    public uint DsgMemBufferLength { get; set; }
    public uint AmountOfInfos { get; set; }
}

public sealed class DsgMemRecord
{
    public string Schema { get; set; } = "astrolabe.dsg-mem.v1";
    public HubReference DsgVarPtr { get; set; } = HubReference.Null;
    public HubReference MemBufferInitial { get; set; } = HubReference.Null;
    public HubReference MemBuffer { get; set; } = HubReference.Null;
}

public sealed class ScriptNodeRecord
{
    public uint Param { get; set; }
    public ushort Padding { get; set; }
    public byte Indent { get; set; }
    public byte Type { get; set; }
}

public sealed class ScriptRecord
{
    public string Schema { get; set; } = "astrolabe.script.v1";
    /// <summary>
    /// <c>shell</c> = offScript pointer shell; <c>nodes</c> = ScriptNode stream; <c>raw</c> = unclassified.
    /// </summary>
    public string Form { get; set; } = "raw";
    public HubReference OffScript { get; set; } = HubReference.Null;
    public uint Unknown04 { get; set; }
    public ScriptNodeRecord[] Nodes { get; set; } = [];
    /// <summary>Full wire bytes when form is raw, or residual trailing bytes for partial parses.</summary>
    public byte[]? WireBytes { get; set; }
}
