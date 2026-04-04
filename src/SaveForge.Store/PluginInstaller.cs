using System.IO.Compression;
using Microsoft.Extensions.Logging;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.Infrastructure.Services;
using SaveForge.PluginHost;
using SaveForge.Security;

namespace SaveForge.Store;

public sealed partial class PluginInstaller : IPluginInstaller
{
    private readonly HttpDownloadService _downloader;
    private readonly ICatalogService _catalog;
    private readonly PluginRegistry _registry;
    private readonly PluginLoader _loader;
    private readonly ILogger<PluginInstaller> _logger;
    private readonly string _pluginDirectory;

    public PluginInstaller(
        HttpDownloadService downloader,
        ICatalogService catalog,
        PluginRegistry registry,
        PluginLoader loader,
        ILogger<PluginInstaller> logger,
        string pluginDirectory)
    {
        _downloader = downloader;
        _catalog = catalog;
        _registry = registry;
        _loader = loader;
        _logger = logger;
        _pluginDirectory = pluginDirectory;
    }

    public async Task InstallAsync(string catalogEntryId, IProgress<long>? progress = null, CancellationToken ct = default)
    {
        var entries = await _catalog.GetCatalogAsync(ct: ct);
        var entry = entries.FirstOrDefault(e => e.Id == catalogEntryId)
            ?? throw new InvalidOperationException($"Plugin '{catalogEntryId}' not found in catalog.");

        var tempZip = Path.GetTempFileName() + ".zip";
        try
        {
            // 1. Download
            LogPluginDownloading(catalogEntryId, entry.PackageUrl);
            await _downloader.DownloadToFileAsync(entry.PackageUrl, tempZip, progress, ct);

            // 2. Verify SHA-256
            if (!await HashVerifier.VerifyAsync(tempZip, entry.Sha256, ct))
            {
                throw new InvalidOperationException($"SHA-256 verification failed for plugin '{catalogEntryId}'.");
            }

            // 3. Unpack
            var targetDir = Path.Combine(_pluginDirectory, entry.Id, entry.LatestVersion);
            Directory.CreateDirectory(targetDir);
            ZipFile.ExtractToDirectory(tempZip, targetDir, overwriteFiles: true);

            // 4. Load
            var (result, context) = _loader.Load(targetDir);

            InstalledPlugin installed;
            switch (result)
            {
                case PluginLoadResult.Succeeded s:
                    installed = new InstalledPlugin
                    {
                        Manifest = s.Plugin.Manifest,
                        LoadState = PluginLoadState.Loaded,
                        AssemblyPath = targetDir,
                        Plugin = s.Plugin
                    };
                    break;

                case PluginLoadResult.Incompatible i:
                    Directory.Delete(targetDir, recursive: true);
                    throw new InvalidOperationException($"Plugin incompatible: {i.Reason}");

                default:
                    var reason = result is PluginLoadResult.Failed f ? f.Reason : "Unknown";
                    Directory.Delete(targetDir, recursive: true);
                    throw new InvalidOperationException($"Plugin failed to load: {reason}");
            }

            _registry.Register(installed, context);
            LogPluginInstalled(catalogEntryId, entry.LatestVersion);
        }
        finally
        {
            if (File.Exists(tempZip))
            {
                File.Delete(tempZip);
            }
        }
    }

    public async Task RemoveAsync(string pluginId, CancellationToken ct = default)
    {
        var existing = _registry.TryGet(pluginId);
        if (existing is null)
        {
            return;
        }

        _registry.Remove(pluginId);

        var pluginIdDir = Path.Combine(_pluginDirectory, pluginId);
        if (Directory.Exists(pluginIdDir))
        {
            Directory.Delete(pluginIdDir, recursive: true);
        }

        LogPluginRemoved(pluginId);
        await Task.CompletedTask;
    }

    [LoggerMessage(Level = LogLevel.Information, Message = "Downloading plugin {Id} from {Url}")]
    private partial void LogPluginDownloading(string id, string url);

    [LoggerMessage(Level = LogLevel.Information, Message = "Installed plugin {Id} v{Ver}")]
    private partial void LogPluginInstalled(string id, string ver);

    [LoggerMessage(Level = LogLevel.Information, Message = "Removed plugin {Id}")]
    private partial void LogPluginRemoved(string id);

    public async Task UpdateAsync(string pluginId, IProgress<long>? progress = null, CancellationToken ct = default)
    {
        var oldPlugin = _registry.TryGet(pluginId);
        var oldVersion = oldPlugin?.Manifest.Version;

        await InstallAsync(pluginId, progress, ct);

        // Remove old version directory if different from new
        if (oldVersion is not null)
        {
            var oldVersionDir = Path.Combine(_pluginDirectory, pluginId, oldVersion);
            var newPlugin = _registry.TryGet(pluginId);
            if (newPlugin is not null && oldVersion != newPlugin.Manifest.Version &&
                Directory.Exists(oldVersionDir))
            {
                Directory.Delete(oldVersionDir, recursive: true);
            }
        }
    }
}
