using Avalonia.Media.Imaging;
using CommunityToolkit.Mvvm.ComponentModel;
using SaveForge.Core.Models;

namespace SaveForge.App.ViewModels;

/// <summary>Wraps InstalledPlugin for the Home sidebar, adding reactive sidebar state.</summary>
public sealed partial class PluginListItem : ObservableObject
{
    public InstalledPlugin Plugin { get; }

    /// <summary>Secondary text shown below plugin name: save count, "path not set", or "load failed".</summary>
    [ObservableProperty]
    private string _subtitle = string.Empty;

    /// <summary>True when the plugin is loaded successfully and can be selected.</summary>
    public bool IsSelectable => Plugin.LoadState == PluginLoadState.Loaded;

    /// <summary>True when this item is the currently selected plugin in the sidebar.</summary>
    [ObservableProperty]
    private bool _isSelected;

    public Bitmap? Icon { get; }

    public bool HasIcon => Icon is not null;

    public string Abbreviation
    {
        get
        {
            var name = Plugin.Manifest.Name;
            if (string.IsNullOrWhiteSpace(name))
            {
                return "??";
            }

            var parts = name.Split(' ', StringSplitOptions.RemoveEmptyEntries);
            return parts.Length >= 2
                ? $"{parts[0][0]}{parts[1][0]}".ToUpperInvariant()
                : (name.Length >= 2 ? name[..2] : name).ToUpperInvariant();
        }
    }

    public PluginListItem(InstalledPlugin plugin, string subtitle)
    {
        Plugin = plugin;
        _subtitle = subtitle;

        if (plugin.IconPath is not null)
        {
            try { Icon = new Bitmap(plugin.IconPath); }
            catch { /* icon load failure is non-fatal */ }
        }
    }
}
