# SaveForge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a cross-platform .NET 8 / Avalonia desktop app for editing game save files via a plugin architecture, including SDK, plugin host, store/catalog, schema-driven form renderer, dark-themed shell UI, and a sample plugin.

**Architecture:** Flat SDK boundary — `SaveForge.SDK` contains only interfaces and DTOs and has no dependency on any other project in the solution. The `PluginHost` uses one `AssemblyLoadContext` per plugin for dependency isolation. The core app wires everything together via `Microsoft.Extensions.DependencyInjection`.

**Tech Stack:** .NET 8, C# (nullable enabled), Avalonia 11.2.3, CommunityToolkit.Mvvm 8.3.2, System.Text.Json, Microsoft.Extensions.DependencyInjection 8.0.1, xUnit 2.9.0

---

## File Map

```
SaveForge.sln

src/SaveForge.SDK/
  SaveForge.SDK.csproj
  IGameEditorPlugin.cs
  ISaveDocument.cs
  IEditorDefinition.cs
  ICustomEditorViewFactory.cs
  PluginManifest.cs
  SaveCapabilities.cs
  Schema/FormSchema.cs
  Schema/FormPage.cs
  Schema/FormSection.cs
  Schema/FieldDefinition.cs        ← abstract base + all concrete subtypes
  Schema/VisibilityRule.cs
  Schema/EnumOption.cs

src/SaveForge.Core/
  SaveForge.Core.csproj
  Models/InstalledPlugin.cs
  Models/CatalogEntry.cs
  Models/PluginLoadResult.cs
  Models/BackupRecord.cs
  Models/AppSettings.cs
  Interfaces/IPluginRegistry.cs
  Interfaces/IBackupService.cs
  Interfaces/ICatalogService.cs
  Interfaces/IPluginInstaller.cs
  Interfaces/IUpdateChecker.cs
  Interfaces/IFileDetectionService.cs

src/SaveForge.Infrastructure/
  SaveForge.Infrastructure.csproj
  Services/FileBackupService.cs
  Services/SettingsService.cs
  Services/HttpDownloadService.cs

src/SaveForge.Security/
  SaveForge.Security.csproj
  HashVerifier.cs
  ManifestValidator.cs

src/SaveForge.PluginHost/
  SaveForge.PluginHost.csproj
  PluginLoadContext.cs
  PluginLoader.cs
  PluginRegistry.cs
  FileDetectionService.cs

src/SaveForge.Store/
  SaveForge.Store.csproj
  Dtos/CatalogManifestDto.cs
  Dtos/CatalogEntryDto.cs
  CatalogService.cs
  PluginInstaller.cs
  UpdateChecker.cs

src/SaveForge.Forms/
  SaveForge.Forms.csproj
  ViewModels/FieldViewModel.cs          ← abstract base
  ViewModels/TextFieldViewModel.cs
  ViewModels/NumericFieldViewModel.cs
  ViewModels/CheckboxFieldViewModel.cs
  ViewModels/EnumFieldViewModel.cs
  ViewModels/ReadOnlyFieldViewModel.cs
  ViewModels/ListFieldViewModel.cs
  ViewModels/ObjectSectionViewModel.cs
  ViewModels/FormSectionViewModel.cs
  ViewModels/FormPageViewModel.cs
  ViewModels/FormRendererViewModel.cs
  Views/FormRendererView.axaml
  Views/FormRendererView.axaml.cs

src/SaveForge.App/
  SaveForge.App.csproj
  Program.cs
  App.axaml
  App.axaml.cs
  Themes/DarkTheme.axaml
  Themes/Colors.axaml
  Controls/TitleBarControl.axaml
  Controls/TitleBarControl.axaml.cs
  Controls/StatusBarControl.axaml
  Controls/StatusBarControl.axaml.cs
  Controls/InfoBarControl.axaml
  Controls/InfoBarControl.axaml.cs
  ViewModels/MainViewModel.cs
  ViewModels/HomeViewModel.cs
  ViewModels/StoreViewModel.cs
  ViewModels/PluginDetailViewModel.cs
  ViewModels/SettingsViewModel.cs
  ViewModels/AboutViewModel.cs
  ViewModels/EditorViewModel.cs
  Views/MainWindow.axaml
  Views/MainWindow.axaml.cs
  Views/HomeView.axaml
  Views/HomeView.axaml.cs
  Views/StoreView.axaml
  Views/StoreView.axaml.cs
  Views/PluginDetailView.axaml
  Views/PluginDetailView.axaml.cs
  Views/SettingsView.axaml
  Views/SettingsView.axaml.cs
  Views/AboutView.axaml
  Views/AboutView.axaml.cs
  Views/EditorView.axaml
  Views/EditorView.axaml.cs

src/SaveForge.SamplePlugin.StardewLike/
  SaveForge.SamplePlugin.StardewLike.csproj
  StardewPlugin.cs
  StardewSaveDocument.cs
  StardewSchema.cs
  manifest.json

tests/SaveForge.Tests/
  SaveForge.Tests.csproj
  ManifestParsingTests.cs
  HashVerifierTests.cs
  ManifestValidatorTests.cs
  PluginLoaderTests.cs
  FileDetectionTests.cs
  BackupServiceTests.cs
  CatalogServiceTests.cs
  PluginInstallerTests.cs
  StardewPluginTests.cs
  VisibilityRuleTests.cs

docs/
  ARCHITECTURE.md
  PLUGIN_AUTHORING.md
  CATALOG_SCHEMA.md
```

---

## Task 1: Solution Scaffold

**Files:**
- Create: `SaveForge.sln`
- Create: all `.csproj` files
- Create: all `src/` and `tests/` subdirectories

- [ ] **Step 1: Create the solution and top-level directories**

```bash
cd I:/Projects/danaketh/SaveForge
dotnet new sln -n SaveForge
mkdir -p src tests docs/superpowers/specs docs/superpowers/plans
```

Expected: `SaveForge.sln` created.

- [ ] **Step 2: Create SDK project**

```bash
dotnet new classlib -n SaveForge.SDK -o src/SaveForge.SDK --framework net8.0
rm src/SaveForge.SDK/Class1.cs
```

Replace `src/SaveForge.SDK/SaveForge.SDK.csproj` with:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <LangVersion>latest</LangVersion>
    <RootNamespace>SaveForge.SDK</RootNamespace>
  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include="Avalonia" Version="11.2.3" />
  </ItemGroup>
</Project>
```

- [ ] **Step 3: Create Core project**

```bash
dotnet new classlib -n SaveForge.Core -o src/SaveForge.Core --framework net8.0
rm src/SaveForge.Core/Class1.cs
```

Replace `src/SaveForge.Core/SaveForge.Core.csproj`:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <LangVersion>latest</LangVersion>
    <RootNamespace>SaveForge.Core</RootNamespace>
  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="..\SaveForge.SDK\SaveForge.SDK.csproj" />
    <PackageReference Include="Microsoft.Extensions.Logging.Abstractions" Version="8.0.2" />
  </ItemGroup>
</Project>
```

- [ ] **Step 4: Create Infrastructure project**

```bash
dotnet new classlib -n SaveForge.Infrastructure -o src/SaveForge.Infrastructure --framework net8.0
rm src/SaveForge.Infrastructure/Class1.cs
```

`src/SaveForge.Infrastructure/SaveForge.Infrastructure.csproj`:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <LangVersion>latest</LangVersion>
    <RootNamespace>SaveForge.Infrastructure</RootNamespace>
  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="..\SaveForge.Core\SaveForge.Core.csproj" />
    <PackageReference Include="Microsoft.Extensions.Logging.Abstractions" Version="8.0.2" />
  </ItemGroup>
</Project>
```

- [ ] **Step 5: Create Security project**

```bash
dotnet new classlib -n SaveForge.Security -o src/SaveForge.Security --framework net8.0
rm src/SaveForge.Security/Class1.cs
```

`src/SaveForge.Security/SaveForge.Security.csproj`:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <LangVersion>latest</LangVersion>
    <RootNamespace>SaveForge.Security</RootNamespace>
  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="..\SaveForge.Core\SaveForge.Core.csproj" />
    <PackageReference Include="Microsoft.Extensions.Logging.Abstractions" Version="8.0.2" />
  </ItemGroup>
</Project>
```

- [ ] **Step 6: Create PluginHost project**

```bash
dotnet new classlib -n SaveForge.PluginHost -o src/SaveForge.PluginHost --framework net8.0
rm src/SaveForge.PluginHost/Class1.cs
```

`src/SaveForge.PluginHost/SaveForge.PluginHost.csproj`:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <LangVersion>latest</LangVersion>
    <RootNamespace>SaveForge.PluginHost</RootNamespace>
  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="..\SaveForge.Core\SaveForge.Core.csproj" />
    <ProjectReference Include="..\SaveForge.SDK\SaveForge.SDK.csproj" />
    <PackageReference Include="Microsoft.Extensions.Logging.Abstractions" Version="8.0.2" />
  </ItemGroup>
</Project>
```

- [ ] **Step 7: Create Store project**

```bash
dotnet new classlib -n SaveForge.Store -o src/SaveForge.Store --framework net8.0
rm src/SaveForge.Store/Class1.cs
```

`src/SaveForge.Store/SaveForge.Store.csproj`:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <LangVersion>latest</LangVersion>
    <RootNamespace>SaveForge.Store</RootNamespace>
  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="..\SaveForge.Core\SaveForge.Core.csproj" />
    <ProjectReference Include="..\SaveForge.Infrastructure\SaveForge.Infrastructure.csproj" />
    <ProjectReference Include="..\SaveForge.Security\SaveForge.Security.csproj" />
    <PackageReference Include="Microsoft.Extensions.Logging.Abstractions" Version="8.0.2" />
  </ItemGroup>
</Project>
```

- [ ] **Step 8: Create Forms project**

```bash
dotnet new classlib -n SaveForge.Forms -o src/SaveForge.Forms --framework net8.0
rm src/SaveForge.Forms/Class1.cs
```

`src/SaveForge.Forms/SaveForge.Forms.csproj`:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <LangVersion>latest</LangVersion>
    <RootNamespace>SaveForge.Forms</RootNamespace>
  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="..\SaveForge.SDK\SaveForge.SDK.csproj" />
    <PackageReference Include="Avalonia" Version="11.2.3" />
    <PackageReference Include="CommunityToolkit.Mvvm" Version="8.3.2" />
  </ItemGroup>
</Project>
```

- [ ] **Step 9: Create App project**

```bash
dotnet new avalonia.app -n SaveForge.App -o src/SaveForge.App
```

Replace `src/SaveForge.App/SaveForge.App.csproj`:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>WinExe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <LangVersion>latest</LangVersion>
    <RootNamespace>SaveForge.App</RootNamespace>
    <ApplicationIcon>Assets/icon.ico</ApplicationIcon>
  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="..\SaveForge.Core\SaveForge.Core.csproj" />
    <ProjectReference Include="..\SaveForge.PluginHost\SaveForge.PluginHost.csproj" />
    <ProjectReference Include="..\SaveForge.Store\SaveForge.Store.csproj" />
    <ProjectReference Include="..\SaveForge.Infrastructure\SaveForge.Infrastructure.csproj" />
    <ProjectReference Include="..\SaveForge.Forms\SaveForge.Forms.csproj" />
    <PackageReference Include="Avalonia" Version="11.2.3" />
    <PackageReference Include="Avalonia.Desktop" Version="11.2.3" />
    <PackageReference Include="Avalonia.Themes.Fluent" Version="11.2.3" />
    <PackageReference Include="Avalonia.Fonts.Inter" Version="11.2.3" />
    <PackageReference Include="CommunityToolkit.Mvvm" Version="8.3.2" />
    <PackageReference Include="Microsoft.Extensions.DependencyInjection" Version="8.0.1" />
    <PackageReference Include="Microsoft.Extensions.Logging" Version="8.0.1" />
  </ItemGroup>
</Project>
```

- [ ] **Step 10: Create SamplePlugin project**

```bash
dotnet new classlib -n SaveForge.SamplePlugin.StardewLike -o src/SaveForge.SamplePlugin.StardewLike --framework net8.0
rm src/SaveForge.SamplePlugin.StardewLike/Class1.cs
```

`src/SaveForge.SamplePlugin.StardewLike/SaveForge.SamplePlugin.StardewLike.csproj`:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <LangVersion>latest</LangVersion>
    <RootNamespace>SaveForge.SamplePlugin.StardewLike</RootNamespace>
  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="..\SaveForge.SDK\SaveForge.SDK.csproj" />
  </ItemGroup>
</Project>
```

- [ ] **Step 11: Create Tests project**

```bash
dotnet new xunit -n SaveForge.Tests -o tests/SaveForge.Tests --framework net8.0
rm tests/SaveForge.Tests/UnitTest1.cs
```

`tests/SaveForge.Tests/SaveForge.Tests.csproj`:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <LangVersion>latest</LangVersion>
    <IsPackable>false</IsPackable>
  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include="Microsoft.NET.Test.Sdk" Version="17.11.1" />
    <PackageReference Include="xunit" Version="2.9.0" />
    <PackageReference Include="xunit.runner.visualstudio" Version="2.8.2" />
    <ProjectReference Include="..\..\src\SaveForge.SDK\SaveForge.SDK.csproj" />
    <ProjectReference Include="..\..\src\SaveForge.Core\SaveForge.Core.csproj" />
    <ProjectReference Include="..\..\src\SaveForge.Infrastructure\SaveForge.Infrastructure.csproj" />
    <ProjectReference Include="..\..\src\SaveForge.Security\SaveForge.Security.csproj" />
    <ProjectReference Include="..\..\src\SaveForge.PluginHost\SaveForge.PluginHost.csproj" />
    <ProjectReference Include="..\..\src\SaveForge.Store\SaveForge.Store.csproj" />
    <ProjectReference Include="..\..\src\SaveForge.SamplePlugin.StardewLike\SaveForge.SamplePlugin.StardewLike.csproj" />
  </ItemGroup>
</Project>
```

- [ ] **Step 12: Add all projects to solution**

```bash
dotnet sln add src/SaveForge.SDK/SaveForge.SDK.csproj
dotnet sln add src/SaveForge.Core/SaveForge.Core.csproj
dotnet sln add src/SaveForge.Infrastructure/SaveForge.Infrastructure.csproj
dotnet sln add src/SaveForge.Security/SaveForge.Security.csproj
dotnet sln add src/SaveForge.PluginHost/SaveForge.PluginHost.csproj
dotnet sln add src/SaveForge.Store/SaveForge.Store.csproj
dotnet sln add src/SaveForge.Forms/SaveForge.Forms.csproj
dotnet sln add src/SaveForge.App/SaveForge.App.csproj
dotnet sln add src/SaveForge.SamplePlugin.StardewLike/SaveForge.SamplePlugin.StardewLike.csproj
dotnet sln add tests/SaveForge.Tests/SaveForge.Tests.csproj
```

- [ ] **Step 13: Create subdirectory structure for source files**

```bash
mkdir -p src/SaveForge.SDK/Schema
mkdir -p src/SaveForge.Core/Models
mkdir -p src/SaveForge.Core/Interfaces
mkdir -p src/SaveForge.Infrastructure/Services
mkdir -p src/SaveForge.PluginHost
mkdir -p src/SaveForge.Store/Dtos
mkdir -p src/SaveForge.Forms/ViewModels
mkdir -p src/SaveForge.Forms/Views
mkdir -p src/SaveForge.App/Themes
mkdir -p src/SaveForge.App/Controls
mkdir -p src/SaveForge.App/ViewModels
mkdir -p src/SaveForge.App/Views
mkdir -p src/SaveForge.App/Assets
```

- [ ] **Step 14: Verify solution restores**

```bash
dotnet restore SaveForge.sln
```

Expected: all packages restored with no errors.

- [ ] **Step 15: Commit scaffold**

```bash
git init
git add .
git commit -m "chore: initial solution scaffold with all project files"
```

---

## Task 2: SDK — Core Interfaces

**Files:**
- Create: `src/SaveForge.SDK/IGameEditorPlugin.cs`
- Create: `src/SaveForge.SDK/ISaveDocument.cs`
- Create: `src/SaveForge.SDK/IEditorDefinition.cs`
- Create: `src/SaveForge.SDK/ICustomEditorViewFactory.cs`
- Create: `src/SaveForge.SDK/PluginManifest.cs`
- Create: `src/SaveForge.SDK/SaveCapabilities.cs`

- [ ] **Step 1: Create IGameEditorPlugin.cs**

```csharp
// src/SaveForge.SDK/IGameEditorPlugin.cs
namespace SaveForge.SDK;

/// <summary>The entry point for a game editor plugin. Implement this interface in your plugin assembly.</summary>
public interface IGameEditorPlugin
{
    /// <summary>Static metadata about this plugin.</summary>
    PluginManifest Manifest { get; }

    /// <summary>File-matching capabilities of this plugin.</summary>
    SaveCapabilities Capabilities { get; }

    /// <summary>
    /// Returns whether this plugin can open the specified file.
    /// </summary>
    /// <param name="file">File metadata (name, extension, size).</param>
    /// <param name="header">First 512 bytes of the file for lightweight content sniffing.</param>
    /// <returns>Whether this plugin can open the file, and a priority score (higher = preferred when multiple plugins match).</returns>
    (bool CanOpen, int Priority) CanOpen(FileInfo file, ReadOnlySpan<byte> header);

    /// <summary>Opens a save file and returns a document handle.</summary>
    Task<ISaveDocument> OpenAsync(string filePath, CancellationToken ct = default);
}
```

- [ ] **Step 2: Create ISaveDocument.cs**

```csharp
// src/SaveForge.SDK/ISaveDocument.cs
namespace SaveForge.SDK;

/// <summary>Represents an open save file. Values are accessed via dot-path keys.</summary>
public interface ISaveDocument
{
    /// <summary>Absolute path to the file on disk.</summary>
    string FilePath { get; }

    /// <summary>True if any value has been modified since the last save.</summary>
    bool IsDirty { get; }

    /// <summary>Gets a value by dot-path (e.g. "player.stats.health").</summary>
    object? GetValue(string dotPath);

    /// <summary>Sets a value by dot-path and marks the document dirty.</summary>
    void SetValue(string dotPath, object? value);

    /// <summary>Returns the editor definition (schema or custom view factory).</summary>
    IEditorDefinition GetEditorDefinition();

    /// <summary>Saves the document back to disk. The caller is responsible for backup.</summary>
    Task SaveAsync(CancellationToken ct = default);
}
```

- [ ] **Step 3: Create IEditorDefinition.cs**

```csharp
// src/SaveForge.SDK/IEditorDefinition.cs
using SaveForge.SDK.Schema;

namespace SaveForge.SDK;

/// <summary>Discriminates between a schema-driven and a custom plugin UI.</summary>
public enum EditorKind { Schema, Custom }

/// <summary>Returned by ISaveDocument.GetEditorDefinition() to tell the host how to render the editor.</summary>
public interface IEditorDefinition
{
    EditorKind Kind { get; }

    /// <summary>Non-null when Kind == Schema.</summary>
    FormSchema? Schema { get; }

    /// <summary>Non-null when Kind == Custom.</summary>
    ICustomEditorViewFactory? ViewFactory { get; }
}
```

- [ ] **Step 4: Create ICustomEditorViewFactory.cs**

```csharp
// src/SaveForge.SDK/ICustomEditorViewFactory.cs
using Avalonia.Controls;

namespace SaveForge.SDK;

/// <summary>
/// Escape hatch for advanced plugins that need a fully custom Avalonia UI.
/// The host calls CreateView and CreateViewModel and wires them together.
/// </summary>
public interface ICustomEditorViewFactory
{
    /// <summary>Creates the Avalonia view control for this document.</summary>
    Control CreateView(ISaveDocument document);

    /// <summary>Creates the view model that the view should be bound to.</summary>
    object CreateViewModel(ISaveDocument document);
}
```

- [ ] **Step 5: Create PluginManifest.cs**

```csharp
// src/SaveForge.SDK/PluginManifest.cs
namespace SaveForge.SDK;

/// <summary>Static metadata embedded in every plugin's manifest.json.</summary>
public sealed class PluginManifest
{
    /// <summary>Unique identifier (lowercase alphanumeric + hyphens, e.g. "stardew-like").</summary>
    public required string Id { get; init; }

    public required string Name { get; init; }

    /// <summary>SemVer string, e.g. "1.0.0".</summary>
    public required string Version { get; init; }

    public required string Author { get; init; }

    public string Description { get; init; } = string.Empty;

    /// <summary>Minimum host version this plugin is compatible with (SemVer string).</summary>
    public required string MinHostVersion { get; init; }

    /// <summary>Maximum host version, or null for no upper bound.</summary>
    public string? MaxHostVersion { get; init; }

    /// <summary>Filename of the plugin DLL relative to the plugin directory.</summary>
    public required string PluginAssembly { get; init; }
}
```

- [ ] **Step 6: Create SaveCapabilities.cs**

```csharp
// src/SaveForge.SDK/SaveCapabilities.cs
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
}
```

- [ ] **Step 7: Build SDK to confirm it compiles**

```bash
dotnet build src/SaveForge.SDK/SaveForge.SDK.csproj
```

Expected: Build succeeded, 0 errors.

- [ ] **Step 8: Commit**

```bash
git add src/SaveForge.SDK/
git commit -m "feat(sdk): add core plugin interfaces and manifest types"
```

---

## Task 3: SDK — Schema Types

**Files:**
- Create: `src/SaveForge.SDK/Schema/EnumOption.cs`
- Create: `src/SaveForge.SDK/Schema/VisibilityRule.cs`
- Create: `src/SaveForge.SDK/Schema/FieldDefinition.cs`
- Create: `src/SaveForge.SDK/Schema/FormSection.cs`
- Create: `src/SaveForge.SDK/Schema/FormPage.cs`
- Create: `src/SaveForge.SDK/Schema/FormSchema.cs`
- Test: `tests/SaveForge.Tests/VisibilityRuleTests.cs`

- [ ] **Step 1: Create EnumOption.cs**

