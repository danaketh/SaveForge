using SaveForge.Core.Interfaces;

namespace SaveForge.PluginHost;

public sealed class FileDetectionService : IFileDetectionService
{
    private readonly IPluginRegistry _registry;
    private const int HeaderSize = 512;

    public FileDetectionService(IPluginRegistry registry)
    {
        _registry = registry;
    }

    public async Task<IReadOnlyList<DetectionResult>> DetectAsync(string filePath, CancellationToken ct = default)
    {
        var file = new FileInfo(filePath);
        if (!file.Exists)
        {
            return Array.Empty<DetectionResult>();
        }

        var header = await ReadHeaderAsync(filePath, ct);

        var results = new List<DetectionResult>();
        foreach (var plugin in _registry.GetLoaded())
        {
            try
            {
                var (canOpen, priority) = plugin.CanOpen(file, header);
                if (canOpen)
                {
                    results.Add(new DetectionResult(plugin, priority));
                }
            }
            catch
            {
                // Plugin detection failures are non-fatal
            }
        }

        return results.OrderByDescending(r => r.Priority).ToList();
    }

    private static async Task<byte[]> ReadHeaderAsync(string filePath, CancellationToken ct)
    {
        await using var stream = File.OpenRead(filePath);
        var buffer = new byte[Math.Min(HeaderSize, stream.Length)];
        _ = await stream.ReadAsync(buffer, ct);
        return buffer;
    }
}
