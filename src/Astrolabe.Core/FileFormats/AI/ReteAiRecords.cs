using Astrolabe.Core.Hub;

namespace Astrolabe.Core.FileFormats.AI;

public sealed class BrainRecord
{
    public string Schema { get; set; } = "astrolabe.brain.v1";
    public HubReference Mind { get; set; } = HubReference.Null;
    public int Unknown04 { get; set; }
    public int Unknown08 { get; set; }
}

public sealed class StateRecord
{
    public string Schema { get; set; } = "astrolabe.state.v1";
    public HubReference Next { get; set; } = HubReference.Null;
    public HubReference Prev { get; set; } = HubReference.Null;
    public HubReference Hdr { get; set; } = HubReference.Null;
    public HubReference AnimRef { get; set; } = HubReference.Null;
    public HubReference TransitionsHead { get; set; } = HubReference.Null;
    public HubReference TransitionsTail { get; set; } = HubReference.Null;
    public uint TransitionsCount { get; set; }
    public HubReference ProhibitsHead { get; set; } = HubReference.Null;
    public HubReference ProhibitsTail { get; set; } = HubReference.Null;
    public uint ProhibitsCount { get; set; }
    public HubReference NextState { get; set; } = HubReference.Null;
    public HubReference MechanicsIdCard { get; set; } = HubReference.Null;
    public uint Unknown30 { get; set; }
    public uint Unknown34 { get; set; }
}

public sealed class MindRecord
{
    public string Schema { get; set; } = "astrolabe.mind.v1";
    public HubReference AiModel { get; set; } = HubReference.Null;
    public HubReference IntelligenceNormal { get; set; } = HubReference.Null;
    public HubReference IntelligenceReflex { get; set; } = HubReference.Null;
    public HubReference DsgMem { get; set; } = HubReference.Null;
    public uint Unknown10 { get; set; }
    public uint Unknown14 { get; set; }
}

public sealed class IntelligenceRecord
{
    public string Schema { get; set; } = "astrolabe.intelligence.v1";
    public HubReference AiModel { get; set; } = HubReference.Null;
    public HubReference ActionTree { get; set; } = HubReference.Null;
    public HubReference Comport { get; set; } = HubReference.Null;
    public HubReference LastComport { get; set; } = HubReference.Null;
    public HubReference ActionTable { get; set; } = HubReference.Null;
    public HubReference DefaultComport { get; set; } = HubReference.Null;
}

public sealed class AiModelRecord
{
    public string Schema { get; set; } = "astrolabe.ai-model.v1";
    public HubReference BehaviorsNormal { get; set; } = HubReference.Null;
    public HubReference BehaviorsReflex { get; set; } = HubReference.Null;
    public HubReference DsgVar { get; set; } = HubReference.Null;
    public HubReference Macros { get; set; } = HubReference.Null;
    public uint Unknown10 { get; set; }
}