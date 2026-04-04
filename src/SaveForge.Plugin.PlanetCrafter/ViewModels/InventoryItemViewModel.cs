using CommunityToolkit.Mvvm.Input;

namespace SaveForge.Plugin.PlanetCrafter.ViewModels;

internal sealed partial class InventoryItemViewModel
{
    private readonly Action<InventoryItemViewModel> _onRemove;

    public int WorldObjectId { get; }
    public string GId { get; }
    public string DisplayName { get; }

    public InventoryItemViewModel(int worldObjectId, string gId, string displayName,
        Action<InventoryItemViewModel> onRemove)
    {
        WorldObjectId = worldObjectId;
        GId = gId;
        DisplayName = displayName;
        _onRemove = onRemove;
    }

    [RelayCommand]
    private void Remove() => _onRemove(this);
}
