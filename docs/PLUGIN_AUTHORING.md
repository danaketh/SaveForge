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
