# Plugin Save Discovery — Design Spec

**Date:** 2026-04-11  
**Status:** Approved

## Overview

Each plugin declares a default directory where its game stores save files, using environment-variable tokens (`%APPDATA%`, `~`, etc.). The user can override this path per plugin; the configured path is persisted in the app's settings file. On the Home screen, SaveForge scans the configured directory recursively and presents a list of matching save files. Clicking a save opens it in the editor. If no path is configured or the path does not exist on disk, the UI prompts the user to set one before showing any list.

---

## 1. Data Model

### 1.1 `SaveCapabilities` (SDK — `SaveForge.SDK`)

Add one nullable property:

```csharp
public string? DefaultSaveDirectory { get; init; }
```

Plugin authors set this to the typical save location using token syntax, e.g.:

```csharp
DefaultSaveDirectory = @"%APPDATA%\MijuGames\The Planet Crafter\Saves"
```

The property is optional. Plugins that don't set it show the "path not configured" state on first run, same as if the user cleared a previously set path.

### 1.2 `AppSettings` (Core — `SaveForge.Core.Models`)

Add one property:

```csharp
public Dictionary<string, string> PluginSaveDirectories { get; set; } = [];
```

Keyed by plugin ID (e.g. `"planet-crafter"`). When a user confirms a path for a plugin, it is written here and flushed to `settings.json` via `SettingsService.SaveAsync`. The configured path takes precedence over `DefaultSaveDirectory`; if no entry exists for a plugin, the default is used as the initial value in the edit field.

### 1.3 `SaveFileInfo` record (Core — `SaveForge.Core.Models`)

```csharp
public sealed record SaveFileInfo(
    string FilePath,
    string FileName,
    DateTimeOffset LastModified,
    long SizeBytes);
```

One instance per file found during discovery. Displayed in the save list: filename, last-modified date, human-readable size.

### 1.4 `ISaveDiscoveryService` interface (Core — `SaveForge.Core.Interfaces`)

```csharp
public interface ISaveDiscoveryService
{
    Task<IReadOnlyList<SaveFileInfo>> DiscoverAsync(
        string directory,
        SaveCapabilities capabilities,
        CancellationToken ct = default);
}
```

Returns all save files found under `directory` matching the plugin's declared extensions, sorted by `LastModified` descending (most recent first). Returns an empty list — never throws — if the directory does not exist or is unreadable.

---

## 2. Infrastructure

### 2.1 `PathTokenExpander` (`SaveForge.Infrastructure`)

Static helper class. Expands the following tokens in a path string:

| Token | Expands to |
|---|---|
| `%APPDATA%` | `Environment.GetFolderPath(SpecialFolder.ApplicationData)` |
| `%LOCALAPPDATA%` | `Environment.GetFolderPath(SpecialFolder.LocalApplicationData)` |
| `%USERPROFILE%` | `Environment.GetFolderPath(SpecialFolder.UserProfile)` |
| `%TEMP%` | `Path.GetTempPath()` |
| `~` (leading) | `Environment.GetFolderPath(SpecialFolder.UserProfile)` |

Unrecognised tokens are left as-is so the user sees them verbatim rather than silently losing data.

```csharp
public static class PathTokenExpander
{
    public static string Expand(string path);
}
```

### 2.2 `SaveDiscoveryService` (`SaveForge.Infrastructure`)

Implements `ISaveDiscoveryService`.

- Expands the `directory` argument via `PathTokenExpander.Expand`.
- Returns an empty list immediately if the resolved directory does not exist.
- Calls `Directory.EnumerateFiles(resolvedDir, "*", SearchOption.AllDirectories)`.
- Filters entries whose extension (case-insensitive) appears in `capabilities.FileExtensions`.
- Per-entry errors (access denied, file vanished mid-scan) are caught and skipped — not surfaced to the caller.
- Builds `SaveFileInfo` from `FileInfo`: `FullName`, `Name`, last-write time as `DateTimeOffset`, `Length`.
- Returns results sorted by `LastModified` descending.

---

## 3. ViewModel

### 3.1 `HomeViewModel` restructure (`SaveForge.App`)

The constructor gains `ISaveDiscoveryService` and `SettingsService` dependencies (both already registered in the DI container).

**New observable properties:**

| Property | Type | Purpose |
|---|---|---|
| `SelectedPlugin` | `InstalledPlugin?` | Drives the right panel; auto-set to first loaded plugin on `Refresh()` |
| `SaveDirectory` | `string?` | Raw token path for selected plugin (settings entry, falling back to `DefaultSaveDirectory`) |
| `ResolvedSaveDirectory` | `string?` | Expanded path, recomputed on every `SaveDirectory` change via `PathTokenExpander` |
| `IsPathConfigured` | `bool` | True when `SaveDirectory` is non-null and resolved path exists on disk |
| `IsEditingPath` | `bool` | True while the inline edit field is visible |
| `EditingPath` | `string?` | Live value in the text field |
| `Saves` | `ObservableCollection<SaveFileInfo>` | Current save list |
| `IsLoadingSaves` | `bool` | Spinner guard during discovery |

**New commands:**

| Command | Parameter | Behaviour |
|---|---|---|
| `SelectPluginCommand` | `InstalledPlugin` | Sets `SelectedPlugin`, loads its path, triggers rescan |
| `StartEditPathCommand` | — | Sets `IsEditingPath = true`, seeds `EditingPath` from `SaveDirectory` |
| `ConfirmPathCommand` | — | Persists `EditingPath` to `AppSettings.PluginSaveDirectories`, saves settings, sets `IsEditingPath = false`, triggers rescan |
| `CancelEditPathCommand` | — | Sets `IsEditingPath = false`, discards `EditingPath` |
| `BrowseForPathCommand` | — | Calls `RequestFolderPath` delegate; on non-null result sets `EditingPath` |
| `OpenSaveCommand` | `SaveFileInfo` | Calls existing `_openEditor` with `saveFileInfo.FilePath` |
| `RefreshSavesCommand` | — | Re-runs discovery for current plugin/path |