```csharp
// src/SaveForge.SDK/Schema/EnumOption.cs
namespace SaveForge.SDK.Schema;

/// <summary>One option in an EnumField dropdown.</summary>
public sealed class EnumOption
{
    public required string Value { get; init; }
    public required string Display { get; init; }
}
```

- [ ] **Step 2: Create VisibilityRule.cs**

```csharp
// src/SaveForge.SDK/Schema/VisibilityRule.cs
namespace SaveForge.SDK.Schema;

public enum VisibilityOperator { Equals, NotEquals, GreaterThan, LessThan }

/// <summary>
/// Makes a field visible only when another field's value satisfies a condition.
/// </summary>
public sealed class VisibilityRule
{
    public required string DependsOnKey { get; init; }
    public required VisibilityOperator Operator { get; init; }
    public required object Value { get; init; }

    /// <summary>Evaluates whether the field should be visible given the current value of DependsOnKey.</summary>
    public bool Evaluate(object? currentValue)
    {
        return Operator switch
        {
            VisibilityOperator.Equals    => AreEqual(currentValue, Value),
            VisibilityOperator.NotEquals => !AreEqual(currentValue, Value),
            VisibilityOperator.GreaterThan => Compare(currentValue, Value) > 0,
            VisibilityOperator.LessThan    => Compare(currentValue, Value) < 0,
            _ => true
        };
    }

    private static bool AreEqual(object? a, object? b)
    {
        if (a is null && b is null) return true;
        if (a is null || b is null) return false;
        try { return Convert.ChangeType(a, b.GetType()).Equals(b); }
        catch { return a.Equals(b); }
    }

    private static int Compare(object? a, object? b)
    {
        if (a is IComparable ca && b is not null)
        {
            try { return ca.CompareTo(Convert.ChangeType(b, a.GetType())); }
            catch { return 0; }
        }
        return 0;
    }
}
```

- [ ] **Step 3: Write failing VisibilityRule tests**

```csharp
// tests/SaveForge.Tests/VisibilityRuleTests.cs
using SaveForge.SDK.Schema;
using Xunit;

namespace SaveForge.Tests;

public class VisibilityRuleTests
{
    [Fact]
    public void Equals_MatchingString_ReturnsTrue()
    {
        var rule = new VisibilityRule { DependsOnKey = "x", Operator = VisibilityOperator.Equals, Value = "admin" };
        Assert.True(rule.Evaluate("admin"));
    }

    [Fact]
    public void Equals_NonMatchingString_ReturnsFalse()
    {
        var rule = new VisibilityRule { DependsOnKey = "x", Operator = VisibilityOperator.Equals, Value = "admin" };
        Assert.False(rule.Evaluate("user"));
    }

    [Fact]
    public void NotEquals_DifferentValue_ReturnsTrue()
    {
        var rule = new VisibilityRule { DependsOnKey = "x", Operator = VisibilityOperator.NotEquals, Value = "admin" };
        Assert.True(rule.Evaluate("user"));
    }

    [Fact]
    public void GreaterThan_LargerInt_ReturnsTrue()
    {
        var rule = new VisibilityRule { DependsOnKey = "x", Operator = VisibilityOperator.GreaterThan, Value = 5 };
        Assert.True(rule.Evaluate(10));
    }

    [Fact]
    public void LessThan_SmallerInt_ReturnsTrue()
    {
        var rule = new VisibilityRule { DependsOnKey = "x", Operator = VisibilityOperator.LessThan, Value = 10 };
        Assert.True(rule.Evaluate(3));
    }

    [Fact]
    public void Equals_NullBothSides_ReturnsTrue()
    {
        var rule = new VisibilityRule { DependsOnKey = "x", Operator = VisibilityOperator.Equals, Value = null! };
        Assert.True(rule.Evaluate(null));
    }
}
```

- [ ] **Step 4: Run tests — expect them to fail (VisibilityRule not yet in place)**

```bash
dotnet test tests/SaveForge.Tests/ --filter "VisibilityRuleTests" -v minimal
```

Expected: FAIL — type not found or compile error.

- [ ] **Step 5: Create FieldDefinition.cs (all subtypes in one file)**

```csharp
// src/SaveForge.SDK/Schema/FieldDefinition.cs
namespace SaveForge.SDK.Schema;

/// <summary>Base class for all field definitions in a FormSchema.</summary>
public abstract class FieldDefinition
{
    /// <summary>Dot-path key used to get/set the value in ISaveDocument (e.g. "player.stats.health").</summary>
    public required string Key { get; init; }

    public required string Label { get; init; }

    public string? Description { get; init; }

    public bool IsReadOnly { get; init; }

    public bool IsRequired { get; init; }

    /// <summary>When set, the field is only shown when the rule evaluates to true.</summary>
    public VisibilityRule? VisibilityRule { get; init; }
}

public sealed class TextField : FieldDefinition
{
    public int? MaxLength { get; init; }
    public string? RegexPattern { get; init; }
}

public sealed class NumericField : FieldDefinition
{
    public double? Min { get; init; }
    public double? Max { get; init; }
    public int DecimalPlaces { get; init; } = 0;
}

public sealed class CheckboxField : FieldDefinition { }

public sealed class EnumField : FieldDefinition
{
    public EnumOption[] Options { get; init; } = Array.Empty<EnumOption>();
}

public sealed class ReadOnlyField : FieldDefinition { }

public sealed class ListField : FieldDefinition
{
    /// <summary>Template defining the shape of each list item.</summary>
    public required FieldDefinition ItemSchema { get; init; }
}

public sealed class ObjectSectionField : FieldDefinition
{
    public FieldDefinition[] Fields { get; init; } = Array.Empty<FieldDefinition>();
}
```

- [ ] **Step 6: Create FormSection.cs, FormPage.cs, FormSchema.cs**

```csharp
// src/SaveForge.SDK/Schema/FormSection.cs
namespace SaveForge.SDK.Schema;

public sealed class FormSection
{
    public string? Heading { get; init; }
    public FieldDefinition[] Fields { get; init; } = Array.Empty<FieldDefinition>();
}
```

```csharp
// src/SaveForge.SDK/Schema/FormPage.cs
namespace SaveForge.SDK.Schema;

public sealed class FormPage
{
    public required string Title { get; init; }
    public FormSection[] Sections { get; init; } = Array.Empty<FormSection>();
}
```

```csharp
// src/SaveForge.SDK/Schema/FormSchema.cs
namespace SaveForge.SDK.Schema;

/// <summary>Top-level schema that drives the generic form renderer.</summary>
public sealed class FormSchema
{
    public FormPage[] Pages { get; init; } = Array.Empty<FormPage>();
}
```

- [ ] **Step 7: Run VisibilityRule tests — expect pass**

```bash
dotnet test tests/SaveForge.Tests/ --filter "VisibilityRuleTests" -v minimal
```

Expected: 6/6 PASS.

- [ ] **Step 8: Commit**

```bash
git add src/SaveForge.SDK/ tests/SaveForge.Tests/VisibilityRuleTests.cs
git commit -m "feat(sdk): add schema types and VisibilityRule with tests"
```

---

## Task 4: Core Domain Models and Interfaces

**Files:**
- Create: `src/SaveForge.Core/Models/InstalledPlugin.cs`
- Create: `src/SaveForge.Core/Models/CatalogEntry.cs`
- Create: `src/SaveForge.Core/Models/PluginLoadResult.cs`
- Create: `src/SaveForge.Core/Models/BackupRecord.cs`
- Create: `src/SaveForge.Core/Models/AppSettings.cs`
- Create: `src/SaveForge.Core/Interfaces/IPluginRegistry.cs`
- Create: `src/SaveForge.Core/Interfaces/IBackupService.cs`
- Create: `src/SaveForge.Core/Interfaces/ICatalogService.cs`
- Create: `src/SaveForge.Core/Interfaces/IPluginInstaller.cs`
- Create: `src/SaveForge.Core/Interfaces/IUpdateChecker.cs`
- Create: `src/SaveForge.Core/Interfaces/IFileDetectionService.cs`

- [ ] **Step 1: Create InstalledPlugin.cs**

```csharp
// src/SaveForge.Core/Models/InstalledPlugin.cs
using SaveForge.SDK;

namespace SaveForge.Core.Models;

public enum PluginLoadState { Loaded, Failed, Incompatible }

public sealed class InstalledPlugin
{
    public required PluginManifest Manifest { get; init; }
    public required PluginLoadState LoadState { get; init; }
    public required string AssemblyPath { get; init; }
    public string? FailureReason { get; init; }

    /// <summary>Non-null only when LoadState == Loaded.</summary>
    public IGameEditorPlugin? Plugin { get; init; }
}
```

- [ ] **Step 2: Create CatalogEntry.cs**

```csharp
// src/SaveForge.Core/Models/CatalogEntry.cs
namespace SaveForge.Core.Models;

public sealed class CatalogEntry
{
    public required string Id { get; init; }
    public required string Name { get; init; }
    public string Description { get; init; } = string.Empty;
    public required string Author { get; init; }
    public required string LatestVersion { get; init; }
    public required string MinHostVersion { get; init; }
    public string? MaxHostVersion { get; init; }
    public required string PackageUrl { get; init; }
    public required string Sha256 { get; init; }
    public string? IconUrl { get; init; }
    public string[] ScreenshotUrls { get; init; } = Array.Empty<string>();
    public string[] Tags { get; init; } = Array.Empty<string>();
    public string? RepositoryUrl { get; init; }

    // Computed at load time by comparing against installed plugins
    public bool IsInstalled { get; init; }
    public bool HasUpdate { get; init; }
}
```

- [ ] **Step 3: Create PluginLoadResult.cs**

```csharp
// src/SaveForge.Core/Models/PluginLoadResult.cs
using SaveForge.SDK;

namespace SaveForge.Core.Models;

public abstract class PluginLoadResult
{
    private PluginLoadResult() { }

    public sealed class Succeeded : PluginLoadResult
    {
        public required IGameEditorPlugin Plugin { get; init; }
    }

    public sealed class Failed : PluginLoadResult
    {
        public required string Reason { get; init; }
    }

    public sealed class Incompatible : PluginLoadResult
    {
        public required string Reason { get; init; }
    }
}
```

- [ ] **Step 4: Create BackupRecord.cs**

```csharp
// src/SaveForge.Core/Models/BackupRecord.cs
namespace SaveForge.Core.Models;

public sealed class BackupRecord
{
    public required string OriginalPath { get; init; }
    public required string BackupPath { get; init; }
    public required DateTimeOffset Timestamp { get; init; }
}
```

- [ ] **Step 5: Create AppSettings.cs**

```csharp
// src/SaveForge.Core/Models/AppSettings.cs
namespace SaveForge.Core.Models;

public sealed class AppSettings
{
    public string CatalogUrl { get; set; } = AppSettingsDefaults.CatalogUrl;
    public string PluginDirectory { get; set; } = AppSettingsDefaults.PluginDirectory;
    public string BackupDirectory { get; set; } = AppSettingsDefaults.BackupDirectory;
    public bool CheckUpdatesOnStartup { get; set; } = true;
}

public static class AppSettingsDefaults
{
    private static readonly string AppData =
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "SaveForge");

    public static string PluginDirectory { get; } = Path.Combine(AppData, "plugins");
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

- [ ] **Step 6: Create interface files**

```csharp
// src/SaveForge.Core/Interfaces/IPluginRegistry.cs
using SaveForge.Core.Models;
using SaveForge.SDK;

namespace SaveForge.Core.Interfaces;

public interface IPluginRegistry
{
    void Register(InstalledPlugin plugin);
    void Remove(string pluginId);
    IReadOnlyList<InstalledPlugin> GetAll();
    InstalledPlugin? TryGet(string pluginId);
    IReadOnlyList<IGameEditorPlugin> GetLoaded();
}
```

```csharp
// src/SaveForge.Core/Interfaces/IBackupService.cs
using SaveForge.Core.Models;

namespace SaveForge.Core.Interfaces;

public interface IBackupService
{
    /// <summary>Creates a timestamped backup of filePath, then atomically writes newContent to filePath.</summary>
    Task<BackupRecord> BackupAndWriteAsync(string filePath, string pluginId, string newContent, CancellationToken ct = default);
}
```

```csharp
// src/SaveForge.Core/Interfaces/ICatalogService.cs
using SaveForge.Core.Models;

namespace SaveForge.Core.Interfaces;

public interface ICatalogService
{
    Task<IReadOnlyList<CatalogEntry>> GetCatalogAsync(bool forceRefresh = false, CancellationToken ct = default);
}
```

```csharp
// src/SaveForge.Core/Interfaces/IPluginInstaller.cs
namespace SaveForge.Core.Interfaces;

public interface IPluginInstaller
{
    Task InstallAsync(string catalogEntryId, IProgress<long>? progress = null, CancellationToken ct = default);
    Task RemoveAsync(string pluginId, CancellationToken ct = default);
    Task UpdateAsync(string pluginId, IProgress<long>? progress = null, CancellationToken ct = default);
}
```

```csharp
// src/SaveForge.Core/Interfaces/IUpdateChecker.cs
using SaveForge.Core.Models;

namespace SaveForge.Core.Interfaces;

public sealed record UpdateInfo(string PluginId, string InstalledVersion, string AvailableVersion);

public interface IUpdateChecker
{
    Task<IReadOnlyList<UpdateInfo>> CheckAsync(CancellationToken ct = default);
}
```

```csharp
// src/SaveForge.Core/Interfaces/IFileDetectionService.cs
using SaveForge.SDK;

namespace SaveForge.Core.Interfaces;

public sealed record DetectionResult(IGameEditorPlugin Plugin, int Priority);

public interface IFileDetectionService
{
    /// <summary>Returns all plugins that can open the file, sorted by priority descending.</summary>
    Task<IReadOnlyList<DetectionResult>> DetectAsync(string filePath, CancellationToken ct = default);
}
```

- [ ] **Step 7: Build Core**

```bash
dotnet build src/SaveForge.Core/SaveForge.Core.csproj
```

Expected: Build succeeded, 0 errors.

- [ ] **Step 8: Commit**

```bash
git add src/SaveForge.Core/
git commit -m "feat(core): add domain models and service interfaces"
```

---

## Task 5: Infrastructure — Settings and Backup Services

**Files:**
- Create: `src/SaveForge.Infrastructure/Services/SettingsService.cs`
- Create: `src/SaveForge.Infrastructure/Services/FileBackupService.cs`
- Create: `src/SaveForge.Infrastructure/Services/HttpDownloadService.cs`
- Test: `tests/SaveForge.Tests/BackupServiceTests.cs`

- [ ] **Step 1: Write failing backup tests**

```csharp
// tests/SaveForge.Tests/BackupServiceTests.cs
using System.IO;
using SaveForge.Infrastructure.Services;
using Xunit;

namespace SaveForge.Tests;

public class BackupServiceTests : IDisposable
{
    private readonly string _tempDir = Path.Combine(Path.GetTempPath(), "gse-test-" + Guid.NewGuid());
    private readonly string _backupDir;

    public BackupServiceTests()
    {
        Directory.CreateDirectory(_tempDir);
        _backupDir = Path.Combine(_tempDir, "backups");
        Directory.CreateDirectory(_backupDir);
    }

    [Fact]
    public async Task BackupAndWrite_CreatesBackupFile()
    {
        var originalPath = Path.Combine(_tempDir, "save.json");
        await File.WriteAllTextAsync(originalPath, """{"v":1}""");
        var svc = new FileBackupService(_backupDir);

        var record = await svc.BackupAndWriteAsync(originalPath, "test-plugin", """{"v":2}""");

        Assert.True(File.Exists(record.BackupPath));
        Assert.Equal("""{"v":1}""", await File.ReadAllTextAsync(record.BackupPath));
    }

    [Fact]
    public async Task BackupAndWrite_WritesNewContentToOriginal()
    {
        var originalPath = Path.Combine(_tempDir, "save2.json");
        await File.WriteAllTextAsync(originalPath, """{"v":1}""");
        var svc = new FileBackupService(_backupDir);

        await svc.BackupAndWriteAsync(originalPath, "test-plugin", """{"v":2}""");

        Assert.Equal("""{"v":2}""", await File.ReadAllTextAsync(originalPath));
    }

    [Fact]
    public async Task BackupAndWrite_OriginalPreservedIfWriteFails()
    {
        var originalPath = Path.Combine(_tempDir, "save3.json");
        await File.WriteAllTextAsync(originalPath, """{"v":1}""");
        var svc = new FileBackupService("/nonexistent/impossible/path");

        await Assert.ThrowsAnyAsync<Exception>(() =>
            svc.BackupAndWriteAsync(originalPath, "test-plugin", """{"v":2}"""));

        Assert.Equal("""{"v":1}""", await File.ReadAllTextAsync(originalPath));
    }

    public void Dispose() => Directory.Delete(_tempDir, recursive: true);
}
```

- [ ] **Step 2: Run tests — expect compile failure**

```bash
dotnet test tests/SaveForge.Tests/ --filter "BackupServiceTests" -v minimal
```

Expected: compile error — `FileBackupService` not found.

- [ ] **Step 3: Create FileBackupService.cs**

```csharp
// src/SaveForge.Infrastructure/Services/FileBackupService.cs
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;

namespace SaveForge.Infrastructure.Services;

public sealed class FileBackupService : IBackupService
{
    private readonly string _backupRootDir;

    public FileBackupService(string backupRootDir)
    {
        _backupRootDir = backupRootDir;
    }

    public async Task<BackupRecord> BackupAndWriteAsync(
        string filePath, string pluginId, string newContent, CancellationToken ct = default)
    {
        var timestamp = DateTimeOffset.UtcNow;
        var backupDir = Path.Combine(_backupRootDir, pluginId);
        Directory.CreateDirectory(backupDir);

        var fileName = Path.GetFileName(filePath);
        var tsStr = timestamp.ToString("yyyy-MM-ddTHH-mm-ss");
        var backupPath = Path.Combine(backupDir, $"{fileName}.{tsStr}.bak");

        // 1. Copy original to backup
        File.Copy(filePath, backupPath, overwrite: false);

        // 2. Write new content to temp file alongside original
        var tempPath = filePath + ".tmp";
        try
        {
            await File.WriteAllTextAsync(tempPath, newContent, ct);
            // 3. Atomic rename
            File.Move(tempPath, filePath, overwrite: true);
        }
        catch
        {
            // Clean up temp if it exists; original is untouched
            if (File.Exists(tempPath))
                File.Delete(tempPath);
            throw;
        }

        return new BackupRecord
        {
            OriginalPath = filePath,
            BackupPath = backupPath,
            Timestamp = timestamp
        };
    }
}
```

- [ ] **Step 4: Run backup tests — expect pass**

```bash
dotnet test tests/SaveForge.Tests/ --filter "BackupServiceTests" -v minimal
```

Expected: 3/3 PASS. (Note: test 3 passes because backup dir is bad, not write path.)

- [ ] **Step 5: Create SettingsService.cs**

```csharp
// src/SaveForge.Infrastructure/Services/SettingsService.cs
using System.Text.Json;
using SaveForge.Core.Models;

namespace SaveForge.Infrastructure.Services;

public sealed class SettingsService
{
    private readonly string _settingsPath;
    private static readonly JsonSerializerOptions _jsonOptions =
        new() { WriteIndented = true };

    public SettingsService(string settingsPath)
    {
        _settingsPath = settingsPath;
    }

    public async Task<AppSettings> LoadAsync(CancellationToken ct = default)
    {
        if (!File.Exists(_settingsPath))
            return new AppSettings();

        try
        {
            var json = await File.ReadAllTextAsync(_settingsPath, ct);
            return JsonSerializer.Deserialize<AppSettings>(json) ?? new AppSettings();
        }
        catch
        {
            return new AppSettings();
        }
    }

    public async Task SaveAsync(AppSettings settings, CancellationToken ct = default)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(_settingsPath)!);
        var json = JsonSerializer.Serialize(settings, _jsonOptions);
        await File.WriteAllTextAsync(_settingsPath, json, ct);
    }
}
```

- [ ] **Step 6: Create HttpDownloadService.cs**

```csharp
// src/SaveForge.Infrastructure/Services/HttpDownloadService.cs
namespace SaveForge.Infrastructure.Services;

public sealed class HttpDownloadService
{
    private readonly HttpClient _httpClient;

    public HttpDownloadService(HttpClient httpClient)
    {
        _httpClient = httpClient;
    }

    /// <summary>Downloads a URL to a local file, reporting bytes written.</summary>
    public async Task DownloadToFileAsync(
        string url, string destinationPath, IProgress<long>? progress = null, CancellationToken ct = default)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(destinationPath)!);

        using var response = await _httpClient.GetAsync(url, HttpCompletionOption.ResponseHeadersRead, ct);
        response.EnsureSuccessStatusCode();

        await using var stream = await response.Content.ReadAsStreamAsync(ct);
        await using var file = File.Create(destinationPath);

        var buffer = new byte[81920];
        long totalRead = 0;
        int read;
        while ((read = await stream.ReadAsync(buffer, ct)) > 0)
        {
            await file.WriteAsync(buffer.AsMemory(0, read), ct);
            totalRead += read;
            progress?.Report(totalRead);
        }
    }

    /// <summary>Fetches a URL as text, falling back to a local file:// URI.</summary>
    public async Task<string> FetchTextAsync(string url, CancellationToken ct = default)
    {
        if (url.StartsWith("file://", StringComparison.OrdinalIgnoreCase))
        {
            var path = new Uri(url).LocalPath;
            return await File.ReadAllTextAsync(path, ct);
        }
        return await _httpClient.GetStringAsync(url, ct);
    }
}
```

- [ ] **Step 7: Build Infrastructure**

```bash
dotnet build src/SaveForge.Infrastructure/SaveForge.Infrastructure.csproj
```

Expected: Build succeeded, 0 errors.

- [ ] **Step 8: Commit**

```bash
git add src/SaveForge.Infrastructure/ tests/SaveForge.Tests/BackupServiceTests.cs
git commit -m "feat(infrastructure): add settings, backup, and HTTP download services"
```

---

## Task 6: Security — Hash Verifier and Manifest Validator

**Files:**
- Create: `src/SaveForge.Security/HashVerifier.cs`
- Create: `src/SaveForge.Security/ManifestValidator.cs`
- Test: `tests/SaveForge.Tests/HashVerifierTests.cs`
- Test: `tests/SaveForge.Tests/ManifestValidatorTests.cs`

- [ ] **Step 1: Write failing hash verifier tests**

```csharp
// tests/SaveForge.Tests/HashVerifierTests.cs
using System.IO;
using System.Security.Cryptography;
using SaveForge.Security;
using Xunit;

