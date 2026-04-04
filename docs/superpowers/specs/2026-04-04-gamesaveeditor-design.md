# SaveForge — Design Specification

**Date:** 2026-04-04  
**Status:** Approved  
**Project:** SaveForge / SaveForge

---

## 1. Overview

SaveForge is a cross-platform desktop application for editing local single-player game save files. It is built on .NET 8 + Avalonia UI with a plugin architecture: the core app provides the shell, plugin management, store/catalog, backup, and generic form rendering. Each game-specific plugin provides save parsing/serialization plus either a schema-driven or custom editor UI.

Plugin authors can add support for new games without modifying the core app.

---

## 2. Technology Choices

| Concern | Choice |
|---|---|
| Runtime | .NET 8 |
| UI framework | Avalonia UI |
| Language | C# with nullable reference types enabled |
| Architecture | MVVM — CommunityToolkit.Mvvm (source generators for commands and observable properties) |
| Plugin loading | AssemblyLoadContext + AssemblyDependencyResolver (one context per plugin) |
| Serialization | System.Text.Json |
| HTTP | System.Net.Http.HttpClient |
| Future updates | Velopack (shell structured to support it; not wired in initial build) |
| Native AOT | Disabled (required for dynamic plugin loading) |

---

## 3. Non-Goals

- Online multiplayer cheating or anti-cheat bypasses
- Memory editing, process injection, kernel drivers
- Cloud accounts, telemetry, or any server backend
- Sandboxing beyond normal AssemblyLoadContext isolation
- Plugin sandboxing (plugins are treated as trusted code)

---

## 4. Solution & Project Layout

```
SaveForge.sln
│
├── src/
│   ├── SaveForge.App/                        # Avalonia startup, DI root, shell window
│   ├── SaveForge.Core/                       # Domain models, interfaces, business logic
│   ├── SaveForge.SDK/                        # Public plugin contract (interfaces + DTOs only)
│   ├── SaveForge.PluginHost/                 # AssemblyLoadContext loader, plugin registry
│   ├── SaveForge.Store/                      # Catalog fetch, cache, install/remove/update
│   ├── SaveForge.Infrastructure/             # File I/O, backup, HTTP, settings persistence
│   ├── SaveForge.Security/                   # SHA-256 verification, manifest validation
│   ├── SaveForge.Forms/                      # Schema-driven Avalonia form renderer
│   └── SaveForge.SamplePlugin.StardewLike/   # Reference plugin
│
├── tests/
│   └── SaveForge.Tests/
│
└── docs/
    ├── ARCHITECTURE.md
    ├── PLUGIN_AUTHORING.md
    └── CATALOG_SCHEMA.md
```

### Dependency Rules

```
App           → Core, PluginHost, Store, Infrastructure, Forms
PluginHost    → Core, SDK
Store         → Core, Infrastructure, Security
Forms         → SDK
SamplePlugin  → SDK only   (plugins never reference Core)
Tests         → all
```

The SDK has zero dependencies on any other project in this solution. It can be published as a standalone NuGet package for third-party plugin authors.

---

## 5. Core Domain Model

### SDK Types (public, visible to plugin authors)

```csharp
// Plugin entry point
interface IGameEditorPlugin
{
    PluginManifest Manifest { get; }
    SaveCapabilities Capabilities { get; }
    (bool CanOpen, int Priority) CanOpen(FileInfo file, ReadOnlySpan<byte> header);
    Task<ISaveDocument> OpenAsync(string filePath, CancellationToken ct);
}

// A loaded save file
interface ISaveDocument
{
    string FilePath { get; }
    bool IsDirty { get; }
    object? GetValue(string dotPath);
    void SetValue(string dotPath, object? value);
    IEditorDefinition GetEditorDefinition();
    Task SaveAsync(CancellationToken ct);
}

// Discriminates schema-driven vs custom UI
enum EditorKind { Schema, Custom }

// Decides schema vs custom UI
interface IEditorDefinition
{
    EditorKind Kind { get; }           // Schema | Custom
    FormSchema? Schema { get; }        // set when Kind == Schema
    ICustomEditorViewFactory? ViewFactory { get; }  // set when Kind == Custom
}

// Escape hatch for advanced plugins
interface ICustomEditorViewFactory
{
    Control CreateView(ISaveDocument document);
    object CreateViewModel(ISaveDocument document);
}
```

