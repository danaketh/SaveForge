using System.Collections.ObjectModel;
using Avalonia.Media.Imaging;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.Infrastructure.Services;

namespace SaveForge.App.ViewModels;

public sealed partial class HomeViewModel : ObservableObject
{
    private readonly IPluginRegistry _registry;
    private readonly ISaveDiscoveryService _discovery;
    private readonly AppSettings _settings;
    private readonly SettingsService _settingsService;
    private readonly Action<string> _navigateToStore;
    private readonly Func<string, Task<EditorViewModel?>> _openEditor;

    // --- collections ---
    public ObservableCollection<PluginListItem> PluginItems { get; } = [];
    public ObservableCollection<SaveFileInfo> Saves { get; } = [];

    // --- observable state ---

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsShowingConfigurePrompt))]
    [NotifyPropertyChangedFor(nameof(IsShowingNoSelection))]
    [NotifyPropertyChangedFor(nameof(IsShowingSaveList))]
    [NotifyPropertyChangedFor(nameof(SelectedPluginIcon))]
    [NotifyPropertyChangedFor(nameof(SelectedPluginHasIcon))]
    private InstalledPlugin? _selectedPlugin;

    public Bitmap? SelectedPluginIcon =>
        PluginItems.FirstOrDefault(i => i.Plugin == SelectedPlugin)?.Icon;

    public bool SelectedPluginHasIcon => SelectedPluginIcon is not null;

    [ObservableProperty]
    private string? _saveDirectory;

    [ObservableProperty]
    private string? _resolvedSaveDirectory;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsShowingConfigurePrompt))]
    [NotifyPropertyChangedFor(nameof(IsShowingSaveList))]
    private bool _isPathConfigured;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsShowingConfigurePrompt))]
    [NotifyPropertyChangedFor(nameof(IsShowingSaveList))]
    [NotifyPropertyChangedFor(nameof(IsShowingEditPath))]
    private bool _isEditingPath;

    [ObservableProperty]
    private string? _editingPath;

    [ObservableProperty]
    private string? _editingPathResolved;

    [ObservableProperty]
    private bool _isLoadingSaves;

    [ObservableProperty]
    private bool _hasSaves;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsShowingNoSelection))]
    private bool _isEmpty;

    // --- computed visibility ---
    public bool IsShowingConfigurePrompt => SelectedPlugin is not null && !IsPathConfigured && !IsEditingPath;
    public bool IsShowingSaveList => IsPathConfigured && !IsEditingPath;
    public bool IsShowingEditPath => IsEditingPath;
    public bool IsShowingNoSelection => SelectedPlugin is null && !IsEmpty;

    /// <summary>Set by HomeView.axaml.cs. Opens a native folder-picker and returns the path, or null.</summary>
    public Func<Task<string?>>? RequestFolderPath { get; set; }

    public HomeViewModel(
        IPluginRegistry registry,
        ISaveDiscoveryService discovery,
        AppSettings settings,
        SettingsService settingsService,
        Action<string> navigateToStore,
        Func<string, Task<EditorViewModel?>> openEditor)
    {
        _registry = registry;
        _discovery = discovery;
        _settings = settings;
        _settingsService = settingsService;
        _navigateToStore = navigateToStore;
        _openEditor = openEditor;

        Saves.CollectionChanged += (_, _) =>
        {
            HasSaves = Saves.Count > 0;
        };

        Refresh();
    }

    public void Refresh()
    {
        PluginItems.Clear();
        foreach (var p in _registry.GetAll())
        {
            PluginItems.Add(new PluginListItem(p, GetInitialSubtitle(p)));
        }

        IsEmpty = PluginItems.Count == 0;

        var first = PluginItems.FirstOrDefault(i => i.IsSelectable);
        if (first is not null)
        {
            SelectPluginCommand.Execute(first.Plugin);
        }
    }

    private static string GetInitialSubtitle(InstalledPlugin p) => p.LoadState switch
    {
        PluginLoadState.Failed => "load failed",
        PluginLoadState.Incompatible => "incompatible",
        _ => string.Empty
    };

    // --- partial property hooks ---

    partial void OnSaveDirectoryChanged(string? value)
    {
        ResolvedSaveDirectory = string.IsNullOrEmpty(value)
            ? null
            : PathTokenExpander.Expand(value);

        IsPathConfigured = !string.IsNullOrEmpty(ResolvedSaveDirectory)
            && Directory.Exists(ResolvedSaveDirectory);

        if (IsPathConfigured)
            _ = RescanAsync();
        else
            Saves.Clear();
    }

    partial void OnEditingPathChanged(string? value)
    {
        EditingPathResolved = string.IsNullOrEmpty(value)
            ? null
            : PathTokenExpander.Expand(value);
    }

    // --- private helpers ---

    private void LoadPathForPlugin(InstalledPlugin? plugin)
    {
        if (plugin is null)
        {
            SaveDirectory = null;
            return;
        }

        var newPath = _settings.PluginSaveDirectories.TryGetValue(plugin.Manifest.Id, out var saved)
            ? saved
            : plugin.Plugin?.Capabilities.DefaultSaveDirectory;

        if (newPath == SaveDirectory)
        {
            // Same path — OnSaveDirectoryChanged won't fire; trigger a rescan manually.
            if (IsPathConfigured)
            {
                _ = RescanAsync();
            }
            return;
        }

        SaveDirectory = newPath;
    }

    private async Task RescanAsync()
    {
        if (SelectedPlugin?.Plugin is null || ResolvedSaveDirectory is null)
        {
            return;
        }

        IsLoadingSaves = true;
        try
        {
            var found = await _discovery.DiscoverAsync(
                ResolvedSaveDirectory,
                SelectedPlugin.Plugin.Capabilities);
            Saves.Clear();
            foreach (var s in found)
            {
                Saves.Add(s);
            }

            UpdateSelectedItemSubtitle();
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            // Discovery failed — leave Saves empty, log for diagnostics
            System.Diagnostics.Debug.WriteLine($"[HomeViewModel] RescanAsync failed: {ex}");
        }
        finally
        {
            IsLoadingSaves = false;
        }
    }

    private void UpdateSelectedItemSubtitle()
    {
        var item = PluginItems.FirstOrDefault(i => i.Plugin == SelectedPlugin);
        if (item is null)
        {
            return;
        }

        item.Subtitle = IsPathConfigured
            ? $"{Saves.Count} save{(Saves.Count == 1 ? "" : "s")}"
            : "path not set";
    }

    // --- commands ---

    [RelayCommand]
    private void SelectPlugin(InstalledPlugin plugin)
    {
        foreach (var item in PluginItems)
        {
            item.IsSelected = item.Plugin == plugin;
        }

        SelectedPlugin = plugin;
        LoadPathForPlugin(plugin);
    }

    [RelayCommand]
    private void StartEditPath()
    {
        EditingPath = SaveDirectory;
        IsEditingPath = true;
    }

    [RelayCommand]
    private async Task ConfirmPathAsync()
    {
        if (SelectedPlugin is null)
        {
            return;
        }

        if (string.IsNullOrEmpty(EditingPath))
        {
            _settings.PluginSaveDirectories.Remove(SelectedPlugin.Manifest.Id);
        }
        else
        {
            _settings.PluginSaveDirectories[SelectedPlugin.Manifest.Id] = EditingPath;
        }

        try
        {
            await _settingsService.SaveAsync(_settings);
        }
        catch
        {
            // Non-fatal — in-memory state is already updated
        }

        SaveDirectory = EditingPath;
        IsEditingPath = false;
    }

    [RelayCommand]
    private void CancelEditPath()
    {
        IsEditingPath = false;
        EditingPath = null;
    }

    [RelayCommand]
    private async Task BrowseForPathAsync()
    {
        if (RequestFolderPath is null)
        {
            return;
        }

        var path = await RequestFolderPath();
        if (path is not null)
        {
            EditingPath = path;
        }
    }

    [RelayCommand]
    private async Task OpenSaveAsync(SaveFileInfo save)
    {
        await _openEditor(save.FilePath);
    }

    [RelayCommand]
    private void BrowseStore() => _navigateToStore("store");

    [RelayCommand]
    private async Task RefreshSavesAsync() => await RescanAsync();
}
