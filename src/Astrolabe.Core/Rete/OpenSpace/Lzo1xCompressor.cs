using System.Diagnostics;

namespace Astrolabe.Core.Rete.OpenSpace;

/// <summary>
/// Invokes the vendored GPL LZO 1.08 <c>lzo1x</c> compressor as an external process so Astrolabe stays CC0-clean.
/// </summary>
internal static class Lzo1xCompressor
{
    public const string ExecutableName = "lzo1x";
    public const string EnvironmentVariable = "ASTROLABE_LZO1X";

    private static string? _resolvedPath;

    public static bool TryCompress(ReadOnlySpan<byte> data, out byte[] compressed)
    {
        compressed = [];
        if (data.IsEmpty)
        {
            return true;
        }

        var compressorPath = ResolvePath();
        if (compressorPath == null)
        {
            return false;
        }

        try
        {
            compressed = Invoke(compressorPath, data);
            return true;
        }
        catch (InvalidDataException)
        {
            return false;
        }
    }

    public static byte[] Compress(ReadOnlySpan<byte> data)
    {
        if (data.IsEmpty)
        {
            return [];
        }

        var compressorPath = ResolvePath()
            ?? throw new InvalidOperationException(BuildMissingToolMessage());

        return Invoke(compressorPath, data);
    }

    public static string? ResolvePath()
    {
        if (_resolvedPath != null)
        {
            return File.Exists(_resolvedPath) ? _resolvedPath : null;
        }

        var overridePath = Environment.GetEnvironmentVariable(EnvironmentVariable);
        if (!string.IsNullOrWhiteSpace(overridePath) && File.Exists(overridePath))
        {
            _resolvedPath = Path.GetFullPath(overridePath);
            return _resolvedPath;
        }

        foreach (var root in EnumerateSearchRoots())
        {
            var candidate = Path.Combine(root, "tools", "lzo1x", ExecutableName);
            if (File.Exists(candidate))
            {
                _resolvedPath = candidate;
                return _resolvedPath;
            }
        }

        var pathExecutable = FindOnPath(ExecutableName);
        if (pathExecutable != null)
        {
            _resolvedPath = pathExecutable;
            return _resolvedPath;
        }

        return null;
    }

    internal static string BuildMissingToolMessage() =>
        $"LZO compression requires the external tool '{ExecutableName}'. " +
        $"Build it with `make -C tools/lzo1x` or set {EnvironmentVariable}.";

    private static byte[] Invoke(string compressorPath, ReadOnlySpan<byte> data)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = compressorPath,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true
        };

        using var process = new Process { StartInfo = startInfo };
        if (!process.Start())
        {
            throw new InvalidDataException($"Failed to start LZO compressor: {compressorPath}");
        }

        process.StandardInput.BaseStream.Write(data);
        process.StandardInput.Close();

        using var outputStream = new MemoryStream();
        process.StandardOutput.BaseStream.CopyTo(outputStream);
        var stderr = process.StandardError.ReadToEnd();
        process.WaitForExit();

        return process.ExitCode switch
        {
            0 => outputStream.ToArray(),
            1 => throw new InvalidDataException("LZO input is incompressible."),
            _ => throw new InvalidDataException(
                $"LZO compressor failed (exit {process.ExitCode}): {stderr.Trim()}")
        };
    }

    private static IEnumerable<string> EnumerateSearchRoots()
    {
        var seen = new HashSet<string>(StringComparer.Ordinal);

        foreach (var seed in new[]
                 {
                     Directory.GetCurrentDirectory(),
                     AppContext.BaseDirectory,
                     Path.GetDirectoryName(typeof(Lzo1xCompressor).Assembly.Location) ?? string.Empty
                 })
        {
            if (string.IsNullOrWhiteSpace(seed))
            {
                continue;
            }

            var dir = new DirectoryInfo(Path.GetFullPath(seed));
            while (dir != null)
            {
                if (seen.Add(dir.FullName))
                {
                    yield return dir.FullName;
                }

                dir = dir.Parent;
            }
        }
    }

    private static string? FindOnPath(string executable)
    {
        var pathValue = Environment.GetEnvironmentVariable("PATH");
        if (string.IsNullOrWhiteSpace(pathValue))
        {
            return null;
        }

        foreach (var directory in pathValue.Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries))
        {
            var candidate = Path.Combine(directory, executable);
            if (File.Exists(candidate))
            {
                return candidate;
            }
        }

        return null;
    }
}