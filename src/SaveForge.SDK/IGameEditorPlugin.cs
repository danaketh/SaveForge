namespace SaveForge.SDK;

/// <summary>The entry point for a game editor plugin. Implement this interface in your plugin assembly.</summary>
public interface IGameEditorPlugin
{
    /// <summary>Static metadata about this plugin.</summary>
    PluginManifest Manifest { get; }

    /// <summary>File-matching capabilities of this plugin.</summary>
    SaveCapabilities Capabilities { get; }

    /// <summary>
    /// Returns whether this plugin can open the specified file.
    /// </summary>
    /// <param name="file">File metadata (name, extension, size).</param>
    /// <param name="header">First 512 bytes of the file for lightweight content sniffing.</param>
    /// <returns>Whether this plugin can open the file, and a priority score (higher = preferred when multiple plugins match).</returns>
    (bool CanOpen, int Priority) CanOpen(FileInfo file, ReadOnlySpan<byte> header);

    /// <summary>Opens a save file and returns a document handle.</summary>
    Task<ISaveDocument> OpenAsync(string filePath, CancellationToken ct = default);
}
