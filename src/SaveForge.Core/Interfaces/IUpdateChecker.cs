namespace SaveForge.Core.Interfaces;

public sealed record UpdateInfo(string PluginId, string InstalledVersion, string AvailableVersion);

public interface IUpdateChecker
{
    Task<IReadOnlyList<UpdateInfo>> CheckAsync(CancellationToken ct = default);
}
