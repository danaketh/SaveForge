# Plugin Save Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Each plugin declares a default save directory; users can override it per-plugin; the Home screen shows a sidebar of plugins and a scrollable save list for the selected plugin.

**Architecture:** SDK gains `DefaultSaveDirectory` on `SaveCapabilities`; a new `SaveDiscoveryService` scans recursively using `PathTokenExpander` for env-var tokens; `HomeViewModel` is restructured around a selected-plugin/save-list model; `HomeView` is rewritten as sidebar + detail panel.

**Tech Stack:** .NET 8, C#, Avalonia 11, CommunityToolkit.Mvvm, xUnit, Microsoft.Extensions.DependencyInjection

---

## File Map

| Action | Path | Purpose |
|--------|------|---------|
| Modify | `src/SaveForge.SDK/SaveCapabilities.cs` | Add `DefaultSaveDirectory` |
| Modify | `src/SaveForge.Core/Models/AppSettings.cs` | Add `PluginSaveDirectories` |
| Create | `src/SaveForge.Core/Models/SaveFileInfo.cs` | Save-file metadata record |
| Create | `src/SaveForge.Core/Interfaces/ISaveDiscoveryService.cs` | Discovery interface |
| Create | `src/SaveForge.Infrastructure/Services/PathTokenExpander.cs` | Env-var token expansion |
| Create | `src/SaveForge.Infrastructure/Services/SaveDiscoveryService.cs` | Recursive file scan |
| Modify | `src/SaveForge.Plugin.PlanetCrafter/PlanetCrafterPlugin.cs` | Add default path |
| Modify | `src/SaveForge.SamplePlugin.StardewLike/StardewPlugin.cs` | Add default path |
| Create | `src/SaveForge.App/ViewModels/PluginListItem.cs` | Sidebar item wrapper |
| Modify | `src/SaveForge.App/ViewModels/HomeViewModel.cs` | Full restructure |
| Modify | `src/SaveForge.App/Views/HomeView.axaml` | Full rewrite |
| Modify | `src/SaveForge.App/Views/HomeView.axaml.cs` | Folder picker delegate |
| Modify | `src/SaveForge.App/Themes/DarkTheme.axaml` | Add selected nav style |
| Modify | `src/SaveForge.App/App.axaml.cs` | DI wiring |
| Create | `tests/SaveForge.Tests/PathTokenExpanderTests.cs` | Unit tests |
| Create | `tests/SaveForge.Tests/SaveDiscoveryServiceTests.cs` | Integration tests |
| Create | `tests/SaveForge.Tests/HomeViewModelTests.cs` | ViewModel unit tests |

---

## Task 1: Add `DefaultSaveDirectory` to `SaveCapabilities`

**Files:**
- Modify: `src/SaveForge.SDK/SaveCapabilities.cs`

- [ ] **Step 1: Add the property**

Replace the contents of `src/SaveForge.SDK/SaveCapabilities.cs` with:

```csharp
namespace SaveForge.SDK;

/// <summary>Describes how a plugin detects which files it can open.</summary>
public sealed class SaveCapabilities
{
    /// <summary>File extensions this plugin handles, e.g. [".json", ".sav"].</summary>
    public string[] FileExtensions { get; init; } = Array.Empty<string>();

    /// <summary>Optional MIME type hints.</summary>
    public string[] MimeHints { get; init; } = Array.Empty<string>();

    /// <summary>Higher value wins when multiple plugins match the same file. Default 0.</summary>
    public int DetectionPriority { get; init; } = 0;

    /// <summary>
    /// Default directory where this game stores saves, using env-var tokens.
    /// Supports %APPDATA%, %LOCALAPPDATA%, %USERPROFILE%, %TEMP%, and leading ~.
    /// Example: @"%APPDATA%\MijuGames\The Planet Crafter\Saves"
    /// Null if the plugin has no known default location.
    /// </summary>
    public string? DefaultSaveDirectory { get; init; }
}
```

- [ ] **Step 2: Verify build**

```bash
cd I:/Projects/danaketh/SaveForge
dotnet build src/SaveForge.SDK/SaveForge.SDK.csproj
```

Expected: `Build succeeded.`

- [ ] **Step 3: Commit**

```bash
cd I:/Projects/danaketh/SaveForge
git add src/SaveForge.SDK/SaveCapabilities.cs
git commit -m "feat(sdk): add DefaultSaveDirectory to SaveCapabilities"
```

---

## Task 2: Core data model — `SaveFileInfo`, `ISaveDiscoveryService`, `PluginSaveDirectories`

**Files:**
- Create: `src/SaveForge.Core/Models/SaveFileInfo.cs`
- Create: `src/SaveForge.Core/Interfaces/ISaveDiscoveryService.cs`
- Modify: `src/SaveForge.Core/Models/AppSettings.cs`

- [ ] **Step 1: Create `SaveFileInfo`**

Create `src/SaveForge.Core/Models/SaveFileInfo.cs`:

```csharp
namespace SaveForge.Core.Models;

public sealed record SaveFileInfo(
    string FilePath,
    string FileName,
    DateTimeOffset LastModified,
    long SizeBytes)
{
    public string LastModifiedDisplay => LastModified.ToString("MMM d, yyyy");

    public string SizeDisplay => SizeBytes < 1_048_576
        ? $"{SizeBytes / 1024.0:0.0} KB"
        : $"{SizeBytes / 1_048_576.0:0.0} MB";
}
```

- [ ] **Step 2: Create `ISaveDiscoveryService`**

Create `src/SaveForge.Core/Interfaces/ISaveDiscoveryService.cs`:

```csharp
using SaveForge.Core.Models;
using SaveForge.SDK;

namespace SaveForge.Core.Interfaces;

public interface ISaveDiscoveryService
{
    /// <summary>
    /// Returns all save files found under <paramref name="directory"/> matching the plugin's
    /// declared extensions, sorted by LastModified descending. Never throws — returns empty
    /// list if directory does not exist or is unreadable.
    /// </summary>
    Task<IReadOnlyList<SaveFileInfo>> DiscoverAsync(
        string directory,
        SaveCapabilities capabilities,
        CancellationToken ct = default);
}
```

