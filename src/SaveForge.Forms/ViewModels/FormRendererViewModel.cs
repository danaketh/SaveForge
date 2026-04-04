using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed partial class FormRendererViewModel : ObservableObject
{
    private readonly ISaveDocument _document;

    public IReadOnlyList<FormPageViewModel> Pages { get; }

    [ObservableProperty]
    private FormPageViewModel? _selectedPage;

    public bool HasValidationErrors => AllFields().Any(f => f.ValidationError is not null);

    public bool CanSave => _document.IsDirty && !HasValidationErrors;

    public FormRendererViewModel(FormSchema schema, ISaveDocument document)
    {
        _document = document;
        Pages = schema.Pages.Select(p => new FormPageViewModel(p, document)).ToList();
        SelectedPage = Pages.Count > 0 ? Pages[0] : null;

        // Re-evaluate CanSave whenever any field changes
        foreach (var field in AllFields())
        {
            field.PropertyChanged += (_, _) => OnPropertyChanged(nameof(CanSave));
        }
    }

    [RelayCommand(CanExecute = nameof(CanSave))]
    private async Task Save(CancellationToken ct)
    {
        await _document.SaveAsync(ct);
        OnPropertyChanged(nameof(CanSave));
    }

    private IEnumerable<FieldViewModel> AllFields() =>
        Pages.SelectMany(p => p.Sections).SelectMany(s => s.Fields);
}
