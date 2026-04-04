using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.Plugin.PlanetCrafter.Models;

namespace SaveForge.Plugin.PlanetCrafter.ViewModels;

internal sealed partial class PlayerViewModel : ObservableObject
{
    private readonly PlayerData _data;
    private readonly PlanetCrafterSave _save;
    private readonly LocalizationService _localization;
    private readonly Action _markDirty;

    public string DisplayName => _data.Host ? $"{_data.Name} (Host)" : _data.Name;

    [ObservableProperty] private double _health;
    [ObservableProperty] private double _oxygen;
    [ObservableProperty] private double _thirst;
    [ObservableProperty] private double _toxic;

    public ObservableCollection<InventoryItemViewModel> InventoryItems { get; } = [];
    public ObservableCollection<InventoryItemViewModel> EquipmentItems { get; } = [];

    [ObservableProperty] private KnownItem? _selectedNewInventoryItem;
    [ObservableProperty] private KnownItem? _selectedNewEquipmentItem;

    public IReadOnlyList<KnownItem> AllKnownItems { get; }

    public PlayerViewModel(PlayerData data, PlanetCrafterSave save,
        LocalizationService localization, Action markDirty)
    {
        _data = data;
        _save = save;
        _localization = localization;
        _markDirty = markDirty;
        AllKnownItems = localization.AllItems;

        _health = data.PlayerGaugeHealth;
        _oxygen = data.PlayerGaugeOxygen;
        _thirst = data.PlayerGaugeThirst;
        _toxic = data.PlayerGaugeToxic;

        LoadItems(data.InventoryId, InventoryItems, item => RemoveFromInventory(item, data.InventoryId));
        LoadItems(data.EquipmentId, EquipmentItems, item => RemoveFromEquipment(item, data.EquipmentId));
    }

    partial void OnHealthChanged(double value) { _data.PlayerGaugeHealth = value; _markDirty(); }
    partial void OnOxygenChanged(double value) { _data.PlayerGaugeOxygen = value; _markDirty(); }
    partial void OnThirstChanged(double value) { _data.PlayerGaugeThirst = value; _markDirty(); }
    partial void OnToxicChanged(double value) { _data.PlayerGaugeToxic = value; _markDirty(); }

    [RelayCommand]
    private void AddInventoryItem()
    {
        if (SelectedNewInventoryItem is null)
        {
            return;
        }

        AddItem(SelectedNewInventoryItem, _data.InventoryId, InventoryItems,
            item => RemoveFromInventory(item, _data.InventoryId));
        SelectedNewInventoryItem = null;
    }

    [RelayCommand]
    private void AddEquipmentItem()
    {
        if (SelectedNewEquipmentItem is null)
        {
            return;
        }

        AddItem(SelectedNewEquipmentItem, _data.EquipmentId, EquipmentItems,
            item => RemoveFromEquipment(item, _data.EquipmentId));
        SelectedNewEquipmentItem = null;
    }

    // ── private helpers ──────────────────────────────────────────────────────

    private void LoadItems(int inventoryId, ObservableCollection<InventoryItemViewModel> target,
        Action<InventoryItemViewModel> onRemove)
    {
        var inv = FindInventory(inventoryId);
        if (inv is null)
        {
            return;
        }

        foreach (var woId in ParseWoIds(inv.WoIds))
        {
            var wo = _save.WorldObjects.Find(w => w.Id == woId);
            if (wo is null)
            {
                continue;
            }

            target.Add(new InventoryItemViewModel(wo.Id, wo.GId,
                _localization.GetItemName(wo.GId), onRemove));
        }
    }

    private void AddItem(KnownItem item, int inventoryId,
        ObservableCollection<InventoryItemViewModel> target,
        Action<InventoryItemViewModel> onRemove)
    {
        var inv = FindInventory(inventoryId);
        if (inv is null)
        {
            return;
        }

        var newId = _save.WorldObjects.Count > 0
            ? _save.WorldObjects.Max(w => w.Id) + 1
            : 1;

        var wo = new WorldObject { Id = newId, GId = item.GId };
        _save.WorldObjects.Add(wo);

        var ids = ParseWoIds(inv.WoIds);
        ids.Add(newId);
        inv.WoIds = SerializeWoIds(ids);

        target.Add(new InventoryItemViewModel(newId, item.GId, item.DisplayName, onRemove));
        _markDirty();
    }

    private void RemoveFromInventory(InventoryItemViewModel item, int inventoryId)
        => RemoveItem(item, inventoryId, InventoryItems);

    private void RemoveFromEquipment(InventoryItemViewModel item, int inventoryId)
        => RemoveItem(item, inventoryId, EquipmentItems);

    private void RemoveItem(InventoryItemViewModel item, int inventoryId,
        ObservableCollection<InventoryItemViewModel> from)
    {
        var inv = FindInventory(inventoryId);
        if (inv is not null)
        {
            var ids = ParseWoIds(inv.WoIds);
            ids.Remove(item.WorldObjectId);
            inv.WoIds = SerializeWoIds(ids);
        }
        from.Remove(item);
        _markDirty();
    }

    private InventoryData? FindInventory(int id) =>
        Array.Find(_save.Inventories, inv => inv.Id == id);

    private static List<int> ParseWoIds(string woIds)
    {
        if (string.IsNullOrWhiteSpace(woIds))
        {
            return [];
        }

        return woIds
            .Split([',', '|'], StringSplitOptions.RemoveEmptyEntries)
            .Select(s => int.TryParse(s.Trim(), out var id) ? id : -1)
            .Where(id => id >= 0)
            .ToList();
    }

    private static string SerializeWoIds(List<int> ids) => string.Join(",", ids);
}
