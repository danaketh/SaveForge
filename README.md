# SaveForge

A cross-platform desktop application for editing local single-player game save files via a plugin system. Each plugin targets one game and provides a schema-driven or fully custom Avalonia UI for editing.

## Requirements

- [.NET 10 SDK](https://dotnet.microsoft.com/download/dotnet/10)

---

## Build & Run

```bash
# Restore packages and build everything
dotnet restore
dotnet build

# Run the application
dotnet run --project src/SaveForge.App/
```

### Release build

```bash
dotnet build -c Release
dotnet run -c Release --project src/SaveForge.App/
```

> **Note:** Release builds treat all warnings as errors. Fix any analyzer warnings before publishing.

---

## Tests

```bash
# Run the full test suite
dotnet test tests/SaveForge.Tests/

# Run with verbose output
dotnet test tests/SaveForge.Tests/ --logger "console;verbosity=normal"
```

Some tests are skipped automatically when the required game save files are not present on the machine (via `Xunit.SkippableFact`).

---

## Available Plugins

Plugins are loaded from a directory and registered at startup. Two plugins ship with the repository.

| Plugin | ID | Game | Status |
|---|---|---|---|
| Planet Crafter Save Editor | `planet-crafter` | [Planet Crafter](https://store.steampowered.com/app/1284190/The_Planet_Crafter/) | Dev |
| StardewLike Save Editor | `stardew-like` | Sample / reference game | Dev |

### Planet Crafter Save Editor

Edits `.json` save files from **The Planet Crafter**. Detected automatically when the file contains a `"terraTokens"` key in its first 512 bytes.

**Editor tabs:**

- **Characters** — select a character from a drop-down; edit survival stats (health, oxygen, thirst, toxic), equipment slots, and inventory items
- **Terra Tokens** — spendable tokens and all-time total
- **Planet State** — terraforming levels (oxygen, heat, pressure, plants, insects, animals)
- **Save Info** — display name, planet ID, crafted objects count

Item names in inventory/equipment are resolved from the game's localization file (`LocalizationSpaceCrafter.txt`). The plugin searches the following paths automatically:

```
%ProgramFiles(x86)%\Steam\steamapps\common\Planet Crafter\Planet Crafter_Data\Resources\LocalizationSpaceCrafter.txt
D:\SteamLibrary\...
E:\SteamLibrary\...
F:\SteamLibrary\...
```

If no localization file is found, raw item IDs are shown as-is.

Default save directory: `%APPDATA%\MijuGames\The Planet Crafter\Saves`

### StardewLike Save Editor (Sample / Reference)

A reference implementation demonstrating the schema-driven plugin pattern. Not tied to a real game.

---

## Plugin Development

Plugins live under `src/`. Any plugin project with `<IsDevPlugin>true</IsDevPlugin>` in its `.csproj` is automatically copied into the app's `_devplugins/` folder after each Debug build via the `CopyToDevPlugins` MSBuild target.

### Dev workflow

```bash
# Build everything — all dev plugins are available on next run
dotnet build

# Build one plugin only — only that plugin loads (fast iteration)
dotnet build src/SaveForge.Plugin.PlanetCrafter/

# Run the app — it picks up whatever is in _devplugins/
dotnet run --project src/SaveForge.App/
```

The `_devplugins/` folder is at `src/SaveForge.App/bin/Debug/net8.0/_devplugins/` and is git-ignored. In Release builds the app uses `%APPDATA%/SaveForge/plugins/` instead.

For full authoring instructions see [docs/PLUGIN_AUTHORING.md](docs/PLUGIN_AUTHORING.md).

---

## Code Quality Tools

The solution enforces code style and quality uniformly across all projects.

| Tool | Purpose | Configuration |
|---|---|---|
| `.editorconfig` | C# code style (indentation, `var` usage, naming rules, file-scoped namespaces) | `.editorconfig` |
| Roslyn analyzers | Framework-level analysis (`EnableNETAnalyzers`, `AnalysisMode=Recommended`) | `Directory.Build.props` |
| Roslynator | Extended code quality rules and auto-fixes | `Directory.Build.props` |
| Centralized Package Management | Single version source for all NuGet packages | `Directory.Packages.props` |

All projects share settings via `Directory.Build.props` and `Directory.Build.targets` — individual `.csproj` files only declare what differs from the defaults.

### Useful dotnet CLI commands

```bash
# Check for code style violations (same checks as CI)
dotnet build -c Release

# Format code according to .editorconfig
dotnet format

# Apply Roslynator fixes
roslynator fix "SaveForge.sln" --msbuild-path "C:\program files\dotnet\sdk\9.0.313"
```

---

## Project Structure

```
SaveForge.sln
src/
  SaveForge.SDK            # Public plugin contract — interfaces, DTOs, schema types
  SaveForge.Core           # Internal domain models and service interfaces
  SaveForge.Infrastructure # File I/O, backup, HTTP download, settings persistence
  SaveForge.Security       # SHA-256 verification, manifest validation
  SaveForge.PluginHost     # AssemblyLoadContext per plugin, registry, file detection
  SaveForge.Store          # Catalog fetch/cache, plugin install/remove/update
  SaveForge.Forms          # Schema-driven Avalonia form renderer
  SaveForge.App            # Avalonia shell, DI root, all pages and view models
  SaveForge.Plugin.PlanetCrafter       # Plugin: Planet Crafter save editor
  SaveForge.SamplePlugin.StardewLike   # Reference plugin (schema-driven pattern)
tests/
  SaveForge.Tests          # xUnit integration + unit tests for all non-UI components
docs/
  ARCHITECTURE.md          # System design and dependency rules
  PLUGIN_AUTHORING.md      # How to write a plugin
  CATALOG_SCHEMA.md        # Plugin catalog format
```

### Dependency rules

```
App → Core, PluginHost, Store, Infrastructure, Forms
PluginHost → Core, SDK
Store → Core, Infrastructure, Security
Forms → SDK
Plugins → SDK only
```

---

## Further Reading

- [Architecture](docs/ARCHITECTURE.md)
- [Plugin Authoring Guide](docs/PLUGIN_AUTHORING.md)
- [Catalog Schema](docs/CATALOG_SCHEMA.md)
