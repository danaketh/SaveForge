using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed class ObjectSectionViewModel : FieldViewModel
{
    public IReadOnlyList<FieldViewModel> Fields { get; }

    public ObjectSectionViewModel(ObjectSectionField definition, ISaveDocument document) : base(definition, document)
    {
        Fields = definition.Fields.Select(f => FieldViewModelFactory.Create(f, document)).ToList();
        RefreshFromDocument();
    }

    public override void RefreshFromDocument()
    {
        foreach (var field in Fields)
        {
            field.RefreshFromDocument();
        }
    }

    protected override void Validate() => ValidationError = null;
}
