namespace Astrolabe.Core.FileFormats.AI;

public sealed class BrainRecord
{
    public string Schema { get; set; } = "astrolabe.brain.v1";
    public int Mind { get; set; }
    public int Unknown04 { get; set; }
    public int Unknown08 { get; set; }
}

public sealed class StateRecord
{
    public string Schema { get; set; } = "astrolabe.state.v1";
    public int Next { get; set; }
    public int Prev { get; set; }
    public int Hdr { get; set; }
    public int AnimRef { get; set; }
    public int TransitionsHead { get; set; }
    public int TransitionsTail { get; set; }
    public uint TransitionsCount { get; set; }
    public int ProhibitsHead { get; set; }
    public int ProhibitsTail { get; set; }
    public uint ProhibitsCount { get; set; }
    public int NextState { get; set; }
    public int MechanicsIdCard { get; set; }
    public uint Unknown30 { get; set; }
    public uint Unknown34 { get; set; }
}

public sealed class MindRecord
{
    public string Schema { get; set; } = "astrolabe.mind.v1";
    public int AiModel { get; set; }
    public int IntelligenceNormal { get; set; }
    public int IntelligenceReflex { get; set; }
    public int DsgMem { get; set; }
    public uint Unknown10 { get; set; }
    public uint Unknown14 { get; set; }
}

public sealed class IntelligenceRecord
{
    public string Schema { get; set; } = "astrolabe.intelligence.v1";
    public int AiModel { get; set; }
    public int ActionTree { get; set; }
    public int Comport { get; set; }
    public int LastComport { get; set; }
    public int ActionTable { get; set; }
    public int DefaultComport { get; set; }
}

public sealed class AiModelRecord
{
    public string Schema { get; set; } = "astrolabe.ai-model.v1";
    public int BehaviorsNormal { get; set; }
    public int BehaviorsReflex { get; set; }
    public int DsgVar { get; set; }
    public int Macros { get; set; }
    public uint Unknown10 { get; set; }
}