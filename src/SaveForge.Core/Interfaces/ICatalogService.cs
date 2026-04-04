using SaveForge.Core.Models;

namespace SaveForge.Core.Interfaces;

public interface ICatalogService
{
    Task<IReadOnlyList<CatalogEntry>> GetCatalogAsync(bool forceRefresh = false, CancellationToken ct = default);
}
