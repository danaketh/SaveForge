using System.Text.Json;
using Microsoft.Extensions.Logging;
using SaveForge.Core.Models;
using SaveForge.SDK;

namespace SaveForge.PluginHost;

public sealed class PluginLoader
{
    private readonly ILogger<PluginLoader> _logger;
    private readonly Version _hostVersion;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true
    };

    public PluginLoader(ILogger<PluginLoader> logger, string hostVersion)
    {
        _logger = logger;
        _hostVersion = Version.Parse(hostVersion);
    }

    /// <summary>
    /// Loads a plugin from a directory that contains manifest.json and the plugin assembly.
    /// Returns a PluginLoadResult.Succeeded, .Failed, or .Incompatible.
    /// </summary>
    public (PluginLoadResult Result, PluginLoadContext? Context) Load(string pluginDirectory)
    {
        var manifestPath = Path.Combine(pluginDirectory, "manifest.json");
        if (!File.Exists(manifestPath))
        {
            return (new PluginLoadResult.Failed { Reason = "manifest.json not found." }, null);
        }

        PluginManifest manifest;
        try
        {
            string json = File.ReadAllText(manifestPath);
            manifest = JsonSerializer.Deserialize<PluginManifest>(json, JsonOptions)
                       ?? throw new InvalidOperationException("Deserialized manifest was null.");
        }
        catch (Exception ex)
        {
            return (new PluginLoadResult.Failed { Reason = $"Failed to parse manifest: {ex.Message}" }, null);
        }

        // Host compatibility check
        var minVersion = Version.Parse(manifest.MinHostVersion);
        if (_hostVersion < minVersion)
        {
            return (new PluginLoadResult.Incompatible { Reason = $"Requires host >= {manifest.MinHostVersion}" }, null);
        }

        if (manifest.MaxHostVersion is not null)
        {
            var maxVersion = Version.Parse(manifest.MaxHostVersion);
            if (_hostVersion > maxVersion)
            {
                return (new PluginLoadResult.Incompatible { Reason = $"Requires host <= {manifest.MaxHostVersion}" }, null);
            }
        }

        var assemblyPath = Path.Combine(pluginDirectory, manifest.PluginAssembly);
        if (!File.Exists(assemblyPath))
        {
            return (new PluginLoadResult.Failed { Reason = $"Assembly not found: {manifest.PluginAssembly}" }, null);
        }

        try
        {
            var context = new PluginLoadContext(assemblyPath);
            var assembly = context.LoadFromAssemblyPath(assemblyPath);

            var pluginType = assembly.GetTypes().FirstOrDefault(t =>
                t.IsClass && !t.IsAbstract &&
                typeof(IGameEditorPlugin).IsAssignableFrom(t));

            if (pluginType is null)
            {
                return (new PluginLoadResult.Failed { Reason = "No IGameEditorPlugin implementation found in assembly." }, null);
            }

            var plugin = (IGameEditorPlugin)Activator.CreateInstance(pluginType)!;
            return (new PluginLoadResult.Succeeded { Plugin = plugin }, context);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to load plugin from {Dir}", pluginDirectory);
            return (new PluginLoadResult.Failed { Reason = $"Assembly load error: {ex.Message}" }, null);
        }
    }
}