- [ ] **Step 3: Add `PluginSaveDirectories` to `AppSettings`**

In `src/SaveForge.Core/Models/AppSettings.cs`, add one property to the `AppSettings` class:

```csharp
public Dictionary<string, string> PluginSaveDirectories { get; set; } = [];
```

The full file becomes:

```csharp
namespace SaveForge.Core.Models;

public sealed class AppSettings
{
    public string CatalogUrl { get; set; } = AppSettingsDefaults.CatalogUrl;
    public string PluginDirectory { get; set; } = AppSettingsDefaults.PluginDirectory;
    public string BackupDirectory { get; set; } = AppSettingsDefaults.BackupDirectory;
    public bool CheckUpdatesOnStartup { get; set; } = true;
    public Dictionary<string, string> PluginSaveDirectories { get; set; } = [];
}

public static class AppSettingsDefaults
{
    private static readonly string AppData =
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "SaveForge");

#if DEBUG
    public static string PluginDirectory { get; } = Path.Combine(AppContext.BaseDirectory, "_devplugins");
#else
    public static string PluginDirectory { get; } = Path.Combine(AppData, "plugins");
#endif
    public static string BackupDirectory { get; } = Path.Combine(AppData, "backups");
    public static string SettingsPath { get; } = Path.Combine(AppData, "settings.json");
    public static string CatalogCachePath { get; } = Path.Combine(AppData, "catalog-cache.json");

#if DEBUG
    public static string CatalogUrl { get; } =
        new Uri(Path.Combine(AppData, "catalog.json")).AbsoluteUri;
#else
    public static string CatalogUrl { get; } =
        "https://raw.githubusercontent.com/YOUR_ORG/YOUR_REPO/main/catalog.json";
#endif
}
```

- [ ] **Step 4: Verify build**

```bash
dotnet build src/SaveForge.Core/SaveForge.Core.csproj
```

Expected: `Build succeeded.`

- [ ] **Step 5: Commit**

```bash
git add src/SaveForge.Core/Models/SaveFileInfo.cs \
        src/SaveForge.Core/Interfaces/ISaveDiscoveryService.cs \
        src/SaveForge.Core/Models/AppSettings.cs
git commit -m "feat(core): add SaveFileInfo, ISaveDiscoveryService, PluginSaveDirectories"
```

---

## Task 3: `PathTokenExpander` (TDD)

**Files:**
- Create: `tests/SaveForge.Tests/PathTokenExpanderTests.cs`
- Create: `src/SaveForge.Infrastructure/Services/PathTokenExpander.cs`

- [ ] **Step 1: Write failing tests**

Create `tests/SaveForge.Tests/PathTokenExpanderTests.cs`:

```csharp
using SaveForge.Infrastructure.Services;
using Xunit;

namespace SaveForge.Tests;

public class PathTokenExpanderTests
{
    [Fact]
    public void Expand_NoTokens_ReturnsSamePath()
    {
        var result = PathTokenExpander.Expand(@"C:\Users\dan\saves");
        Assert.Equal(@"C:\Users\dan\saves", result);
    }

    [Fact]
    public void Expand_AppDataToken_ReplacesWithFolder()
    {
        var expected = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
        var result = PathTokenExpander.Expand(@"%APPDATA%\MyGame\Saves");
        Assert.StartsWith(expected, result, StringComparison.OrdinalIgnoreCase);
        Assert.EndsWith(@"\MyGame\Saves", result, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Expand_LocalAppDataToken_ReplacesWithFolder()
    {
        var expected = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        var result = PathTokenExpander.Expand(@"%LOCALAPPDATA%\Game");
        Assert.StartsWith(expected, result, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Expand_UserProfileToken_ReplacesWithFolder()
    {
        var expected = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var result = PathTokenExpander.Expand(@"%USERPROFILE%\Documents\Saves");
        Assert.StartsWith(expected, result, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Expand_TempToken_ReplacesWithTempPath()
    {
        var expected = Path.GetTempPath().TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        var result = PathTokenExpander.Expand(@"%TEMP%\saves");
        Assert.StartsWith(expected, result, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Expand_LeadingTilde_ReplacesWithUserProfile()
    {
        var expected = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var result = PathTokenExpander.Expand(@"~/saves/game");
        Assert.StartsWith(expected, result, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Expand_UnknownToken_LeavesItAsIs()
    {
        var result = PathTokenExpander.Expand(@"%UNKNOWN_TOKEN%\saves");
        Assert.Equal(@"%UNKNOWN_TOKEN%\saves", result);
    }

    [Fact]
    public void Expand_EmptyString_ReturnsEmptyString()
    {
        var result = PathTokenExpander.Expand(string.Empty);
        Assert.Equal(string.Empty, result);
    }

    [Fact]
    public void Expand_TokenCaseInsensitive_ReplacesCorrectly()
    {
        var expected = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
        var result = PathTokenExpander.Expand(@"%appdata%\Game");
        Assert.StartsWith(expected, result, StringComparison.OrdinalIgnoreCase);
    }
}
```

- [ ] **Step 2: Run tests — confirm they fail**

```bash
dotnet test tests/SaveForge.Tests/ --filter "PathTokenExpanderTests" 2>&1 | tail -5
```

Expected: error about `PathTokenExpander` not found.

- [ ] **Step 3: Implement `PathTokenExpander`**

Create `src/SaveForge.Infrastructure/Services/PathTokenExpander.cs`:

