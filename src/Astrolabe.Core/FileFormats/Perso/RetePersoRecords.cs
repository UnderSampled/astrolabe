using Astrolabe.Core.Hub;

namespace Astrolabe.Core.FileFormats.Perso;

public sealed class PersoRecord
{
    public string Schema { get; set; } = "astrolabe.perso.v1";
    public HubReference Perso3dData { get; set; } = HubReference.Null;
    public HubReference StdGame { get; set; } = HubReference.Null;
    public HubReference Dynam { get; set; } = HubReference.Null;
    public uint Unknown0C { get; set; }
    public HubReference Brain { get; set; } = HubReference.Null;
    public HubReference Camera { get; set; } = HubReference.Null;
    public HubReference CollSet { get; set; } = HubReference.Null;
    public HubReference MsWay { get; set; } = HubReference.Null;
    public HubReference MsLight { get; set; } = HubReference.Null;
    public uint Unknown24 { get; set; }
    public HubReference SectInfo { get; set; } = HubReference.Null;
    public uint Unknown2C { get; set; }
    public HubReference Unknown30 { get; set; } = HubReference.Null;
    public uint Unknown34 { get; set; }
    public uint Unknown38 { get; set; }
    public uint Unknown3C { get; set; }
}

public sealed class Perso3dDataRecord
{
    public string Schema { get; set; } = "astrolabe.perso-3d-data.v1";
    public HubReference StateInitial { get; set; } = HubReference.Null;
    public HubReference StateCurrent { get; set; } = HubReference.Null;
    public HubReference State2 { get; set; } = HubReference.Null;
    public HubReference ObjectList { get; set; } = HubReference.Null;
    public HubReference ObjectListInitial { get; set; } = HubReference.Null;
    public HubReference Family { get; set; } = HubReference.Null;
    public int Unknown18 { get; set; }
    public int Unknown1C { get; set; }
}

public sealed class StandardGameRecord
{
    public string Schema { get; set; } = "astrolabe.standard-game.v1";
    public uint ObjectType0 { get; set; }
    public uint ObjectType1 { get; set; }
    public uint ObjectType2 { get; set; }
    public HubReference SuperObject { get; set; } = HubReference.Null;
    public byte[] Unknown10 { get; set; } = [];
}

public sealed class ObjectListRecord
{
    public string Schema { get; set; } = "astrolabe.object-list.v1";
    public HubReference Next { get; set; } = HubReference.Null;
    public HubReference Prev { get; set; } = HubReference.Null;
    public HubReference Hdr { get; set; } = HubReference.Null;
    public HubReference Entries { get; set; } = HubReference.Null;
    public uint NumEntries { get; set; }
}

public sealed class SpawnableEntryRecord
{
    public string Schema { get; set; } = "astrolabe.spawnable-entry.v1";
    public HubReference Next { get; set; } = HubReference.Null;
    public HubReference Prev { get; set; } = HubReference.Null;
    public HubReference Hdr { get; set; } = HubReference.Null;
    public uint Index { get; set; }
    public HubReference Perso { get; set; } = HubReference.Null;
}

public sealed class PersoSectorInfoRecord
{
    public string Schema { get; set; } = "astrolabe.perso-sector-info.v1";
    public HubReference Sector { get; set; } = HubReference.Null;
    public byte[] Unknown04 { get; set; } = [];
}

public sealed class TransitionRecord
{
    public string Schema { get; set; } = "astrolabe.transition.v1";
    public HubReference Next { get; set; } = HubReference.Null;
    public HubReference Prev { get; set; } = HubReference.Null;
    public HubReference Hdr { get; set; } = HubReference.Null;
    public HubReference TargetState { get; set; } = HubReference.Null;
    public HubReference StateToGo { get; set; } = HubReference.Null;
}