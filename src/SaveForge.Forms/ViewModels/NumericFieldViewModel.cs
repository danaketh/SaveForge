using CommunityToolkit.Mvvm.ComponentModel;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed partial class NumericFieldViewModel : FieldViewModel
{
    private readonly NumericField _def;

    public int DecimalPlaces => _def.DecimalPlaces;

    [ObservableProperty]
    private double _value;

    public NumericFieldViewModel(NumericField definition, ISaveDocument document) : base(definition, document)
    {
        _def = definition;
        RefreshFromDocument();
    }

    private bool _refreshing;

    partial void OnValueChanged(double value)
    {
        if (_refreshing)
            return;
        var stored = _def.DecimalPlaces == 0 ? (object)(int)value : value;
        Document.SetValue(_def.Key, stored);
        Validate();
    }

    public override void RefreshFromDocument()
    {
        var raw = Document.GetValue(_def.Key);
        _refreshing = true;
        Value = raw is null ? 0 : Convert.ToDouble(raw, System.Globalization.CultureInfo.InvariantCulture);
        _refreshing = false;
    }

    protected override void Validate()
    {
        if (_def.Min.HasValue && Value < _def.Min.Value)
        {
            ValidationError = $"{_def.Label} must be at least {_def.Min}.";
            return;
        }
        if (_def.Max.HasValue && Value > _def.Max.Value)
        {
            ValidationError = $"{_def.Label} must be at most {_def.Max}.";
            return;
        }
        ValidationError = null;
    }
}
