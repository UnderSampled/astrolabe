using System.Text.Json;

namespace Astrolabe.Core.Serialization;

public interface IStructCodecBinding
{
    string Kind { get; }
    int? FixedSize { get; }
    IReadOnlyList<PointerField> PointerFields { get; }
    bool IsPointerArray { get; }
    bool UsesExternalBinaryPayload { get; }
    string PointerArrayPropertyName { get; }
    int PointerEntryStride { get; }
    IReadOnlyDictionary<string, string> PointerFieldAliases { get; }

    IReadOnlyList<PointerField> ResolvePointerFields(int serializedByteLength);
    IReadOnlyList<PointerField> EnumeratePointerFields(ReadOnlySpan<byte> data);
    object ReadFromBytes(ReadOnlySpan<byte> data, int offset, int length);
    object ReadFromJsonElement(JsonElement json);
    object ReadFromJsonPath(string packageRoot, string jsonPath);
    byte[] WriteFromObject(object value);
    byte[] WriteFromJsonElement(JsonElement json);
    byte[] WriteFromJsonPath(string packageRoot, string jsonPath);
    void WriteJson(string packageRoot, string jsonPath, object value);
}

internal sealed class StructCodecBinding<T> : IStructCodecBinding
{
    private readonly IStructCodec<T> _codec;

    public StructCodecBinding(IStructCodec<T> codec)
    {
        _codec = codec;
    }

    public string Kind => _codec.Kind;
    public int? FixedSize => _codec.FixedSize;
    public IReadOnlyList<PointerField> PointerFields => _codec.PointerFields;
    public bool IsPointerArray => _codec is IPointerArrayCodec;
    public bool UsesExternalBinaryPayload => typeof(T) == typeof(Codecs.OpaqueBinaryRecord);
    public string PointerArrayPropertyName =>
        (_codec as IPointerArrayCodec)?.PointerArrayPropertyName ?? "values";
    public int PointerEntryStride =>
        (_codec as IPointerArrayCodec)?.PointerEntryStride ?? 4;
    public IReadOnlyDictionary<string, string> PointerFieldAliases =>
        (_codec as IPointerFieldAliases)?.PointerFieldAliases
        ?? StructCodecRegistry.EmptyPointerFieldAliases;

    public IReadOnlyList<PointerField> ResolvePointerFields(int serializedByteLength) =>
        (_codec as IPointerArrayCodec)?.GetPointerFieldsForLength(serializedByteLength)
        ?? _codec.PointerFields;

    public IReadOnlyList<PointerField> EnumeratePointerFields(ReadOnlySpan<byte> data) =>
        (_codec as IPointerArrayCodec)?.EnumeratePointerFields(data)
        ?? _codec.PointerFields;

    public object ReadFromBytes(ReadOnlySpan<byte> data, int offset, int length) =>
        _codec.Read(data, offset, length)!;

    public object ReadFromJsonElement(JsonElement json)
    {
        if (UsesExternalBinaryPayload)
        {
            return OpaqueBinaryStorage.Read(json, _codec.Schema);
        }

        return _codec.FromJson(json)!;
    }

    public object ReadFromJsonPath(string packageRoot, string jsonPath)
    {
        if (UsesExternalBinaryPayload)
        {
            return OpaqueBinaryStorage.Read(packageRoot, jsonPath, _codec.Schema);
        }

        using var document = JsonDocument.Parse(File.ReadAllText(jsonPath));
        return _codec.FromJson(document.RootElement)!;
    }

    public byte[] WriteFromObject(object value) =>
        _codec.Write((T)value);

    public byte[] WriteFromJsonElement(JsonElement json) =>
        _codec.Write((T)ReadFromJsonElement(json));

    public byte[] WriteFromJsonPath(string packageRoot, string jsonPath)
    {
        return WriteFromObject(ReadFromJsonPath(packageRoot, jsonPath));
    }

    public void WriteJson(string packageRoot, string jsonPath, object value)
    {
        if (UsesExternalBinaryPayload)
        {
            OpaqueBinaryStorage.Write(packageRoot, jsonPath, (Codecs.OpaqueBinaryRecord)value);
            return;
        }

        Directory.CreateDirectory(Path.GetDirectoryName(jsonPath)!);
        using var stream = File.Create(jsonPath);
        using var writer = new Utf8JsonWriter(stream, new JsonWriterOptions { Indented = true });
        _codec.ToJson((T)value, writer);
        writer.Flush();
    }
}

public static class StructCodecRegistry
{
    internal static readonly IReadOnlyDictionary<string, string> EmptyPointerFieldAliases =
        new Dictionary<string, string>();

    private static readonly Dictionary<string, IStructCodecBinding> Bindings =
        new(StringComparer.OrdinalIgnoreCase);