namespace SaveForge.Tests;

public class HashVerifierTests : IDisposable
{
    private readonly string _tempFile = Path.GetTempFileName();

    [Fact]
    public async Task VerifyAsync_CorrectHash_ReturnsTrue()
    {
        await File.WriteAllTextAsync(_tempFile, "hello world");
        var expected = await ComputeHashAsync(_tempFile);

        Assert.True(await HashVerifier.VerifyAsync(_tempFile, expected));
    }

    [Fact]
    public async Task VerifyAsync_WrongHash_ReturnsFalse()
    {
        await File.WriteAllTextAsync(_tempFile, "hello world");

        Assert.False(await HashVerifier.VerifyAsync(_tempFile, "0000000000000000000000000000000000000000000000000000000000000000"));
    }

    [Fact]
    public async Task VerifyAsync_CaseInsensitive_ReturnsTrue()
    {
        await File.WriteAllTextAsync(_tempFile, "test");
        var upper = (await ComputeHashAsync(_tempFile)).ToUpperInvariant();

        Assert.True(await HashVerifier.VerifyAsync(_tempFile, upper));
    }

    private static async Task<string> ComputeHashAsync(string path)
    {
        using var sha = SHA256.Create();
        await using var stream = File.OpenRead(path);
        var bytes = await sha.ComputeHashAsync(stream);
        return Convert.ToHexString(bytes).ToLowerInvariant();
    }

    public void Dispose()
    {
        if (File.Exists(_tempFile)) File.Delete(_tempFile);
    }
}
```

- [ ] **Step 2: Run tests — expect compile failure**

```bash
dotnet test tests/SaveForge.Tests/ --filter "HashVerifierTests" -v minimal
```

Expected: compile error.

- [ ] **Step 3: Create HashVerifier.cs**

```csharp
// src/SaveForge.Security/HashVerifier.cs
using System.Security.Cryptography;

namespace SaveForge.Security;

public static class HashVerifier
{
    /// <summary>
    /// Returns true if the SHA-256 of the file at filePath matches expectedSha256 (hex, case-insensitive).
    /// </summary>
    public static async Task<bool> VerifyAsync(string filePath, string expectedSha256, CancellationToken ct = default)
    {
        using var sha = SHA256.Create();
        await using var stream = File.OpenRead(filePath);
        var hash = await sha.ComputeHashAsync(stream, ct);
        var actual = Convert.ToHexString(hash);
        return string.Equals(actual, expectedSha256, StringComparison.OrdinalIgnoreCase);
    }
}
```

- [ ] **Step 4: Run hash tests — expect pass**

```bash
dotnet test tests/SaveForge.Tests/ --filter "HashVerifierTests" -v minimal
```

Expected: 3/3 PASS.

- [ ] **Step 5: Write failing manifest validator tests**

```csharp
// tests/SaveForge.Tests/ManifestValidatorTests.cs
using SaveForge.SDK;
using SaveForge.Security;
using Xunit;

namespace SaveForge.Tests;

public class ManifestValidatorTests
{
    private static PluginManifest ValidManifest() => new()
    {
        Id = "my-plugin",
        Name = "My Plugin",
        Version = "1.0.0",
        Author = "Author",
        MinHostVersion = "1.0.0",
        PluginAssembly = "MyPlugin.dll"
    };

    [Fact]
    public void Validate_ValidManifest_ReturnsNoErrors()
    {
        var errors = ManifestValidator.Validate(ValidManifest());
        Assert.Empty(errors);
    }

    [Fact]
    public void Validate_EmptyId_ReturnsError()
    {
        var m = ValidManifest() with { Id = "" };
        Assert.Contains(ManifestValidator.Validate(m), e => e.Contains("Id"));
    }

    [Fact]
    public void Validate_InvalidIdChars_ReturnsError()
    {
        var m = ValidManifest() with { Id = "My Plugin!" };
        Assert.Contains(ManifestValidator.Validate(m), e => e.Contains("Id"));
    }

    [Fact]
    public void Validate_InvalidVersion_ReturnsError()
    {
        var m = ValidManifest() with { Version = "not-a-version" };
        Assert.Contains(ManifestValidator.Validate(m), e => e.Contains("Version"));
    }

    [Fact]
    public void Validate_EmptyPluginAssembly_ReturnsError()
    {
        var m = ValidManifest() with { PluginAssembly = "" };
        Assert.Contains(ManifestValidator.Validate(m), e => e.Contains("PluginAssembly"));
    }
}
```

- [ ] **Step 6: Run manifest validator tests — expect compile failure**

```bash
dotnet test tests/SaveForge.Tests/ --filter "ManifestValidatorTests" -v minimal
```

Expected: compile error.

- [ ] **Step 7: Create ManifestValidator.cs**

```csharp
// src/SaveForge.Security/ManifestValidator.cs
using System.Text.RegularExpressions;
using SaveForge.SDK;

namespace SaveForge.Security;

public static class ManifestValidator
{
    private static readonly Regex IdPattern = new("^[a-z0-9-]+$", RegexOptions.Compiled);

    /// <summary>Returns a list of validation error messages. Empty = valid.</summary>
    public static IReadOnlyList<string> Validate(PluginManifest manifest)
    {
        var errors = new List<string>();

        if (string.IsNullOrWhiteSpace(manifest.Id))
            errors.Add("Id is required.");
        else if (!IdPattern.IsMatch(manifest.Id))
            errors.Add("Id must contain only lowercase letters, digits, and hyphens.");

        if (string.IsNullOrWhiteSpace(manifest.Name))
            errors.Add("Name is required.");

        if (!TryParseVersion(manifest.Version, out _))
            errors.Add($"Version '{manifest.Version}' is not a valid version string.");

        if (string.IsNullOrWhiteSpace(manifest.Author))
            errors.Add("Author is required.");

        if (!TryParseVersion(manifest.MinHostVersion, out _))
            errors.Add($"MinHostVersion '{manifest.MinHostVersion}' is not a valid version string.");

        if (manifest.MaxHostVersion is not null && !TryParseVersion(manifest.MaxHostVersion, out _))
            errors.Add($"MaxHostVersion '{manifest.MaxHostVersion}' is not a valid version string.");

        if (string.IsNullOrWhiteSpace(manifest.PluginAssembly))
            errors.Add("PluginAssembly is required.");

        return errors;
    }

    private static bool TryParseVersion(string? s, out Version version)
    {
        version = new Version();
        if (string.IsNullOrWhiteSpace(s)) return false;
        return Version.TryParse(s, out version!);
    }
}
```

- [ ] **Step 8: Run all security tests — expect pass**

```bash
dotnet test tests/SaveForge.Tests/ --filter "HashVerifierTests|ManifestValidatorTests" -v minimal
```

Expected: 8/8 PASS.

- [ ] **Step 9: Commit**

```bash
git add src/SaveForge.Security/ tests/SaveForge.Tests/HashVerifierTests.cs tests/SaveForge.Tests/ManifestValidatorTests.cs
git commit -m "feat(security): add hash verifier and manifest validator with tests"
```

---

## Task 7: PluginHost — Load Context, Loader, Registry, Detection

**Files:**
- Create: `src/SaveForge.PluginHost/PluginLoadContext.cs`
- Create: `src/SaveForge.PluginHost/PluginLoader.cs`
- Create: `src/SaveForge.PluginHost/PluginRegistry.cs`
- Create: `src/SaveForge.PluginHost/FileDetectionService.cs`
- Test: `tests/SaveForge.Tests/PluginLoaderTests.cs`
- Test: `tests/SaveForge.Tests/FileDetectionTests.cs`

- [ ] **Step 1: Create PluginLoadContext.cs**

```csharp
// src/SaveForge.PluginHost/PluginLoadContext.cs
using System.Reflection;
using System.Runtime.Loader;

namespace SaveForge.PluginHost;

/// <summary>
/// Isolated AssemblyLoadContext for a single plugin.
/// Shared assemblies (Avalonia, SDK, System.*) fall through to the default context
/// so the host and plugin share the same type instances.
/// </summary>
internal sealed class PluginLoadContext : AssemblyLoadContext
{
    private readonly AssemblyDependencyResolver _resolver;

    private static readonly string[] SharedPrefixes =
    [
        "Avalonia",
        "SaveForge.SDK",
        "System.",
        "Microsoft.",
        "netstandard",
        "mscorlib",
        "WindowsBase"
    ];

    public PluginLoadContext(string pluginAssemblyPath) : base(isCollectible: true)
    {
        _resolver = new AssemblyDependencyResolver(pluginAssemblyPath);
    }

    protected override Assembly? Load(AssemblyName assemblyName)
    {
        if (IsShared(assemblyName.Name))
            return null; // delegate to Default context

        var path = _resolver.ResolveAssemblyToPath(assemblyName);
        return path is not null ? LoadFromAssemblyPath(path) : null;
    }

    protected override IntPtr LoadUnmanagedDll(string unmanagedDllName)
    {
        var path = _resolver.ResolveUnmanagedDllToPath(unmanagedDllName);
        return path is not null ? LoadUnmanagedDllFromPath(path) : IntPtr.Zero;
    }

    private static bool IsShared(string? name)
    {
        if (name is null) return false;
        foreach (var prefix in SharedPrefixes)
            if (name.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
                return true;
        return false;
    }
}
```

- [ ] **Step 2: Create PluginLoader.cs**

```csharp
// src/SaveForge.PluginHost/PluginLoader.cs
using System.Text.Json;
using SaveForge.Core.Models;
using SaveForge.SDK;
using Microsoft.Extensions.Logging;

namespace SaveForge.PluginHost;

public sealed class PluginLoader
{
    private readonly ILogger<PluginLoader> _logger;
    private readonly Version _hostVersion;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true
    };

    public PluginLoader(ILogger<PluginLoader> logger, string hostVersion)
    {
        _logger = logger;
        _hostVersion = Version.Parse(hostVersion);
    }

    /// <summary>
    /// Loads a plugin from a directory that contains manifest.json and the plugin assembly.
    /// Returns a PluginLoadResult.Succeeded, .Failed, or .Incompatible.
    /// </summary>
    public (PluginLoadResult Result, PluginLoadContext? Context) Load(string pluginDirectory)
    {
        var manifestPath = Path.Combine(pluginDirectory, "manifest.json");
        if (!File.Exists(manifestPath))
            return (new PluginLoadResult.Failed { Reason = "manifest.json not found." }, null);

        PluginManifest manifest;
        try
        {
            var json = File.ReadAllText(manifestPath);
            manifest = JsonSerializer.Deserialize<PluginManifest>(json, JsonOptions)
                       ?? throw new InvalidOperationException("Deserialized manifest was null.");
        }
        catch (Exception ex)
        {
            return (new PluginLoadResult.Failed { Reason = $"Failed to parse manifest: {ex.Message}" }, null);
        }

        // Host compatibility check
        var minVersion = Version.Parse(manifest.MinHostVersion);
        if (_hostVersion < minVersion)
            return (new PluginLoadResult.Incompatible { Reason = $"Requires host >= {manifest.MinHostVersion}" }, null);

        if (manifest.MaxHostVersion is not null)
        {
            var maxVersion = Version.Parse(manifest.MaxHostVersion);
            if (_hostVersion > maxVersion)
                return (new PluginLoadResult.Incompatible { Reason = $"Requires host <= {manifest.MaxHostVersion}" }, null);
        }

        var assemblyPath = Path.Combine(pluginDirectory, manifest.PluginAssembly);
        if (!File.Exists(assemblyPath))
            return (new PluginLoadResult.Failed { Reason = $"Assembly not found: {manifest.PluginAssembly}" }, null);

        try
        {
            var context = new PluginLoadContext(assemblyPath);
            var assembly = context.LoadFromAssemblyPath(assemblyPath);

            var pluginType = assembly.GetTypes().FirstOrDefault(t =>
                t.IsClass && !t.IsAbstract &&
                typeof(IGameEditorPlugin).IsAssignableFrom(t));

            if (pluginType is null)
                return (new PluginLoadResult.Failed { Reason = "No IGameEditorPlugin implementation found in assembly." }, null);

            var plugin = (IGameEditorPlugin)Activator.CreateInstance(pluginType)!;
            return (new PluginLoadResult.Succeeded { Plugin = plugin }, context);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to load plugin from {Dir}", pluginDirectory);
            return (new PluginLoadResult.Failed { Reason = $"Assembly load error: {ex.Message}" }, null);
        }
    }
}
```

- [ ] **Step 3: Create PluginRegistry.cs**

```csharp
// src/SaveForge.PluginHost/PluginRegistry.cs
using System.Collections.Concurrent;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.SDK;

namespace SaveForge.PluginHost;

public sealed class PluginRegistry : IPluginRegistry
{
    private readonly ConcurrentDictionary<string, (InstalledPlugin Plugin, PluginLoadContext? Context)> _entries = new();

    public void Register(InstalledPlugin plugin, PluginLoadContext? context = null)
    {
        _entries[plugin.Manifest.Id] = (plugin, context);
    }

    // IPluginRegistry implementation
    void IPluginRegistry.Register(InstalledPlugin plugin) => Register(plugin, null);

    public void Remove(string pluginId)
    {
        if (_entries.TryRemove(pluginId, out var entry))
            entry.Context?.Unload();
    }

    public IReadOnlyList<InstalledPlugin> GetAll() =>
        _entries.Values.Select(e => e.Plugin).ToList();

    public InstalledPlugin? TryGet(string pluginId) =>
        _entries.TryGetValue(pluginId, out var e) ? e.Plugin : null;

    public IReadOnlyList<IGameEditorPlugin> GetLoaded() =>
        _entries.Values
            .Where(e => e.Plugin.LoadState == PluginLoadState.Loaded && e.Plugin.Plugin is not null)
            .Select(e => e.Plugin.Plugin!)
            .ToList();
}
```

- [ ] **Step 4: Create FileDetectionService.cs**

```csharp
// src/SaveForge.PluginHost/FileDetectionService.cs
using SaveForge.Core.Interfaces;

namespace SaveForge.PluginHost;

public sealed class FileDetectionService : IFileDetectionService
{
    private readonly IPluginRegistry _registry;
    private const int HeaderSize = 512;

    public FileDetectionService(IPluginRegistry registry)
    {
        _registry = registry;
    }

    public async Task<IReadOnlyList<DetectionResult>> DetectAsync(string filePath, CancellationToken ct = default)
    {
        var file = new FileInfo(filePath);
        if (!file.Exists)
            return Array.Empty<DetectionResult>();

        var header = await ReadHeaderAsync(filePath, ct);

        var results = new List<DetectionResult>();
        foreach (var plugin in _registry.GetLoaded())
        {
            try
            {
                var (canOpen, priority) = plugin.CanOpen(file, header);
                if (canOpen)
                    results.Add(new DetectionResult(plugin, priority));
            }
            catch
            {
                // Plugin detection failures are non-fatal
            }
        }

        return results.OrderByDescending(r => r.Priority).ToList();
    }

    private static async Task<byte[]> ReadHeaderAsync(string filePath, CancellationToken ct)
    {
        await using var stream = File.OpenRead(filePath);
        var buffer = new byte[Math.Min(HeaderSize, stream.Length)];
        _ = await stream.ReadAsync(buffer, ct);
        return buffer;
    }
}
```

- [ ] **Step 5: Write PluginLoader tests (using a fake in-memory scenario)**

```csharp
// tests/SaveForge.Tests/PluginLoaderTests.cs
using System.IO;
using System.Text.Json;
using SaveForge.Core.Models;
using SaveForge.PluginHost;
using Microsoft.Extensions.Logging.Abstractions;
using Xunit;

namespace SaveForge.Tests;

public class PluginLoaderTests : IDisposable
{
    private readonly string _tempDir = Path.Combine(Path.GetTempPath(), "gse-loader-" + Guid.NewGuid());

    public PluginLoaderTests() => Directory.CreateDirectory(_tempDir);

    [Fact]
    public void Load_MissingManifest_ReturnsFailed()
    {
        var loader = new PluginLoader(NullLogger<PluginLoader>.Instance, "1.0.0");
        var (result, _) = loader.Load(_tempDir);

        Assert.IsType<PluginLoadResult.Failed>(result);
    }

    [Fact]
    public void Load_IncompatibleMinVersion_ReturnsIncompatible()
    {
        WriteManifest(new { id = "x", name = "X", version = "1.0.0", author = "A",
            minHostVersion = "99.0.0", pluginAssembly = "x.dll" });

        var loader = new PluginLoader(NullLogger<PluginLoader>.Instance, "1.0.0");
        var (result, _) = loader.Load(_tempDir);

        Assert.IsType<PluginLoadResult.Incompatible>(result);
    }

    [Fact]
    public void Load_IncompatibleMaxVersion_ReturnsIncompatible()
    {
        WriteManifest(new { id = "x", name = "X", version = "1.0.0", author = "A",
            minHostVersion = "1.0.0", maxHostVersion = "1.0.0", pluginAssembly = "x.dll" });

        var loader = new PluginLoader(NullLogger<PluginLoader>.Instance, "2.0.0");
        var (result, _) = loader.Load(_tempDir);

        Assert.IsType<PluginLoadResult.Incompatible>(result);
    }

    [Fact]
    public void Load_MissingAssembly_ReturnsFailed()
    {
        WriteManifest(new { id = "x", name = "X", version = "1.0.0", author = "A",
            minHostVersion = "1.0.0", pluginAssembly = "missing.dll" });

        var loader = new PluginLoader(NullLogger<PluginLoader>.Instance, "1.0.0");
        var (result, _) = loader.Load(_tempDir);

        Assert.IsType<PluginLoadResult.Failed>(result);
    }

    private void WriteManifest(object manifest)
    {
        var json = JsonSerializer.Serialize(manifest);
        File.WriteAllText(Path.Combine(_tempDir, "manifest.json"), json);
    }

    public void Dispose() => Directory.Delete(_tempDir, recursive: true);
}
```

- [ ] **Step 6: Write FileDetection tests**

```csharp
// tests/SaveForge.Tests/FileDetectionTests.cs
using System.IO;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.PluginHost;
using SaveForge.SDK;
using Xunit;

namespace SaveForge.Tests;

public class FileDetectionTests : IDisposable
{
    private readonly string _tempFile = Path.GetTempFileName();

    [Fact]
    public async Task DetectAsync_NoPlugins_ReturnsEmpty()
    {
        var registry = new PluginRegistry();
        var svc = new FileDetectionService(registry);

        var results = await svc.DetectAsync(_tempFile);

        Assert.Empty(results);
    }

    [Fact]
    public async Task DetectAsync_MatchingPlugin_ReturnsIt()
    {
        var registry = new PluginRegistry();
        registry.Register(new InstalledPlugin
        {
            Manifest = MakeManifest("p1"),
            LoadState = PluginLoadState.Loaded,
            AssemblyPath = "",
            Plugin = new AlwaysMatchPlugin("p1", priority: 0)
        });
        var svc = new FileDetectionService(registry);

        var results = await svc.DetectAsync(_tempFile);

        Assert.Single(results);
        Assert.Equal("p1", results[0].Plugin.Manifest.Id);
    }

    [Fact]
    public async Task DetectAsync_MultipleMatches_SortedByPriorityDescending()
    {
        var registry = new PluginRegistry();
        registry.Register(MakeInstalled("low", new AlwaysMatchPlugin("low", priority: 0)));
        registry.Register(MakeInstalled("high", new AlwaysMatchPlugin("high", priority: 10)));
        var svc = new FileDetectionService(registry);

        var results = await svc.DetectAsync(_tempFile);

        Assert.Equal(2, results.Count);
        Assert.Equal("high", results[0].Plugin.Manifest.Id);
    }

    private static PluginManifest MakeManifest(string id) => new()
    {
        Id = id, Name = id, Version = "1.0.0",
        Author = "Test", MinHostVersion = "1.0.0", PluginAssembly = "x.dll"
    };

    private static InstalledPlugin MakeInstalled(string id, IGameEditorPlugin plugin) => new()
    {
        Manifest = MakeManifest(id),
        LoadState = PluginLoadState.Loaded,
        AssemblyPath = "",
        Plugin = plugin
    };

    public void Dispose()
    {
        if (File.Exists(_tempFile)) File.Delete(_tempFile);
    }

    // Test double
    private sealed class AlwaysMatchPlugin : IGameEditorPlugin
    {
        private readonly int _priority;
        public AlwaysMatchPlugin(string id, int priority)
        {
            _priority = priority;
            Manifest = new PluginManifest { Id = id, Name = id, Version = "1.0.0",
                Author = "T", MinHostVersion = "1.0.0", PluginAssembly = "x.dll" };
            Capabilities = new SaveCapabilities();
        }
        public PluginManifest Manifest { get; }
        public SaveCapabilities Capabilities { get; }
        public (bool, int) CanOpen(FileInfo f, ReadOnlySpan<byte> h) => (true, _priority);
        public Task<ISaveDocument> OpenAsync(string p, CancellationToken ct) => throw new NotImplementedException();
    }
}
```

- [ ] **Step 7: Run PluginHost tests — expect pass**

```bash
dotnet test tests/SaveForge.Tests/ --filter "PluginLoaderTests|FileDetectionTests" -v minimal
```

Expected: 7/7 PASS.

- [ ] **Step 8: Commit**

```bash
git add src/SaveForge.PluginHost/ tests/SaveForge.Tests/PluginLoaderTests.cs tests/SaveForge.Tests/FileDetectionTests.cs
git commit -m "feat(plugin-host): add load context, loader, registry, and file detection"
```

---

## Task 8: PluginHost — Startup Scanner

**Files:**
- Create: `src/SaveForge.PluginHost/PluginHostService.cs`

- [ ] **Step 1: Create PluginHostService.cs**

```csharp
// src/SaveForge.PluginHost/PluginHostService.cs
using SaveForge.Core.Models;
using Microsoft.Extensions.Logging;

