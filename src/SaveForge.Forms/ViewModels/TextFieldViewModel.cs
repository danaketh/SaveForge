using System.Text.RegularExpressions;
using CommunityToolkit.Mvvm.ComponentModel;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed partial class TextFieldViewModel : FieldViewModel
{
    private readonly TextField _def;

    [ObservableProperty]
    private string _value = string.Empty;

    public TextFieldViewModel(TextField definition, ISaveDocument document) : base(definition, document)
    {
        _def = definition;
        RefreshFromDocument();
    }

    private bool _refreshing;

    partial void OnValueChanged(string value)
    {
        if (_refreshing)
            return;
        Document.SetValue(_def.Key, value);
        Validate();
    }

    public override void RefreshFromDocument()
    {
        _refreshing = true;
        Value = Document.GetValue(_def.Key)?.ToString() ?? string.Empty;
        _refreshing = false;
    }

    protected override void Validate()
    {
        if (_def.IsRequired && string.IsNullOrWhiteSpace(Value))
        {
            ValidationError = $"{_def.Label} is required.";
            return;
        }
        if (_def.MaxLength.HasValue && Value.Length > _def.MaxLength)
        {
            ValidationError = $"{_def.Label} must not exceed {_def.MaxLength} characters.";
            return;
        }
        if (_def.RegexPattern is not null && !string.IsNullOrEmpty(Value) &&
            !Regex.IsMatch(Value, _def.RegexPattern))
        {
            ValidationError = $"{_def.Label} has an invalid format.";
            return;
        }
        ValidationError = null;
    }
}
