using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed class FormSectionViewModel
{
    public string? Heading { get; }
    public IReadOnlyList<FieldViewModel> Fields { get; }

    public FormSectionViewModel(FormSection section, ISaveDocument document)
    {
        Heading = section.Heading;
        Fields = section.Fields.Select(f => FieldViewModelFactory.Create(f, document)).ToList();
    }
}
