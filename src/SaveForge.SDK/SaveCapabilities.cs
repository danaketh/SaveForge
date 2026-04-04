namespace SaveForge.SDK;

/// <summary>Describes how a plugin detects which files it can open.</summary>
public sealed class SaveCapabilities
{
    /// <summary>File extensions this plugin handles, e.g. [".json", ".sav"].</summary>
    public string[] FileExtensions { get; init; } = Array.Empty<string>();

    /// <summary>Optional MIME type hints.</summary>
    public string[] MimeHints { get; init; } = Array.Empty<string>();

    /// <summary>Higher value wins when multiple plugins match the same file. Default 0.</summary>
    public int DetectionPriority { get; init; }

    /// <summary>
    /// Default directory where this game stores saves, using env-var tokens.
    /// Supports %APPDATA%, %LOCALAPPDATA%, %USERPROFILE%, %TEMP%, and leading ~.
    /// Example: @"%APPDATA%\MijuGames\The Planet Crafter\Saves"
    /// Null if the plugin has no known default location.
    /// </summary>
    public string? DefaultSaveDirectory { get; init; }
}
