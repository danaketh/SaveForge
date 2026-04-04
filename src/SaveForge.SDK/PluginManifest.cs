namespace SaveForge.SDK;

/// <summary>Static metadata embedded in every plugin's manifest.json.</summary>
public sealed class PluginManifest
{
    /// <summary>Unique identifier (lowercase alphanumeric + hyphens, e.g. "stardew-like").</summary>
    public required string Id { get; init; }

    public required string Name { get; init; }

    /// <summary>SemVer string, e.g. "1.0.0".</summary>
    public required string Version { get; init; }

    public required string Author { get; init; }

    public string Description { get; init; } = string.Empty;

    /// <summary>Minimum host version this plugin is compatible with (SemVer string).</summary>
    public required string MinHostVersion { get; init; }

    /// <summary>Maximum host version, or null for no upper bound.</summary>
    public string? MaxHostVersion { get; init; }

    /// <summary>Filename of the plugin DLL relative to the plugin directory.</summary>
    public required string PluginAssembly { get; init; }
}