namespace SaveForge.PluginHost;

/// <summary>
/// Scans the plugin directory on startup and loads all valid plugins into the registry.
/// </summary>
public sealed class PluginHostService
{
    private readonly PluginLoader _loader;
    private readonly PluginRegistry _registry;
    private readonly ILogger<PluginHostService> _logger;

    public PluginHostService(PluginLoader loader, PluginRegistry registry, ILogger<PluginHostService> logger)
    {
        _loader = loader;
        _registry = registry;
        _logger = logger;
    }

    public void LoadAll(string pluginDirectory)
    {
        if (!Directory.Exists(pluginDirectory))
        {
            _logger.LogInformation("Plugin directory {Dir} does not exist; no plugins loaded.", pluginDirectory);
            return;
        }

        // Each plugin lives at: {pluginDirectory}/{id}/{version}/
        foreach (var idDir in Directory.EnumerateDirectories(pluginDirectory))
        {
            var versionDirs = Directory.EnumerateDirectories(idDir)
                .OrderByDescending(d => d) // use latest version if multiple
                .Take(1);

            foreach (var versionDir in versionDirs)
            {
                var manifestPath = Path.Combine(versionDir, "manifest.json");
                if (!File.Exists(manifestPath)) continue;

                var (result, context) = _loader.Load(versionDir);

                InstalledPlugin installed;
                switch (result)
                {
                    case PluginLoadResult.Succeeded s:
                        installed = new InstalledPlugin
                        {
                            Manifest = s.Plugin.Manifest,
                            LoadState = PluginLoadState.Loaded,
                            AssemblyPath = versionDir,
                            Plugin = s.Plugin
                        };
                        _logger.LogInformation("Loaded plugin {Id} v{Ver}", s.Plugin.Manifest.Id, s.Plugin.Manifest.Version);
                        break;

                    case PluginLoadResult.Incompatible i:
                        var mIncompat = TryReadManifestId(manifestPath);
                        installed = new InstalledPlugin
                        {
                            Manifest = mIncompat,
                            LoadState = PluginLoadState.Incompatible,
                            AssemblyPath = versionDir,
                            FailureReason = i.Reason
                        };
                        _logger.LogWarning("Plugin at {Dir} is incompatible: {Reason}", versionDir, i.Reason);
                        break;

                    default:
                        var mFailed = TryReadManifestId(manifestPath);
                        var reason = result is PluginLoadResult.Failed f ? f.Reason : "Unknown";
                        installed = new InstalledPlugin
                        {
                            Manifest = mFailed,
                            LoadState = PluginLoadState.Failed,
                            AssemblyPath = versionDir,
                            FailureReason = reason
                        };
                        _logger.LogError("Plugin at {Dir} failed to load: {Reason}", versionDir, reason);
                        break;
                }

                _registry.Register(installed, context);
            }
        }
    }

    private static PluginManifest TryReadManifestId(string manifestPath)
    {
        try
        {
            var json = File.ReadAllText(manifestPath);
            using var doc = System.Text.Json.JsonDocument.Parse(json);
            var id = doc.RootElement.TryGetProperty("id", out var idEl) ? idEl.GetString() ?? "unknown" : "unknown";
            var name = doc.RootElement.TryGetProperty("name", out var nameEl) ? nameEl.GetString() ?? id : id;
            return new PluginManifest { Id = id, Name = name, Version = "0.0.0", Author = "?", MinHostVersion = "0.0.0", PluginAssembly = "" };
        }
        catch
        {
            return new PluginManifest { Id = "unknown", Name = "Unknown", Version = "0.0.0", Author = "?", MinHostVersion = "0.0.0", PluginAssembly = "" };
        }
    }
}
```

- [ ] **Step 2: Build PluginHost**

```bash
dotnet build src/SaveForge.PluginHost/SaveForge.PluginHost.csproj
```

Expected: Build succeeded.

- [ ] **Step 3: Commit**

```bash
git add src/SaveForge.PluginHost/PluginHostService.cs
git commit -m "feat(plugin-host): add startup scanner service"
```

---

## Task 9: Store — Catalog Service and Plugin Installer

**Files:**
- Create: `src/SaveForge.Store/Dtos/CatalogManifestDto.cs`
- Create: `src/SaveForge.Store/Dtos/CatalogEntryDto.cs`
- Create: `src/SaveForge.Store/CatalogService.cs`
- Create: `src/SaveForge.Store/PluginInstaller.cs`
- Create: `src/SaveForge.Store/UpdateChecker.cs`
- Test: `tests/SaveForge.Tests/CatalogServiceTests.cs`
- Test: `tests/SaveForge.Tests/PluginInstallerTests.cs`

- [ ] **Step 1: Create DTOs**

```csharp
// src/SaveForge.Store/Dtos/CatalogManifestDto.cs
using System.Text.Json.Serialization;

namespace SaveForge.Store.Dtos;

public sealed class CatalogManifestDto
{
    [JsonPropertyName("schemaVersion")]
    public string SchemaVersion { get; init; } = "1.0";

    [JsonPropertyName("generatedAt")]
    public DateTime GeneratedAt { get; init; }

    [JsonPropertyName("plugins")]
    public CatalogEntryDto[] Plugins { get; init; } = Array.Empty<CatalogEntryDto>();
}
```

```csharp
// src/SaveForge.Store/Dtos/CatalogEntryDto.cs
using System.Text.Json.Serialization;

namespace SaveForge.Store.Dtos;

public sealed class CatalogEntryDto
{
    [JsonPropertyName("id")]
    public required string Id { get; init; }

    [JsonPropertyName("name")]
    public required string Name { get; init; }

    [JsonPropertyName("description")]
    public string Description { get; init; } = string.Empty;

    [JsonPropertyName("author")]
    public required string Author { get; init; }

    [JsonPropertyName("latestVersion")]
    public required string LatestVersion { get; init; }

    [JsonPropertyName("minHostVersion")]
    public required string MinHostVersion { get; init; }

    [JsonPropertyName("maxHostVersion")]
    public string? MaxHostVersion { get; init; }

    [JsonPropertyName("packageUrl")]
    public required string PackageUrl { get; init; }

    [JsonPropertyName("sha256")]
    public required string Sha256 { get; init; }

    [JsonPropertyName("iconUrl")]
    public string? IconUrl { get; init; }

    [JsonPropertyName("screenshotUrls")]
    public string[] ScreenshotUrls { get; init; } = Array.Empty<string>();

    [JsonPropertyName("tags")]
    public string[] Tags { get; init; } = Array.Empty<string>();

    [JsonPropertyName("repositoryUrl")]
    public string? RepositoryUrl { get; init; }
}
```

- [ ] **Step 2: Write failing catalog tests**

```csharp
// tests/SaveForge.Tests/CatalogServiceTests.cs
using System.IO;
using System.Text.Json;
using SaveForge.Core.Models;
using SaveForge.Infrastructure.Services;
using SaveForge.Store;
using SaveForge.Store.Dtos;
using Xunit;

namespace SaveForge.Tests;

public class CatalogServiceTests : IDisposable
{
    private readonly string _tempDir = Path.Combine(Path.GetTempPath(), "gse-catalog-" + Guid.NewGuid());

    public CatalogServiceTests() => Directory.CreateDirectory(_tempDir);

    [Fact]
    public async Task GetCatalogAsync_LocalFile_ReturnsParsedEntries()
    {
        var catalogPath = Path.Combine(_tempDir, "catalog.json");
        var dto = new CatalogManifestDto
        {
            SchemaVersion = "1.0",
            GeneratedAt = DateTime.UtcNow,
            Plugins = [new CatalogEntryDto
            {
                Id = "test-plugin", Name = "Test", Author = "A",
                LatestVersion = "1.0.0", MinHostVersion = "1.0.0",
                PackageUrl = "http://example.com/p.zip", Sha256 = "abc"
            }]
        };
        await File.WriteAllTextAsync(catalogPath, JsonSerializer.Serialize(dto));

        var url = new Uri(catalogPath).AbsoluteUri;
        var cachePath = Path.Combine(_tempDir, "cache.json");
        var downloader = new HttpDownloadService(new HttpClient());
        var svc = new CatalogService(downloader, url, cachePath, installedPluginIds: new HashSet<string>());

        var entries = await svc.GetCatalogAsync();

        Assert.Single(entries);
        Assert.Equal("test-plugin", entries[0].Id);
    }

    [Fact]
    public async Task GetCatalogAsync_MarksInstalledPlugin()
    {
        var catalogPath = Path.Combine(_tempDir, "catalog2.json");
        var dto = new CatalogManifestDto
        {
            Plugins = [new CatalogEntryDto
            {
                Id = "installed-plugin", Name = "Installed", Author = "A",
                LatestVersion = "1.0.0", MinHostVersion = "1.0.0",
                PackageUrl = "http://example.com/p.zip", Sha256 = "abc"
            }]
        };
        await File.WriteAllTextAsync(catalogPath, JsonSerializer.Serialize(dto));

        var url = new Uri(catalogPath).AbsoluteUri;
        var cachePath = Path.Combine(_tempDir, "cache2.json");
        var downloader = new HttpDownloadService(new HttpClient());
        var svc = new CatalogService(downloader, url, cachePath,
            installedPluginIds: new HashSet<string> { "installed-plugin" });

        var entries = await svc.GetCatalogAsync();

        Assert.True(entries[0].IsInstalled);
    }

    public void Dispose() => Directory.Delete(_tempDir, recursive: true);
}
```

- [ ] **Step 3: Create CatalogService.cs**

```csharp
// src/SaveForge.Store/CatalogService.cs
using System.Text.Json;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.Infrastructure.Services;
using SaveForge.Store.Dtos;

namespace SaveForge.Store;

public sealed class CatalogService : ICatalogService
{
    private readonly HttpDownloadService _downloader;
    private readonly string _catalogUrl;
    private readonly string _cachePath;
    private readonly IReadOnlySet<string> _installedPluginIds;
    private IReadOnlyList<CatalogEntry>? _cache;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true
    };

    public CatalogService(
        HttpDownloadService downloader,
        string catalogUrl,
        string cachePath,
        IReadOnlySet<string> installedPluginIds)
    {
        _downloader = downloader;
        _catalogUrl = catalogUrl;
        _cachePath = cachePath;
        _installedPluginIds = installedPluginIds;
    }

    public async Task<IReadOnlyList<CatalogEntry>> GetCatalogAsync(
        bool forceRefresh = false, CancellationToken ct = default)
    {
        if (_cache is not null && !forceRefresh)
            return _cache;

        CatalogManifestDto? dto = null;

        try
        {
            var json = await _downloader.FetchTextAsync(_catalogUrl, ct);
            dto = JsonSerializer.Deserialize<CatalogManifestDto>(json, JsonOptions);
            // Cache to disk
            Directory.CreateDirectory(Path.GetDirectoryName(_cachePath)!);
            await File.WriteAllTextAsync(_cachePath, json, ct);
        }
        catch
        {
            // Fall back to disk cache
            if (File.Exists(_cachePath))
            {
                try
                {
                    var json = await File.ReadAllTextAsync(_cachePath, ct);
                    dto = JsonSerializer.Deserialize<CatalogManifestDto>(json, JsonOptions);
                }
                catch { /* swallow */ }
            }
        }

        if (dto is null)
            return Array.Empty<CatalogEntry>();

        _cache = dto.Plugins.Select(ToEntry).ToList();
        return _cache;
    }

    private CatalogEntry ToEntry(CatalogEntryDto dto)
    {
        var isInstalled = _installedPluginIds.Contains(dto.Id);
        return new CatalogEntry
        {
            Id = dto.Id,
            Name = dto.Name,
            Description = dto.Description,
            Author = dto.Author,
            LatestVersion = dto.LatestVersion,
            MinHostVersion = dto.MinHostVersion,
            MaxHostVersion = dto.MaxHostVersion,
            PackageUrl = dto.PackageUrl,
            Sha256 = dto.Sha256,
            IconUrl = dto.IconUrl,
            ScreenshotUrls = dto.ScreenshotUrls,
            Tags = dto.Tags,
            RepositoryUrl = dto.RepositoryUrl,
            IsInstalled = isInstalled,
            HasUpdate = false // UpdateChecker sets this separately
        };
    }
}
```

- [ ] **Step 4: Run catalog tests — expect pass**

```bash
dotnet test tests/SaveForge.Tests/ --filter "CatalogServiceTests" -v minimal
```

Expected: 2/2 PASS.

- [ ] **Step 5: Write failing installer tests**

```csharp
// tests/SaveForge.Tests/PluginInstallerTests.cs
using System.IO;
using System.IO.Compression;
using System.Security.Cryptography;
using System.Text.Json;
using SaveForge.Core.Models;
using SaveForge.Infrastructure.Services;
using SaveForge.PluginHost;
using SaveForge.Security;
using SaveForge.Store;
using SaveForge.Store.Dtos;
using Microsoft.Extensions.Logging.Abstractions;
using Xunit;

namespace SaveForge.Tests;

public class PluginInstallerTests : IDisposable
{
    private readonly string _tempDir = Path.Combine(Path.GetTempPath(), "gse-install-" + Guid.NewGuid());
    private readonly string _pluginDir;
    private readonly string _catalogDir;

    public PluginInstallerTests()
    {
        Directory.CreateDirectory(_tempDir);
        _pluginDir = Path.Combine(_tempDir, "plugins");
        _catalogDir = Path.Combine(_tempDir, "catalog");
        Directory.CreateDirectory(_catalogDir);
    }

    [Fact]
    public async Task InstallAsync_ValidPackage_ExtractsToPluginDir()
    {
        var (zipPath, sha256) = CreateValidPluginZip("my-plugin", "1.0.0");
        var catalogUrl = CreateCatalog("my-plugin", "1.0.0", new Uri(zipPath).AbsoluteUri, sha256);

        var installer = CreateInstaller(catalogUrl);
        await installer.InstallAsync("my-plugin");

        var expectedDir = Path.Combine(_pluginDir, "my-plugin", "1.0.0");
        Assert.True(Directory.Exists(expectedDir));
        Assert.True(File.Exists(Path.Combine(expectedDir, "manifest.json")));
    }

    [Fact]
    public async Task InstallAsync_BadHash_ThrowsAndDoesNotInstall()
    {
        var (zipPath, _) = CreateValidPluginZip("bad-plugin", "1.0.0");
        var catalogUrl = CreateCatalog("bad-plugin", "1.0.0", new Uri(zipPath).AbsoluteUri, "0000000000000000000000000000000000000000000000000000000000000000");

        var installer = CreateInstaller(catalogUrl);

        await Assert.ThrowsAnyAsync<Exception>(() => installer.InstallAsync("bad-plugin"));
        Assert.False(Directory.Exists(Path.Combine(_pluginDir, "bad-plugin")));
    }

    private PluginInstaller CreateInstaller(string catalogUrl)
    {
        var downloader = new HttpDownloadService(new HttpClient());
        var registry = new PluginRegistry();
        var loader = new PluginLoader(NullLogger<PluginLoader>.Instance, "1.0.0");
        var cachePath = Path.Combine(_tempDir, "cache.json");
        var catalogService = new CatalogService(downloader, catalogUrl, cachePath, new HashSet<string>());
        return new PluginInstaller(downloader, catalogService, registry, loader,
            NullLogger<PluginInstaller>.Instance, _pluginDir);
    }

    private (string ZipPath, string Sha256) CreateValidPluginZip(string id, string version)
    {
        var zipPath = Path.Combine(_tempDir, $"{id}-{version}.zip");
        using var zip = ZipFile.Open(zipPath, ZipArchiveMode.Create);
        var manifest = new { id, name = id, version, author = "Test",
            minHostVersion = "1.0.0", pluginAssembly = "dummy.dll" };
        var entry = zip.CreateEntry("manifest.json");
        using var w = new StreamWriter(entry.Open());
        w.Write(JsonSerializer.Serialize(manifest));

        var sha256 = ComputeSha256(zipPath);
        return (zipPath, sha256);
    }

    private string CreateCatalog(string id, string version, string packageUrl, string sha256)
    {
        var dto = new CatalogManifestDto
        {
            Plugins = [new CatalogEntryDto { Id = id, Name = id, Author = "T",
                LatestVersion = version, MinHostVersion = "1.0.0",
                PackageUrl = packageUrl, Sha256 = sha256 }]
        };
        var path = Path.Combine(_catalogDir, "catalog.json");
        File.WriteAllText(path, JsonSerializer.Serialize(dto));
        return new Uri(path).AbsoluteUri;
    }

    private static string ComputeSha256(string path)
    {
        using var sha = SHA256.Create();
        using var stream = File.OpenRead(path);
        return Convert.ToHexString(sha.ComputeHash(stream)).ToLowerInvariant();
    }

    public void Dispose() => Directory.Delete(_tempDir, recursive: true);
}
```

- [ ] **Step 6: Create PluginInstaller.cs**

```csharp
// src/SaveForge.Store/PluginInstaller.cs
using System.IO.Compression;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.Infrastructure.Services;
using SaveForge.PluginHost;
using SaveForge.Security;
using Microsoft.Extensions.Logging;

namespace SaveForge.Store;

public sealed class PluginInstaller : IPluginInstaller
{
    private readonly HttpDownloadService _downloader;
    private readonly ICatalogService _catalog;
    private readonly PluginRegistry _registry;
    private readonly PluginLoader _loader;
    private readonly ILogger<PluginInstaller> _logger;
    private readonly string _pluginDirectory;

    public PluginInstaller(
        HttpDownloadService downloader,
        ICatalogService catalog,
        PluginRegistry registry,
        PluginLoader loader,
        ILogger<PluginInstaller> logger,
        string pluginDirectory)
    {
        _downloader = downloader;
        _catalog = catalog;
        _registry = registry;
        _loader = loader;
        _logger = logger;
        _pluginDirectory = pluginDirectory;
    }

    public async Task InstallAsync(string catalogEntryId, IProgress<long>? progress = null, CancellationToken ct = default)
    {
        var entries = await _catalog.GetCatalogAsync(ct: ct);
        var entry = entries.FirstOrDefault(e => e.Id == catalogEntryId)
            ?? throw new InvalidOperationException($"Plugin '{catalogEntryId}' not found in catalog.");

        var tempZip = Path.GetTempFileName() + ".zip";
        try
        {
            // 1. Download
            _logger.LogInformation("Downloading plugin {Id} from {Url}", catalogEntryId, entry.PackageUrl);
            await _downloader.DownloadToFileAsync(entry.PackageUrl, tempZip, progress, ct);

            // 2. Verify SHA-256
            if (!await HashVerifier.VerifyAsync(tempZip, entry.Sha256, ct))
                throw new InvalidOperationException($"SHA-256 verification failed for plugin '{catalogEntryId}'.");

            // 3. Unpack
            var targetDir = Path.Combine(_pluginDirectory, entry.Id, entry.LatestVersion);
            Directory.CreateDirectory(targetDir);
            ZipFile.ExtractToDirectory(tempZip, targetDir, overwriteFiles: true);

            // 4. Load
            var (result, context) = _loader.Load(targetDir);

            InstalledPlugin installed;
            switch (result)
            {
                case PluginLoadResult.Succeeded s:
                    installed = new InstalledPlugin
                    {
                        Manifest = s.Plugin.Manifest,
                        LoadState = PluginLoadState.Loaded,
                        AssemblyPath = targetDir,
                        Plugin = s.Plugin
                    };
                    break;

                case PluginLoadResult.Incompatible i:
                    Directory.Delete(targetDir, recursive: true);
                    throw new InvalidOperationException($"Plugin incompatible: {i.Reason}");

                default:
                    var reason = result is PluginLoadResult.Failed f ? f.Reason : "Unknown";
                    Directory.Delete(targetDir, recursive: true);
                    throw new InvalidOperationException($"Plugin failed to load: {reason}");
            }

            _registry.Register(installed, context);
            _logger.LogInformation("Installed plugin {Id} v{Ver}", catalogEntryId, entry.LatestVersion);
        }
        finally
        {
            if (File.Exists(tempZip)) File.Delete(tempZip);
        }
    }

    public async Task RemoveAsync(string pluginId, CancellationToken ct = default)
    {
        var existing = _registry.TryGet(pluginId);
        if (existing is null) return;

        _registry.Remove(pluginId);

        var pluginIdDir = Path.Combine(_pluginDirectory, pluginId);
        if (Directory.Exists(pluginIdDir))
            Directory.Delete(pluginIdDir, recursive: true);

        _logger.LogInformation("Removed plugin {Id}", pluginId);
        await Task.CompletedTask;
    }

    public async Task UpdateAsync(string pluginId, IProgress<long>? progress = null, CancellationToken ct = default)
    {
        var oldPlugin = _registry.TryGet(pluginId);
        var oldVersion = oldPlugin?.Manifest.Version;

        await InstallAsync(pluginId, progress, ct);

        // Remove old version directory if different from new
        if (oldVersion is not null)
        {
            var oldVersionDir = Path.Combine(_pluginDirectory, pluginId, oldVersion);
            var newPlugin = _registry.TryGet(pluginId);
            if (newPlugin is not null && oldVersion != newPlugin.Manifest.Version &&
                Directory.Exists(oldVersionDir))
            {
                Directory.Delete(oldVersionDir, recursive: true);
            }
        }
    }
}
```

- [ ] **Step 7: Create UpdateChecker.cs**

```csharp
// src/SaveForge.Store/UpdateChecker.cs
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;

namespace SaveForge.Store;

public sealed class UpdateChecker : IUpdateChecker
{
    private readonly ICatalogService _catalog;
    private readonly IPluginRegistry _registry;

    public UpdateChecker(ICatalogService catalog, IPluginRegistry registry)
    {
        _catalog = catalog;
        _registry = registry;
    }

