using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;

namespace SaveForge.App.ViewModels;

public sealed partial class StoreViewModel : ObservableObject
{
    private readonly ICatalogService _catalog;
    private readonly IPluginInstaller _installer;
    private IReadOnlyList<CatalogEntry> _allEntries = Array.Empty<CatalogEntry>();

    public ObservableCollection<CatalogEntry> DisplayedEntries { get; } = [];

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(DisplayedEntries))]
    private string _searchText = string.Empty;

    [ObservableProperty]
    private bool _isLoading;

    [ObservableProperty]
    private string? _errorMessage;

    public StoreViewModel(ICatalogService catalog, IPluginInstaller installer)
    {
        _catalog = catalog;
        _installer = installer;
    }

    partial void OnSearchTextChanged(string value) => ApplyFilter();

    [RelayCommand]
    public async Task LoadAsync(CancellationToken ct)
    {
        IsLoading = true;
        ErrorMessage = null;
        try
        {
            _allEntries = await _catalog.GetCatalogAsync(ct: ct);
            ApplyFilter();
        }
        catch (Exception ex)
        {
            ErrorMessage = $"Failed to load catalog: {ex.Message}";
        }
        finally
        {
            IsLoading = false;
        }
    }

    [RelayCommand]
    public async Task InstallAsync(CatalogEntry entry, CancellationToken ct)
    {
        try
        {
            await _installer.InstallAsync(entry.Id, ct: ct);
            await LoadAsync(ct);
        }
        catch (Exception ex)
        {
            ErrorMessage = $"Install failed: {ex.Message}";
        }
    }

    private void ApplyFilter()
    {
        DisplayedEntries.Clear();
        var filtered = string.IsNullOrWhiteSpace(SearchText)
            ? _allEntries
            : _allEntries.Where(e =>
                e.Name.Contains(SearchText, StringComparison.OrdinalIgnoreCase) ||
                e.Tags.Any(t => t.Contains(SearchText, StringComparison.OrdinalIgnoreCase)));
        foreach (var e in filtered)
        {
            DisplayedEntries.Add(e);
        }
    }
}