**Schema types:**

```
PluginManifest        id, name, version (SemVer string), author, description,
                      minHostVersion, maxHostVersion, pluginAssembly

SaveCapabilities      fileExtensions (string[]), mimeHints (string[]),
                      detectionPriority (int, default 0)

FormSchema            pages: FormPage[]
FormPage              title: string, sections: FormSection[]
FormSection           heading: string?, fields: FieldDefinition[]

FieldDefinition (abstract base)
  — key: string (dot-path into ISaveDocument)
  — label: string
  — description: string?
  — isReadOnly: bool
  — isRequired: bool
  — visibilityRule: VisibilityRule?

  Concrete subtypes:
  TextField           maxLength?, regexPattern?
  NumericField        min?, max?, decimalPlaces
  CheckboxField       (no extras)
  EnumField           options: EnumOption[] (value + display)
  ReadOnlyField       (no extras)
  ListField           itemSchema: FieldDefinition
  ObjectSectionField  fields: FieldDefinition[]

VisibilityRule        dependsOnKey: string, operator: Equals|NotEquals|GreaterThan|LessThan, value: object
```

### Core Internal Types (not exposed to plugins)

```
InstalledPlugin       manifest, loadState (Loaded|Failed|Incompatible), assemblyPath, failureReason?
CatalogEntry          all InstalledPlugin fields + packageUrl, sha256, iconUrl, screenshotUrls[],
                      tags[], repositoryUrl, isInstalled, hasUpdate
PluginLoadResult      Success(IGameEditorPlugin) | Failed(reason) | Incompatible(reason)
BackupRecord          originalPath, backupPath, timestamp
AppSettings           catalogUrl, pluginDirectory, backupDirectory, checkUpdatesOnStartup,
                      theme (Dark | Light — Dark default)

IPluginRegistry       RegisterPlugin(LoadedPlugin), RemovePlugin(string id),
                      GetAll() → IReadOnlyList<LoadedPlugin>,
                      TryGet(string id) → LoadedPlugin?
```

---

## 6. Plugin Loading

### AssemblyLoadContext Per Plugin

```
PluginLoadContext : AssemblyLoadContext
  — constructed with AssemblyDependencyResolver(pluginAssemblyPath)
  — overrides Load(): try resolve via resolver first, then fall back to Default
  — LoadUnmanagedDll(): same pattern
  — collectible = true (enables future unload support)
```

The `PluginHost` service:
1. Scans `plugins/` directory on startup
2. Parses each `manifest.json`
3. Checks host version compatibility
4. Creates a `PluginLoadContext`, loads the assembly, resolves `IGameEditorPlugin` via reflection
5. Stores result in `IPluginRegistry`
6. Failed/incompatible plugins are recorded but do not block the rest

### Plugin Directory Layout

```
%AppData%/SaveForge/
├── plugins/
│   └── {plugin-id}/
│       └── {version}/
│           ├── manifest.json
│           ├── PluginAssembly.dll
│           ├── PluginAssembly.deps.json
│           └── assets/
├── backups/
│   └── {plugin-id}/
│       └── {filename}.{timestamp}.bak
├── catalog-cache.json
└── settings.json
```

### File Detection

```
foreach plugin in registry:
    (canOpen, priority) = plugin.CanOpen(file, header)

candidates sorted descending by priority
if count == 1  → use it
if count > 1   → prompt user to choose
if count == 0  → show "No plugin found" message
```

Plugins receive the first 512 bytes as `header` for lightweight sniffing without full parse.

---

## 7. Plugin Lifecycle

### Install

1. Download zip to temp file (progress reported via `IProgress<long>`)
2. Verify SHA-256 against catalog entry (`Security` project)
3. Unpack to `plugins/{id}/{version}/`
4. Parse and validate `manifest.json`
5. Check host version compatibility
6. Load into new `PluginLoadContext`
7. Register in `IPluginRegistry`
8. UI refreshes

### Remove