    public async Task<IReadOnlyList<UpdateInfo>> CheckAsync(CancellationToken ct = default)
    {
        var entries = await _catalog.GetCatalogAsync(forceRefresh: true, ct: ct);
        var updates = new List<UpdateInfo>();

        foreach (var installed in _registry.GetAll())
        {
            if (installed.LoadState != PluginLoadState.Loaded) continue;

            var catalogEntry = entries.FirstOrDefault(e => e.Id == installed.Manifest.Id);
            if (catalogEntry is null) continue;

            if (IsNewerVersion(installed.Manifest.Version, catalogEntry.LatestVersion))
                updates.Add(new UpdateInfo(installed.Manifest.Id, installed.Manifest.Version, catalogEntry.LatestVersion));
        }

        return updates;
    }

    private static bool IsNewerVersion(string current, string latest)
    {
        if (!Version.TryParse(current, out var cur)) return false;
        if (!Version.TryParse(latest, out var lat)) return false;
        return lat > cur;
    }
}
```

- [ ] **Step 8: Run store tests — expect pass**

```bash
dotnet test tests/SaveForge.Tests/ --filter "CatalogServiceTests|PluginInstallerTests" -v minimal
```

Expected: 4/4 PASS.

- [ ] **Step 9: Commit**

```bash
git add src/SaveForge.Store/ tests/SaveForge.Tests/CatalogServiceTests.cs tests/SaveForge.Tests/PluginInstallerTests.cs
git commit -m "feat(store): add catalog service, plugin installer, and update checker"
```

---

## Task 10: Sample Plugin — StardewLike

**Files:**
- Create: `src/SaveForge.SamplePlugin.StardewLike/manifest.json`
- Create: `src/SaveForge.SamplePlugin.StardewLike/StardewSchema.cs`
- Create: `src/SaveForge.SamplePlugin.StardewLike/StardewSaveDocument.cs`
- Create: `src/SaveForge.SamplePlugin.StardewLike/StardewPlugin.cs`
- Test: `tests/SaveForge.Tests/StardewPluginTests.cs`

- [ ] **Step 1: Write failing StardewLike tests**

```csharp
// tests/SaveForge.Tests/StardewPluginTests.cs
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;
using SaveForge.SDK;
using SaveForge.SamplePlugin.StardewLike;
using Xunit;

namespace SaveForge.Tests;

public class StardewPluginTests : IDisposable
{
    private readonly string _tempFile = Path.GetTempFileName() + ".json";

    private const string SampleSave = """
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
        """;

    [Fact]
    public async Task OpenAsync_LoadsPlayerName()
    {
        await File.WriteAllTextAsync(_tempFile, SampleSave);
        var plugin = new StardewPlugin();
        var doc = await plugin.OpenAsync(_tempFile);

        Assert.Equal("Farmer", doc.GetValue("player.name"));
    }

    [Fact]
    public async Task SetValue_MakesDocumentDirty()
    {
        await File.WriteAllTextAsync(_tempFile, SampleSave);
        var plugin = new StardewPlugin();
        var doc = await plugin.OpenAsync(_tempFile);

        doc.SetValue("player.name", "NewFarmer");

        Assert.True(doc.IsDirty);
    }

    [Fact]
    public async Task SaveAsync_RoundTrip_PreservesChanges()
    {
        await File.WriteAllTextAsync(_tempFile, SampleSave);
        var plugin = new StardewPlugin();
        var doc = await plugin.OpenAsync(_tempFile);

        doc.SetValue("player.money", 9999);
        await doc.SaveAsync();

        var doc2 = await plugin.OpenAsync(_tempFile);
        var money = Convert.ToInt32(doc2.GetValue("player.money"));
        Assert.Equal(9999, money);
    }

    [Fact]
    public async Task CanOpen_JsonWithPlayerKey_ReturnsTrue()
    {
        await File.WriteAllTextAsync(_tempFile, SampleSave);
        var plugin = new StardewPlugin();
        var header = System.Text.Encoding.UTF8.GetBytes(SampleSave[..Math.Min(256, SampleSave.Length)]);

        var (canOpen, _) = plugin.CanOpen(new FileInfo(_tempFile), header);

        Assert.True(canOpen);
    }

    [Fact]
    public async Task CanOpen_NonJsonFile_ReturnsFalse()
    {
        var txtFile = _tempFile + ".txt";
        await File.WriteAllTextAsync(txtFile, "hello");
        var plugin = new StardewPlugin();
        var header = System.Text.Encoding.UTF8.GetBytes("hello");

        var (canOpen, _) = plugin.CanOpen(new FileInfo(txtFile), header);

        Assert.False(canOpen);
        if (File.Exists(txtFile)) File.Delete(txtFile);
    }

    [Fact]
    public async Task GetEditorDefinition_ReturnsSchema()
    {
        await File.WriteAllTextAsync(_tempFile, SampleSave);
        var plugin = new StardewPlugin();
        var doc = await plugin.OpenAsync(_tempFile);
        var def = doc.GetEditorDefinition();

        Assert.Equal(EditorKind.Schema, def.Kind);
        Assert.NotNull(def.Schema);
    }

    public void Dispose()
    {
        if (File.Exists(_tempFile)) File.Delete(_tempFile);
    }
}
```

- [ ] **Step 2: Run tests — expect compile failure**

```bash
dotnet test tests/SaveForge.Tests/ --filter "StardewPluginTests" -v minimal
```

Expected: compile error.

- [ ] **Step 3: Create manifest.json**

```json
{
  "id": "stardew-like",
  "name": "StardewLike Save Editor",
  "description": "Edit saves for the StardewLike sample game.",
  "author": "SaveForge Team",
  "version": "1.0.0",
  "minHostVersion": "1.0.0",
  "maxHostVersion": null,
  "pluginAssembly": "SaveForge.SamplePlugin.StardewLike.dll"
}
```

- [ ] **Step 4: Create StardewSchema.cs**

```csharp
// src/SaveForge.SamplePlugin.StardewLike/StardewSchema.cs
using SaveForge.SDK.Schema;

namespace SaveForge.SamplePlugin.StardewLike;

internal static class StardewSchema
{
    public static FormSchema Build() => new()
    {
        Pages =
        [
            new FormPage
            {
                Title = "Player",
                Sections =
                [
                    new FormSection
                    {
                        Heading = "Identity",
                        Fields =
                        [
                            new TextField
                            {
                                Key = "player.name",
                                Label = "Name",
                                Description = "The farmer's name.",
                                IsRequired = true,
                                MaxLength = 50
                            },
                            new NumericField
                            {
                                Key = "player.level",
                                Label = "Level",
                                Min = 1,
                                Max = 100,
                                DecimalPlaces = 0
                            },
                            new NumericField
                            {
                                Key = "player.money",
                                Label = "Money",
                                Min = 0,
                                DecimalPlaces = 0
                            }
                        ]
                    },
                    new FormSection
                    {
                        Heading = "Stats",
                        Fields =
                        [
                            new NumericField
                            {
                                Key = "player.stats.health",
                                Label = "Health",
                                Min = 0,
                                Max = 100,
                                DecimalPlaces = 0
                            },
                            new NumericField
                            {
                                Key = "player.stats.energy",
                                Label = "Energy",
                                Min = 0,
                                Max = 100,
                                DecimalPlaces = 0
                            }
                        ]
                    }
                ]
            },
            new FormPage
            {
                Title = "Inventory",
                Sections =
                [
                    new FormSection
                    {
                        Fields =
                        [
                            new ListField
                            {
                                Key = "inventory",
                                Label = "Items",
                                ItemSchema = new ObjectSectionField
                                {
                                    Key = "",
                                    Label = "Item",
                                    Fields =
                                    [
                                        new TextField { Key = "id", Label = "Item ID", IsRequired = true },
                                        new NumericField { Key = "count", Label = "Count", Min = 0, DecimalPlaces = 0 }
                                    ]
                                }
                            }
                        ]
                    }
                ]
            }
        ]
    };
}
```

- [ ] **Step 5: Create StardewSaveDocument.cs**

```csharp
// src/SaveForge.SamplePlugin.StardewLike/StardewSaveDocument.cs
using System.Text.Json;
using System.Text.Json.Nodes;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.SamplePlugin.StardewLike;

public sealed class StardewSaveDocument : ISaveDocument
{
    private JsonNode _root;
    private bool _isDirty;

    private static readonly JsonSerializerOptions PrettyJson = new() { WriteIndented = true };

    public StardewSaveDocument(string filePath, JsonNode root)
    {
        FilePath = filePath;
        _root = root;
    }

    public string FilePath { get; }
    public bool IsDirty => _isDirty;

    public object? GetValue(string dotPath)
    {
        var node = Navigate(dotPath);
        return node switch
        {
            JsonValue v when v.TryGetValue<string>(out var s)  => s,
            JsonValue v when v.TryGetValue<int>(out var i)     => i,
            JsonValue v when v.TryGetValue<long>(out var l)    => l,
            JsonValue v when v.TryGetValue<double>(out var d)  => d,
            JsonValue v when v.TryGetValue<bool>(out var b)    => b,
            JsonArray arr                                       => arr,
            _ => null
        };
    }

    public void SetValue(string dotPath, object? value)
    {
        var parts = dotPath.Split('.', StringSplitOptions.RemoveEmptyEntries);
        if (parts.Length == 0) return;

        JsonNode? current = _root;
        for (int i = 0; i < parts.Length - 1; i++)
        {
            if (current is JsonObject obj)
                current = obj[parts[i]];
            else
                return;
        }

        if (current is JsonObject parent)
        {
            parent[parts[^1]] = value switch
            {
                null     => null,
                string s => JsonValue.Create(s),
                int n    => JsonValue.Create(n),
                long l   => JsonValue.Create(l),
                double d => JsonValue.Create(d),
                bool b   => JsonValue.Create(b),
                _        => JsonValue.Create(value.ToString())
            };
            _isDirty = true;
        }
    }

    public IEditorDefinition GetEditorDefinition() =>
        new SchemaEditorDefinition(StardewSchema.Build());

    public async Task SaveAsync(CancellationToken ct = default)
    {
        var json = _root.ToJsonString(PrettyJson);
        await File.WriteAllTextAsync(FilePath, json, ct);
        _isDirty = false;
    }

    private JsonNode? Navigate(string dotPath)
    {
        var parts = dotPath.Split('.', StringSplitOptions.RemoveEmptyEntries);
        JsonNode? current = _root;
        foreach (var part in parts)
        {
            if (current is JsonObject obj)
                current = obj[part];
            else
                return null;
        }
        return current;
    }
}

internal sealed class SchemaEditorDefinition : IEditorDefinition
{
    public SchemaEditorDefinition(FormSchema schema) => Schema = schema;
    public EditorKind Kind => EditorKind.Schema;
    public FormSchema? Schema { get; }
    public ICustomEditorViewFactory? ViewFactory => null;
}
```

- [ ] **Step 6: Create StardewPlugin.cs**

```csharp
// src/SaveForge.SamplePlugin.StardewLike/StardewPlugin.cs
using System.Text;
using System.Text.Json.Nodes;
using SaveForge.SDK;

namespace SaveForge.SamplePlugin.StardewLike;

public sealed class StardewPlugin : IGameEditorPlugin
{
    public PluginManifest Manifest { get; } = new()
    {
        Id = "stardew-like",
        Name = "StardewLike Save Editor",
        Version = "1.0.0",
        Author = "SaveForge Team",
        Description = "Edit saves for the StardewLike sample game.",
        MinHostVersion = "1.0.0",
        PluginAssembly = "SaveForge.SamplePlugin.StardewLike.dll"
    };

    public SaveCapabilities Capabilities { get; } = new()
    {
        FileExtensions = [".json"],
        DetectionPriority = 5
    };

    public (bool CanOpen, int Priority) CanOpen(FileInfo file, ReadOnlySpan<byte> header)
    {
        if (!file.Extension.Equals(".json", StringComparison.OrdinalIgnoreCase))
            return (false, 0);

        // Look for "player" key in the first 256 bytes
        var text = Encoding.UTF8.GetString(header[..Math.Min(256, header.Length)]);
        return text.Contains("\"player\"", StringComparison.Ordinal)
            ? (true, Capabilities.DetectionPriority)
            : (false, 0);
    }

    public async Task<ISaveDocument> OpenAsync(string filePath, CancellationToken ct = default)
    {
        var json = await File.ReadAllTextAsync(filePath, ct);
        var root = JsonNode.Parse(json)
            ?? throw new InvalidOperationException("Failed to parse save file as JSON.");
        return new StardewSaveDocument(filePath, root);
    }
}
```

- [ ] **Step 7: Run StardewLike tests — expect pass**

```bash
dotnet test tests/SaveForge.Tests/ --filter "StardewPluginTests" -v minimal
```

Expected: 6/6 PASS.

- [ ] **Step 8: Commit**

```bash
git add src/SaveForge.SamplePlugin.StardewLike/ tests/SaveForge.Tests/StardewPluginTests.cs
git commit -m "feat(sample-plugin): implement StardewLike save editor with tests"
```

---

## Task 11: Forms — Field ViewModels

**Files:**
- Create: `src/SaveForge.Forms/ViewModels/FieldViewModel.cs`
- Create: `src/SaveForge.Forms/ViewModels/TextFieldViewModel.cs`
- Create: `src/SaveForge.Forms/ViewModels/NumericFieldViewModel.cs`
- Create: `src/SaveForge.Forms/ViewModels/CheckboxFieldViewModel.cs`
- Create: `src/SaveForge.Forms/ViewModels/EnumFieldViewModel.cs`
- Create: `src/SaveForge.Forms/ViewModels/ReadOnlyFieldViewModel.cs`
- Create: `src/SaveForge.Forms/ViewModels/ListFieldViewModel.cs`
- Create: `src/SaveForge.Forms/ViewModels/ObjectSectionViewModel.cs`

- [ ] **Step 1: Create FieldViewModel.cs (abstract base)**

```csharp
// src/SaveForge.Forms/ViewModels/FieldViewModel.cs
using CommunityToolkit.Mvvm.ComponentModel;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public abstract partial class FieldViewModel : ObservableObject
{
    protected readonly ISaveDocument Document;

    [ObservableProperty]
    private bool _isVisible = true;

    [ObservableProperty]
    private string? _validationError;

    public FieldDefinition Definition { get; }

    protected FieldViewModel(FieldDefinition definition, ISaveDocument document)
    {
        Definition = definition;
        Document = document;
    }

    /// <summary>Re-loads the value from the document (called on initial render and after external changes).</summary>
    public abstract void RefreshFromDocument();

    /// <summary>Evaluates the visibility rule against the current document state.</summary>
    public void EvaluateVisibility(Func<string, object?> valueProvider)
    {
        if (Definition.VisibilityRule is null)
        {
            IsVisible = true;
            return;
        }
        var depValue = valueProvider(Definition.VisibilityRule.DependsOnKey);
        IsVisible = Definition.VisibilityRule.Evaluate(depValue);
    }

    protected abstract void Validate();
}
```

- [ ] **Step 2: Create TextFieldViewModel.cs**

```csharp
// src/SaveForge.Forms/ViewModels/TextFieldViewModel.cs
using System.Text.RegularExpressions;
using CommunityToolkit.Mvvm.ComponentModel;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed partial class TextFieldViewModel : FieldViewModel
{
    private readonly TextField _def;

    [ObservableProperty]
    private string _value = string.Empty;

    public TextFieldViewModel(TextField definition, ISaveDocument document) : base(definition, document)
    {
        _def = definition;
        RefreshFromDocument();
    }

    partial void OnValueChanged(string value)
    {
        Document.SetValue(_def.Key, value);
        Validate();
    }

    public override void RefreshFromDocument()
    {
        _value = Document.GetValue(_def.Key)?.ToString() ?? string.Empty;
        OnPropertyChanged(nameof(Value));
    }

    protected override void Validate()
    {
        if (_def.IsRequired && string.IsNullOrWhiteSpace(Value))
        {
            ValidationError = $"{_def.Label} is required.";
            return;
        }
        if (_def.MaxLength.HasValue && Value.Length > _def.MaxLength)
        {
            ValidationError = $"{_def.Label} must not exceed {_def.MaxLength} characters.";
            return;
        }
        if (_def.RegexPattern is not null && !string.IsNullOrEmpty(Value) &&
            !Regex.IsMatch(Value, _def.RegexPattern))
        {
            ValidationError = $"{_def.Label} has an invalid format.";
            return;
        }
        ValidationError = null;
    }
}
```

- [ ] **Step 3: Create NumericFieldViewModel.cs**

```csharp
// src/SaveForge.Forms/ViewModels/NumericFieldViewModel.cs
using CommunityToolkit.Mvvm.ComponentModel;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed partial class NumericFieldViewModel : FieldViewModel
{
    private readonly NumericField _def;

    [ObservableProperty]
    private double _value;

    public NumericFieldViewModel(NumericField definition, ISaveDocument document) : base(definition, document)
    {
        _def = definition;
        RefreshFromDocument();
    }

    partial void OnValueChanged(double value)
    {
        var stored = _def.DecimalPlaces == 0 ? (object)(int)value : value;
        Document.SetValue(_def.Key, stored);
        Validate();
    }

    public override void RefreshFromDocument()
    {
        var raw = Document.GetValue(_def.Key);
        _value = raw is null ? 0 : Convert.ToDouble(raw);
        OnPropertyChanged(nameof(Value));
    }

    protected override void Validate()
    {
        if (_def.Min.HasValue && Value < _def.Min.Value)
        {
            ValidationError = $"{_def.Label} must be at least {_def.Min}.";
            return;
        }
        if (_def.Max.HasValue && Value > _def.Max.Value)
        {
            ValidationError = $"{_def.Label} must be at most {_def.Max}.";
            return;
        }
        ValidationError = null;
    }
}
```

- [ ] **Step 4: Create CheckboxFieldViewModel.cs**

```csharp
// src/SaveForge.Forms/ViewModels/CheckboxFieldViewModel.cs
using CommunityToolkit.Mvvm.ComponentModel;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed partial class CheckboxFieldViewModel : FieldViewModel
{
    private readonly CheckboxField _def;

    [ObservableProperty]
    private bool _value;

    public CheckboxFieldViewModel(CheckboxField definition, ISaveDocument document) : base(definition, document)
    {
        _def = definition;
        RefreshFromDocument();
    }

    partial void OnValueChanged(bool value)
    {
        Document.SetValue(_def.Key, value);
        Validate();
    }

    public override void RefreshFromDocument()
    {
        var raw = Document.GetValue(_def.Key);
        _value = raw is bool b ? b : false;
        OnPropertyChanged(nameof(Value));
    }

    protected override void Validate() => ValidationError = null;
}
```

- [ ] **Step 5: Create EnumFieldViewModel.cs**

```csharp
// src/SaveForge.Forms/ViewModels/EnumFieldViewModel.cs
using CommunityToolkit.Mvvm.ComponentModel;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed partial class EnumFieldViewModel : FieldViewModel
{
    private readonly EnumField _def;

    public IReadOnlyList<EnumOption> Options { get; }

    [ObservableProperty]
    private EnumOption? _selectedOption;

    public EnumFieldViewModel(EnumField definition, ISaveDocument document) : base(definition, document)
    {
        _def = definition;
        Options = definition.Options;
        RefreshFromDocument();
    }

    partial void OnSelectedOptionChanged(EnumOption? option)
    {
        if (option is not null)
            Document.SetValue(_def.Key, option.Value);
        Validate();
    }

    public override void RefreshFromDocument()
    {
        var raw = Document.GetValue(_def.Key)?.ToString();
        _selectedOption = Options.FirstOrDefault(o => o.Value == raw) ?? Options.FirstOrDefault();
        OnPropertyChanged(nameof(SelectedOption));
    }

    protected override void Validate()
    {
        ValidationError = _def.IsRequired && SelectedOption is null
            ? $"{_def.Label} is required."
            : null;
    }
}
```

- [ ] **Step 6: Create ReadOnlyFieldViewModel.cs**

```csharp
// src/SaveForge.Forms/ViewModels/ReadOnlyFieldViewModel.cs
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed class ReadOnlyFieldViewModel : FieldViewModel
{
    public string DisplayValue { get; private set; } = string.Empty;

    public ReadOnlyFieldViewModel(ReadOnlyField definition, ISaveDocument document) : base(definition, document)
    {
        RefreshFromDocument();
    }

    public override void RefreshFromDocument()
    {
        DisplayValue = Document.GetValue(Definition.Key)?.ToString() ?? string.Empty;
        OnPropertyChanged(nameof(DisplayValue));
    }

    protected override void Validate() => ValidationError = null;
}
```

- [ ] **Step 7: Create ListFieldViewModel.cs**

```csharp
// src/SaveForge.Forms/ViewModels/ListFieldViewModel.cs
using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed partial class ListFieldViewModel : FieldViewModel
{
    private readonly ListField _def;

    public ObservableCollection<FieldViewModel> Items { get; } = [];

    public ListFieldViewModel(ListField definition, ISaveDocument document) : base(definition, document)
    {
        _def = definition;
        RefreshFromDocument();
    }

    public override void RefreshFromDocument()
    {
        // List items are read from the document lazily; complex binding handled by renderer
        Validate();
    }

    [RelayCommand]
    private void AddItem()
    {
        // Creates an empty item VM — full implementation depends on item schema type
        var itemVm = FieldViewModelFactory.Create(_def.ItemSchema, Document);
        Items.Add(itemVm);
    }

    [RelayCommand]
    private void RemoveItem(FieldViewModel item)
    {
        Items.Remove(item);
    }

    protected override void Validate() => ValidationError = null;
}
```

- [ ] **Step 8: Create ObjectSectionViewModel.cs**

```csharp
// src/SaveForge.Forms/ViewModels/ObjectSectionViewModel.cs
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed class ObjectSectionViewModel : FieldViewModel
{
    public IReadOnlyList<FieldViewModel> Fields { get; }

    public ObjectSectionViewModel(ObjectSectionField definition, ISaveDocument document) : base(definition, document)
    {
        Fields = definition.Fields.Select(f => FieldViewModelFactory.Create(f, document)).ToList();
        RefreshFromDocument();
    }

    public override void RefreshFromDocument()
    {
        foreach (var field in Fields)
            field.RefreshFromDocument();
    }

    protected override void Validate() => ValidationError = null;
}
```

- [ ] **Step 9: Create FieldViewModelFactory.cs**

```csharp
// src/SaveForge.Forms/ViewModels/FieldViewModelFactory.cs
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public static class FieldViewModelFactory
{
    public static FieldViewModel Create(FieldDefinition definition, ISaveDocument document) =>
        definition switch
        {
            TextField tf           => new TextFieldViewModel(tf, document),
            NumericField nf        => new NumericFieldViewModel(nf, document),
            CheckboxField cf       => new CheckboxFieldViewModel(cf, document),
            EnumField ef           => new EnumFieldViewModel(ef, document),
            ReadOnlyField rf       => new ReadOnlyFieldViewModel(rf, document),
            ListField lf           => new ListFieldViewModel(lf, document),
            ObjectSectionField osf => new ObjectSectionViewModel(osf, document),
            _ => throw new NotSupportedException($"Unsupported field type: {definition.GetType().Name}")
        };
}
```

- [ ] **Step 10: Build Forms**

```bash
dotnet build src/SaveForge.Forms/SaveForge.Forms.csproj
```

Expected: Build succeeded.

- [ ] **Step 11: Commit**

```bash
git add src/SaveForge.Forms/ViewModels/
git commit -m "feat(forms): add field view models for all schema field types"
```

---

## Task 12: Forms — Renderer ViewModel and View

**Files:**
- Create: `src/SaveForge.Forms/ViewModels/FormSectionViewModel.cs`
- Create: `src/SaveForge.Forms/ViewModels/FormPageViewModel.cs`
- Create: `src/SaveForge.Forms/ViewModels/FormRendererViewModel.cs`
- Create: `src/SaveForge.Forms/Views/FormRendererView.axaml`
- Create: `src/SaveForge.Forms/Views/FormRendererView.axaml.cs`

- [ ] **Step 1: Create FormSectionViewModel.cs**

```csharp
// src/SaveForge.Forms/ViewModels/FormSectionViewModel.cs
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed class FormSectionViewModel
{
    public string? Heading { get; }
    public IReadOnlyList<FieldViewModel> Fields { get; }

    public FormSectionViewModel(FormSection section, ISaveDocument document)
    {
        Heading = section.Heading;
        Fields = section.Fields.Select(f => FieldViewModelFactory.Create(f, document)).ToList();
    }
}
```

- [ ] **Step 2: Create FormPageViewModel.cs**

```csharp
// src/SaveForge.Forms/ViewModels/FormPageViewModel.cs
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed class FormPageViewModel
{
    public string Title { get; }
    public IReadOnlyList<FormSectionViewModel> Sections { get; }

    public FormPageViewModel(FormPage page, ISaveDocument document)
    {
        Title = page.Title;
        Sections = page.Sections.Select(s => new FormSectionViewModel(s, document)).ToList();
    }
}
```

- [ ] **Step 3: Create FormRendererViewModel.cs**

```csharp
// src/SaveForge.Forms/ViewModels/FormRendererViewModel.cs
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Forms.ViewModels;

