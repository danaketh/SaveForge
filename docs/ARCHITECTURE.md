# Architecture

## Project Map

| Project | Role |
|---|---|
| `SaveForge.SDK` | Public plugin contract — interfaces and DTOs only. No dependencies. |
| `SaveForge.Core` | Internal domain models and service interfaces. |
| `SaveForge.Infrastructure` | File I/O, backup, HTTP download, settings persistence. |
| `SaveForge.Security` | SHA-256 verification, manifest validation. |
| `SaveForge.PluginHost` | AssemblyLoadContext per plugin, registry, file detection. |
| `SaveForge.Store` | Catalog fetch/cache, plugin install/remove/update. |
| `SaveForge.Forms` | Schema-driven Avalonia form renderer. |
| `SaveForge.App` | Avalonia shell, DI root, all pages. |
| `SaveForge.SamplePlugin.StardewLike` | Reference plugin — schema-driven. |
| `SaveForge.Tests` | xUnit tests for all non-UI components. |

## Dependency Rules

```
App → Core, PluginHost, Store, Infrastructure, Forms
PluginHost → Core, SDK
Store → Core, Infrastructure, Security
Forms → SDK
SamplePlugin → SDK only
```

## Key Flows

### Plugin Load on Startup
1. `App.axaml.cs` calls `PluginHostService.LoadAll(pluginsDir)`
2. Scanner walks `{pluginsDir}/{id}/{version}/`
3. `PluginLoader.Load(dir)` → parses manifest → checks compatibility → loads via `PluginLoadContext`
4. Results stored in `PluginRegistry`

### Open Save File
1. User picks a file via OS dialog
2. `FileDetectionService.DetectAsync(path)` — all loaded plugins vote
3. Highest priority plugin wins (tie → user prompted)
4. `plugin.OpenAsync(path)` → `ISaveDocument`
5. `ISaveDocument.GetEditorDefinition()` → `EditorKind.Schema` or `Custom`
6. `MainViewModel.OpenEditor(new EditorViewModel(doc, ...))`

### Save with Backup
1. `EditorViewModel.SaveAsync()`
2. Copies original to `.bak` file in backup directory
3. Calls `ISaveDocument.SaveAsync()` which writes new content to `.tmp`, renames `.tmp` → original
4. If rename fails: `.tmp` deleted, original untouched

### Plugin Install
1. Catalog entry looked up by ID
2. Zip downloaded to temp file
3. SHA-256 verified
4. Zip extracted to `{pluginsDir}/{id}/{version}/`
5. Loaded via `PluginLoader` → registered in `PluginRegistry`

## Update Check UX
- Runs after `MainWindow.OnOpened` fires
- Status bar shows "Checking…" while running
- On success: clears silently (no updates) or shows `InfoBar` (updates found)
- On error: status bar shows retry message — app remains fully usable
