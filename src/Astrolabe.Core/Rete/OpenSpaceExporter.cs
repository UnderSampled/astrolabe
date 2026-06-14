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
        if (!StructCodecRegistry.TryGet(kind, out _))
        {
            throw new InvalidDataException($"No struct codec registered for kind '{kind}'.");
        }

        var resolver = ReferenceAddressResolver.CreateForExport(packageDir);
        return ReferenceJson.WriteElementBytesForExport(packageDir, kind, relativeDataPath, resolver);
    }

    public static string? FindTargetBlockKey(string packageRoot, int address) =>
        OpenSpace.RelocationGenerator.FindTargetBlockKey(packageRoot, address);

    public static RelocationTableDocument GenerateRtb(
        string packageRoot,
        string fileName,
        IReadOnlyList<string> targetPackageRoots) =>
        OpenSpace.RelocationGenerator.GenerateRtb(packageRoot, fileName, targetPackageRoots);

    public static RelocationTableDocument GenerateFixLevelRtb(
        string fixPackageRoot,
        string levelPackageRoot,
        string fileName) =>
        OpenSpace.RelocationGenerator.GenerateFixLevelRtb(fixPackageRoot, levelPackageRoot, fileName);
}