public sealed partial class FormRendererViewModel : ObservableObject
{
    private readonly ISaveDocument _document;

    public IReadOnlyList<FormPageViewModel> Pages { get; }

    [ObservableProperty]
    private FormPageViewModel? _selectedPage;

    public bool HasValidationErrors => AllFields().Any(f => f.ValidationError is not null);

    public bool CanSave => _document.IsDirty && !HasValidationErrors;

    public FormRendererViewModel(FormSchema schema, ISaveDocument document)
    {
        _document = document;
        Pages = schema.Pages.Select(p => new FormPageViewModel(p, document)).ToList();
        SelectedPage = Pages.FirstOrDefault();

        // Re-evaluate CanSave whenever any field changes
        foreach (var field in AllFields())
            field.PropertyChanged += (_, _) => OnPropertyChanged(nameof(CanSave));
    }

    [RelayCommand(CanExecute = nameof(CanSave))]
    private async Task Save(CancellationToken ct)
    {
        await _document.SaveAsync(ct);
        OnPropertyChanged(nameof(CanSave));
    }

    private IEnumerable<FieldViewModel> AllFields() =>
        Pages.SelectMany(p => p.Sections).SelectMany(s => s.Fields);
}
```

- [ ] **Step 4: Create FormRendererView.axaml**

```xml
<!-- src/SaveForge.Forms/Views/FormRendererView.axaml -->
<UserControl xmlns="https://github.com/avaloniaui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
             xmlns:vm="clr-namespace:SaveForge.Forms.ViewModels"
             xmlns:sdk="clr-namespace:SaveForge.SDK.Schema;assembly=SaveForge.SDK"
             x:Class="SaveForge.Forms.Views.FormRendererView"
             x:DataType="vm:FormRendererViewModel">

  <Grid RowDefinitions="Auto,*,Auto">

    <!-- Page tabs -->
    <TabControl Grid.Row="0"
                ItemsSource="{Binding Pages}"
                SelectedItem="{Binding SelectedPage}">
      <TabControl.ItemTemplate>
        <DataTemplate x:DataType="vm:FormPageViewModel">
          <TextBlock Text="{Binding Title}" />
        </DataTemplate>
      </TabControl.ItemTemplate>
      <TabControl.ContentTemplate>
        <DataTemplate x:DataType="vm:FormPageViewModel">
          <ScrollViewer>
            <ItemsControl ItemsSource="{Binding Sections}" Margin="8">
              <ItemsControl.ItemTemplate>
                <DataTemplate x:DataType="vm:FormSectionViewModel">
                  <StackPanel Margin="0,0,0,12">
                    <TextBlock Text="{Binding Heading}"
                               FontWeight="SemiBold"
                               IsVisible="{Binding Heading, Converter={x:Static StringConverters.IsNotNullOrEmpty}}"
                               Margin="0,0,0,4" />
                    <ItemsControl ItemsSource="{Binding Fields}">
                      <ItemsControl.DataTemplates>

                        <!-- TextField -->
                        <DataTemplate DataType="{x:Type vm:TextFieldViewModel}">
                          <StackPanel IsVisible="{Binding IsVisible}" Margin="0,4">
                            <TextBlock Text="{Binding Definition.Label}" />
                            <TextBox Text="{Binding Value}"
                                     IsReadOnly="{Binding Definition.IsReadOnly}"
                                     Watermark="{Binding Definition.Description}" />
                            <TextBlock Text="{Binding ValidationError}"
                                       Foreground="#ff4757"
                                       IsVisible="{Binding ValidationError, Converter={x:Static StringConverters.IsNotNullOrEmpty}}" />
                          </StackPanel>
                        </DataTemplate>

                        <!-- NumericField -->
                        <DataTemplate DataType="{x:Type vm:NumericFieldViewModel}">
                          <StackPanel IsVisible="{Binding IsVisible}" Margin="0,4">
                            <TextBlock Text="{Binding Definition.Label}" />
                            <NumericUpDown Value="{Binding Value}"
                                           IsReadOnly="{Binding Definition.IsReadOnly}"
                                           FormatString="{Binding Definition.DecimalPlaces, StringFormat='F{0}'}" />
                            <TextBlock Text="{Binding ValidationError}"
                                       Foreground="#ff4757"
                                       IsVisible="{Binding ValidationError, Converter={x:Static StringConverters.IsNotNullOrEmpty}}" />
                          </StackPanel>
                        </DataTemplate>

                        <!-- CheckboxField -->
                        <DataTemplate DataType="{x:Type vm:CheckboxFieldViewModel}">
                          <StackPanel IsVisible="{Binding IsVisible}" Margin="0,4" Orientation="Horizontal">
                            <CheckBox IsChecked="{Binding Value}"
                                      IsEnabled="{Binding !Definition.IsReadOnly}"
                                      Content="{Binding Definition.Label}" />
                          </StackPanel>
                        </DataTemplate>

                        <!-- EnumField -->
                        <DataTemplate DataType="{x:Type vm:EnumFieldViewModel}">
                          <StackPanel IsVisible="{Binding IsVisible}" Margin="0,4">
                            <TextBlock Text="{Binding Definition.Label}" />
                            <ComboBox ItemsSource="{Binding Options}"
                                      SelectedItem="{Binding SelectedOption}"
                                      IsEnabled="{Binding !Definition.IsReadOnly}">
                              <ComboBox.ItemTemplate>
                                <DataTemplate x:DataType="sdk:EnumOption">
                                  <TextBlock Text="{Binding Display}" />
                                </DataTemplate>
                              </ComboBox.ItemTemplate>
                            </ComboBox>
                          </StackPanel>
                        </DataTemplate>

                        <!-- ReadOnlyField -->
                        <DataTemplate DataType="{x:Type vm:ReadOnlyFieldViewModel}">
                          <StackPanel IsVisible="{Binding IsVisible}" Margin="0,4">
                            <TextBlock Text="{Binding Definition.Label}" />
                            <TextBlock Text="{Binding DisplayValue}" Opacity="0.7" />
                          </StackPanel>
                        </DataTemplate>

                      </ItemsControl.DataTemplates>
                    </ItemsControl>
                  </StackPanel>
                </DataTemplate>
              </ItemsControl.ItemTemplate>
            </ItemsControl>
          </ScrollViewer>
        </DataTemplate>
      </TabControl.ContentTemplate>
    </TabControl>

    <!-- Save button -->
    <Button Grid.Row="2"
            Content="Save"
            Command="{Binding SaveCommand}"
            HorizontalAlignment="Right"
            Margin="8"
            Classes="accent" />

  </Grid>
</UserControl>
```

- [ ] **Step 5: Create FormRendererView.axaml.cs**

```csharp
// src/SaveForge.Forms/Views/FormRendererView.axaml.cs
using Avalonia.Controls;
using Avalonia.Markup.Xaml;

namespace SaveForge.Forms.Views;

public partial class FormRendererView : UserControl
{
    public FormRendererView()
    {
        InitializeComponent();
    }
}
```

- [ ] **Step 6: Build Forms project**

```bash
dotnet build src/SaveForge.Forms/SaveForge.Forms.csproj
```

Expected: Build succeeded.

- [ ] **Step 7: Commit**

```bash
git add src/SaveForge.Forms/
git commit -m "feat(forms): add form renderer view model and Avalonia view"
```

---

## Task 13: App — Dark Theme and Custom Chrome

**Files:**
- Create: `src/SaveForge.App/Themes/Colors.axaml`
- Create: `src/SaveForge.App/Themes/DarkTheme.axaml`
- Create: `src/SaveForge.App/Controls/TitleBarControl.axaml`
- Create: `src/SaveForge.App/Controls/TitleBarControl.axaml.cs`
- Create: `src/SaveForge.App/Controls/StatusBarControl.axaml`
- Create: `src/SaveForge.App/Controls/StatusBarControl.axaml.cs`
- Create: `src/SaveForge.App/Controls/InfoBarControl.axaml`
- Create: `src/SaveForge.App/Controls/InfoBarControl.axaml.cs`

- [ ] **Step 1: Create Colors.axaml**

```xml
<!-- src/SaveForge.App/Themes/Colors.axaml -->
<ResourceDictionary xmlns="https://github.com/avaloniaui"
                    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">

  <Color x:Key="ColorBackground">#1a1a2e</Color>
  <Color x:Key="ColorSurface">#16213e</Color>
  <Color x:Key="ColorSurfaceElevated">#0f3460</Color>
  <Color x:Key="ColorAccent">#e94560</Color>
  <Color x:Key="ColorTextPrimary">#eaeaea</Color>
  <Color x:Key="ColorTextSecondary">#8892a4</Color>
  <Color x:Key="ColorBorder">#2a2a4a</Color>
  <Color x:Key="ColorDanger">#ff4757</Color>
  <Color x:Key="ColorSuccess">#2ed573</Color>

  <SolidColorBrush x:Key="BrushBackground" Color="{StaticResource ColorBackground}" />
  <SolidColorBrush x:Key="BrushSurface" Color="{StaticResource ColorSurface}" />
  <SolidColorBrush x:Key="BrushSurfaceElevated" Color="{StaticResource ColorSurfaceElevated}" />
  <SolidColorBrush x:Key="BrushAccent" Color="{StaticResource ColorAccent}" />
  <SolidColorBrush x:Key="BrushTextPrimary" Color="{StaticResource ColorTextPrimary}" />
  <SolidColorBrush x:Key="BrushTextSecondary" Color="{StaticResource ColorTextSecondary}" />
  <SolidColorBrush x:Key="BrushBorder" Color="{StaticResource ColorBorder}" />
  <SolidColorBrush x:Key="BrushDanger" Color="{StaticResource ColorDanger}" />
  <SolidColorBrush x:Key="BrushSuccess" Color="{StaticResource ColorSuccess}" />

</ResourceDictionary>
```

- [ ] **Step 2: Create DarkTheme.axaml**

```xml
<!-- src/SaveForge.App/Themes/DarkTheme.axaml -->
<ResourceDictionary xmlns="https://github.com/avaloniaui"
                    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">

  <ResourceDictionary.MergedDictionaries>
    <ResourceInclude Source="/Themes/Colors.axaml" />
  </ResourceDictionary.MergedDictionaries>

  <!-- Window background -->
  <Style Selector="Window">
    <Setter Property="Background" Value="{StaticResource BrushBackground}" />
    <Setter Property="Foreground" Value="{StaticResource BrushTextPrimary}" />
  </Style>

  <!-- Panels -->
  <Style Selector="Panel, StackPanel, DockPanel, Grid">
    <Setter Property="Background" Value="Transparent" />
  </Style>

  <!-- Text -->
  <Style Selector="TextBlock">
    <Setter Property="Foreground" Value="{StaticResource BrushTextPrimary}" />
  </Style>

  <!-- TextBox -->
  <Style Selector="TextBox">
    <Setter Property="Background" Value="{StaticResource BrushSurface}" />
    <Setter Property="Foreground" Value="{StaticResource BrushTextPrimary}" />
    <Setter Property="BorderBrush" Value="{StaticResource BrushBorder}" />
    <Setter Property="CaretBrush" Value="{StaticResource BrushAccent}" />
  </Style>

  <!-- Button default -->
  <Style Selector="Button">
    <Setter Property="Background" Value="{StaticResource BrushSurfaceElevated}" />
    <Setter Property="Foreground" Value="{StaticResource BrushTextPrimary}" />
    <Setter Property="BorderBrush" Value="{StaticResource BrushBorder}" />
    <Setter Property="Padding" Value="12,6" />
    <Setter Property="CornerRadius" Value="4" />
  </Style>
  <Style Selector="Button:pointerover /template/ ContentPresenter">
    <Setter Property="Background" Value="{StaticResource BrushSurfaceElevated}" />
    <Setter Property="Opacity" Value="0.85" />
  </Style>

  <!-- Button accent class -->
  <Style Selector="Button.accent">
    <Setter Property="Background" Value="{StaticResource BrushAccent}" />
    <Setter Property="Foreground" Value="White" />
  </Style>

  <!-- ComboBox -->
  <Style Selector="ComboBox">
    <Setter Property="Background" Value="{StaticResource BrushSurface}" />
    <Setter Property="Foreground" Value="{StaticResource BrushTextPrimary}" />
    <Setter Property="BorderBrush" Value="{StaticResource BrushBorder}" />
  </Style>

  <!-- NumericUpDown -->
  <Style Selector="NumericUpDown">
    <Setter Property="Background" Value="{StaticResource BrushSurface}" />
    <Setter Property="Foreground" Value="{StaticResource BrushTextPrimary}" />
    <Setter Property="BorderBrush" Value="{StaticResource BrushBorder}" />
  </Style>

  <!-- TabControl -->
  <Style Selector="TabControl">
    <Setter Property="Background" Value="Transparent" />
  </Style>

  <!-- ScrollViewer -->
  <Style Selector="ScrollViewer">
    <Setter Property="Background" Value="Transparent" />
  </Style>

  <!-- Sidebar nav item -->
  <Style Selector="ListBoxItem.nav-item">
    <Setter Property="Padding" Value="12,10" />
    <Setter Property="CornerRadius" Value="6" />
    <Setter Property="Foreground" Value="{StaticResource BrushTextSecondary}" />
  </Style>
  <Style Selector="ListBoxItem.nav-item:selected">
    <Setter Property="Background" Value="{StaticResource BrushSurfaceElevated}" />
    <Setter Property="Foreground" Value="{StaticResource BrushTextPrimary}" />
  </Style>

</ResourceDictionary>
```

- [ ] **Step 3: Create TitleBarControl.axaml**

```xml
<!-- src/SaveForge.App/Controls/TitleBarControl.axaml -->
<UserControl xmlns="https://github.com/avaloniaui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
             x:Class="SaveForge.App.Controls.TitleBarControl"
             Height="38"
             Background="{StaticResource BrushSurface}">

  <Grid ColumnDefinitions="Auto,*,Auto">

    <!-- App icon + title (drag region is set in code-behind) -->
    <StackPanel Grid.Column="0" Orientation="Horizontal" Spacing="8" Margin="12,0,0,0" VerticalAlignment="Center">
      <TextBlock Text="GSE" FontWeight="Bold" Foreground="{StaticResource BrushAccent}" />
      <TextBlock Text="SaveForge" Foreground="{StaticResource BrushTextSecondary}" FontSize="13" />
    </StackPanel>

    <!-- Drag region placeholder -->
    <Panel Grid.Column="1" x:Name="DragRegion" Background="Transparent" />

    <!-- Window controls -->
    <StackPanel Grid.Column="2" Orientation="Horizontal">
      <Button x:Name="MinimizeButton" Content="─" Width="46" Height="38"
              Background="Transparent" BorderThickness="0"
              Foreground="{StaticResource BrushTextSecondary}" />
      <Button x:Name="MaximizeButton" Content="□" Width="46" Height="38"
              Background="Transparent" BorderThickness="0"
              Foreground="{StaticResource BrushTextSecondary}" />
      <Button x:Name="CloseButton" Content="✕" Width="46" Height="38"
              Background="Transparent" BorderThickness="0"
              Foreground="{StaticResource BrushTextSecondary}"
              Classes="close-btn" />
    </StackPanel>

  </Grid>
</UserControl>
```

- [ ] **Step 4: Create TitleBarControl.axaml.cs**

```csharp
// src/SaveForge.App/Controls/TitleBarControl.axaml.cs
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Input;
using Avalonia.Markup.Xaml;

namespace SaveForge.App.Controls;

public partial class TitleBarControl : UserControl
{
    public TitleBarControl()
    {
        InitializeComponent();
    }

    protected override void OnApplyTemplate(TemplateAppliedEventArgs e)
    {
        base.OnApplyTemplate(e);
        WireButtons();
    }

    protected override void OnAttachedToVisualTree(VisualTreeAttachmentEventArgs e)
    {
        base.OnAttachedToVisualTree(e);
        WireButtons();
    }

    private void WireButtons()
    {
        var window = TopLevel.GetTopLevel(this) as Window;
        if (window is null) return;

        if (this.FindControl<Button>("MinimizeButton") is { } min)
            min.Click += (_, _) => window.WindowState = WindowState.Minimized;

        if (this.FindControl<Button>("MaximizeButton") is { } max)
            max.Click += (_, _) =>
                window.WindowState = window.WindowState == WindowState.Maximized
                    ? WindowState.Normal
                    : WindowState.Maximized;

        if (this.FindControl<Button>("CloseButton") is { } close)
            close.Click += (_, _) => window.Close();

        if (this.FindControl<Panel>("DragRegion") is { } drag)
            drag.PointerPressed += (_, args) =>
            {
                if (args.GetCurrentPoint(drag).Properties.IsLeftButtonPressed)
                    window.BeginMoveDrag(args);
            };
    }
}
```

- [ ] **Step 5: Create StatusBarControl.axaml**

```xml
<!-- src/SaveForge.App/Controls/StatusBarControl.axaml -->
<UserControl xmlns="https://github.com/avaloniaui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
             x:Class="SaveForge.App.Controls.StatusBarControl"
             Height="28" Background="{StaticResource BrushSurface}"
             IsVisible="{Binding HasMessage}">
  <DockPanel LastChildFill="True" Margin="8,0">
    <TextBlock Text="{Binding Message}"
               VerticalAlignment="Center"
               Foreground="{StaticResource BrushTextSecondary}"
               FontSize="12" />
  </DockPanel>
</UserControl>
```

- [ ] **Step 6: Create StatusBarControl.axaml.cs**

```csharp
// src/SaveForge.App/Controls/StatusBarControl.axaml.cs
using Avalonia.Controls;
using Avalonia.Markup.Xaml;

namespace SaveForge.App.Controls;

public partial class StatusBarControl : UserControl
{
    public StatusBarControl()
    {
        InitializeComponent();
    }
}
```

- [ ] **Step 7: Create InfoBarControl.axaml**

```xml
<!-- src/SaveForge.App/Controls/InfoBarControl.axaml -->
<UserControl xmlns="https://github.com/avaloniaui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
             x:Class="SaveForge.App.Controls.InfoBarControl"
             IsVisible="{Binding IsVisible}"
             Background="{StaticResource BrushSurfaceElevated}">
  <DockPanel Margin="12,8" LastChildFill="True">
    <Button DockPanel.Dock="Right"
            Content="✕"
            Command="{Binding DismissCommand}"
            Background="Transparent"
            BorderThickness="0"
            Padding="4" />
    <TextBlock Text="{Binding Message}"
               VerticalAlignment="Center"
               Foreground="{StaticResource BrushTextPrimary}" />
  </DockPanel>
</UserControl>
```

- [ ] **Step 8: Create InfoBarControl.axaml.cs**

```csharp
// src/SaveForge.App/Controls/InfoBarControl.axaml.cs
using Avalonia.Controls;
using Avalonia.Markup.Xaml;

namespace SaveForge.App.Controls;