```csharp
namespace SaveForge.Infrastructure.Services;

/// <summary>Expands environment-variable tokens in directory path strings.</summary>
public static class PathTokenExpander
{
    private static readonly (string Token, Func<string> Resolver)[] _knownTokens =
    [
        ("%APPDATA%",      () => Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData)),
        ("%LOCALAPPDATA%", () => Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData)),
        ("%USERPROFILE%",  () => Environment.GetFolderPath(Environment.SpecialFolder.UserProfile)),
        ("%TEMP%",         () => Path.GetTempPath().TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)),
    ];

    /// <summary>
    /// Replaces known tokens in <paramref name="path"/> with their runtime values.
    /// Unrecognised tokens are left as-is. Leading <c>~</c> is treated as %USERPROFILE%.
    /// </summary>
    public static string Expand(string path)
    {
        if (string.IsNullOrEmpty(path))
            return path;

        // Handle leading tilde
        if (path.StartsWith("~/", StringComparison.Ordinal) || path.StartsWith(@"~\", StringComparison.Ordinal))
        {
            var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            path = home + path[1..];
        }

        foreach (var (token, resolver) in _knownTokens)
        {
            if (path.Contains(token, StringComparison.OrdinalIgnoreCase))
                path = path.Replace(token, resolver(), StringComparison.OrdinalIgnoreCase);
        }

        return path;
    }
}
```

- [ ] **Step 4: Run tests — confirm they pass**

```bash
dotnet test tests/SaveForge.Tests/ --filter "PathTokenExpanderTests"
```

Expected: `Passed! - Failed: 0, Passed: 9`

- [ ] **Step 5: Commit**

```bash
git add src/SaveForge.Infrastructure/Services/PathTokenExpander.cs \
        tests/SaveForge.Tests/PathTokenExpanderTests.cs
git commit -m "feat(infrastructure): add PathTokenExpander with token expansion"
```

---

## Task 4: `SaveDiscoveryService` (TDD)

**Files:**
- Create: `tests/SaveForge.Tests/SaveDiscoveryServiceTests.cs`
- Create: `src/SaveForge.Infrastructure/Services/SaveDiscoveryService.cs`

- [ ] **Step 1: Write failing tests**

Create `tests/SaveForge.Tests/SaveDiscoveryServiceTests.cs`:

```csharp
using SaveForge.Core.Interfaces;
using SaveForge.Infrastructure.Services;
using SaveForge.SDK;
using Xunit;

namespace SaveForge.Tests;

public class SaveDiscoveryServiceTests : IDisposable
{
    private readonly string _root = Path.Combine(Path.GetTempPath(), $"sds-test-{Guid.NewGuid()}");
    private readonly ISaveDiscoveryService _svc = new SaveDiscoveryService();

    public SaveDiscoveryServiceTests()
    {
        Directory.CreateDirectory(_root);
    }

    [Fact]
    public async Task DiscoverAsync_EmptyDirectory_ReturnsEmpty()
    {
        var caps = new SaveCapabilities { FileExtensions = [".json"] };
        var results = await _svc.DiscoverAsync(_root, caps);
        Assert.Empty(results);
    }

    [Fact]
    public async Task DiscoverAsync_MatchingFile_ReturnsIt()
    {
        File.WriteAllText(Path.Combine(_root, "save.json"), "{}");
        var caps = new SaveCapabilities { FileExtensions = [".json"] };

        var results = await _svc.DiscoverAsync(_root, caps);

        Assert.Single(results);
        Assert.Equal("save.json", results[0].FileName);
    }

    [Fact]
    public async Task DiscoverAsync_NonMatchingExtension_Excluded()
    {
        File.WriteAllText(Path.Combine(_root, "save.txt"), "data");
        var caps = new SaveCapabilities { FileExtensions = [".json"] };

        var results = await _svc.DiscoverAsync(_root, caps);

        Assert.Empty(results);
    }

    [Fact]
    public async Task DiscoverAsync_RecursesIntoSubdirectories()
    {
        var sub = Directory.CreateDirectory(Path.Combine(_root, "slot1"));
        File.WriteAllText(Path.Combine(sub.FullName, "save.json"), "{}");
        var caps = new SaveCapabilities { FileExtensions = [".json"] };

        var results = await _svc.DiscoverAsync(_root, caps);

        Assert.Single(results);
        Assert.Equal("save.json", results[0].FileName);
    }

    [Fact]
    public async Task DiscoverAsync_SortedByLastModifiedDescending()
    {
        var older = Path.Combine(_root, "old.json");
        var newer = Path.Combine(_root, "new.json");
        File.WriteAllText(older, "{}");
        File.SetLastWriteTimeUtc(older, DateTime.UtcNow.AddHours(-1));
        File.WriteAllText(newer, "{}");
        File.SetLastWriteTimeUtc(newer, DateTime.UtcNow);
        var caps = new SaveCapabilities { FileExtensions = [".json"] };

        var results = await _svc.DiscoverAsync(_root, caps);

        Assert.Equal(2, results.Count);
        Assert.Equal("new.json", results[0].FileName);
        Assert.Equal("old.json", results[1].FileName);
    }

    [Fact]
    public async Task DiscoverAsync_DirectoryDoesNotExist_ReturnsEmpty()
    {
        var missing = Path.Combine(_root, "nonexistent");
        var caps = new SaveCapabilities { FileExtensions = [".json"] };

        var results = await _svc.DiscoverAsync(missing, caps);

        Assert.Empty(results);
    }

    [Fact]
    public async Task DiscoverAsync_ExtensionMatchIsCaseInsensitive()
    {
        File.WriteAllText(Path.Combine(_root, "SAVE.JSON"), "{}");
        var caps = new SaveCapabilities { FileExtensions = [".json"] };

        var results = await _svc.DiscoverAsync(_root, caps);

        Assert.Single(results);
    }

    [Fact]
    public async Task DiscoverAsync_PathWithToken_ExpandsBeforeScanning()
    {
        // Use the real temp dir via %TEMP% token
        var caps = new SaveCapabilities { FileExtensions = [".json"] };
        // Should not throw even when the expanded dir has files we don't care about
        var results = await _svc.DiscoverAsync("%TEMP%", caps);
        // Just assert no exception; result count varies by machine
        Assert.NotNull(results);
    }

    public void Dispose()
    {
        if (Directory.Exists(_root))
            Directory.Delete(_root, recursive: true);
    }
}
```

