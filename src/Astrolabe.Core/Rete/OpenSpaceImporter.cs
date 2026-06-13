namespace Astrolabe.Core.Rete;

public static class OpenSpaceImporter
{
    public static RetePackageManifest ImportLevel(string levelDir, string outputDir) =>
        OpenSpacePackageCodec.ImportLevel(levelDir, outputDir);
}