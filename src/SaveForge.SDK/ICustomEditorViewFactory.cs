using Avalonia.Controls;

namespace SaveForge.SDK;

/// <summary>
/// Escape hatch for advanced plugins that need a fully custom Avalonia UI.
/// The host calls CreateView and CreateViewModel and wires them together.
/// </summary>
public interface ICustomEditorViewFactory
{
    /// <summary>Creates the Avalonia view control for this document.</summary>
    Control CreateView(ISaveDocument document);

    /// <summary>Creates the view model that the view should be bound to.</summary>
    object CreateViewModel(ISaveDocument document);
}