- [ ] **Step 2: Run tests — confirm they fail**

```bash
dotnet test tests/SaveForge.Tests/ --filter "SaveDiscoveryServiceTests" 2>&1 | tail -5
```

Expected: compile error — `SaveDiscoveryService` not found.

- [ ] **Step 3: Implement `SaveDiscoveryService`**

Create `src/SaveForge.Infrastructure/Services/SaveDiscoveryService.cs`:

```csharp
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.SDK;

namespace SaveForge.Infrastructure.Services;

public sealed class SaveDiscoveryService : ISaveDiscoveryService
{
    public Task<IReadOnlyList<SaveFileInfo>> DiscoverAsync(
        string directory,
        SaveCapabilities capabilities,
        CancellationToken ct = default)
    {
        var resolved = PathTokenExpander.Expand(directory);

        if (!Directory.Exists(resolved))
            return Task.FromResult<IReadOnlyList<SaveFileInfo>>([]);

        var extensions = new HashSet<string>(
            capabilities.FileExtensions,
            StringComparer.OrdinalIgnoreCase);

        var results = new List<SaveFileInfo>();

        try
        {
            foreach (var path in Directory.EnumerateFiles(resolved, "*", SearchOption.AllDirectories))
            {
                ct.ThrowIfCancellationRequested();

                var ext = Path.GetExtension(path);
                if (!extensions.Contains(ext))
                    continue;

                try
                {
                    var info = new FileInfo(path);
                    results.Add(new SaveFileInfo(
                        FilePath: info.FullName,
                        FileName: info.Name,
                        LastModified: new DateTimeOffset(info.LastWriteTimeUtc, TimeSpan.Zero),
                        SizeBytes: info.Length));
                }
                catch
                {
                    // File vanished or access denied — skip this entry
                }
            }
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch
        {
            // Directory became unreadable mid-scan — return what we have
        }

        results.Sort((a, b) => b.LastModified.CompareTo(a.LastModified));
        return Task.FromResult<IReadOnlyList<SaveFileInfo>>(results);
    }
}
```

- [ ] **Step 4: Run tests — confirm they pass**

```bash
dotnet test tests/SaveForge.Tests/ --filter "SaveDiscoveryServiceTests"
```

Expected: `Passed! - Failed: 0, Passed: 8`

- [ ] **Step 5: Commit**

```bash
git add src/SaveForge.Infrastructure/Services/SaveDiscoveryService.cs \
        tests/SaveForge.Tests/SaveDiscoveryServiceTests.cs
git commit -m "feat(infrastructure): add SaveDiscoveryService with recursive file scan"
```

---

## Task 5: Add default save paths to plugins

**Files:**
- Modify: `src/SaveForge.Plugin.PlanetCrafter/PlanetCrafterPlugin.cs`
- Modify: `src/SaveForge.SamplePlugin.StardewLike/StardewPlugin.cs`

- [ ] **Step 1: Update PlanetCrafter plugin**

In `src/SaveForge.Plugin.PlanetCrafter/PlanetCrafterPlugin.cs`, replace the `Capabilities` property:

```csharp
    public SaveCapabilities Capabilities { get; } = new()
    {
        FileExtensions = [".json"],
        DetectionPriority = 20,
        DefaultSaveDirectory = @"%APPDATA%\MijuGames\The Planet Crafter\Saves"
    };
```

- [ ] **Step 2: Update Stardew sample plugin**

In `src/SaveForge.SamplePlugin.StardewLike/StardewPlugin.cs`, replace the `Capabilities` property:

```csharp
    public SaveCapabilities Capabilities { get; } = new()
    {
        FileExtensions = [".json"],
        DetectionPriority = 5,
        DefaultSaveDirectory = @"%APPDATA%\StardewValley\Saves"
    };
```

- [ ] **Step 3: Verify build**

```bash
dotnet build src/SaveForge.Plugin.PlanetCrafter/ src/SaveForge.SamplePlugin.StardewLike/
```

Expected: `Build succeeded.`

- [ ] **Step 4: Commit**

```bash
git add src/SaveForge.Plugin.PlanetCrafter/PlanetCrafterPlugin.cs \
        src/SaveForge.SamplePlugin.StardewLike/StardewPlugin.cs
git commit -m "feat(plugins): add DefaultSaveDirectory to PlanetCrafter and Stardew plugins"
```

---

## Task 6: Create `PluginListItem`

**Files:**
- Create: `src/SaveForge.App/ViewModels/PluginListItem.cs`

- [ ] **Step 1: Create the class**

Create `src/SaveForge.App/ViewModels/PluginListItem.cs`:

```csharp
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

    public PluginListItem(InstalledPlugin plugin, string subtitle)
    {
        Plugin = plugin;
        _subtitle = subtitle;
    }
}
```

- [ ] **Step 2: Verify build**

```bash
dotnet build src/SaveForge.App/SaveForge.App.csproj
```

Expected: `Build succeeded.`

- [ ] **Step 3: Commit**

```bash
git add src/SaveForge.App/ViewModels/PluginListItem.cs
git commit -m "feat(app): add PluginListItem sidebar wrapper"
```

---

## Task 7: Restructure `HomeViewModel` (TDD)

**Files:**
- Create: `tests/SaveForge.Tests/HomeViewModelTests.cs`
- Modify: `src/SaveForge.App/ViewModels/HomeViewModel.cs`

- [ ] **Step 1: Write failing tests**

Create `tests/SaveForge.Tests/HomeViewModelTests.cs`:

