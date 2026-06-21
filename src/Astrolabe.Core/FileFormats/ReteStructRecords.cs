using Astrolabe.Core.Hub;

namespace Astrolabe.Core.FileFormats;

public class SuperObjectRecord
{
    public string Schema { get; set; } = "astrolabe.super-object.v1";
    public uint TypeCode { get; set; }
    public string Type { get; set; } = "";
    public HubReference OffData { get; set; } = HubReference.Null;
    public HubReference ChildrenHead { get; set; } = HubReference.Null;
    public HubReference ChildrenTail { get; set; } = HubReference.Null;
    public uint ChildrenCount { get; set; }
    public HubReference BrotherNext { get; set; } = HubReference.Null;
    public HubReference BrotherPrev { get; set; } = HubReference.Null;
    public HubReference Parent { get; set; } = HubReference.Null;
    public HubReference Matrix { get; set; } = HubReference.Null;
    public HubReference StaticMatrix { get; set; } = HubReference.Null;
    public HubReference GlobalMatrix { get; set; } = HubReference.Null;
    public uint DrawFlags { get; set; }
    public uint Flags { get; set; }
    public HubReference BoundingVolume { get; set; } = HubReference.Null;
}

public sealed class MatrixRecord
{
    public string Schema { get; set; } = "astrolabe.matrix.v1";
    public uint Type { get; set; }
    public float[] Translation { get; set; } = [];
    public float[] BasisX { get; set; } = [];
    public float[] BasisY { get; set; } = [];
    public float[] BasisZ { get; set; } = [];
    public string ExtraBase64 { get; set; } = "";
}

public sealed class UInt32Record
{
    public string Schema { get; set; } = "astrolabe.uint32-record.v1";
    public string Type { get; set; } = "";
    public uint[] Values { get; set; } = [];
}

public sealed class Float3ArrayRecord
{
    public string Schema { get; set; } = "astrolabe.float3-array.v1";
    public string Type { get; set; } = "";
    public float[][] Values { get; set; } = [];
}

public sealed class PointerArrayRecord
{
    public string Schema { get; set; } = "astrolabe.pointer-array.v1";
    public string Type { get; set; } = "";
    public HubReference[] Values { get; set; } = [];
}

public sealed class UInt16ArrayRecord
{
    public string Schema { get; set; } = "astrolabe.uint16-array.v1";
    public string Type { get; set; } = "";
    public ushort[] Values { get; set; } = [];
}

public sealed class FloatArrayRecord
{
    public string Schema { get; set; } = "astrolabe.float-array.v1";
    public string Type { get; set; } = "";
    public float[] Values { get; set; } = [];
}

public sealed class Float2ArrayRecord
{
    public string Schema { get; set; } = "astrolabe.float2-array.v1";
    public string Type { get; set; } = "";
    public float[][] Values { get; set; } = [];
}