public partial class InfoBarControl : UserControl
{
    public InfoBarControl()
    {
        InitializeComponent();
    }
}
```

- [ ] **Step 9: Commit**

```bash
git add src/SaveForge.App/Themes/ src/SaveForge.App/Controls/
git commit -m "feat(app): add dark theme palette and custom window chrome controls"
```

---

## Task 14: App — View Models

**Files:**
- Create: `src/SaveForge.App/ViewModels/StatusBarViewModel.cs`
- Create: `src/SaveForge.App/ViewModels/InfoBarViewModel.cs`
- Create: `src/SaveForge.App/ViewModels/MainViewModel.cs`
- Create: `src/SaveForge.App/ViewModels/HomeViewModel.cs`
- Create: `src/SaveForge.App/ViewModels/StoreViewModel.cs`
- Create: `src/SaveForge.App/ViewModels/PluginDetailViewModel.cs`
- Create: `src/SaveForge.App/ViewModels/SettingsViewModel.cs`
- Create: `src/SaveForge.App/ViewModels/AboutViewModel.cs`
- Create: `src/SaveForge.App/ViewModels/EditorViewModel.cs`

- [ ] **Step 1: Create StatusBarViewModel.cs**

```csharp
// src/SaveForge.App/ViewModels/StatusBarViewModel.cs
using CommunityToolkit.Mvvm.ComponentModel;

namespace SaveForge.App.ViewModels;

public sealed partial class StatusBarViewModel : ObservableObject
{
    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(HasMessage))]
    private string _message = string.Empty;

    public bool HasMessage => !string.IsNullOrEmpty(Message);

    public void SetMessage(string message) => Message = message;
    public void Clear() => Message = string.Empty;
}
```

- [ ] **Step 2: Create InfoBarViewModel.cs**

```csharp
// src/SaveForge.App/ViewModels/InfoBarViewModel.cs
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace SaveForge.App.ViewModels;

public sealed partial class InfoBarViewModel : ObservableObject
{
    [ObservableProperty]
    private bool _isVisible;

    [ObservableProperty]
    private string _message = string.Empty;

    public void Show(string message)
    {
        Message = message;
        IsVisible = true;
    }

    [RelayCommand]
    private void Dismiss() => IsVisible = false;
}
```

- [ ] **Step 3: Create HomeViewModel.cs**

```csharp
// src/SaveForge.App/ViewModels/HomeViewModel.cs
using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;

namespace SaveForge.App.ViewModels;

public sealed partial class HomeViewModel : ObservableObject
{
    private readonly IPluginRegistry _registry;
    private readonly IFileDetectionService _detection;
    private readonly Action<string> _navigateToStore;
    private readonly Func<string, Task<EditorViewModel?>> _openEditor;

    public ObservableCollection<InstalledPlugin> InstalledPlugins { get; } = [];

    [ObservableProperty]
    private bool _isEmpty;

    public HomeViewModel(
        IPluginRegistry registry,
        IFileDetectionService detection,
        Action<string> navigateToStore,
        Func<string, Task<EditorViewModel?>> openEditor)
    {
        _registry = registry;
        _detection = detection;
        _navigateToStore = navigateToStore;
        _openEditor = openEditor;
        Refresh();
    }

    public void Refresh()
    {
        InstalledPlugins.Clear();
        foreach (var p in _registry.GetAll())
            InstalledPlugins.Add(p);
        IsEmpty = InstalledPlugins.Count == 0;
    }

    [RelayCommand]
    private void BrowseStore() => _navigateToStore("store");

    [RelayCommand]
    private async Task OpenFile()
    {
        // File dialog is opened by the view; this command is wired to dialog result
    }
}
```

- [ ] **Step 4: Create StoreViewModel.cs**

```csharp
// src/SaveForge.App/ViewModels/StoreViewModel.cs
using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;

namespace SaveForge.App.ViewModels;

public sealed partial class StoreViewModel : ObservableObject
{
    private readonly ICatalogService _catalog;
    private readonly IPluginInstaller _installer;
    private IReadOnlyList<CatalogEntry> _allEntries = Array.Empty<CatalogEntry>();

    public ObservableCollection<CatalogEntry> DisplayedEntries { get; } = [];

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(DisplayedEntries))]
    private string _searchText = string.Empty;

    [ObservableProperty]
    private bool _isLoading;

    [ObservableProperty]
    private string? _errorMessage;

    public StoreViewModel(ICatalogService catalog, IPluginInstaller installer)
    {
        _catalog = catalog;
        _installer = installer;
    }

    partial void OnSearchTextChanged(string value) => ApplyFilter();

    [RelayCommand]
    public async Task LoadAsync(CancellationToken ct)
    {
        IsLoading = true;
        ErrorMessage = null;
        try
        {
            _allEntries = await _catalog.GetCatalogAsync(ct: ct);
            ApplyFilter();
        }
        catch (Exception ex)
        {
            ErrorMessage = $"Failed to load catalog: {ex.Message}";
        }
        finally
        {
            IsLoading = false;
        }
    }

    [RelayCommand]
    public async Task InstallAsync(CatalogEntry entry, CancellationToken ct)
    {
        try
        {
            await _installer.InstallAsync(entry.Id, ct: ct);
            await LoadAsync(ct);
        }
        catch (Exception ex)
        {
            ErrorMessage = $"Install failed: {ex.Message}";
        }
    }

    private void ApplyFilter()
    {
        DisplayedEntries.Clear();
        var filtered = string.IsNullOrWhiteSpace(SearchText)
            ? _allEntries
            : _allEntries.Where(e =>
                e.Name.Contains(SearchText, StringComparison.OrdinalIgnoreCase) ||
                e.Tags.Any(t => t.Contains(SearchText, StringComparison.OrdinalIgnoreCase)));
        foreach (var e in filtered)
            DisplayedEntries.Add(e);
    }
}
```

- [ ] **Step 5: Create PluginDetailViewModel.cs**

```csharp
// src/SaveForge.App/ViewModels/PluginDetailViewModel.cs
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
        StatusMessage = "Installing…";
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
        StatusMessage = "Removing…";
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
        StatusMessage = "Updating…";
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
```

- [ ] **Step 6: Create SettingsViewModel.cs**

```csharp
// src/SaveForge.App/ViewModels/SettingsViewModel.cs
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.Infrastructure.Services;

namespace SaveForge.App.ViewModels;

public sealed partial class SettingsViewModel : ObservableObject
{
    private readonly SettingsService _settingsService;
    private readonly IUpdateChecker _updateChecker;
    private readonly StatusBarViewModel _statusBar;
    private readonly InfoBarViewModel _infoBar;

    [ObservableProperty] private string _catalogUrl = string.Empty;
    [ObservableProperty] private string _pluginDirectory = string.Empty;
    [ObservableProperty] private string _backupDirectory = string.Empty;

    public SettingsViewModel(
        AppSettings settings,
        SettingsService settingsService,
        IUpdateChecker updateChecker,
        StatusBarViewModel statusBar,
        InfoBarViewModel infoBar)
    {
        _settingsService = settingsService;
        _updateChecker = updateChecker;
        _statusBar = statusBar;
        _infoBar = infoBar;
        CatalogUrl = settings.CatalogUrl;
        PluginDirectory = settings.PluginDirectory;
        BackupDirectory = settings.BackupDirectory;
    }

    [RelayCommand]
    private async Task SaveSettingsAsync()
    {
        var settings = new AppSettings
        {
            CatalogUrl = CatalogUrl,
            PluginDirectory = PluginDirectory,
            BackupDirectory = BackupDirectory
        };
        await _settingsService.SaveAsync(settings);
        _statusBar.SetMessage("Settings saved.");
    }

    [RelayCommand]
    private async Task CheckUpdatesAsync(CancellationToken ct)
    {
        _statusBar.SetMessage("Checking for plugin updates…");
        try
        {
            var updates = await _updateChecker.CheckAsync(ct);
            if (updates.Count > 0)
                _infoBar.Show($"Updates available for {updates.Count} plugin(s) — Review in Store.");
            else
                _statusBar.SetMessage("All plugins are up to date.");
        }
        catch
        {
            _statusBar.SetMessage("Update check failed.");
        }
    }
}
```

- [ ] **Step 7: Create AboutViewModel.cs**

```csharp
// src/SaveForge.App/ViewModels/AboutViewModel.cs
using System.Reflection;
using CommunityToolkit.Mvvm.ComponentModel;

namespace SaveForge.App.ViewModels;

public sealed class AboutViewModel : ObservableObject
{
    public string AppVersion { get; } =
        Assembly.GetExecutingAssembly().GetName().Version?.ToString() ?? "0.0.0";

    public string HostVersion { get; } = "1.0.0";
}
```

- [ ] **Step 8: Create EditorViewModel.cs**

```csharp
// src/SaveForge.App/ViewModels/EditorViewModel.cs
using Avalonia.Controls;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.Forms.ViewModels;
using SaveForge.SDK;

namespace SaveForge.App.ViewModels;

public sealed partial class EditorViewModel : ObservableObject
{
    private readonly ISaveDocument _document;
    private readonly IBackupService _backup;
    private readonly StatusBarViewModel _statusBar;

    [ObservableProperty]
    private FormRendererViewModel? _formRenderer;

    [ObservableProperty]
    private Control? _customView;

    [ObservableProperty]
    private bool _isSchemaEditor;

    public string FileName => Path.GetFileName(_document.FilePath);

    public EditorViewModel(ISaveDocument document, IBackupService backup, StatusBarViewModel statusBar)
    {
        _document = document;
        _backup = backup;
        _statusBar = statusBar;

        var def = document.GetEditorDefinition();
        IsSchemaEditor = def.Kind == EditorKind.Schema;

        if (def.Kind == EditorKind.Schema && def.Schema is not null)
            FormRenderer = new FormRendererViewModel(def.Schema, document);
        else if (def.Kind == EditorKind.Custom && def.ViewFactory is not null)
            CustomView = def.ViewFactory.CreateView(document);
    }

    [RelayCommand]
    private async Task SaveAsync(CancellationToken ct)
    {
        try
        {
            // 1. Copy original before any write (backup the unchanged file)
            var timestamp = DateTimeOffset.UtcNow.ToString("yyyy-MM-ddTHH-mm-ss");
            var backupDir = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
                "SaveForge", "backups", "editor");
            Directory.CreateDirectory(backupDir);
            var backupPath = Path.Combine(backupDir,
                $"{Path.GetFileName(_document.FilePath)}.{timestamp}.bak");
            File.Copy(_document.FilePath, backupPath, overwrite: false);

            // 2. Let the document write its new content to disk
            await _document.SaveAsync(ct);

            _statusBar.SetMessage($"Saved. Backup: {backupPath}");
        }
        catch (Exception ex)
        {
            _statusBar.SetMessage($"Save failed: {ex.Message}");
        }
    }
}
```

- [ ] **Step 9: Create MainViewModel.cs**

```csharp
// src/SaveForge.App/ViewModels/MainViewModel.cs
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;

namespace SaveForge.App.ViewModels;

public sealed partial class MainViewModel : ObservableObject
{
    private readonly IUpdateChecker _updateChecker;
    private readonly StatusBarViewModel _statusBar;
    private readonly InfoBarViewModel _infoBar;

    public StatusBarViewModel StatusBar { get; }
    public InfoBarViewModel InfoBar { get; }

    [ObservableProperty]
    private ObservableObject? _currentPage;

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
            "home"    => (ObservableObject)Home,
            "store"   => Store,
            "settings"=> Settings,
            "about"   => About,
            _         => Home
        };
    }

    /// <summary>Called after the window is fully shown. Runs the update check in the background.</summary>
    public async Task RunPostLoadTasksAsync()
    {
        _statusBar.SetMessage("Checking for plugin updates…");
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
```

- [ ] **Step 10: Build App ViewModels**

```bash
dotnet build src/SaveForge.App/SaveForge.App.csproj 2>&1 | head -30
```

Expected: Build succeeded or only AXAML-related warnings (views not yet wired).

- [ ] **Step 11: Commit**

```bash
git add src/SaveForge.App/ViewModels/
git commit -m "feat(app): add all page view models with update check UX"
```

---

## Task 15: App — Views (AXAML)

**Files:**
- Create: `src/SaveForge.App/Views/MainWindow.axaml`
- Create: `src/SaveForge.App/Views/MainWindow.axaml.cs`
- Create: `src/SaveForge.App/Views/HomeView.axaml`
- Create: `src/SaveForge.App/Views/HomeView.axaml.cs`
- Create: `src/SaveForge.App/Views/StoreView.axaml`
- Create: `src/SaveForge.App/Views/StoreView.axaml.cs`
- Create: `src/SaveForge.App/Views/PluginDetailView.axaml`
- Create: `src/SaveForge.App/Views/PluginDetailView.axaml.cs`
- Create: `src/SaveForge.App/Views/SettingsView.axaml`
- Create: `src/SaveForge.App/Views/SettingsView.axaml.cs`
- Create: `src/SaveForge.App/Views/AboutView.axaml`
- Create: `src/SaveForge.App/Views/AboutView.axaml.cs`
- Create: `src/SaveForge.App/Views/EditorView.axaml`
- Create: `src/SaveForge.App/Views/EditorView.axaml.cs`

- [ ] **Step 1: Create MainWindow.axaml**

```xml
<!-- src/SaveForge.App/Views/MainWindow.axaml -->
<Window xmlns="https://github.com/avaloniaui"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        xmlns:ctrl="clr-namespace:SaveForge.App.Controls"
        xmlns:vm="clr-namespace:SaveForge.App.ViewModels"
        xmlns:views="clr-namespace:SaveForge.App.Views"
        x:Class="SaveForge.App.Views.MainWindow"
        x:DataType="vm:MainViewModel"
        Title="SaveForge"
        Width="1100" Height="720"
        MinWidth="800" MinHeight="500"
        ExtendClientAreaToDecorationsHint="True"
        ExtendClientAreaChromeHints="NoChrome"
        ExtendClientAreaTitleBarHeightHint="-1"
        WindowStartupLocation="CenterScreen"
        Background="{StaticResource BrushBackground}">

  <Window.Resources>
    <ResourceDictionary>
      <ResourceDictionary.MergedDictionaries>
        <ResourceInclude Source="/Themes/DarkTheme.axaml" />
      </ResourceDictionary.MergedDictionaries>
    </ResourceDictionary>
  </Window.Resources>

  <Grid RowDefinitions="38,*,Auto">

    <!-- Custom title bar -->
    <ctrl:TitleBarControl Grid.Row="0" />

    <!-- Main layout: sidebar + content -->
    <Grid Grid.Row="1" ColumnDefinitions="200,*">

      <!-- Sidebar -->
      <Border Grid.Column="0" Background="{StaticResource BrushSurface}"
              BorderBrush="{StaticResource BrushBorder}" BorderThickness="0,0,1,0">
        <StackPanel Margin="8,16">
          <ListBoxItem Classes="nav-item" Command="{Binding NavigateToCommand}" CommandParameter="home">
            <TextBlock Text="Home" />
          </ListBoxItem>
          <ListBoxItem Classes="nav-item" Command="{Binding NavigateToCommand}" CommandParameter="store">
            <TextBlock Text="Store" />
          </ListBoxItem>
          <ListBoxItem Classes="nav-item" Command="{Binding NavigateToCommand}" CommandParameter="settings">
            <TextBlock Text="Settings" />
          </ListBoxItem>
          <ListBoxItem Classes="nav-item" Command="{Binding NavigateToCommand}" CommandParameter="about">
            <TextBlock Text="About" />
          </ListBoxItem>
        </StackPanel>
      </Border>

      <!-- InfoBar + content -->
      <Grid Grid.Column="1" RowDefinitions="Auto,*">
        <ctrl:InfoBarControl Grid.Row="0" DataContext="{Binding InfoBar}" />
        <ContentControl Grid.Row="1" Content="{Binding CurrentPage}">
          <ContentControl.DataTemplates>
            <DataTemplate DataType="{x:Type vm:HomeViewModel}">
              <views:HomeView />
            </DataTemplate>
            <DataTemplate DataType="{x:Type vm:StoreViewModel}">
              <views:StoreView />
            </DataTemplate>
            <DataTemplate DataType="{x:Type vm:PluginDetailViewModel}">
              <views:PluginDetailView />
            </DataTemplate>
            <DataTemplate DataType="{x:Type vm:SettingsViewModel}">
              <views:SettingsView />
            </DataTemplate>
            <DataTemplate DataType="{x:Type vm:AboutViewModel}">
              <views:AboutView />
            </DataTemplate>
            <DataTemplate DataType="{x:Type vm:EditorViewModel}">
              <views:EditorView />
            </DataTemplate>
          </ContentControl.DataTemplates>
        </ContentControl>
      </Grid>

    </Grid>

    <!-- Status bar -->
    <ctrl:StatusBarControl Grid.Row="2" DataContext="{Binding StatusBar}" />

  </Grid>
</Window>
```

- [ ] **Step 2: Create MainWindow.axaml.cs**

```csharp
// src/SaveForge.App/Views/MainWindow.axaml.cs
using Avalonia;
using Avalonia.Controls;
using Avalonia.Markup.Xaml;
using SaveForge.App.ViewModels;

namespace SaveForge.App.Views;

public partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
    }

    protected override void OnOpened(EventArgs e)
    {
        base.OnOpened(e);
        if (DataContext is MainViewModel vm)
            _ = vm.RunPostLoadTasksAsync();
    }
}
```

- [ ] **Step 3: Create HomeView.axaml**

```xml
<!-- src/SaveForge.App/Views/HomeView.axaml -->
<UserControl xmlns="https://github.com/avaloniaui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
             xmlns:vm="clr-namespace:SaveForge.App.ViewModels"
             xmlns:core="clr-namespace:SaveForge.Core.Models;assembly=SaveForge.Core"
             x:Class="SaveForge.App.Views.HomeView"
             x:DataType="vm:HomeViewModel">
  <Grid>
    <!-- Empty state -->
    <StackPanel IsVisible="{Binding IsEmpty}"
                VerticalAlignment="Center" HorizontalAlignment="Center" Spacing="12">
      <TextBlock Text="No plugins installed." Foreground="{StaticResource BrushTextSecondary}"
                 FontSize="16" HorizontalAlignment="Center" />
      <Button Content="Browse Store" Command="{Binding BrowseStoreCommand}"
              Classes="accent" HorizontalAlignment="Center" />
    </StackPanel>

    <!-- Installed plugins grid -->
    <ScrollViewer IsVisible="{Binding !IsEmpty}">
      <WrapPanel Margin="16">
        <ItemsControl ItemsSource="{Binding InstalledPlugins}">
          <ItemsControl.ItemTemplate>
            <DataTemplate x:DataType="core:InstalledPlugin">
              <Border Background="{StaticResource BrushSurface}"
                      BorderBrush="{StaticResource BrushBorder}"
                      BorderThickness="1" CornerRadius="8"
                      Padding="16" Margin="8" Width="220">
                <StackPanel Spacing="8">
                  <TextBlock Text="{Binding Manifest.Name}" FontWeight="SemiBold" FontSize="15" />
                  <TextBlock Text="{Binding Manifest.Author}" Foreground="{StaticResource BrushTextSecondary}" />
                  <TextBlock Text="{Binding Manifest.Version}" Foreground="{StaticResource BrushTextSecondary}" FontSize="12" />
                  <TextBlock Text="{Binding FailureReason}" Foreground="{StaticResource BrushDanger}"
                             IsVisible="{Binding FailureReason, Converter={x:Static StringConverters.IsNotNullOrEmpty}}"
                             FontSize="12" />
                  <Button Content="Open File" IsEnabled="{Binding Plugin, Converter={x:Static ObjectConverters.IsNotNull}}"
                          HorizontalAlignment="Stretch" />
                </StackPanel>
              </Border>
            </DataTemplate>
          </ItemsControl.ItemTemplate>
        </ItemsControl>
      </WrapPanel>
    </ScrollViewer>
  </Grid>
</UserControl>
```

- [ ] **Step 4: Create HomeView.axaml.cs**

```csharp
// src/SaveForge.App/Views/HomeView.axaml.cs
using Avalonia.Controls;
using Avalonia.Markup.Xaml;

namespace SaveForge.App.Views;

public partial class HomeView : UserControl
{
    public HomeView()
    {
        InitializeComponent();
    }
}
```

- [ ] **Step 5: Create StoreView.axaml**

```xml
<!-- src/SaveForge.App/Views/StoreView.axaml -->
<UserControl xmlns="https://github.com/avaloniaui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
             xmlns:vm="clr-namespace:SaveForge.App.ViewModels"
             xmlns:core="clr-namespace:SaveForge.Core.Models;assembly=SaveForge.Core"
             x:Class="SaveForge.App.Views.StoreView"
             x:DataType="vm:StoreViewModel">
  <Grid RowDefinitions="Auto,Auto,*">
    <TextBlock Grid.Row="0" Text="Plugin Store" FontSize="20" FontWeight="Bold" Margin="16,16,16,8" />
    <TextBox Grid.Row="1" Text="{Binding SearchText}" Watermark="Search plugins…" Margin="16,0,16,8" />

    <TextBlock Grid.Row="2" Text="{Binding ErrorMessage}" Foreground="{StaticResource BrushDanger}"
               IsVisible="{Binding ErrorMessage, Converter={x:Static StringConverters.IsNotNullOrEmpty}}"
               Margin="16" />

    <ScrollViewer Grid.Row="2" IsVisible="{Binding ErrorMessage, Converter={x:Static StringConverters.IsNullOrEmpty}}">
      <ItemsControl ItemsSource="{Binding DisplayedEntries}" Margin="8">
        <ItemsControl.ItemTemplate>
          <DataTemplate x:DataType="core:CatalogEntry">
            <Border Background="{StaticResource BrushSurface}"
                    BorderBrush="{StaticResource BrushBorder}"
                    BorderThickness="1" CornerRadius="8"
                    Padding="16" Margin="8">
              <Grid ColumnDefinitions="*,Auto">
                <StackPanel Grid.Column="0" Spacing="4">
                  <TextBlock Text="{Binding Name}" FontWeight="SemiBold" FontSize="15" />
                  <TextBlock Text="{Binding Author}" Foreground="{StaticResource BrushTextSecondary}" />
                  <TextBlock Text="{Binding Description}" TextWrapping="Wrap" MaxWidth="400" />
                  <StackPanel Orientation="Horizontal" Spacing="8">
                    <Border Background="{StaticResource BrushAccent}" CornerRadius="4" Padding="6,2"
                            IsVisible="{Binding HasUpdate}">
                      <TextBlock Text="Update Available" FontSize="11" Foreground="White" />
                    </Border>
                    <Border Background="{StaticResource BrushSuccess}" CornerRadius="4" Padding="6,2"
                            IsVisible="{Binding IsInstalled}">
                      <TextBlock Text="Installed" FontSize="11" Foreground="White" />
                    </Border>
                  </StackPanel>
                </StackPanel>
                <Button Grid.Column="1" Content="Install"
                        IsVisible="{Binding !IsInstalled}"
                        Classes="accent"
                        VerticalAlignment="Center" />
              </Grid>
            </Border>
          </DataTemplate>
        </ItemsControl.ItemTemplate>
      </ItemsControl>
    </ScrollViewer>
  </Grid>