```csharp
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
    private SettingsService SettingsSvc => new(_settingsPath);

    private HomeViewModel MakeVm() =>
        new(_registry, _discovery, _settings, SettingsSvc,
            _ => { }, _ => Task.FromResult<EditorViewModel?>(null));

    // --- helpers ---

    private static InstalledPlugin MakeLoaded(string id, string? defaultDir = null) => new()
    {
        Manifest = new PluginManifest
        {
            Id = id, Name = id, Version = "1.0.0",
            Author = "T", MinHostVersion = "1.0.0", PluginAssembly = "x.dll"
        },
        LoadState = PluginLoadState.Loaded,
        AssemblyPath = string.Empty,
        Plugin = new StubPlugin(id, defaultDir)
    };

    private static InstalledPlugin MakeFailed(string id) => new()
    {
        Manifest = new PluginManifest
        {
            Id = id, Name = id, Version = "1.0.0",
            Author = "T", MinHostVersion = "1.0.0", PluginAssembly = "x.dll"
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
        var settingsSvc = SettingsSvc;
        var vm = new HomeViewModel(_registry, _discovery, _settings, settingsSvc,
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
        if (File.Exists(_settingsPath)) File.Delete(_settingsPath);
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
                Id = id, Name = id, Version = "1.0.0",
                Author = "T", MinHostVersion = "1.0.0", PluginAssembly = "x.dll"
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
```

- [ ] **Step 2: Run tests — confirm they fail**

```bash
dotnet test tests/SaveForge.Tests/ --filter "HomeViewModelTests" 2>&1 | tail -8
```

Expected: compile errors — `HomeViewModel` constructor signature mismatch and missing properties.

- [ ] **Step 3: Rewrite `HomeViewModel`**

Replace the entire contents of `src/SaveForge.App/ViewModels/HomeViewModel.cs`:

```csharp
using System.Collections.ObjectModel;
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
    private InstalledPlugin? _selectedPlugin;

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
    private bool _isEmpty;

    // --- computed visibility ---
    public bool IsShowingConfigurePrompt => SelectedPlugin is not null && !IsPathConfigured && !IsEditingPath;
    public bool IsShowingSaveList        => IsPathConfigured && !IsEditingPath;
    public bool IsShowingEditPath        => IsEditingPath;
    public bool IsShowingNoSelection     => SelectedPlugin is null && !IsEmpty;

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
            PluginItems.Add(new PluginListItem(p, GetInitialSubtitle(p)));

        IsEmpty = PluginItems.Count == 0;

        var first = PluginItems.FirstOrDefault(i => i.IsSelectable);
        if (first is not null)
            SelectPluginCommand.Execute(first.Plugin);
    }

    private static string GetInitialSubtitle(InstalledPlugin p) => p.LoadState switch
    {
        PluginLoadState.Failed       => "load failed",
        PluginLoadState.Incompatible => "incompatible",
        _                            => string.Empty
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
        Saves.Clear();
        if (plugin is null)
        {
            SaveDirectory = null;
            return;
        }

        SaveDirectory = _settings.PluginSaveDirectories.TryGetValue(plugin.Manifest.Id, out var saved)
            ? saved
            : plugin.Plugin?.Capabilities.DefaultSaveDirectory;
    }

    private async Task RescanAsync()
    {
        if (SelectedPlugin?.Plugin is null || ResolvedSaveDirectory is null) return;

        IsLoadingSaves = true;
        try
        {
            var found = await _discovery.DiscoverAsync(
                ResolvedSaveDirectory,
                SelectedPlugin.Plugin.Capabilities);
            Saves.Clear();
            foreach (var s in found)
                Saves.Add(s);

            UpdateSelectedItemSubtitle();
        }
        finally
        {
            IsLoadingSaves = false;
        }
    }

    private void UpdateSelectedItemSubtitle()
    {
        var item = PluginItems.FirstOrDefault(i => i.Plugin == SelectedPlugin);
        if (item is null) return;

        item.Subtitle = IsPathConfigured
            ? $"{Saves.Count} save{(Saves.Count == 1 ? "" : "s")}"
            : "path not set";
    }

    // --- commands ---

    [RelayCommand]
    private void SelectPlugin(InstalledPlugin plugin)
    {
        foreach (var item in PluginItems)
            item.IsSelected = item.Plugin == plugin;

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
        if (SelectedPlugin is null) return;

        _settings.PluginSaveDirectories[SelectedPlugin.Manifest.Id] = EditingPath ?? string.Empty;
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
        if (RequestFolderPath is null) return;
        var path = await RequestFolderPath();
        if (path is not null)
            EditingPath = path;
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
```

- [ ] **Step 4: Run tests — confirm they pass**

```bash
dotnet test tests/SaveForge.Tests/ --filter "HomeViewModelTests"
```

Expected: `Passed! - Failed: 0, Passed: 11`

- [ ] **Step 5: Commit**

```bash
git add src/SaveForge.App/ViewModels/HomeViewModel.cs \
        tests/SaveForge.Tests/HomeViewModelTests.cs
git commit -m "feat(app): restructure HomeViewModel with plugin selection and save discovery"
```

---

## Task 8: Rewrite `HomeView`

**Files:**
- Modify: `src/SaveForge.App/Views/HomeView.axaml`
- Modify: `src/SaveForge.App/Views/HomeView.axaml.cs`
- Modify: `src/SaveForge.App/Themes/DarkTheme.axaml`

- [ ] **Step 1: Add selected nav-item style to `DarkTheme.axaml`**

In `src/SaveForge.App/Themes/DarkTheme.axaml`, append the following two styles before the closing `</Styles>` tag:

```xml
  <!-- Sidebar nav button — selected state -->
  <Style Selector="Button.nav-item.selected">
    <Setter Property="Foreground" Value="{StaticResource BrushTextPrimary}" />
  </Style>
  <Style Selector="Button.nav-item.selected /template/ ContentPresenter">
    <Setter Property="Background" Value="{StaticResource BrushSurface}" />
    <Setter Property="BorderBrush" Value="{StaticResource BrushAccent}" />
  </Style>
```

- [ ] **Step 2: Rewrite `HomeView.axaml`**

Replace the entire contents of `src/SaveForge.App/Views/HomeView.axaml`:

