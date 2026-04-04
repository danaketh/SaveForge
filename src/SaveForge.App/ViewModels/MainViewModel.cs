using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.Core.Interfaces;

namespace SaveForge.App.ViewModels;

public sealed partial class MainViewModel : ObservableObject
{
    private readonly IUpdateChecker _updateChecker;
    private readonly StatusBarViewModel _statusBar;
    private readonly InfoBarViewModel _infoBar;

    public StatusBarViewModel StatusBar { get; }
    public InfoBarViewModel InfoBar { get; }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsHome))]
    [NotifyPropertyChangedFor(nameof(IsEditorOpen))]
    [NotifyPropertyChangedFor(nameof(CurrentEditor))]
    private ObservableObject? _currentPage;

    public bool IsHome => CurrentPage is HomeViewModel or EditorViewModel;
    public bool IsEditorOpen => CurrentPage is EditorViewModel;
    public EditorViewModel? CurrentEditor => CurrentPage as EditorViewModel;

    public HomeViewModel Home { get; }
    public StoreViewModel Store { get; }
    public SettingsViewModel Settings { get; }
    public AboutViewModel About { get; }

    public MainViewModel(
        HomeViewModel home,
        StoreViewModel store,
        SettingsViewModel settings,
        AboutViewModel about,
        IUpdateChecker updateChecker,
        StatusBarViewModel statusBar,
        InfoBarViewModel infoBar)
    {
        Home = home;
        Store = store;
        Settings = settings;
        About = about;
        _updateChecker = updateChecker;
        StatusBar = statusBar;
        InfoBar = infoBar;
        _statusBar = statusBar;
        _infoBar = infoBar;

        CurrentPage = Home;
    }

    [RelayCommand]
    private void NavigateTo(string page)
    {
        CurrentPage = page switch
        {
            "home" => (ObservableObject)Home,
            "store" => Store,
            "settings" => Settings,
            "about" => About,
            _ => Home
        };
    }

    /// <summary>Called after the window is fully shown. Runs the update check in the background.</summary>
    public async Task RunPostLoadTasksAsync()
    {
        _statusBar.SetMessage("Checking for plugin updates\u2026");
        try
        {
            var updates = await _updateChecker.CheckAsync();
            if (updates.Count > 0)
            {
                _statusBar.Clear();
                _infoBar.Show($"Updates available for {updates.Count} plugin(s). Go to Store to update.");
            }
            else
            {
                _statusBar.Clear();
            }
        }
        catch
        {
            _statusBar.SetMessage("Update check failed. Retry in Settings.");
        }
    }

    public void OpenEditor(EditorViewModel editor) => CurrentPage = editor;
}