**`RequestFolderPath` delegate:**

```csharp
public Func<Task<string?>>? RequestFolderPath { get; set; }
```

Set by `HomeView.axaml.cs` on `DataContextChanged`. Calls `StorageProvider.OpenFolderPickerAsync` and returns the selected folder's local path, or `null` if cancelled. This mirrors the existing file-picker pattern in the codebase.

**Path resolution logic** (runs whenever `SelectedPlugin` changes or settings are confirmed):

1. Look up plugin ID in `AppSettings.PluginSaveDirectories`.
2. If found, use that value; otherwise fall back to `plugin.Plugin?.Capabilities.DefaultSaveDirectory`.
3. Set `SaveDirectory` and recompute `ResolvedSaveDirectory`.
4. If `IsPathConfigured`, kick off `RefreshSavesCommand`.

**Rescan logic:**

Runs `ISaveDiscoveryService.DiscoverAsync(ResolvedSaveDirectory, plugin.Plugin!.Capabilities)`, updates `Saves`. Sets `IsLoadingSaves` around the call. Runs on a background thread; marshals results back to the UI thread.

The existing `OpenFileCommand` (manual file picker) is removed — saves are opened exclusively through the save list.

---

## 4. UI

### 4.1 `HomeView.axaml` — full rewrite

Layout: two-column `Grid` (left sidebar fixed at 200 px, right panel fills remaining space).

**Left sidebar — plugin list**

`ItemsControl` over `InstalledPlugins`. Each item is a `Button` styled as a nav item:

- **Name** (primary text)
- **Subtitle** (secondary text): save count (e.g. "3 saves"), "path not set", or "load failed"
- Disabled and grey when `LoadState != Loaded`
- Selected item highlighted with an accent left border
- `Command` bound to `SelectPluginCommand`, `CommandParameter` bound to the item

**Right panel — three mutually exclusive states** (switched via `IsVisible` bindings):

1. **Path not configured** (`!IsPathConfigured && !IsEditingPath`): centred `📁` icon, "No save path configured" heading, explanatory line, "Set Path" button that invokes `StartEditPathCommand`.

2. **Save list** (`IsPathConfigured && !IsEditingPath`):
   - Plugin name header
   - Path row: token string left-aligned, `✏️` button right-aligned (invokes `StartEditPathCommand`)
   - Resolved path hint below (`→ C:\Users\...`) bound to `ResolvedSaveDirectory`
   - Save count label
   - `ItemsControl` over `Saves` — each row: filename (primary), date + size (secondary), full row clickable via `OpenSaveCommand`
   - "No saves found" empty state when `Saves.Count == 0`

3. **Editing path** (`IsEditingPath`):
   - Plugin name header
   - Inline edit row: `TextBox` bound to `EditingPath`, `📂` folder-browse button, `✓` confirm button, `✕` cancel button
   - Resolved path hint below (live, updates as `EditingPath` changes)
   - Save list dimmed (opacity 0.4, `IsHitTestVisible="False"`) so context is preserved but interaction is blocked

### 4.2 `HomeView.axaml.cs`

- Removes `OpenFileButton_Click` handler
- Sets `RequestFolderPath` on `DataContextChanged` using `StorageProvider.OpenFolderPickerAsync`

---

## 5. Plugin Updates

### `PlanetCrafterPlugin`

```csharp
public SaveCapabilities Capabilities { get; } = new()
{
    FileExtensions = [".json"],
    DetectionPriority = 20,
    DefaultSaveDirectory = @"%APPDATA%\MijuGames\The Planet Crafter\Saves"
};
```

### `StardewPlugin` (sample)

```csharp
DefaultSaveDirectory = @"%APPDATA%\StardewValley\Saves"
```

---

## 6. DI Registration (`App.axaml.cs`)

```csharp
services.AddSingleton<ISaveDiscoveryService, SaveDiscoveryService>();
```

`HomeViewModel` factory updated to inject `ISaveDiscoveryService` and `SettingsService`. The `navigateToStore` stub is already fixed; the `openEditor` callback remains as-is (now called from `OpenSaveCommand` instead of a manual file picker).

---

## 7. Error Handling

| Scenario | Behaviour |
|---|---|
| `SaveDirectory` is null or empty | Treated as not configured; "path not configured" state shown |
| `SaveDirectory` is set but resolved path does not exist on disk | `IsPathConfigured = false`; "path not configured" state shown |
| Directory exists but no matching files | Discovery returns empty list; "no saves found" state shown |
| Access denied on a subdirectory | Entry skipped silently; rest of scan continues |
| Path resolves to empty string | Treated as not configured |
| Settings save fails | Log warning; UI not blocked (in-memory state is still correct) |

---

## 8. Testing

- `PathTokenExpander`: unit tests for each token, unknown tokens left intact, empty string input, path with no tokens.
- `SaveDiscoveryService`: integration tests against a temporary directory tree — matching extensions found recursively, non-matching extensions excluded, missing directory returns empty, access errors on subdirs skipped.
- `HomeViewModel`: unit tests for path resolution priority (settings > default), `ConfirmPathCommand` persists and rescans, `CancelEditPathCommand` discards changes, `SelectPluginCommand` switches state correctly.