```xml
<UserControl xmlns="https://github.com/avaloniaui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
             xmlns:vm="clr-namespace:SaveForge.App.ViewModels"
             xmlns:models="clr-namespace:SaveForge.Core.Models;assembly=SaveForge.Core"
             x:Class="SaveForge.App.Views.HomeView"
             x:DataType="vm:HomeViewModel">
  <Grid>

    <!-- Empty state: no plugins installed -->
    <StackPanel IsVisible="{Binding IsEmpty}"
                VerticalAlignment="Center" HorizontalAlignment="Center" Spacing="12">
      <TextBlock Text="No plugins installed."
                 Foreground="{StaticResource BrushTextSecondary}"
                 FontSize="16" HorizontalAlignment="Center" />
      <Button Content="Browse Store" Command="{Binding BrowseStoreCommand}"
              Classes="accent" HorizontalAlignment="Center" />
    </StackPanel>

    <!-- Main layout: sidebar | detail panel -->
    <Grid IsVisible="{Binding !IsEmpty}" ColumnDefinitions="200,*">

      <!-- Sidebar -->
      <Border Grid.Column="0"
              BorderBrush="{StaticResource BrushBorder}" BorderThickness="0,0,1,0">
        <ScrollViewer>
          <ItemsControl ItemsSource="{Binding PluginItems}" Margin="8,8">
            <ItemsControl.ItemTemplate>
              <DataTemplate x:DataType="vm:PluginListItem">
                <Button IsEnabled="{Binding IsSelectable}"
                        Classes="nav-item"
                        Classes.selected="{Binding IsSelected}"
                        Click="PluginItem_Click">
                  <StackPanel Spacing="2">
                    <TextBlock Text="{Binding Plugin.Manifest.Name}"
                               FontWeight="SemiBold" />
                    <TextBlock Text="{Binding Subtitle}"
                               FontSize="11"
                               Foreground="{StaticResource BrushTextSecondary}"
                               IsVisible="{Binding Subtitle, Converter={x:Static StringConverters.IsNotNullOrEmpty}}" />
                  </StackPanel>
                </Button>
              </DataTemplate>
            </ItemsControl.ItemTemplate>
          </ItemsControl>
        </ScrollViewer>
      </Border>

      <!-- Detail panel -->
      <Grid Grid.Column="1">

        <!-- No loadable plugins -->
        <StackPanel IsVisible="{Binding IsShowingNoSelection}"
                    VerticalAlignment="Center" HorizontalAlignment="Center" Spacing="8">
          <TextBlock Text="No plugins could be loaded."
                     Foreground="{StaticResource BrushTextSecondary}"
                     FontSize="15" HorizontalAlignment="Center" />
          <TextBlock Text="Check the Store or plugin compatibility."
                     Foreground="{StaticResource BrushTextSecondary}"
                     FontSize="13" HorizontalAlignment="Center" />
        </StackPanel>

        <!-- Path not configured -->
        <StackPanel IsVisible="{Binding IsShowingConfigurePrompt}"
                    VerticalAlignment="Center" HorizontalAlignment="Center" Spacing="8">
          <TextBlock Text="📁" FontSize="32" HorizontalAlignment="Center" />
          <TextBlock Text="No save path configured"
                     FontSize="16" FontWeight="SemiBold" HorizontalAlignment="Center" />
          <TextBlock Text="Set a path so SaveForge can find your saves."
                     Foreground="{StaticResource BrushTextSecondary}"
                     HorizontalAlignment="Center" />
          <Button Content="Set Path" Command="{Binding StartEditPathCommand}"
                  Classes="accent" HorizontalAlignment="Center" Margin="0,4,0,0" />
        </StackPanel>

        <!-- Save list (path configured, not editing) -->
        <Grid IsVisible="{Binding IsShowingSaveList}" RowDefinitions="Auto,Auto,Auto,*">
          <TextBlock Grid.Row="0"
                     Text="{Binding SelectedPlugin.Manifest.Name}"
                     FontSize="16" FontWeight="SemiBold" Margin="16,12,16,4" />

          <!-- Path row -->
          <Grid Grid.Row="1" Margin="16,0,8,0" ColumnDefinitions="*,Auto">
            <TextBlock Grid.Column="0"
                       Text="{Binding SaveDirectory}"
                       Foreground="{StaticResource BrushTextSecondary}"
                       VerticalAlignment="Center"
                       TextTrimming="CharacterEllipsis" />
            <Button Grid.Column="1"
                    Content="✏️"
                    Command="{Binding StartEditPathCommand}"
                    Background="Transparent" BorderThickness="0"
                    Padding="6,4" ToolTip.Tip="Edit save path" />
          </Grid>

          <!-- Resolved path hint -->
          <TextBlock Grid.Row="2"
                     Margin="16,2,16,8"
                     Text="{Binding ResolvedSaveDirectory, StringFormat='→ {0}'}"
                     FontSize="11" Foreground="{StaticResource BrushTextSecondary}"
                     IsVisible="{Binding ResolvedSaveDirectory, Converter={x:Static StringConverters.IsNotNullOrEmpty}}" />

          <!-- Save list -->
          <ScrollViewer Grid.Row="3">
            <StackPanel Margin="8,0,8,8">
              <!-- No saves found -->
              <TextBlock Text="No save files found at this path."
                         Foreground="{StaticResource BrushTextSecondary}"
                         Margin="8,8"
                         IsVisible="{Binding !HasSaves}" />
              <!-- Save items -->
              <ItemsControl ItemsSource="{Binding Saves}">
                <ItemsControl.ItemTemplate>
                  <DataTemplate x:DataType="models:SaveFileInfo">
                    <Button HorizontalAlignment="Stretch"
                            HorizontalContentAlignment="Left"
                            Click="SaveItem_Click"
                            Margin="0,2"
                            Background="{StaticResource BrushSurface}"
                            BorderBrush="{StaticResource BrushBorder}"
                            BorderThickness="1"
                            CornerRadius="4"
                            Padding="12,8">
                      <StackPanel Spacing="2">
                        <TextBlock Text="{Binding FileName}" FontWeight="SemiBold" />
                        <TextBlock FontSize="11" Foreground="{StaticResource BrushTextSecondary}"
                                   Text="{Binding LastModifiedDisplay}" />
                      </StackPanel>
                    </Button>
                  </DataTemplate>
                </ItemsControl.ItemTemplate>
              </ItemsControl>
            </StackPanel>
          </ScrollViewer>
        </Grid>

        <!-- Editing path -->
        <Grid IsVisible="{Binding IsShowingEditPath}" RowDefinitions="Auto,Auto,Auto,*">
          <TextBlock Grid.Row="0"
                     Text="{Binding SelectedPlugin.Manifest.Name}"
                     FontSize="16" FontWeight="SemiBold" Margin="16,12,16,4" />

          <!-- Inline edit row -->
          <Grid Grid.Row="1" Margin="16,0,8,0" ColumnDefinitions="*,Auto,Auto,Auto">
            <TextBox Grid.Column="0"
                     Text="{Binding EditingPath}"
                     Watermark="e.g. %APPDATA%\MyGame\Saves"
                     VerticalAlignment="Center" />
            <Button Grid.Column="1"
                    Content="📂"
                    Command="{Binding BrowseForPathCommand}"
                    Background="Transparent" BorderThickness="0"
                    Padding="6,4" Margin="4,0,0,0"
                    ToolTip.Tip="Browse for folder" />
            <Button Grid.Column="2"
                    Content="✓"
                    Command="{Binding ConfirmPathCommand}"
                    Background="Transparent" BorderThickness="0"
                    Foreground="{StaticResource BrushAccent}"
                    Padding="6,4" Margin="2,0" />
            <Button Grid.Column="3"
                    Content="✕"
                    Command="{Binding CancelEditPathCommand}"
                    Background="Transparent" BorderThickness="0"
                    Foreground="{StaticResource BrushDanger}"
                    Padding="6,4" />
          </Grid>

          <!-- Resolved hint (live) -->
          <TextBlock Grid.Row="2"
                     Margin="16,4,16,8"
                     Text="{Binding EditingPathResolved, StringFormat='→ {0}'}"
                     FontSize="11" Foreground="{StaticResource BrushTextSecondary}"
                     IsVisible="{Binding EditingPathResolved, Converter={x:Static StringConverters.IsNotNullOrEmpty}}" />

          <!-- Dimmed save list -->
          <ScrollViewer Grid.Row="3" Opacity="0.4" IsHitTestVisible="False">
            <ItemsControl ItemsSource="{Binding Saves}" Margin="8,0,8,8">
              <ItemsControl.ItemTemplate>
                <DataTemplate x:DataType="models:SaveFileInfo">
                  <Border Margin="0,2"
                          Background="{StaticResource BrushSurface}"
                          BorderBrush="{StaticResource BrushBorder}"
                          BorderThickness="1" CornerRadius="4" Padding="12,8">
                    <TextBlock Text="{Binding FileName}" FontWeight="SemiBold" />
                  </Border>
                </DataTemplate>
              </ItemsControl.ItemTemplate>
            </ItemsControl>
          </ScrollViewer>
        </Grid>

      </Grid>
    </Grid>

  </Grid>
</UserControl>
```

