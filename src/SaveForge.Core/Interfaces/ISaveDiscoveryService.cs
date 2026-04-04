using SaveForge.Core.Models;
using SaveForge.SDK;

namespace SaveForge.Core.Interfaces;

public interface ISaveDiscoveryService
{
    /// <summary>
    /// Returns all save files found under <paramref name="directory"/> matching the plugin's
    /// declared extensions, sorted by LastModified descending. Never throws — returns empty
    /// list if directory does not exist or is unreadable.
    /// </summary>
    Task<IReadOnlyList<SaveFileInfo>> DiscoverAsync(
        string directory,
        SaveCapabilities capabilities,
        CancellationToken ct = default);
}
