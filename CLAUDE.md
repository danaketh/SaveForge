# SaveForge — Claude Context

## Project Overview

SaveForge is a cross-platform Avalonia desktop application for editing local single-player game save files through a plugin system. Users install plugins (one per game) that know how to parse/serialize a specific save format and provide a schema-driven or custom Avalonia UI for editing.

## Tech Stack

- **.NET 8**, **C#**, `LangVersion=latest`, nullable enabled
- **Avalonia 11** — cross-platform UI (WinExe output)
- **CommunityToolkit.Mvvm** — MVVM pattern (ViewModels)
- **Microsoft.Extensions.DependencyInjection / Logging** — DI container and structured logging
- **xUnit** — tests (in `tests/SaveForge.Tests/`)

## Solution Layout

```
SaveForge.sln
src/
  SaveForge.SDK           # Public plugin contract — interfaces/DTOs, no logic deps
  SaveForge.Core          # Internal domain models, service interfaces
  SaveForge.Infrastructure # File I/O, backup, HTTP download, settings persistence
  SaveForge.Security      # SHA-256 verification, manifest validation
  SaveForge.PluginHost    # AssemblyLoadContext per plugin, registry, file detection
  SaveForge.Store         # Catalog fetch/cache, plugin install/remove/update
  SaveForge.Forms         # Schema-driven Avalonia form renderer
  SaveForge.App           # Avalonia shell, DI root, all pages/viewmodels
  SaveForge.Plugin.PlanetCrafter      # Plugin: Planet Crafter save editor
  SaveForge.SamplePlugin.StardewLike  # Reference plugin (schema-driven pattern)
tests/
  SaveForge.Tests         # xUnit integration + unit tests for all non-UI components
```

### Dependency Rules (enforced by architecture, not tooling)

```
App → Core, PluginHost, Store, Infrastructure, Forms
PluginHost → Core, SDK
Store → Core, Infrastructure, Security
Forms → SDK
Plugins → SDK only
```

## Build and Run

```bash
# Full solution
dotnet restore
dotnet build

# Run the app
dotnet run --project src/SaveForge.App/

# Tests
dotnet test tests/SaveForge.Tests/
```

## Dev Mode: Plugin Development

In `Debug` configuration, `AppSettings.PluginDirectory` resolves to
`{AppContext.BaseDirectory}/_devplugins/` (next to the compiled executable).

Any plugin project with `<IsDevPlugin>true</IsDevPlugin>` in its csproj has a
`CopyToDevPlugins` MSBuild target (defined in `Directory.Build.targets`) that
copies its built DLL + `manifest.json` into that directory automatically on build.

Workflow:
```bash
# Build everything — all dev plugins are available
dotnet build

# Build only one plugin — only that plugin loads (fast iteration)
dotnet build src/SaveForge.Plugin.PlanetCrafter/

# Then run the app — it picks up whatever is in _devplugins/
dotnet run --project src/SaveForge.App/
```

The `_devplugins/` folder lives inside `src/SaveForge.App/bin/Debug/net8.0/` and
is git-ignored. In Release builds, the app uses the user-profile plugin directory
(`%APPDATA%/SaveForge/plugins/`) as normal.

Current dev plugins (set `<IsDevPlugin>true</IsDevPlugin>` in their csproj):
- `SaveForge.Plugin.PlanetCrafter`
- `SaveForge.SamplePlugin.StardewLike`

## Key Domain Concepts

### Plugin Loading Flow (startup)
1. `App.axaml.cs → PluginHostService.LoadAll(pluginDirectory)`
2. Walks `{pluginsDir}/{id}/{version}/manifest.json`
3. `PluginLoader.Load(dir)` → checks compatibility → `PluginLoadContext` (isolated ALC)
4. Registered in `PluginRegistry`

### Open Save File
1. User picks file → `FileDetectionService.DetectAsync(path)` → all plugins vote via `CanOpen`
2. Highest priority wins → `plugin.OpenAsync(path)` → `ISaveDocument`
3. `ISaveDocument.GetEditorDefinition()` → `EditorKind.Schema` or `Custom`
4. `MainViewModel.OpenEditor(new EditorViewModel(doc))`

### Save with Backup
1. `EditorViewModel.SaveAsync()`
2. Copy original → `.bak` in backup directory
3. `ISaveDocument.SaveAsync()` → write to `.tmp` → rename `.tmp` → original
4. If rename fails: `.tmp` deleted, original untouched

## Code Quality Tooling

The solution uses:
- **`.editorconfig`** — C# code style (4-space indent, file-scoped namespaces, var preferences, naming rules)
- **`Directory.Build.props`** — shared MSBuild properties for all projects; Roslynator analyzers
- **`Directory.Packages.props`** — Centralized Package Management (all `PackageReference` elements are version-free)
- **`Directory.Build.targets`** — `CopyToDevPlugins` MSBuild target for dev workflow
- **Roslynator.Analyzers + Roslynator.CodeFixes** — applied to all projects as `PrivateAssets=all`
- `EnableNETAnalyzers=true`, `AnalysisMode=Recommended`, `EnforceCodeStyleInBuild=true`
- `TreatWarningsAsErrors=true` in Release configuration

## Conventions

- **File-scoped namespaces** everywhere (`namespace Foo.Bar;` not `namespace Foo.Bar { }`)
- **`sealed`** on concrete classes unless inheritance is intended
- **Records** for immutable data transfer objects
- **Primary constructors** are fine for simple DI classes (C# 12)
- **Async methods** always accept `CancellationToken ct = default`
- **No `public` modifier** on types inside `internal` assemblies (SDK is the public surface)
- Plugin projects reference **only `SaveForge.SDK`**

## Plugin Authoring Quick Reference

```csharp
// 1. Implement IGameEditorPlugin (one class per plugin)
// 2. Implement ISaveDocument (returned by OpenAsync)
// 3. Define FormSchema for schema-driven UI, OR implement ICustomEditorViewFactory
// 4. Provide manifest.json next to the DLL
```

See `docs/PLUGIN_AUTHORING.md` for full guide.

## Tests

Tests live in `tests/SaveForge.Tests/`. The test project directly references all
source projects (not via plugin isolation) for integration testing. `Xunit.SkippableFact`
is used to skip tests that require game save files not present in CI.

Run with: `dotnet test tests/SaveForge.Tests/`
