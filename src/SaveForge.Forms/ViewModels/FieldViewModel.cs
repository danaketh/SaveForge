using CommunityToolkit.Mvvm.ComponentModel;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public abstract partial class FieldViewModel : ObservableObject
{
    protected ISaveDocument Document { get; }

    [ObservableProperty]
    private bool _isVisible = true;

    [ObservableProperty]
    private string? _validationError;

    public FieldDefinition Definition { get; }

    protected FieldViewModel(FieldDefinition definition, ISaveDocument document)
    {
        Definition = definition;
        Document = document;
    }

    /// <summary>Re-loads the value from the document (called on initial render and after external changes).</summary>
    public abstract void RefreshFromDocument();

    /// <summary>Evaluates the visibility rule against the current document state.</summary>
    public void EvaluateVisibility(Func<string, object?> valueProvider)
    {
        if (Definition.VisibilityRule is null)
        {
            IsVisible = true;
            return;
        }
        var depValue = valueProvider(Definition.VisibilityRule.DependsOnKey);
        IsVisible = Definition.VisibilityRule.Evaluate(depValue);
    }

    protected abstract void Validate();
}