    static StructCodecRegistry()
    {
        Register(Codecs.SuperObjectCodec.Instance);
        Register(Codecs.MatrixCodec.Instance);
        Register(Codecs.GeometricObjectCodec.Instance);
        Register(Codecs.PhysicalObjectCodec.Instance);
        Register(Codecs.IpoCodec.Instance);
        Register(Codecs.VisualSetCodec.Instance);
        Register(Codecs.ElementTrianglesCodec.Instance);
        Register(Codecs.RadiosityHeaderCodec.Instance);
        Register(Codecs.GameMaterialCodec.Instance);
        Register(Codecs.VisualMaterialCodec.Instance);
        Register(Codecs.UInt32RecordCodec.BoundingVolume);
        Register(Codecs.UInt32RecordCodec.CollideMaterial);
        Register(Codecs.Float3ArrayCodec.Vertices);
        Register(Codecs.Float3ArrayCodec.Normals);
        Register(Codecs.Float3ArrayCodec.TriangleNormals);
        Register(Codecs.PointerArrayCodec.ElementPtrs);
        Register(Codecs.PointerArrayCodec.LodDataOffsets);
        Register(Codecs.PointerArrayCodec.AnimChannelPtrs);
        Register(Codecs.PointerArrayCodec.ScriptPtrs);
        Register(Codecs.PointerArrayCodec.DsgVarPtrIndirect);
        Register(Codecs.PointerArrayCodec.CollideElementPtrs);
        Register(Codecs.UInt16ArrayCodec.ElementTypes);
        Register(Codecs.UInt16ArrayCodec.VertexIndices);
        Register(Codecs.UInt16ArrayCodec.UvMapping);
        Register(Codecs.UInt16ArrayCodec.Triangles);
        Register(Codecs.FloatArrayCodec.LodDistances);
        Register(Codecs.Float2ArrayCodec.Uvs);
        Register(Codecs.AnimChannelCodec.Instance);
        Register(Codecs.ElementSpritesCodec.Instance);
        Register(Codecs.PersoCodec.Instance);
        Register(Codecs.Perso3dDataCodec.Instance);
        Register(Codecs.BrainCodec.Instance);
        Register(Codecs.StateCodec.Instance);
        Register(Codecs.AnimFramesCodec.Instance);
        Register(Codecs.AnimationMontrealCodec.Instance);
        Register(Codecs.AnimHierarchiesHeaderCodec.Instance);
        Register(Codecs.TransitionCodec.Instance);
        Register(Codecs.CollideSetCodec.Instance);
        Register(Codecs.StandardGameCodec.Instance);
        Register(Codecs.ObjectListCodec.Instance);
        Register(Codecs.SpawnableEntryCodec.Instance);
        Register(Codecs.MindCodec.Instance);
        Register(Codecs.IntelligenceCodec.Instance);
        Register(Codecs.AiModelCodec.Instance);
        Register(Codecs.PersoSectorInfoCodec.Instance);
        Register(Codecs.SectorCodec.Instance);
        Register(Codecs.FixedBinaryStructCodec.ActionTable);
        Register(Codecs.FixedBinaryStructCodec.ActionTree);
        Register(Codecs.FixedBinaryStructCodec.DsgVar);
        Register(Codecs.FixedBinaryStructCodec.Dynam);
        Register(Codecs.FixedBinaryStructCodec.DsgMem);
        Register(Codecs.FixedBinaryStructCodec.BehaviorListNormal);
        Register(Codecs.FixedBinaryStructCodec.BehaviorListReflex);
        Register(Codecs.BehaviorArrayCodec.BehaviorsNormal);
        Register(Codecs.BehaviorArrayCodec.BehaviorsReflex);
        Register(Codecs.AnimHierarchiesCodec.Instance);
        Register(Codecs.ScriptBlobCodec.Instance);
        Register(Codecs.AlwaysSuperObjectsCodec.Instance);
        Register(Codecs.RawBlobCodec.Instance);
        Register(Codecs.FixedBinaryStructCodec.ObjectTypeEntry);
        Register(Codecs.FixedBinaryStructCodec.SectorCollideGeo);
        Register(Codecs.FixedBinaryStructCodec.CollideZoneList("collidezdxlist"));
        Register(Codecs.FixedBinaryStructCodec.CollideZoneList("collidezddlist"));
        Register(Codecs.FixedBinaryStructCodec.CollideZoneList("collidezdelist"));
        Register(Codecs.FixedBinaryStructCodec.CollideZone("collidezdxzone"));
        Register(Codecs.FixedBinaryStructCodec.CollideZone("collidezddzone"));
        Register(Codecs.FixedBinaryStructCodec.CollideZone("collidezdezone"));
    }

    public static void Register<T>(IStructCodec<T> codec)
    {
        Bindings[codec.Kind] = new StructCodecBinding<T>(codec);
    }

    public static bool TryGet(string kind, out IStructCodecBinding binding) =>
        Bindings.TryGetValue(kind, out binding!);

    public static byte[] ReadElementBytes(string intermediateDir, string dataPath, string kind)
    {
        var fullPath = ResolvePath(intermediateDir, dataPath);
        if (dataPath.EndsWith(".json", StringComparison.OrdinalIgnoreCase) &&
            TryGet(kind, out var codec))
        {
            return codec.WriteFromJsonPath(intermediateDir, fullPath);
        }

        return File.ReadAllBytes(fullPath);
    }

    private static string ResolvePath(string rootDir, string relativePath)
    {
        return Path.Combine(relativePath.Split('/').Prepend(rootDir).ToArray());
    }
}
