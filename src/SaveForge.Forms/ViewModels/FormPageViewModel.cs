using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed class FormPageViewModel
{
    public string Title { get; }
    public IReadOnlyList<FormSectionViewModel> Sections { get; }

    public FormPageViewModel(FormPage page, ISaveDocument document)
    {
        Title = page.Title;
        Sections = page.Sections.Select(s => new FormSectionViewModel(s, document)).ToList();
    }
}
