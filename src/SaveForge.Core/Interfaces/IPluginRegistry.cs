using SaveForge.Core.Models;
using SaveForge.SDK;

namespace SaveForge.Core.Interfaces;

public interface IPluginRegistry
{
    void Register(InstalledPlugin plugin);
    void Remove(string pluginId);
    IReadOnlyList<InstalledPlugin> GetAll();
    InstalledPlugin? TryGet(string pluginId);
    IReadOnlyList<IGameEditorPlugin> GetLoaded();
}
