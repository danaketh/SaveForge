using System.Collections.Concurrent;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.SDK;

namespace SaveForge.PluginHost;

public sealed class PluginRegistry : IPluginRegistry
{
    private readonly ConcurrentDictionary<string, (InstalledPlugin Plugin, PluginLoadContext? Context)> _entries = new();

    public void Register(InstalledPlugin plugin, PluginLoadContext? context = null)
    {
        _entries[plugin.Manifest.Id] = (plugin, context);
    }

    // IPluginRegistry implementation
    void IPluginRegistry.Register(InstalledPlugin plugin) => Register(plugin, null);

    public void Remove(string pluginId)
    {
        if (_entries.TryRemove(pluginId, out var entry))
        {
            entry.Context?.Unload();
        }
    }

    public IReadOnlyList<InstalledPlugin> GetAll() =>
        _entries.Values.Select(e => e.Plugin).ToList();

    public InstalledPlugin? TryGet(string pluginId) =>
        _entries.TryGetValue(pluginId, out var e) ? e.Plugin : null;

    public IReadOnlyList<IGameEditorPlugin> GetLoaded() =>
        _entries.Values
            .Where(e => e.Plugin.LoadState == PluginLoadState.Loaded && e.Plugin.Plugin is not null)
            .Select(e => e.Plugin.Plugin!)
            .ToList();
}
