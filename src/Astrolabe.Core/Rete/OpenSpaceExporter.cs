using System.Text.Json;
using Astrolabe.Core.Serialization;

namespace Astrolabe.Core.Rete;

public static class OpenSpaceExporter
{
    public static void ExportLevel(string packageDir, string outputDir) =>
        OpenSpacePackageCodec.ExportLevel(packageDir, outputDir);

    public static IReadOnlyList<OpenSpace.RelocationComparisonResult> CompareGeneratedRelocations(string packageDir) =>
        OpenSpacePackageCodec.CompareGeneratedRelocations(packageDir);

    public static byte[] PreviewStructuredElementBytes(string packageDir, string kind, string relativeDataPath)
    {
        if (!StructCodecRegistry.TryGet(kind, out var codec))
        {
            throw new InvalidDataException($"No struct codec registered for kind '{kind}'.");
        }

        var resolver = ReferenceAddressResolver.CreateForExport(packageDir);

        var elementPath = ReferenceUri.Resolve(packageDir, relativeDataPath).FilePath;
        using var document = JsonDocument.Parse(File.ReadAllText(elementPath));
        using var resolvedDocument = ReferenceJson.ResolvePointersForExport(
            document.RootElement,
            packageDir,
            codec,
            resolver);

        return codec.WriteFromJsonElement(resolvedDocument.RootElement);
    }

    public static string? FindTargetBlockKey(string packageRoot, int address) =>
        OpenSpace.RelocationGenerator.FindTargetBlockKey(packageRoot, address);

    public static RelocationTableDocument GenerateRtb(
        string packageRoot,
        string fileName,
        IReadOnlyList<string> targetPackageRoots) =>
        OpenSpace.RelocationGenerator.GenerateRtb(packageRoot, fileName, targetPackageRoots);
}
