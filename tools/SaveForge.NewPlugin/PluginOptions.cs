namespace SaveForge.NewPlugin;

internal sealed class PluginOptions
{
    /// <summary>PascalCase name used for class/namespace/assembly names, e.g. "MyGame".</summary>
    public required string Name { get; init; }

    /// <summary>Human-readable display name, e.g. "My Game Save Editor".</summary>
    public required string DisplayName { get; init; }

    /// <summary>Plugin author.</summary>
    public required string Author { get; init; }

    /// <summary>Short plugin description.</summary>
    public required string Description { get; init; }

    /// <summary>Comma-separated file extensions, e.g. ".sav,.json".</summary>
    public required string[] Extensions { get; init; }

    /// <summary>String searched in the file header to confirm this plugin owns the file.</summary>
    public required string DetectionKey { get; init; }

    /// <summary>Default save directory path (may contain %APPDATA% etc.).</summary>
    public required string SaveDirectory { get; init; }

    /// <summary>schema = schema-driven form renderer; custom = full Avalonia UserControl.</summary>
    public required EditorType EditorType { get; init; }

    // ── Derived ──────────────────────────────────────────────────────────────

    /// <summary>Kebab-case plugin ID used in the manifest, e.g. "my-game".</summary>
    public string Id => ToKebabCase(Name);

    /// <summary>Fully qualified assembly name, e.g. "SaveForge.Plugin.MyGame".</summary>
    public string Assembly => $"SaveForge.Plugin.{Name}";

    /// <summary>Relative path to the new project directory from the solution root.</summary>
    public string RelativeProjectDir => $"src/{Assembly}";

    private static string ToKebabCase(string pascal)
    {
        var sb = new System.Text.StringBuilder();
        for (int i = 0; i < pascal.Length; i++)
        {
            if (i > 0 && char.IsUpper(pascal[i]))
                sb.Append('-');
            sb.Append(char.ToLowerInvariant(pascal[i]));
        }
        return sb.ToString();
    }
}

internal enum EditorType { Schema, Custom }
