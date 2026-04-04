namespace SaveForge.Core.Interfaces;

public interface IPluginInstaller
{
    Task InstallAsync(string catalogEntryId, IProgress<long>? progress = null, CancellationToken ct = default);
    Task RemoveAsync(string pluginId, CancellationToken ct = default);
    Task UpdateAsync(string pluginId, IProgress<long>? progress = null, CancellationToken ct = default);
}
