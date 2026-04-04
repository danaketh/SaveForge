using SaveForge.SDK.Schema;

namespace SaveForge.SDK;

/// <summary>Discriminates between a schema-driven and a custom plugin UI.</summary>
public enum EditorKind { Schema, Custom }

/// <summary>Returned by ISaveDocument.GetEditorDefinition() to tell the host how to render the editor.</summary>
public interface IEditorDefinition
{
    EditorKind Kind { get; }

    /// <summary>Non-null when Kind == Schema.</summary>
    FormSchema? Schema { get; }

    /// <summary>Non-null when Kind == Custom.</summary>
    ICustomEditorViewFactory? ViewFactory { get; }
}
