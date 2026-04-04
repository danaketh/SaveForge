using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.Plugin.PlanetCrafter.Models;

namespace SaveForge.Plugin.PlanetCrafter.ViewModels;

internal sealed partial class PlanetCrafterEditorViewModel : ObservableObject
{
    private readonly PlanetCrafterDocument _document;
    private readonly PlanetCrafterSave _save;

    // ── Characters ───────────────────────────────────────────────────────────
    public ObservableCollection<PlayerViewModel> Players { get; }
    [ObservableProperty] private PlayerViewModel? _selectedPlayer;

    // ── Terra Tokens ─────────────────────────────────────────────────────────
    [ObservableProperty] private double _terraTokens;
    [ObservableProperty] private double _allTimeTerraTokens;
    [ObservableProperty] private string _unlockedGroups = "";

    // ── Planet State ─────────────────────────────────────────────────────────
    [ObservableProperty] private double _oxygenLevel;
    [ObservableProperty] private double _heatLevel;
    [ObservableProperty] private double _pressureLevel;
    [ObservableProperty] private double _plantsLevel;
    [ObservableProperty] private double _insectsLevel;
    [ObservableProperty] private double _animalsLevel;

    // ── Save Info ────────────────────────────────────────────────────────────
    [ObservableProperty] private string _saveDisplayName = "";
    [ObservableProperty] private string _planetId = "";
    [ObservableProperty] private int _craftedObjects;

    // ── Status ───────────────────────────────────────────────────────────────
    [ObservableProperty] private string _saveStatus = "";

    public PlanetCrafterEditorViewModel(PlanetCrafterDocument document, LocalizationService localization)
    {
        _document = document;
        _save = document.GetSave();

        Players = new ObservableCollection<PlayerViewModel>(
            _save.Players.Select(p =>
                new PlayerViewModel(p, _save, localization, document.MarkDirty)));

        SelectedPlayer = Players.FirstOrDefault();

        // Terra Tokens
        _terraTokens = (double)_save.TerraTokens.TerraTokens;
        _allTimeTerraTokens = (double)_save.TerraTokens.AllTimeTerraTokens;
        _unlockedGroups = _save.TerraTokens.UnlockedGroups ?? "";

        // Planet
        _oxygenLevel = _save.Planet.UnitOxygenLevel;
        _heatLevel = _save.Planet.UnitHeatLevel;
        _pressureLevel = _save.Planet.UnitPressureLevel;
        _plantsLevel = _save.Planet.UnitPlantsLevel;
        _insectsLevel = _save.Planet.UnitInsectsLevel;
        _animalsLevel = _save.Planet.UnitAnimalsLevel;

        // Save Info
        _saveDisplayName = _save.DisplayInfo.SaveDisplayName ?? "";
        _planetId = _save.DisplayInfo.PlanetId ?? "";
        _craftedObjects = _save.Stats.CraftedObjects;
    }

    // ── Property → model sync ────────────────────────────────────────────────

    partial void OnTerraTokensChanged(double value)
    {
        _save.TerraTokens.TerraTokens = (long)value;
        _document.MarkDirty();
    }

    partial void OnOxygenLevelChanged(double value) { _save.Planet.UnitOxygenLevel = value; _document.MarkDirty(); }
    partial void OnHeatLevelChanged(double value) { _save.Planet.UnitHeatLevel = value; _document.MarkDirty(); }
    partial void OnPressureLevelChanged(double value) { _save.Planet.UnitPressureLevel = value; _document.MarkDirty(); }
    partial void OnPlantsLevelChanged(double value) { _save.Planet.UnitPlantsLevel = value; _document.MarkDirty(); }
    partial void OnInsectsLevelChanged(double value) { _save.Planet.UnitInsectsLevel = value; _document.MarkDirty(); }
    partial void OnAnimalsLevelChanged(double value) { _save.Planet.UnitAnimalsLevel = value; _document.MarkDirty(); }

    partial void OnSaveDisplayNameChanged(string value)
    {
        _save.DisplayInfo.SaveDisplayName = value;
        _document.MarkDirty();
    }

    // ── Save command ─────────────────────────────────────────────────────────

    [RelayCommand]
    private async Task SaveAsync(CancellationToken ct)
    {
        try
        {
            var timestamp = DateTimeOffset.UtcNow.ToString("yyyy-MM-ddTHH-mm-ss");
            var backupDir = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
                "SaveForge", "backups", "editor");
            Directory.CreateDirectory(backupDir);
            var backupPath = Path.Combine(backupDir,
                $"{Path.GetFileName(_document.FilePath)}.{timestamp}.bak");
            File.Copy(_document.FilePath, backupPath, overwrite: false);

            await _document.SaveAsync(ct);
            SaveStatus = $"Saved. Backup: {backupPath}";
        }
        catch (Exception ex)
        {
            SaveStatus = $"Save failed: {ex.Message}";
        }
    }
}
