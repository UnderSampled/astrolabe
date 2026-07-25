using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;

namespace Astrolabe.Core.Rete;

/// <summary>Loads animation families + transform pool for a package.</summary>
internal sealed class AnimationTreeStore
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    public AnimationFamiliesDocument? Families { get; private set; }
    public AnimationTransformsDocument? Transforms { get; private set; }

    public bool IsLoaded => Families != null || Transforms != null;

    public void Load(string packageDir)
    {
        Families = null;
        Transforms = null;

        var familiesPath = Path.Combine(packageDir, AnimationFamiliesDocument.RelativePath);
        if (File.Exists(familiesPath))
        {
            Families = JsonSerializer.Deserialize<AnimationFamiliesDocument>(
                           File.ReadAllText(familiesPath), JsonOptions)
                       ?? throw new InvalidDataException($"Could not read {familiesPath}");
        }

        var transformsPath = Path.Combine(packageDir, AnimationTransformsDocument.RelativePath);
        if (File.Exists(transformsPath))
        {
            Transforms = JsonSerializer.Deserialize<AnimationTransformsDocument>(
                             File.ReadAllText(transformsPath), JsonOptions)
                         ?? throw new InvalidDataException($"Could not read {transformsPath}");
        }
    }

    public static void Write(
        string packageDir,
        AnimationFamiliesDocument? families,
        AnimationTransformsDocument? transforms)
    {
        if (families != null)
        {
            var path = Path.Combine(packageDir, AnimationFamiliesDocument.RelativePath);
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            File.WriteAllText(path, JsonSerializer.Serialize(families, JsonOptions));
        }

        if (transforms != null)
        {
            var path = Path.Combine(packageDir, AnimationTransformsDocument.RelativePath);
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            File.WriteAllText(path, JsonSerializer.Serialize(transforms, JsonOptions));
        }
    }

    public bool TryGetTransform(string id, out TransformRecord transform)
    {
        transform = null!;
        return Transforms != null && Transforms.ById.TryGetValue(id, out transform!);
    }

    public bool TryGetNode(string id, out AnimationNode node)
    {
        node = null!;
        if (Families == null)
        {
            return false;
        }

        return Families.ById.TryGetValue(id, out node!);
    }
}
