using System.Security.Cryptography;
using System.Text.Json;
using Astrolabe.Core.Serialization.Codecs;

namespace Astrolabe.Core.Serialization;

internal static class OpaqueBinaryStorage
{
    public static OpaqueBinaryRecord Read(JsonElement json, string schema, string? packageRoot = null, string? jsonPath = null)
    {
        var record = JsonStructCodec.Deserialize<OpaqueBinaryRecord>(json, schema);
        LoadData(record, packageRoot, jsonPath);
        return record;
    }

    public static OpaqueBinaryRecord Read(string packageRoot, string jsonPath, string schema)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(jsonPath));
        return Read(document.RootElement, schema, packageRoot, jsonPath);
    }

    public static void Write(string packageRoot, string jsonPath, OpaqueBinaryRecord value)
    {
        var binaryPath = string.IsNullOrWhiteSpace(value.Path)
            ? GetDefaultBinaryPath(packageRoot, jsonPath)
            : NormalizeUriPath(value.Path);
        var fullBinaryPath = ResolvePath(packageRoot, binaryPath);

        Directory.CreateDirectory(Path.GetDirectoryName(fullBinaryPath)!);
        File.WriteAllBytes(fullBinaryPath, value.Data);

        var document = new OpaqueBinaryRecord
        {
            Schema = value.Schema,
            Path = binaryPath,
            Sha256 = HashBytes(value.Data),
            Pointers = value.Pointers.Count == 0
                ? new Dictionary<string, string?>(StringComparer.OrdinalIgnoreCase)
                : new Dictionary<string, string?>(value.Pointers, StringComparer.OrdinalIgnoreCase)
        };

        Directory.CreateDirectory(Path.GetDirectoryName(jsonPath)!);
        File.WriteAllText(jsonPath, JsonSerializer.Serialize(document, JsonStructCodec.Options));
    }

    private static void LoadData(OpaqueBinaryRecord record, string? packageRoot, string? jsonPath)
    {
        if (!string.IsNullOrWhiteSpace(record.Path))
        {
            if (string.IsNullOrWhiteSpace(packageRoot) || string.IsNullOrWhiteSpace(jsonPath))
            {
                throw new InvalidDataException(
                    $"Opaque binary JSON at '{jsonPath ?? "<inline>"}' requires path-aware loading.");
            }

            var fullBinaryPath = ResolvePath(packageRoot, record.Path);
            if (!File.Exists(fullBinaryPath))
            {
                throw new FileNotFoundException($"Opaque binary payload not found: {fullBinaryPath}");
            }

            record.Path = NormalizeUriPath(record.Path);
            record.Data = File.ReadAllBytes(fullBinaryPath);
            record.LegacyData = null;

            if (!string.IsNullOrWhiteSpace(record.Sha256))
            {
                var actualHash = HashBytes(record.Data);
                if (!record.Sha256.Equals(actualHash, StringComparison.OrdinalIgnoreCase))
                {
                    throw new InvalidDataException(
                        $"Opaque binary payload hash mismatch for '{fullBinaryPath}'.");
                }
            }

            return;
        }

        record.Data = record.LegacyData ?? [];
        record.LegacyData = null;
    }

    private static string GetDefaultBinaryPath(string packageRoot, string jsonPath)
    {
        var relativeJsonPath = Path.GetRelativePath(packageRoot, jsonPath);
        return NormalizeUriPath(Path.ChangeExtension(relativeJsonPath, ".bin"));
    }

    private static string ResolvePath(string packageRoot, string relativePath)
    {
        var normalized = relativePath.Replace('/', Path.DirectorySeparatorChar);
        return Path.GetFullPath(Path.Combine(packageRoot, normalized));
    }

    private static string NormalizeUriPath(string path) =>
        path.Replace(Path.DirectorySeparatorChar, '/').Replace(Path.AltDirectorySeparatorChar, '/');

    private static string HashBytes(byte[] data) =>
        Convert.ToHexString(SHA256.HashData(data)).ToLowerInvariant();
}
