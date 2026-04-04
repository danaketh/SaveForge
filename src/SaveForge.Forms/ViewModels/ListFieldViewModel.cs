using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed partial class ListFieldViewModel : FieldViewModel
{
    private readonly ListField _def;

    public ObservableCollection<FieldViewModel> Items { get; } = [];

    public ListFieldViewModel(ListField definition, ISaveDocument document) : base(definition, document)
    {
        _def = definition;
        RefreshFromDocument();
    }

    public override void RefreshFromDocument()
    {
        // List items are read from the document lazily; complex binding handled by renderer
        Validate();
    }

    [RelayCommand]
    private void AddItem()
    {
        // Creates an empty item VM — full implementation depends on item schema type
        var itemVm = FieldViewModelFactory.Create(_def.ItemSchema, Document);
        Items.Add(itemVm);
    }

    [RelayCommand]
    private void RemoveItem(FieldViewModel item)
    {
        Items.Remove(item);
    }

    protected override void Validate() => ValidationError = null;
}
