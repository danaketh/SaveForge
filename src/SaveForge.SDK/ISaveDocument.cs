namespace SaveForge.SDK;

/// <summary>Represents an open save file. Values are accessed via dot-path keys.</summary>
public interface ISaveDocument
{
    /// <summary>Absolute path to the file on disk.</summary>
    string FilePath { get; }

    /// <summary>True if any value has been modified since the last save.</summary>
    bool IsDirty { get; }

    /// <summary>Gets a value by dot-path (e.g. "player.stats.health").</summary>
    object? GetValue(string dotPath);

    /// <summary>Sets a value by dot-path and marks the document dirty.</summary>
    void SetValue(string dotPath, object? value);

    /// <summary>Returns the editor definition (schema or custom view factory).</summary>
    IEditorDefinition GetEditorDefinition();

    /// <summary>Saves the document back to disk. The caller is responsible for backup.</summary>
    Task SaveAsync(CancellationToken ct = default);
}
