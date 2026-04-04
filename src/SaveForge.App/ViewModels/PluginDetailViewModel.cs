using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;

namespace SaveForge.App.ViewModels;

public sealed partial class PluginDetailViewModel : ObservableObject
{
    private readonly IPluginInstaller _installer;

    public CatalogEntry Entry { get; }

    [ObservableProperty]
    private bool _isBusy;

    [ObservableProperty]
    private string? _statusMessage;

    public PluginDetailViewModel(CatalogEntry entry, IPluginInstaller installer)
    {
        Entry = entry;
        _installer = installer;
    }

    [RelayCommand]
    private async Task InstallAsync(CancellationToken ct)
    {
        IsBusy = true;
        StatusMessage = "Installing\u2026";
        try
        {
            await _installer.InstallAsync(Entry.Id, ct: ct);
            StatusMessage = "Installed successfully.";
        }
        catch (Exception ex)
        {
            StatusMessage = $"Failed: {ex.Message}";
        }
        finally { IsBusy = false; }
    }

    [RelayCommand]
    private async Task RemoveAsync(CancellationToken ct)
    {
        IsBusy = true;
        StatusMessage = "Removing\u2026";
        try
        {
            await _installer.RemoveAsync(Entry.Id, ct);
            StatusMessage = "Removed.";
        }
        catch (Exception ex)
        {
            StatusMessage = $"Failed: {ex.Message}";
        }
        finally { IsBusy = false; }
    }

    [RelayCommand]
    private async Task UpdateAsync(CancellationToken ct)
    {
        IsBusy = true;
        StatusMessage = "Updating\u2026";
        try
        {
            await _installer.UpdateAsync(Entry.Id, ct: ct);
            StatusMessage = "Updated successfully.";
        }
        catch (Exception ex)
        {
            StatusMessage = $"Failed: {ex.Message}";
        }
        finally { IsBusy = false; }
    }
}
