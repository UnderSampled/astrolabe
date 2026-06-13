namespace Astrolabe.Core.FileFormats.Perso;

public sealed class PersoRecord
{
    public string Schema { get; set; } = "astrolabe.perso.v1";
    public int Perso3dData { get; set; }
    public int StdGame { get; set; }
    public int Dynam { get; set; }
    public uint Unknown0C { get; set; }
    public int Brain { get; set; }
    public int Camera { get; set; }
    public int CollSet { get; set; }
    public int MsWay { get; set; }
    public int MsLight { get; set; }
    public uint Unknown24 { get; set; }
    public int SectInfo { get; set; }
    public uint Unknown2C { get; set; }
    public int Unknown30 { get; set; }
    public uint Unknown34 { get; set; }
    public uint Unknown38 { get; set; }
    public uint Unknown3C { get; set; }
}

public sealed class Perso3dDataRecord
{
    public string Schema { get; set; } = "astrolabe.perso-3d-data.v1";
    public int StateInitial { get; set; }
    public int StateCurrent { get; set; }
    public int State2 { get; set; }
    public int ObjectList { get; set; }
    public int ObjectListInitial { get; set; }
    public int Family { get; set; }
    public int Unknown18 { get; set; }
    public int Unknown1C { get; set; }
}

public sealed class StandardGameRecord
{
    public string Schema { get; set; } = "astrolabe.standard-game.v1";
    public uint ObjectType0 { get; set; }
    public uint ObjectType1 { get; set; }
    public uint ObjectType2 { get; set; }
    public int SuperObject { get; set; }
    public byte[] Unknown10 { get; set; } = [];
}

public sealed class ObjectListRecord
{
    public string Schema { get; set; } = "astrolabe.object-list.v1";
    public int Next { get; set; }
    public int Prev { get; set; }
    public int Hdr { get; set; }
    public int Entries { get; set; }
    public uint NumEntries { get; set; }
}

public sealed class SpawnableEntryRecord
{
    public string Schema { get; set; } = "astrolabe.spawnable-entry.v1";
    public int Next { get; set; }
    public int Prev { get; set; }
    public int Hdr { get; set; }
    public uint Index { get; set; }
    public int Perso { get; set; }
}

public sealed class PersoSectorInfoRecord
{
    public string Schema { get; set; } = "astrolabe.perso-sector-info.v1";
    public int Sector { get; set; }
    public byte[] Unknown04 { get; set; } = [];
}

public sealed class TransitionRecord
{
    public string Schema { get; set; } = "astrolabe.transition.v1";
    public int Next { get; set; }
    public int Prev { get; set; }
    public int Hdr { get; set; }
    public int TargetState { get; set; }
    public int StateToGo { get; set; }
}