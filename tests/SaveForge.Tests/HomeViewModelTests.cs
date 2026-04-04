using SaveForge.App.ViewModels;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.Infrastructure.Services;
using SaveForge.PluginHost;
using SaveForge.SDK;
using Xunit;

namespace SaveForge.Tests;

public class HomeViewModelTests : IDisposable
{
    private readonly string _settingsPath =
        Path.Combine(Path.GetTempPath(), $"hvm-test-{Guid.NewGuid()}.json");

    private readonly PluginRegistry _registry = new();
    private readonly StubDiscovery _discovery = new();
    private readonly AppSettings _settings = new();
    private readonly SettingsService _settingsSvc;

    public HomeViewModelTests()
    {
        _settingsSvc = new SettingsService(_settingsPath);
    }

    private HomeViewModel MakeVm() =>
        new(_registry, _discovery, _settings, _settingsSvc,
            _ => { }, _ => Task.FromResult<EditorViewModel?>(null));

    // --- helpers ---

    private static InstalledPlugin MakeLoaded(string id, string? defaultDir = null) => new()
    {
        Manifest = new PluginManifest
        {
            Id = id,
            Name = id,
            Version = "1.0.0",
            Author = "T",
            MinHostVersion = "1.0.0",
            PluginAssembly = "x.dll"
        },
        LoadState = PluginLoadState.Loaded,
        AssemblyPath = string.Empty,
        Plugin = new StubPlugin(id, defaultDir)
    };

    private static InstalledPlugin MakeFailed(string id) => new()
    {
        Manifest = new PluginManifest
        {
            Id = id,
            Name = id,
            Version = "1.0.0",
            Author = "T",
            MinHostVersion = "1.0.0",
            PluginAssembly = "x.dll"
        },
        LoadState = PluginLoadState.Failed,
        AssemblyPath = string.Empty,
        FailureReason = "bad dll"
    };

    // --- tests ---

    [Fact]
    public void Refresh_AutoSelectsFirstLoadedPlugin()
    {
        _registry.Register(MakeLoaded("p1"));
        var vm = MakeVm();
        Assert.NotNull(vm.SelectedPlugin);
        Assert.Equal("p1", vm.SelectedPlugin!.Manifest.Id);
    }

    [Fact]
    public void Refresh_SkipsFailedPluginsForAutoSelect()
    {
        _registry.Register(MakeFailed("bad"));
        _registry.Register(MakeLoaded("good"));
        var vm = MakeVm();
        Assert.Equal("good", vm.SelectedPlugin!.Manifest.Id);
    }

    [Fact]
    public void SelectPlugin_LoadsDefaultPathWhenNoSettingsEntry()
    {
        var plugin = MakeLoaded("p1", defaultDir: @"C:\DefaultSaves");
        _registry.Register(plugin);
        var vm = MakeVm();

        vm.SelectPluginCommand.Execute(plugin);

        Assert.Equal(@"C:\DefaultSaves", vm.SaveDirectory);
    }

    [Fact]
    public void SelectPlugin_PrefersSettingsOverDefault()
    {
        var plugin = MakeLoaded("p1", defaultDir: @"C:\DefaultSaves");
        _registry.Register(plugin);
        _settings.PluginSaveDirectories["p1"] = @"C:\CustomSaves";
        var vm = MakeVm();

        vm.SelectPluginCommand.Execute(plugin);

        Assert.Equal(@"C:\CustomSaves", vm.SaveDirectory);
    }

    [Fact]
    public void StartEditPath_SeedsEditingPathFromSaveDirectory()
    {
        var plugin = MakeLoaded("p1", defaultDir: @"C:\Saves");
        _registry.Register(plugin);
        var vm = MakeVm();

        vm.StartEditPathCommand.Execute(null);

        Assert.True(vm.IsEditingPath);
        Assert.Equal(vm.SaveDirectory, vm.EditingPath);
    }

    [Fact]
    public void CancelEditPath_ExitsEditModeAndDiscardsChanges()
    {
        var plugin = MakeLoaded("p1", defaultDir: @"C:\Saves");
        _registry.Register(plugin);
        var vm = MakeVm();
        vm.StartEditPathCommand.Execute(null);
        vm.EditingPath = @"C:\Changed";

        vm.CancelEditPathCommand.Execute(null);

        Assert.False(vm.IsEditingPath);
        Assert.Equal(@"C:\Saves", vm.SaveDirectory); // unchanged
    }

