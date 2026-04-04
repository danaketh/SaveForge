using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public static class FieldViewModelFactory
{
    public static FieldViewModel Create(FieldDefinition definition, ISaveDocument document) =>
        definition switch
        {
            TextField tf => new TextFieldViewModel(tf, document),
            NumericField nf => new NumericFieldViewModel(nf, document),
            CheckboxField cf => new CheckboxFieldViewModel(cf, document),
            EnumField ef => new EnumFieldViewModel(ef, document),
            ReadOnlyField rf => new ReadOnlyFieldViewModel(rf, document),
            ListField lf => new ListFieldViewModel(lf, document),
            ObjectSectionField osf => new ObjectSectionViewModel(osf, document),
            _ => throw new NotSupportedException($"Unsupported field type: {definition.GetType().Name}")
        };
}
