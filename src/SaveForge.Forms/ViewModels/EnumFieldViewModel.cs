using CommunityToolkit.Mvvm.ComponentModel;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed partial class EnumFieldViewModel : FieldViewModel
{
    private readonly EnumField _def;

    public IReadOnlyList<EnumOption> Options { get; }

    [ObservableProperty]
    private EnumOption? _selectedOption;

    public EnumFieldViewModel(EnumField definition, ISaveDocument document) : base(definition, document)
    {
        _def = definition;
        Options = definition.Options;
        RefreshFromDocument();
    }

    private bool _refreshing;

    partial void OnSelectedOptionChanged(EnumOption? value)
    {
        if (_refreshing)
            return;
        if (value is not null)
            Document.SetValue(_def.Key, value.Value);
        Validate();
    }

    public override void RefreshFromDocument()
    {
        var raw = Document.GetValue(_def.Key)?.ToString();
        _refreshing = true;
        SelectedOption = Options.FirstOrDefault(o => o.Value == raw) ?? (Options.Count > 0 ? Options[0] : null);
        _refreshing = false;
    }

    protected override void Validate()
    {
        ValidationError = _def.IsRequired && SelectedOption is null
            ? $"{_def.Label} is required."
            : null;
    }
}
