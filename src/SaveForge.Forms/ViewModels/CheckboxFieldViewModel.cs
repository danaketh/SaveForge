using CommunityToolkit.Mvvm.ComponentModel;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed partial class CheckboxFieldViewModel : FieldViewModel
{
    private readonly CheckboxField _def;

    [ObservableProperty]
    private bool _value;

    public CheckboxFieldViewModel(CheckboxField definition, ISaveDocument document) : base(definition, document)
    {
        _def = definition;
        RefreshFromDocument();
    }

    private bool _refreshing;

    partial void OnValueChanged(bool value)
    {
        if (_refreshing)
            return;
        Document.SetValue(_def.Key, value);
        Validate();
    }

    public override void RefreshFromDocument()
    {
        var raw = Document.GetValue(_def.Key);
        _refreshing = true;
        Value = raw is bool b ? b : false;
        _refreshing = false;
    }

    protected override void Validate() => ValidationError = null;
}
