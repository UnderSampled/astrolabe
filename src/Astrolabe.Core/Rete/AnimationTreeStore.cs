using System.Text.Json;
using Astrolabe.Core.FileFormats.Animation;
using Astrolabe.Core.Serialization;

namespace Astrolabe.Core.Rete;

internal sealed class AnimationTreeStore
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        PropertyNameCaseInsensitive = true
    };

    private AnimationTreeDocument? _document;

    public bool IsLoaded => _document != null;

    public AnimationTreeDocument Document =>
        _document ?? throw new InvalidOperationException("Animation tree is not loaded.");

    public void Load(string packageDir)
    {
        var path = Path.Combine(packageDir, AnimationTreeDocument.RelativePath);
        if (!File.Exists(path))
        {
            _document = null;
            return;
        }

        _document = JsonSerializer.Deserialize<AnimationTreeDocument>(File.ReadAllText(path), JsonOptions)
            ?? throw new InvalidDataException($"Could not read {path}");
    }

    public static void Write(string packageDir, AnimationTreeDocument document)
    {
        var path = Path.Combine(packageDir, AnimationTreeDocument.RelativePath);
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllText(path, JsonSerializer.Serialize(document, JsonOptions));
    }

    public bool TryResolveTransformAddress(string? jsonPointer, out int virtualAddress)
    {
        virtualAddress = 0;
        if (_document == null ||
            !AnimationTreePaths.TryParseTransformFragment(jsonPointer, out var transformIndex) ||
            transformIndex >= _document.Transforms.Count)
        {
            return false;
        }

        virtualAddress = _document.Transforms[transformIndex].VirtualAddress;
        return virtualAddress != 0;
    }

    public bool TryGetElementRecord(string? jsonPointer, out AnimationTreeElementEntry entry)
    {
        entry = null!;
        if (_document == null ||
            !AnimationTreePaths.TryParseElementFragment(jsonPointer, out var virtualAddress))
        {
            return false;
        }

        return _document.Elements.TryGetValue(virtualAddress.ToString("X8"), out entry!);
    }

    public bool TryGetTransform(int transformIndex, out TransformRecord transform)
    {
        transform = null!;
        if (_document == null || transformIndex < 0 || transformIndex >= _document.Transforms.Count)
        {
            return false;
        }

        transform = _document.Transforms[transformIndex];
        return true;
    }
}