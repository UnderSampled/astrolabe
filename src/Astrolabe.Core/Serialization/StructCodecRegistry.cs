using System.Text.Json;

namespace Astrolabe.Core.Serialization;

public interface IStructCodecBinding
{
    string Kind { get; }

    object ReadFromBytes(ReadOnlySpan<byte> data, int offset, int length);
    byte[] WriteFromObject(object value);
    byte[] WriteFromJsonPath(string jsonPath);
    void WriteJson(string jsonPath, object value);
}

internal sealed class StructCodecBinding<T> : IStructCodecBinding
{
    private readonly IStructCodec<T> _codec;

    public StructCodecBinding(IStructCodec<T> codec)
    {
        _codec = codec;
    }

    public string Kind => _codec.Kind;

    public object ReadFromBytes(ReadOnlySpan<byte> data, int offset, int length) =>
        _codec.Read(data, offset, length)!;

    public byte[] WriteFromObject(object value) =>
        _codec.Write((T)value);

    public byte[] WriteFromJsonPath(string jsonPath)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(jsonPath));
        return _codec.Write(_codec.FromJson(document.RootElement));
    }

    public void WriteJson(string jsonPath, object value)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(jsonPath)!);
        using var stream = File.Create(jsonPath);
        using var writer = new Utf8JsonWriter(stream, new JsonWriterOptions { Indented = true });
        _codec.ToJson((T)value, writer);
        writer.Flush();
    }
}

public static class StructCodecRegistry
{
    private static readonly Dictionary<string, IStructCodecBinding> Bindings =
        new(StringComparer.OrdinalIgnoreCase);

    static StructCodecRegistry()
    {
        Register(Codecs.SuperObjectCodec.Instance);
        Register(Codecs.MatrixCodec.Instance);
        Register(Codecs.GeometricObjectCodec.Instance);
        Register(Codecs.PhysicalObjectCodec.Instance);
        Register(Codecs.IpoCodec.Instance);
        Register(Codecs.GameMaterialCodec.Instance);
        Register(Codecs.VisualMaterialCodec.Instance);
        Register(Codecs.UInt32RecordCodec.BoundingVolume);
        Register(Codecs.UInt32RecordCodec.CollideMaterial);
        Register(Codecs.Float3ArrayCodec.Vertices);
        Register(Codecs.Float3ArrayCodec.Normals);
        Register(Codecs.Float3ArrayCodec.TriangleNormals);
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
            return codec.WriteFromJsonPath(fullPath);
        }

        return File.ReadAllBytes(fullPath);
    }

    private static string ResolvePath(string rootDir, string relativePath)
    {
        return Path.Combine(relativePath.Split('/').Prepend(rootDir).ToArray());
    }
}