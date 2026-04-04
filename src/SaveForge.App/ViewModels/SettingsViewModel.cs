using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.Infrastructure.Services;

namespace SaveForge.App.ViewModels;

public sealed partial class SettingsViewModel : ObservableObject
{
    private readonly SettingsService _settingsService;
    private readonly IUpdateChecker _updateChecker;
    private readonly StatusBarViewModel _statusBar;
    private readonly InfoBarViewModel _infoBar;

    [ObservableProperty] private string _catalogUrl = string.Empty;
    [ObservableProperty] private string _pluginDirectory = string.Empty;
    [ObservableProperty] private string _backupDirectory = string.Empty;

    public SettingsViewModel(
        AppSettings settings,
        SettingsService settingsService,
        IUpdateChecker updateChecker,
        StatusBarViewModel statusBar,
        InfoBarViewModel infoBar)
    {
        _settingsService = settingsService;
        _updateChecker = updateChecker;
        _statusBar = statusBar;
        _infoBar = infoBar;
        CatalogUrl = settings.CatalogUrl;
        PluginDirectory = settings.PluginDirectory;
        BackupDirectory = settings.BackupDirectory;
    }

    [RelayCommand]
    private async Task SaveSettingsAsync()
    {
        var settings = new AppSettings
        {
            CatalogUrl = CatalogUrl,
            PluginDirectory = PluginDirectory,
            BackupDirectory = BackupDirectory
        };
        await _settingsService.SaveAsync(settings);
        _statusBar.SetMessage("Settings saved.");
    }

    [RelayCommand]
    private async Task CheckUpdatesAsync(CancellationToken ct)
    {
        _statusBar.SetMessage("Checking for plugin updates\u2026");
        try
        {
            var updates = await _updateChecker.CheckAsync(ct);
            if (updates.Count > 0)
            {
                _infoBar.Show($"Updates available for {updates.Count} plugin(s) \u2014 Review in Store.");
            }
            else
            {
                _statusBar.SetMessage("All plugins are up to date.");
            }
        }
        catch
        {
            _statusBar.SetMessage("Update check failed.");
        }
    }
}