1. Unload `PluginLoadContext` (mark for GC)
2. Delete `plugins/{id}/{version}/` directory
3. Remove from registry

### Update

1. Install new version (steps above)
2. If successful, remove old version directory
3. Reload in UI

### Backup (before every save-file write)

```
1. Determine backup path:
   %AppData%/SaveForge/backups/{plugin-id}/{filename}.{ISO8601-compact}.bak
2. Copy original file to backup path
3. Write new content to a temp file alongside the original
4. Atomic rename temp → original
5. If rename fails: delete temp, original is untouched
6. Surface backup path to user in status bar
```

---

## 8. Store / Catalog

### Catalog Manifest JSON (hosted at configured URL)

```json
{
  "schemaVersion": "1.0",
  "generatedAt": "2026-04-04T00:00:00Z",
  "plugins": [
    {
      "id": "stardew-like",
      "name": "StardewLike Save Editor",
      "description": "Edit StardewLike game saves.",
      "author": "Sample Author",
      "latestVersion": "1.0.0",
      "minHostVersion": "1.0.0",
      "maxHostVersion": null,
      "packageUrl": "https://example.com/plugins/stardew-like-1.0.0.zip",
      "sha256": "abc123...",
      "iconUrl": "https://example.com/plugins/stardew-like-icon.png",
      "screenshotUrls": [],
      "tags": ["rpg", "farming"],
      "repositoryUrl": "https://github.com/example/stardew-like-plugin"
    }
  ]
}
```

### Catalog URL Strategy

| Build config | Default catalog URL |
|---|---|
| Debug / Development | `file:///{AppDataDir}/catalog.json` (bundled local file) |
| Release | Configurable GitHub raw URL (set in `appsettings.json`, overridable in Settings UI) |

### Catalog Service Behavior

- Fetches JSON from configured URL
- Caches to `catalog-cache.json` with a `fetchedAt` timestamp
- Cache is used if fetch fails (with a visible "using cached catalog" notice)
- Compares catalog entries against installed plugins to compute `isInstalled` and `hasUpdate`

---

## 9. Avalonia Shell UI

### Window Chrome

- `ExtendClientAreaToDecorationsHint = true`
- `ExtendClientAreaChromeHints = NoChrome`
- Custom `TitleBarControl`: drag region, app icon, title text, minimize/maximize/close buttons
- Fully dark-themed; button hover/press states styled to match palette

### Dark Theme Palette

| Token | Value |
|---|---|
| Background | `#1a1a2e` |
| Surface | `#16213e` |
| Surface elevated | `#0f3460` |
| Accent | `#e94560` |
| Text primary | `#eaeaea` |
| Text secondary | `#8892a4` |
| Border | `#2a2a4a` |
| Danger | `#ff4757` |
| Success | `#2ed573` |

### Navigation

Left sidebar with icon + label. `MainViewModel.CurrentPage` drives content region via `DataTemplate` switching. No third-party routing library.

### Pages

| Page | Purpose |
|---|---|
| `HomePage` | Grid of installed plugin cards; "Open File" per plugin; empty state with "Browse Store" CTA |
| `StorePage` | Searchable/filterable catalog grid; Install/Installed/Update-available badges |
| `PluginDetailPage` | Full entry: description, screenshots, author, versions, install/remove/update |
| `SettingsPage` | Catalog URL, plugin dir, backup dir, "Check for updates" button |
| `AboutPage` | App version, host version, changelog link |
| `EditorPage` | Hosts `FormRendererView` or custom plugin control for an open save file |

### Update Check UX

1. Shell renders fully
2. Background `Task` starts update check
3. Status bar (bottom of window) shows subtle spinner + "Checking for plugin updates…"
4. **No updates:** status bar clears silently
5. **Updates found:** non-modal `InfoBar` at top of content: "Updates available for N plugin(s) — Review" with dismiss button
6. **Error:** status bar shows "Update check failed" + "Retry" link; app fully usable

---

## 10. Form Renderer

`FormRendererViewModel` takes `FormSchema` + `ISaveDocument` and constructs a reactive view model tree. `FormRendererView` renders it via `ItemsControl` + `DataTemplate` per field type.

