using SaveForge.SDK;

namespace SaveForge.Core.Models;

public enum PluginLoadState { Loaded, Failed, Incompatible }

public sealed class InstalledPlugin
{
    public required PluginManifest Manifest { get; init; }
    public required PluginLoadState LoadState { get; init; }
    public required string AssemblyPath { get; init; }
    public string? FailureReason { get; init; }

    /// <summary>Absolute path to Assets/icon.png, or null if none.</summary>
    public string? IconPath { get; init; }

    /// <summary>Non-null only when LoadState == Loaded.</summary>
    public IGameEditorPlugin? Plugin { get; init; }
}