    [Fact]
    public async Task ConfirmPath_PersistsToSettingsAndUpdatesSaveDirectory()
    {
        var plugin = MakeLoaded("p1", defaultDir: @"C:\Saves");
        _registry.Register(plugin);
        var vm = new HomeViewModel(_registry, _discovery, _settings, _settingsSvc,
            _ => { }, _ => Task.FromResult<EditorViewModel?>(null));
        vm.StartEditPathCommand.Execute(null);
        vm.EditingPath = @"C:\NewPath";

        await vm.ConfirmPathCommand.ExecuteAsync(null);

        Assert.False(vm.IsEditingPath);
        Assert.Equal(@"C:\NewPath", vm.SaveDirectory);
        Assert.Equal(@"C:\NewPath", _settings.PluginSaveDirectories["p1"]);
    }

    [Fact]
    public void OnSaveDirectoryChanged_SetsResolvedSaveDirectory()
    {
        var plugin = MakeLoaded("p1");
        _registry.Register(plugin);
        var vm = MakeVm();

        vm.SelectPluginCommand.Execute(plugin);
        // Raw path without tokens — should resolve to itself
        vm.SaveDirectory = @"C:\TestSaves";

        Assert.Equal(@"C:\TestSaves", vm.ResolvedSaveDirectory);
    }

    [Fact]
    public void OnEditingPathChanged_UpdatesEditingPathResolved()
    {
        var plugin = MakeLoaded("p1");
        _registry.Register(plugin);
        var vm = MakeVm();
        vm.StartEditPathCommand.Execute(null);

        vm.EditingPath = @"C:\SomeDir";

        Assert.Equal(@"C:\SomeDir", vm.EditingPathResolved);
    }

    [Fact]
    public void IsShowingConfigurePrompt_TrueWhenPluginSelectedButPathNotConfigured()
    {
        // Plugin with no default path and no settings entry
        var plugin = MakeLoaded("p1", defaultDir: null);
        _registry.Register(plugin);
        var vm = MakeVm();

        Assert.True(vm.IsShowingConfigurePrompt);
    }

    [Fact]
    public void IsShowingEditPath_TrueOnlyWhenEditing()
    {
        var plugin = MakeLoaded("p1");
        _registry.Register(plugin);
        var vm = MakeVm();

        Assert.False(vm.IsShowingEditPath);
        vm.StartEditPathCommand.Execute(null);
        Assert.True(vm.IsShowingEditPath);
    }

    public void Dispose()
    {
        if (File.Exists(_settingsPath))
        {
            File.Delete(_settingsPath);
        }
    }

    // --- test doubles ---

    private sealed class StubDiscovery : ISaveDiscoveryService
    {
        public IReadOnlyList<SaveFileInfo> NextResult { get; set; } = [];
        public Task<IReadOnlyList<SaveFileInfo>> DiscoverAsync(
            string directory, SaveCapabilities capabilities, CancellationToken ct = default)
            => Task.FromResult(NextResult);
    }

    private sealed class StubPlugin : IGameEditorPlugin
    {
        private readonly string? _defaultDir;
        public StubPlugin(string id, string? defaultDir)
        {
            _defaultDir = defaultDir;
            Manifest = new PluginManifest
            {
                Id = id,
                Name = id,
                Version = "1.0.0",
                Author = "T",
                MinHostVersion = "1.0.0",
                PluginAssembly = "x.dll"
            };
            Capabilities = new SaveCapabilities
            {
                FileExtensions = [".json"],
                DefaultSaveDirectory = defaultDir
            };
        }
        public PluginManifest Manifest { get; }
        public SaveCapabilities Capabilities { get; }
        public (bool CanOpen, int Priority) CanOpen(FileInfo f, ReadOnlySpan<byte> h) => (true, 0);
        public Task<ISaveDocument> OpenAsync(string p, CancellationToken ct) => throw new NotImplementedException();
    }
}