- [ ] **Step 3: Rewrite `HomeView.axaml.cs`**

Replace the entire contents of `src/SaveForge.App/Views/HomeView.axaml.cs`:

```csharp
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using SaveForge.App.ViewModels;
using SaveForge.Core.Models;

namespace SaveForge.App.Views;

public partial class HomeView : UserControl
{
    public HomeView()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is HomeViewModel vm)
                vm.RequestFolderPath = OpenFolderPickerAsync;
        };
    }

    private async Task<string?> OpenFolderPickerAsync()
    {
        var topLevel = TopLevel.GetTopLevel(this);
        if (topLevel is null) return null;

        var folders = await topLevel.StorageProvider.OpenFolderPickerAsync(
            new FolderPickerOpenOptions
            {
                Title = "Select Save Directory",
                AllowMultiple = false
            });

        return folders.Count > 0 ? folders[0].Path.LocalPath : null;
    }

    private void PluginItem_Click(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not HomeViewModel vm) return;
        if (sender is Button { DataContext: PluginListItem item } && item.IsSelectable)
            vm.SelectPluginCommand.Execute(item.Plugin);
    }

    private void SaveItem_Click(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not HomeViewModel vm) return;
        if (sender is Button { DataContext: SaveFileInfo save })
            vm.OpenSaveCommand.Execute(save);
    }
}
```

- [ ] **Step 4: Verify build**

```bash
dotnet build src/SaveForge.App/SaveForge.App.csproj
```

Expected: `Build succeeded.`

- [ ] **Step 5: Commit**

```bash
git add src/SaveForge.App/Views/HomeView.axaml \
        src/SaveForge.App/Views/HomeView.axaml.cs \
        src/SaveForge.App/Themes/DarkTheme.axaml
git commit -m "feat(app): rewrite HomeView with sidebar plugin list and save discovery panel"
```

---

## Task 9: Wire DI and final build

**Files:**
- Modify: `src/SaveForge.App/App.axaml.cs`

- [ ] **Step 1: Register `ISaveDiscoveryService` and update `HomeViewModel` factory**

Replace the `HomeViewModel` registration block and add the discovery service registration in `src/SaveForge.App/App.axaml.cs`. The `BuildServices` method becomes:

