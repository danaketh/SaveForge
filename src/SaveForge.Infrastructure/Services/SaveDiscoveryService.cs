using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.SDK;

namespace SaveForge.Infrastructure.Services;

public sealed class SaveDiscoveryService : ISaveDiscoveryService
{
    public Task<IReadOnlyList<SaveFileInfo>> DiscoverAsync(
        string directory,
        SaveCapabilities capabilities,
        CancellationToken ct = default)
    {
        return Task.Run<IReadOnlyList<SaveFileInfo>>(() => Scan(directory, capabilities, ct), ct);
    }

    private static List<SaveFileInfo> Scan(
        string directory,
        SaveCapabilities capabilities,
        CancellationToken ct)
    {
        var resolved = PathTokenExpander.Expand(directory);

        if (!Directory.Exists(resolved))
        {
            return [];
        }

        var extensions = new HashSet<string>(
            capabilities.FileExtensions,
            StringComparer.OrdinalIgnoreCase);

        var results = new List<SaveFileInfo>();

        try
        {
            foreach (var path in Directory.EnumerateFiles(resolved, "*", SearchOption.AllDirectories))
            {
                ct.ThrowIfCancellationRequested();

                var ext = Path.GetExtension(path);
                if (!extensions.Contains(ext))
                {
                    continue;
                }

                try
                {
                    var info = new FileInfo(path);
                    results.Add(new SaveFileInfo(
                        FilePath: info.FullName,
                        FileName: info.Name,
                        LastModified: new DateTimeOffset(info.LastWriteTimeUtc, TimeSpan.Zero),
                        SizeBytes: info.Length));
                }
                catch
                {
                    // File vanished or access denied — skip this entry
                }
            }
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch
        {
            // Directory became unreadable mid-scan — return what we have
        }

        results.Sort((a, b) => b.LastModified.CompareTo(a.LastModified));
        return results;
    }
}
