using Microsoft.Extensions.Logging;
using SaveForge.Core.Models;
using SaveForge.SDK;

namespace SaveForge.PluginHost;

/// <summary>
/// Scans the plugin directory on startup and loads all valid plugins into the registry.
/// </summary>
public sealed partial class PluginHostService
{
    private readonly PluginLoader _loader;
    private readonly PluginRegistry _registry;
    private readonly ILogger<PluginHostService> _logger;

    public PluginHostService(PluginLoader loader, PluginRegistry registry, ILogger<PluginHostService> logger)
    {
        _loader = loader;
        _registry = registry;
        _logger = logger;
    }

    public void LoadAll(string pluginDirectory)
    {
        if (!Directory.Exists(pluginDirectory))
        {
            LogPluginDirMissing(pluginDirectory);
            return;
        }

        // Each plugin lives at: {pluginDirectory}/{id}/{version}/
        foreach (var idDir in Directory.EnumerateDirectories(pluginDirectory))
        {
            var versionDirs = Directory.EnumerateDirectories(idDir)
                .OrderByDescending(d => d) // use latest version if multiple
                .Take(1);

            foreach (var versionDir in versionDirs)
            {
                var manifestPath = Path.Combine(versionDir, "manifest.json");
                if (!File.Exists(manifestPath))
                {
                    continue;
                }

                var (result, context) = _loader.Load(versionDir);

                var iconPath = Path.Combine(versionDir, "Assets", "icon.png");
                var resolvedIconPath = File.Exists(iconPath) ? iconPath : null;

                InstalledPlugin installed;
                switch (result)
                {
                    case PluginLoadResult.Succeeded s:
                        installed = new InstalledPlugin
                        {
                            Manifest = s.Plugin.Manifest,
                            LoadState = PluginLoadState.Loaded,
                            AssemblyPath = versionDir,
                            IconPath = resolvedIconPath,
                            Plugin = s.Plugin
                        };
                        LogPluginLoaded(s.Plugin.Manifest.Id, s.Plugin.Manifest.Version);
                        break;

                    case PluginLoadResult.Incompatible i:
                        var mIncompat = TryReadManifestId(manifestPath);
                        installed = new InstalledPlugin
                        {
                            Manifest = mIncompat,
                            LoadState = PluginLoadState.Incompatible,
                            AssemblyPath = versionDir,
                            IconPath = resolvedIconPath,
                            FailureReason = i.Reason
                        };
                        _logger.LogWarning("Plugin at {Dir} is incompatible: {Reason}", versionDir, i.Reason);
                        break;

                    default:
                        var mFailed = TryReadManifestId(manifestPath);
                        var reason = result is PluginLoadResult.Failed f ? f.Reason : "Unknown";
                        installed = new InstalledPlugin
                        {
                            Manifest = mFailed,
                            LoadState = PluginLoadState.Failed,
                            AssemblyPath = versionDir,
                            IconPath = resolvedIconPath,
                            FailureReason = reason
                        };
                        _logger.LogError("Plugin at {Dir} failed to load: {Reason}", versionDir, reason);
                        break;
                }

                _registry.Register(installed, context);
            }
        }
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Plugin directory {Dir} does not exist; no plugins loaded.")]
    private partial void LogPluginDirMissing(string dir);

    [LoggerMessage(Level = LogLevel.Information, Message = "Loaded plugin {Id} v{Ver}")]
    private partial void LogPluginLoaded(string id, string ver);

    private static PluginManifest TryReadManifestId(string manifestPath)
    {
        try
        {
            var json = File.ReadAllText(manifestPath);
            using var doc = System.Text.Json.JsonDocument.Parse(json);
            var id = doc.RootElement.TryGetProperty("id", out var idEl) ? idEl.GetString() ?? "unknown" : "unknown";
            var name = doc.RootElement.TryGetProperty("name", out var nameEl) ? nameEl.GetString() ?? id : id;
            return new PluginManifest { Id = id, Name = name, Version = "0.0.0", Author = "?", MinHostVersion = "0.0.0", PluginAssembly = "" };
        }
        catch
        {
            return new PluginManifest { Id = "unknown", Name = "Unknown", Version = "0.0.0", Author = "?", MinHostVersion = "0.0.0", PluginAssembly = "" };
        }
    }
}