</UserControl>
```

- [ ] **Step 6: Create StoreView.axaml.cs**

```csharp
// src/SaveForge.App/Views/StoreView.axaml.cs
using Avalonia.Controls;
using Avalonia.Markup.Xaml;
using SaveForge.App.ViewModels;

namespace SaveForge.App.Views;

public partial class StoreView : UserControl
{
    public StoreView()
    {
        InitializeComponent();
    }

    protected override void OnDataContextChanged(EventArgs e)
    {
        base.OnDataContextChanged(e);
        if (DataContext is StoreViewModel vm)
            _ = vm.LoadAsync(CancellationToken.None);
    }
}
```

- [ ] **Step 7: Create PluginDetailView.axaml + code-behind**

```xml
<!-- src/SaveForge.App/Views/PluginDetailView.axaml -->
<UserControl xmlns="https://github.com/avaloniaui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
             xmlns:vm="clr-namespace:SaveForge.App.ViewModels"
             x:Class="SaveForge.App.Views.PluginDetailView"
             x:DataType="vm:PluginDetailViewModel">
  <ScrollViewer>
    <StackPanel Margin="24" Spacing="12">
      <TextBlock Text="{Binding Entry.Name}" FontSize="24" FontWeight="Bold" />
      <TextBlock Text="{Binding Entry.Author}" Foreground="{StaticResource BrushTextSecondary}" />
      <TextBlock Text="{Binding Entry.Description}" TextWrapping="Wrap" />
      <TextBlock Text="{Binding Entry.LatestVersion}" />
      <StackPanel Orientation="Horizontal" Spacing="8">
        <Button Content="Install" Command="{Binding InstallCommand}" Classes="accent"
                IsVisible="{Binding !Entry.IsInstalled}" />
        <Button Content="Update" Command="{Binding UpdateCommand}" Classes="accent"
                IsVisible="{Binding Entry.HasUpdate}" />
        <Button Content="Remove" Command="{Binding RemoveCommand}"
                IsVisible="{Binding Entry.IsInstalled}" />
      </StackPanel>
      <TextBlock Text="{Binding StatusMessage}"
                 IsVisible="{Binding StatusMessage, Converter={x:Static StringConverters.IsNotNullOrEmpty}}" />
    </StackPanel>
  </ScrollViewer>
</UserControl>
```

```csharp
// src/SaveForge.App/Views/PluginDetailView.axaml.cs
using Avalonia.Controls;
using Avalonia.Markup.Xaml;
namespace SaveForge.App.Views;
public partial class PluginDetailView : UserControl
{
    public PluginDetailView() => InitializeComponent();
}
```

- [ ] **Step 8: Create SettingsView.axaml + code-behind**

```xml
<!-- src/SaveForge.App/Views/SettingsView.axaml -->
<UserControl xmlns="https://github.com/avaloniaui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
             xmlns:vm="clr-namespace:SaveForge.App.ViewModels"
             x:Class="SaveForge.App.Views.SettingsView"
             x:DataType="vm:SettingsViewModel">
  <StackPanel Margin="24" Spacing="16" MaxWidth="600">
    <TextBlock Text="Settings" FontSize="20" FontWeight="Bold" />
    <StackPanel Spacing="4">
      <TextBlock Text="Catalog URL" />
      <TextBox Text="{Binding CatalogUrl}" />
    </StackPanel>
    <StackPanel Spacing="4">
      <TextBlock Text="Plugin Directory" />
      <TextBox Text="{Binding PluginDirectory}" />
    </StackPanel>
    <StackPanel Spacing="4">
      <TextBlock Text="Backup Directory" />
      <TextBox Text="{Binding BackupDirectory}" />
    </StackPanel>
    <StackPanel Orientation="Horizontal" Spacing="8">
      <Button Content="Save Settings" Command="{Binding SaveSettingsCommand}" Classes="accent" />
      <Button Content="Check for Updates" Command="{Binding CheckUpdatesCommand}" />
    </StackPanel>
  </StackPanel>
</UserControl>
```

```csharp
// src/SaveForge.App/Views/SettingsView.axaml.cs
using Avalonia.Controls;
using Avalonia.Markup.Xaml;
namespace SaveForge.App.Views;
public partial class SettingsView : UserControl
{
    public SettingsView() => InitializeComponent();
}
```

- [ ] **Step 9: Create AboutView.axaml + code-behind**

```xml
<!-- src/SaveForge.App/Views/AboutView.axaml -->
<UserControl xmlns="https://github.com/avaloniaui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
             xmlns:vm="clr-namespace:SaveForge.App.ViewModels"
             x:Class="SaveForge.App.Views.AboutView"
             x:DataType="vm:AboutViewModel">
  <StackPanel Margin="24" Spacing="12" VerticalAlignment="Center" HorizontalAlignment="Center">
    <TextBlock Text="SaveForge" FontSize="28" FontWeight="Bold" HorizontalAlignment="Center" />
    <TextBlock Text="{Binding AppVersion, StringFormat='Version {0}'}"
               Foreground="{StaticResource BrushTextSecondary}" HorizontalAlignment="Center" />
    <TextBlock Text="{Binding HostVersion, StringFormat='Host API {0}'}"
               Foreground="{StaticResource BrushTextSecondary}" HorizontalAlignment="Center" />
  </StackPanel>
</UserControl>
```

```csharp
// src/SaveForge.App/Views/AboutView.axaml.cs
using Avalonia.Controls;
using Avalonia.Markup.Xaml;
namespace SaveForge.App.Views;
public partial class AboutView : UserControl
{
    public AboutView() => InitializeComponent();
}
```

- [ ] **Step 10: Create EditorView.axaml + code-behind**

```xml
<!-- src/SaveForge.App/Views/EditorView.axaml -->
<UserControl xmlns="https://github.com/avaloniaui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
             xmlns:vm="clr-namespace:SaveForge.App.ViewModels"
             xmlns:forms="clr-namespace:SaveForge.Forms.Views;assembly=SaveForge.Forms"
             x:Class="SaveForge.App.Views.EditorView"
             x:DataType="vm:EditorViewModel">
  <Grid RowDefinitions="Auto,*">
    <TextBlock Grid.Row="0" Text="{Binding FileName}"
               FontSize="16" FontWeight="SemiBold" Margin="16,12" />
    <!-- Schema editor -->
    <forms:FormRendererView Grid.Row="1"
                             DataContext="{Binding FormRenderer}"
                             IsVisible="{Binding IsSchemaEditor}" />
    <!-- Custom plugin view -->
    <ContentControl Grid.Row="1"
                    Content="{Binding CustomView}"
                    IsVisible="{Binding !IsSchemaEditor}" />
  </Grid>
</UserControl>
```

```csharp
// src/SaveForge.App/Views/EditorView.axaml.cs
using Avalonia.Controls;
using Avalonia.Markup.Xaml;
namespace SaveForge.App.Views;
public partial class EditorView : UserControl
{
    public EditorView() => InitializeComponent();
}
```

- [ ] **Step 11: Commit**

```bash
git add src/SaveForge.App/Views/
git commit -m "feat(app): add all page views with dark theme styling"
```

---

## Task 16: App — DI and Startup

**Files:**
- Create: `src/SaveForge.App/Program.cs`
- Create: `src/SaveForge.App/App.axaml`
- Create: `src/SaveForge.App/App.axaml.cs`

- [ ] **Step 1: Create Program.cs**

```csharp
// src/SaveForge.App/Program.cs
using Avalonia;

namespace SaveForge.App;

class Program
{
    [STAThread]
    public static void Main(string[] args) =>
        BuildAvaloniaApp().StartWithClassicDesktopLifetime(args);

    public static AppBuilder BuildAvaloniaApp() =>
        AppBuilder.Configure<App>()
            .UsePlatformDetect()
            .WithInterFont()
            .LogToTrace();
}
```

- [ ] **Step 2: Create App.axaml**

```xml
<!-- src/SaveForge.App/App.axaml -->
<Application xmlns="https://github.com/avaloniaui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
             x:Class="SaveForge.App.App"
             RequestedThemeVariant="Dark">
  <Application.Styles>
    <FluentTheme />
  </Application.Styles>
</Application>
```

- [ ] **Step 3: Create App.axaml.cs**

```csharp
// src/SaveForge.App/App.axaml.cs
using Avalonia;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using SaveForge.App.ViewModels;
using SaveForge.App.Views;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.Infrastructure.Services;
using SaveForge.PluginHost;
using SaveForge.Store;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace SaveForge.App;

public partial class App : Application
{
    private ServiceProvider? _services;

    public override void Initialize() => AvaloniaXamlLoader.Load(this);

    public override void OnFrameworkInitializationCompleted()
    {
        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
        {
            _services = BuildServices();

            // Load plugins on startup
            var hostSvc = _services.GetRequiredService<PluginHostService>();
            var settings = _services.GetRequiredService<AppSettings>();
            hostSvc.LoadAll(settings.PluginDirectory);

            var mainVm = _services.GetRequiredService<MainViewModel>();
            var window = new MainWindow { DataContext = mainVm };
            desktop.MainWindow = window;
        }
        base.OnFrameworkInitializationCompleted();
    }

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

        // Security — no DI needed (static methods)

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
                sp.GetRequiredService<IFileDetectionService>(),
                _ => { },
                _ => Task.FromResult<EditorViewModel?>(null)));
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
}
```

- [ ] **Step 4: Build the full solution**

```bash
dotnet build SaveForge.sln
```

Expected: Build succeeded with 0 errors. Warnings about AXAML data context are acceptable.

- [ ] **Step 5: Run the app to confirm it starts**

```bash
dotnet run --project src/SaveForge.App/SaveForge.App.csproj
```

Expected: Window opens with dark theme, sidebar nav, custom title bar.

- [ ] **Step 6: Commit**

```bash
git add src/SaveForge.App/App.axaml src/SaveForge.App/App.axaml.cs src/SaveForge.App/Program.cs
git commit -m "feat(app): wire DI container and startup sequence"
```

---

## Task 17: Manifest Parsing Tests and Final Test Run

**Files:**
- Create: `tests/SaveForge.Tests/ManifestParsingTests.cs`

- [ ] **Step 1: Write manifest parsing tests**

```csharp
// tests/SaveForge.Tests/ManifestParsingTests.cs
using System.Text.Json;
using SaveForge.SDK;
using Xunit;

namespace SaveForge.Tests;

public class ManifestParsingTests
{
    private static readonly JsonSerializerOptions Options = new() { PropertyNameCaseInsensitive = true };

    [Fact]
    public void Deserialize_ValidJson_ReturnsManifest()
    {
        var json = """
            {
              "id": "stardew-like",
              "name": "StardewLike Save Editor",
              "version": "1.0.0",
              "author": "Team",
              "minHostVersion": "1.0.0",
              "pluginAssembly": "StardewLike.dll"
            }
            """;

        var manifest = JsonSerializer.Deserialize<PluginManifest>(json, Options)!;

        Assert.Equal("stardew-like", manifest.Id);
        Assert.Equal("1.0.0", manifest.Version);
        Assert.Null(manifest.MaxHostVersion);
    }

    [Fact]
    public void Deserialize_WithMaxVersion_ParsesCorrectly()
    {
        var json = """
            {
              "id": "x", "name": "X", "version": "2.0.0",
              "author": "A", "minHostVersion": "1.0.0",
              "maxHostVersion": "3.0.0",
              "pluginAssembly": "x.dll"
            }
            """;

        var manifest = JsonSerializer.Deserialize<PluginManifest>(json, Options)!;

        Assert.Equal("3.0.0", manifest.MaxHostVersion);
    }

    [Fact]
    public void Deserialize_MissingOptionalDescription_DefaultsToEmpty()
    {
        var json = """
            {
              "id": "x", "name": "X", "version": "1.0.0",
              "author": "A", "minHostVersion": "1.0.0",
              "pluginAssembly": "x.dll"
            }
            """;

        var manifest = JsonSerializer.Deserialize<PluginManifest>(json, Options)!;

        Assert.Equal(string.Empty, manifest.Description);
    }
}
```

- [ ] **Step 2: Run all tests**

```bash
dotnet test tests/SaveForge.Tests/ -v minimal
```

Expected: All tests pass. Green.

- [ ] **Step 3: Commit**

```bash
git add tests/SaveForge.Tests/ManifestParsingTests.cs
git commit -m "test: add manifest parsing tests and confirm all tests pass"
```

---

## Task 18: Documentation

**Files:**
- Create: `docs/ARCHITECTURE.md`
- Create: `docs/PLUGIN_AUTHORING.md`
- Create: `docs/CATALOG_SCHEMA.md`
- Create: `README.md`

- [ ] **Step 1: Create ARCHITECTURE.md**

```markdown
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
2. `IBackupService.BackupAndWriteAsync(path, pluginId, content)`
3. Copies original → `.bak`, writes new content to `.tmp`, renames `.tmp` → original
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
```

- [ ] **Step 2: Create PLUGIN_AUTHORING.md**

````markdown
# Plugin Authoring Guide

## Overview

A plugin is a .NET 8 class library that references only `SaveForge.SDK`. The minimal
authoring workflow is:

1. Define a typed save model
2. Define a `FormSchema`
3. Implement load/save
4. Ship a `manifest.json`

## Step 1: Create the project

```bash
dotnet new classlib -n MyGame.SaveEditor --framework net8.0
dotnet add package SaveForge.SDK
```

## Step 2: Implement IGameEditorPlugin

```csharp
public sealed class MyGamePlugin : IGameEditorPlugin
{
    public PluginManifest Manifest { get; } = new()
    {
        Id = "my-game",
        Name = "MyGame Save Editor",
        Version = "1.0.0",
        Author = "You",
        MinHostVersion = "1.0.0",
        PluginAssembly = "MyGame.SaveEditor.dll"
    };

    public SaveCapabilities Capabilities { get; } = new()
    {
        FileExtensions = [".sav"],
        DetectionPriority = 5
    };

    public (bool CanOpen, int Priority) CanOpen(FileInfo file, ReadOnlySpan<byte> header)
    {
        if (!file.Extension.Equals(".sav", StringComparison.OrdinalIgnoreCase))
            return (false, 0);
        // Optional: inspect header bytes for magic number
        return (true, Capabilities.DetectionPriority);
    }

    public async Task<ISaveDocument> OpenAsync(string filePath, CancellationToken ct = default)
    {
        var content = await File.ReadAllTextAsync(filePath, ct);
        return new MyGameDocument(filePath, content);
    }
}
```

## Step 3: Implement ISaveDocument

```csharp
public sealed class MyGameDocument : ISaveDocument
{
    private readonly Dictionary<string, object?> _data;
    private bool _isDirty;

    public string FilePath { get; }
    public bool IsDirty => _isDirty;

    public MyGameDocument(string filePath, string json)
    {
        FilePath = filePath;
        _data = JsonSerializer.Deserialize<Dictionary<string, object?>>(json) ?? [];
    }

    public object? GetValue(string dotPath) => _data.GetValueOrDefault(dotPath);

    public void SetValue(string dotPath, object? value)
    {
        _data[dotPath] = value;
        _isDirty = true;
    }

    public IEditorDefinition GetEditorDefinition() =>
        new SchemaDefinition(MyGameSchema.Build());

    public async Task SaveAsync(CancellationToken ct = default)
    {
        await File.WriteAllTextAsync(FilePath, JsonSerializer.Serialize(_data), ct);
        _isDirty = false;
    }
}
```

## Step 4: Define a FormSchema

```csharp
public static class MyGameSchema
{
    public static FormSchema Build() => new()
    {
        Pages = [
            new FormPage
            {
                Title = "Player",
                Sections = [
                    new FormSection
                    {
                        Fields = [
                            new TextField { Key = "name", Label = "Name", IsRequired = true },
                            new NumericField { Key = "level", Label = "Level", Min = 1, Max = 99 }
                        ]
                    }
                ]
            }
        ]
    };
}
```

## Step 5: Create manifest.json

```json
{
  "id": "my-game",
  "name": "MyGame Save Editor",
  "description": "Edit saves for MyGame.",
  "author": "Your Name",
  "version": "1.0.0",
  "minHostVersion": "1.0.0",
  "maxHostVersion": null,
  "pluginAssembly": "MyGame.SaveEditor.dll"
}
```

## Step 6: Package

Zip the following into `my-game-1.0.0.zip`:
- `manifest.json`
- `MyGame.SaveEditor.dll`
- `MyGame.SaveEditor.deps.json`
- Any dependencies not already in the host

## Custom UI (Advanced)

If the schema renderer is insufficient, implement `ICustomEditorViewFactory`:

```csharp
public sealed class MyViewFactory : ICustomEditorViewFactory
{
    public Control CreateView(ISaveDocument doc) => new MyEditorView();
    public object CreateViewModel(ISaveDocument doc) => new MyEditorViewModel(doc);
}
```

Return `EditorKind.Custom` from `IEditorDefinition.Kind` and set `ViewFactory`.
````

- [ ] **Step 3: Create CATALOG_SCHEMA.md**

````markdown
# Catalog Schema

The catalog is a JSON file served from a configured URL. The app downloads and caches it.

## Schema

```json
{
  "schemaVersion": "1.0",
  "generatedAt": "2026-04-04T00:00:00Z",
  "plugins": [
    {
      "id": "my-plugin",
      "name": "My Plugin",
      "description": "Short description shown in the store.",
      "author": "Author Name",
      "latestVersion": "1.2.0",
      "minHostVersion": "1.0.0",
      "maxHostVersion": null,
      "packageUrl": "https://example.com/releases/my-plugin-1.2.0.zip",
      "sha256": "lowercase hex SHA-256 of the zip file",
      "iconUrl": "https://example.com/icons/my-plugin.png",
      "screenshotUrls": [
        "https://example.com/screenshots/my-plugin-1.png"
      ],
      "tags": ["rpg", "jrpg"],
      "repositoryUrl": "https://github.com/example/my-plugin"
    }
  ]
}
```

## Field Reference

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | string | Yes | Unique plugin identifier (lowercase, hyphens) |
| `name` | string | Yes | Display name |
| `description` | string | No | Short description |
| `author` | string | Yes | Author name |
| `latestVersion` | string | Yes | SemVer string |
| `minHostVersion` | string | Yes | Minimum compatible host version |
| `maxHostVersion` | string/null | No | Maximum compatible host version, null = no limit |
| `packageUrl` | string | Yes | Direct download URL for the zip package |
| `sha256` | string | Yes | Lowercase hex SHA-256 of the zip |
| `iconUrl` | string/null | No | Icon image URL |
| `screenshotUrls` | string[] | No | Screenshot image URLs |
| `tags` | string[] | No | Search tags |
| `repositoryUrl` | string/null | No | Source repository URL |

## Hosting on GitHub

Place `catalog.json` in a public GitHub repo. The raw URL format is:

```
https://raw.githubusercontent.com/{owner}/{repo}/{branch}/catalog.json
```

Set this URL in the app's Settings page or in `appsettings.json` for the release build.
````

- [ ] **Step 4: Create README.md**

```markdown
# SaveForge (SaveForge)

A cross-platform desktop application for editing local single-player game save files via a plugin system.

## Building

Requirements: .NET 8 SDK

```bash
dotnet restore
dotnet build
dotnet run --project src/SaveForge.App/
```

## Running Tests

```bash
dotnet test tests/SaveForge.Tests/
```

## Plugin Development

See [docs/PLUGIN_AUTHORING.md](docs/PLUGIN_AUTHORING.md).

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Catalog Format

See [docs/CATALOG_SCHEMA.md](docs/CATALOG_SCHEMA.md).
```

- [ ] **Step 5: Final full build and test**

```bash
dotnet build SaveForge.sln
dotnet test tests/SaveForge.Tests/ -v minimal
```

Expected: Build succeeded. All tests green.

- [ ] **Step 6: Commit**

```bash
git add docs/ README.md
git commit -m "docs: add architecture, plugin authoring guide, catalog schema, and README"
```

---

## Summary

| Task | Deliverable |
|---|---|
| 1 | Solution scaffold — all .csproj files and references |
| 2 | SDK interfaces: IGameEditorPlugin, ISaveDocument, IEditorDefinition |
| 3 | SDK schema types + VisibilityRule with tests |
| 4 | Core domain models and service interfaces |
| 5 | Infrastructure: settings, backup (with tests), HTTP downloader |
| 6 | Security: HashVerifier, ManifestValidator (with tests) |
| 7 | PluginHost: LoadContext, Loader, Registry, FileDetection (with tests) |
| 8 | PluginHost: startup scanner |
| 9 | Store: CatalogService, PluginInstaller, UpdateChecker (with tests) |
| 10 | SamplePlugin: StardewLike (with tests) |
| 11 | Forms: all field view models |
| 12 | Forms: FormRendererViewModel + AXAML view |
| 13 | App: dark theme palette + custom chrome controls |
| 14 | App: all page view models |
| 15 | App: all page AXAML views |
| 16 | App: DI wiring + Program.cs |
| 17 | Final test run |
| 18 | Documentation: ARCHITECTURE, PLUGIN_AUTHORING, CATALOG_SCHEMA, README |
