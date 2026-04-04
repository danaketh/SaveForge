using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;

namespace SaveForge.Store;

public sealed class UpdateChecker : IUpdateChecker
{
    private readonly ICatalogService _catalog;
    private readonly IPluginRegistry _registry;

    public UpdateChecker(ICatalogService catalog, IPluginRegistry registry)
    {
        _catalog = catalog;
        _registry = registry;
    }

    public async Task<IReadOnlyList<UpdateInfo>> CheckAsync(CancellationToken ct = default)
    {
        var entries = await _catalog.GetCatalogAsync(forceRefresh: true, ct: ct);
        var updates = new List<UpdateInfo>();

        foreach (var installed in _registry.GetAll())
        {
            if (installed.LoadState != PluginLoadState.Loaded)
            {
                continue;
            }

            var catalogEntry = entries.FirstOrDefault(e => e.Id == installed.Manifest.Id);
            if (catalogEntry is null)
            {
                continue;
            }

            if (IsNewerVersion(installed.Manifest.Version, catalogEntry.LatestVersion))
            {
                updates.Add(new UpdateInfo(installed.Manifest.Id, installed.Manifest.Version, catalogEntry.LatestVersion));
            }
        }

        return updates;
    }

    private static bool IsNewerVersion(string current, string latest)
    {
        if (!Version.TryParse(current, out var cur))
        {
            return false;
        }

        if (!Version.TryParse(latest, out var lat))
        {
            return false;
        }

        return lat > cur;
    }
}