```csharp
    private static ServiceProvider BuildServices()
    {
        var services = new ServiceCollection();

        // Logging
        services.AddLogging(b => b.AddConsole().SetMinimumLevel(LogLevel.Information));

        // Settings (loaded synchronously at startup)
        var settingsService = new SettingsService(AppSettingsDefaults.SettingsPath);
        var settings = settingsService.LoadAsync().GetAwaiter().GetResult();
        services.AddSingleton(settings);
        services.AddSingleton(settingsService);

        // Infrastructure
        services.AddSingleton<HttpClient>();
        services.AddSingleton<HttpDownloadService>();
        services.AddSingleton<IBackupService>(sp =>
            new FileBackupService(settings.BackupDirectory));
        services.AddSingleton<ISaveDiscoveryService, SaveDiscoveryService>();

        // Plugin Host
        services.AddSingleton<PluginRegistry>();
        services.AddSingleton<IPluginRegistry>(sp => sp.GetRequiredService<PluginRegistry>());
        services.AddSingleton<PluginLoader>(sp =>
            new PluginLoader(sp.GetRequiredService<ILogger<PluginLoader>>(), "1.0.0"));
        services.AddSingleton<PluginHostService>();
        services.AddSingleton<IFileDetectionService, FileDetectionService>();

        // Store
        services.AddSingleton<ICatalogService>(sp =>
        {
            var registry = sp.GetRequiredService<IPluginRegistry>();
            var installedIds = new HashSet<string>(registry.GetAll().Select(p => p.Manifest.Id));
            return new CatalogService(
                sp.GetRequiredService<HttpDownloadService>(),
                settings.CatalogUrl,
                AppSettingsDefaults.CatalogCachePath,
                installedIds);
        });
        services.AddSingleton<IPluginInstaller>(sp =>
            new PluginInstaller(
                sp.GetRequiredService<HttpDownloadService>(),
                sp.GetRequiredService<ICatalogService>(),
                sp.GetRequiredService<PluginRegistry>(),
                sp.GetRequiredService<PluginLoader>(),
                sp.GetRequiredService<ILogger<PluginInstaller>>(),
                settings.PluginDirectory));
        services.AddSingleton<IUpdateChecker>(sp =>
            new UpdateChecker(
                sp.GetRequiredService<ICatalogService>(),
                sp.GetRequiredService<IPluginRegistry>()));

        // App ViewModels
        services.AddSingleton<StatusBarViewModel>();
        services.AddSingleton<InfoBarViewModel>();
        services.AddSingleton<AboutViewModel>();
        services.AddSingleton<HomeViewModel>(sp =>
            new HomeViewModel(
                sp.GetRequiredService<IPluginRegistry>(),
                sp.GetRequiredService<ISaveDiscoveryService>(),
                sp.GetRequiredService<AppSettings>(),
                sp.GetRequiredService<SettingsService>(),
                page => sp.GetRequiredService<MainViewModel>().NavigateToCommand.Execute(page),
                async path =>
                {
                    var detection = sp.GetRequiredService<IFileDetectionService>();
                    var results = await detection.DetectAsync(path);
                    if (results.Count == 0) return null;
                    var doc = await results[0].Plugin.OpenAsync(path);
                    var editor = new EditorViewModel(
                        doc,
                        sp.GetRequiredService<IBackupService>(),
                        sp.GetRequiredService<StatusBarViewModel>());
                    sp.GetRequiredService<MainViewModel>().OpenEditor(editor);
                    return editor;
                }));
        services.AddSingleton<StoreViewModel>(sp =>
            new StoreViewModel(
                sp.GetRequiredService<ICatalogService>(),
                sp.GetRequiredService<IPluginInstaller>()));
        services.AddSingleton<SettingsViewModel>(sp =>
            new SettingsViewModel(
                settings,
                sp.GetRequiredService<SettingsService>(),
                sp.GetRequiredService<IUpdateChecker>(),
                sp.GetRequiredService<StatusBarViewModel>(),
                sp.GetRequiredService<InfoBarViewModel>()));
        services.AddSingleton<MainViewModel>();

        return services.BuildServiceProvider();
    }
```

Also add the missing `using` at the top of the file:

```csharp
using SaveForge.Infrastructure.Services;
```

(It may already be there — add only if missing.)

- [ ] **Step 2: Build the full solution**

```bash
dotnet build
```

Expected: `Build succeeded.` with 0 errors.

- [ ] **Step 3: Run all tests**

```bash
dotnet test tests/SaveForge.Tests/
```

Expected: all tests pass, 0 failures.

- [ ] **Step 4: Commit**

```bash
git add src/SaveForge.App/App.axaml.cs
git commit -m "feat(app): wire ISaveDiscoveryService and updated HomeViewModel into DI"
```

---

## Self-Review

**Spec coverage check:**

| Spec requirement | Task |
|---|---|
| Plugin declares `DefaultSaveDirectory` via `SaveCapabilities` | Task 1 |
| `PluginSaveDirectories` persisted in `AppSettings` | Task 2 |
| `SaveFileInfo` record | Task 2 |
| `ISaveDiscoveryService` interface | Task 2 |
| `PathTokenExpander` with `%APPDATA%`, `%LOCALAPPDATA%`, `%USERPROFILE%`, `%TEMP%`, `~` | Task 3 |
| `SaveDiscoveryService` recursive scan, extension filter, sorted by date | Task 4 |
| Per-entry errors skipped silently | Task 4 |
| PlanetCrafter default path | Task 5 |
| Stardew sample default path | Task 5 |
| `PluginListItem` sidebar wrapper | Task 6 |
| `HomeViewModel` restructure — all properties and commands from spec | Task 7 |
| Path resolution priority: settings > default | Task 7 |
| Rescan on plugin select and on path confirm | Task 7 |
| `HomeView` sidebar + detail layout | Task 8 |
| Path-not-configured state | Task 8 |
| Save list with filename + date | Task 8 |
| Inline path edit with folder picker | Task 8 |
| Dimmed save list while editing | Task 8 |
| DI registration | Task 9 |
| Error handling: missing dir → empty list | Task 4 step 3 |
| Error handling: empty path → not configured | Task 7 `OnSaveDirectoryChanged` |
| Settings save failure non-fatal | Task 7 `ConfirmPathAsync` try/catch |
| `PathTokenExpander` unit tests | Task 3 |
| `SaveDiscoveryService` integration tests | Task 4 |
| `HomeViewModel` unit tests | Task 7 |
