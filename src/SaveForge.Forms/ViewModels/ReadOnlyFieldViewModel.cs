using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed class ReadOnlyFieldViewModel : FieldViewModel
{
    public string DisplayValue { get; private set; } = string.Empty;

    public ReadOnlyFieldViewModel(ReadOnlyField definition, ISaveDocument document) : base(definition, document)
    {
        RefreshFromDocument();
    }

    public override void RefreshFromDocument()
    {
        DisplayValue = Document.GetValue(Definition.Key)?.ToString() ?? string.Empty;
        OnPropertyChanged(nameof(DisplayValue));
    }

    protected override void Validate() => ValidationError = null;
}