| FieldDefinition | Avalonia Control |
|---|---|
| `TextField` | `TextBox` |
| `NumericField` | `NumericUpDown` (min/max bound) |
| `CheckboxField` | `ToggleSwitch` |
| `EnumField` | `ComboBox` |
| `ReadOnlyField` | `TextBlock` |
| `ListField` | `ItemsControl` + add/remove buttons + recursive template |
| `ObjectSectionField` | `GroupBox` with recursive field rendering |

- Dot-path binding: `ISaveDocument.GetValue` / `SetValue` called on field change
- Visibility rules evaluated reactively; fields show/hide as document values change
- Validation errors shown inline below each field
- Save button enabled only when `IsDirty && !HasValidationErrors`

---

## 11. Sample Plugin — StardewLike

**Detection:** `.json` extension AND first 256 bytes contain `"player"` key.

**Save format:**
```json
{
  "player": {
    "name": "Farmer",
    "level": 5,
    "money": 1000,
    "stats": { "health": 100, "energy": 80 }
  },
  "inventory": [
    { "id": "wood", "count": 50 }
  ]
}
```

**Schema fields:** `player.name` (required, max 50), `player.level` (1–100), `player.money` (≥0), `player.stats.health` (0–100), `player.stats.energy` (0–100), `inventory` (list of item objects).

**Implementation:**
- `StardewSaveDocument : ISaveDocument` — backed by `JsonNode` for flexible get/set
- `StardewPlugin : IGameEditorPlugin` — returns `EditorKind.Schema`
- Load: `JsonNode.Parse(await File.ReadAllTextAsync(path, ct))`
- Save: `await File.WriteAllTextAsync(path, node.ToJsonString(prettyPrint), ct)`
- Validation: enforced in `SetValue` and surfaced via `FormRendererViewModel`

---

## 12. Security

- SHA-256 of downloaded zip verified against catalog `sha256` field before unpacking
- `manifest.json` inside the zip is validated against expected schema before loading
- Compatibility check: host version must be within plugin's `minHostVersion`..`maxHostVersion`
- No code signing required in v1; clear display of plugin metadata so users can make informed choices
- Plugins run as trusted code within the host process (no sandbox beyond LoadContext isolation)

---

## 13. Testing

| Test | Project | What is covered |
|---|---|---|
| Manifest parsing | Tests | Valid + invalid manifest JSON |
| Compatibility check | Tests | Version boundary conditions |
| SHA-256 verification | Tests | Correct hash passes, tampered zip fails |
| Plugin install | Tests | Happy path + network error + bad hash |
| Plugin loading | Tests | LoadContext isolation, failed assembly |
| File matching | Tests | Extension match, header sniff, multi-plugin tie |
| Backup creation | Tests | File copied, atomic rename, failure preservation |
| StardewLike roundtrip | Tests | Load JSON → mutate → save → reload → values match |
| Form schema binding | Tests | GetValue/SetValue via dot-path |
| Visibility rules | Tests | Rule evaluation logic |

---

## 14. Developer Documentation

| File | Audience | Content |
|---|---|---|
| `docs/ARCHITECTURE.md` | Contributors | Project map, dependency rules, key flows |
| `docs/PLUGIN_AUTHORING.md` | Plugin authors | Step-by-step: define model, define schema, implement load/save |
| `docs/CATALOG_SCHEMA.md` | Catalog maintainers | Full JSON schema for `catalog.json` |

---

## 15. Decisions & Rationale

| Decision | Rationale |
|---|---|
| SDK has no dependencies | Plugin authors can reference it as a standalone NuGet without pulling in core internals |
| One AssemblyLoadContext per plugin | Dependency isolation; prevents version conflicts between plugins |
| Atomic rename for save | Guarantees original file is never partially overwritten |
| No Native AOT | Required to support dynamic plugin assembly loading |
| Dark-first custom chrome | Target audience (gamers) expects dark UI; custom chrome gives full brand control |
| Post-load background update check | App is fully usable immediately; update check failure never blocks the user |
| Local catalog.json for dev | No live server needed during development or testing |
| Schema-driven forms as primary path | Keeps plugin authoring minimal: define model + schema + load/save = done |
