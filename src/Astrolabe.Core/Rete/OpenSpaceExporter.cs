namespace Astrolabe.Core.Rete;

public static class OpenSpaceExporter
{
    public static void ExportLevel(string packageDir, string outputDir) =>
        OpenSpacePackageCodec.ExportLevel(packageDir, outputDir);
}