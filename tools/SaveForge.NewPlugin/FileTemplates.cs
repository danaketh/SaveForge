namespace SaveForge.NewPlugin;

/// <summary>
/// File content templates for the generated plugin scaffold.
///
/// Interpolation convention across all templates:
///   $$"""..."""  →  {{expr}} for substitution, bare { } for code/AXAML braces.
///   """..."""    →  non-interpolated; used for fixed code blocks with no substitutions.
/// </summary>
internal static class FileTemplates
{
    public static string Csproj(PluginOptions o)
    {
        var extra = o.EditorType == EditorType.Custom ? """

  <ItemGroup>
    <PackageReference Include="Avalonia" />
    <PackageReference Include="CommunityToolkit.Mvvm" />
  </ItemGroup>
""" : "";

        return $$"""
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <RootNamespace>{{o.Assembly}}</RootNamespace>
    <IsDevPlugin>true</IsDevPlugin>
  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="..\SaveForge.SDK\SaveForge.SDK.csproj" />
  </ItemGroup>{{extra}}
</Project>
""";
    }

    public static string Manifest(PluginOptions o) => $$"""
{
  "id": "{{o.Id}}",
  "name": "{{o.DisplayName}}",
  "description": "{{o.Description}}",
  "author": "{{o.Author}}",
  "version": "1.0.0",
  "minHostVersion": "1.0.0",
  "maxHostVersion": null,
  "pluginAssembly": "{{o.Assembly}}.dll"
}
""";

    public static string PluginClass(PluginOptions o)
    {
        var extensionsList = string.Join(", ", o.Extensions.Select(e => $"\"{e}\""));
        var firstExt = o.Extensions[0];

        var detectionSnippet = string.IsNullOrWhiteSpace(o.DetectionKey)
            ? "        // TODO: implement detection logic for this save format\n        return (true, Capabilities.DetectionPriority);"
            : $$"""
        var text = Encoding.UTF8.GetString(header[..Math.Min(512, header.Length)]);
        return text.Contains("{{o.DetectionKey}}", StringComparison.Ordinal)
            ? (true, Capabilities.DetectionPriority)
            : (false, 0);
""";

        return $$"""
using System.Text;
using SaveForge.SDK;

namespace {{o.Assembly}};

public sealed class {{o.Name}}Plugin : IGameEditorPlugin
{
    public PluginManifest Manifest { get; } = new()
    {
        Id             = "{{o.Id}}",
        Name           = "{{o.DisplayName}}",
        Version        = "1.0.0",
        Author         = "{{o.Author}}",
        Description    = "{{o.Description}}",
        MinHostVersion = "1.0.0",
        PluginAssembly = "{{o.Assembly}}.dll"
    };

    public SaveCapabilities Capabilities { get; } = new()
    {
        FileExtensions       = [{{extensionsList}}],
        DetectionPriority    = 10,
        DefaultSaveDirectory = @"{{o.SaveDirectory}}"
    };

    public (bool CanOpen, int Priority) CanOpen(FileInfo file, ReadOnlySpan<byte> header)
    {
        if (!file.Extension.Equals("{{firstExt}}", StringComparison.OrdinalIgnoreCase))
            return (false, 0);

{{detectionSnippet}}
    }

    public async Task<ISaveDocument> OpenAsync(string filePath, CancellationToken ct = default)
    {
        var content = await File.ReadAllTextAsync(filePath, ct);
        // TODO: parse content into your domain model
        return new {{o.Name}}Document(filePath, content);
    }
}
""";
    }

    public static string DocumentClass(PluginOptions o)
    {
        var editorDef = o.EditorType == EditorType.Schema
            ? $"new SchemaEditorDefinition({o.Name}Schema.Build())"
            : $"new CustomEditorDefinition(new {o.Name}EditorViewFactory(this))";

        var extraUsings = o.EditorType == EditorType.Schema
            ? "using SaveForge.SDK.Schema;\n"
            : "";

        // Non-interpolated raw strings: { and } are always literal here.
        var helperClass = o.EditorType == EditorType.Schema ? """

internal sealed class SchemaEditorDefinition(FormSchema schema) : IEditorDefinition
{
    public EditorKind Kind => EditorKind.Schema;
    public FormSchema? Schema => schema;
    public ICustomEditorViewFactory? ViewFactory => null;
}
""" : """

internal sealed class CustomEditorDefinition(ICustomEditorViewFactory factory) : IEditorDefinition
{
    public EditorKind Kind => EditorKind.Custom;
    public FormSchema? Schema => null;
    public ICustomEditorViewFactory? ViewFactory => factory;
}
""";

        return $$"""
using SaveForge.SDK;
{{extraUsings}}
namespace {{o.Assembly}};

public sealed class {{o.Name}}Document : ISaveDocument
{
    // TODO: replace string with your parsed domain model type
    private readonly string _rawContent;
    private bool _isDirty;

    public {{o.Name}}Document(string filePath, string rawContent)
    {
        FilePath = filePath;
        _rawContent = rawContent;
    }

    public string FilePath { get; }
    public bool IsDirty => _isDirty;

    public object? GetValue(string dotPath)
    {
        // TODO: navigate your domain model using dotPath
        return null;
    }

    public void SetValue(string dotPath, object? value)
    {
        // TODO: mutate your domain model at dotPath
        _isDirty = true;
    }

    public IEditorDefinition GetEditorDefinition() => {{editorDef}};

    public async Task SaveAsync(CancellationToken ct = default)
    {
        // TODO: serialize your domain model and write to FilePath
        await Task.CompletedTask;
        _isDirty = false;
    }
}
{{helperClass}}
""";
    }

    public static string SchemaClass(PluginOptions o) => $$"""
using SaveForge.SDK.Schema;

namespace {{o.Assembly}};

internal static class {{o.Name}}Schema
{
    public static FormSchema Build() => new()
    {
        Pages =
        [
            new FormPage
            {
                Title = "General",
                Sections =
                [
                    new FormSection
                    {
                        Heading = "TODO: section heading",
                        Fields =
                        [
                            // TODO: add fields — see docs/PLUGIN_AUTHORING.md for the field type list
                            new TextField
                            {
                                Key = "player.name",
                                Label = "Name",
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
                            }
                        ]
                    }
                ]
            }
        ]
    };
}
""";

    public static string ViewFactory(PluginOptions o) => $$"""
using Avalonia.Controls;
using {{o.Assembly}}.ViewModels;
using {{o.Assembly}}.Views;
using SaveForge.SDK;

namespace {{o.Assembly}};

internal sealed class {{o.Name}}EditorViewFactory({{o.Name}}Document document) : ICustomEditorViewFactory
{
    public Control CreateView(ISaveDocument _)
    {
        var vm = new {{o.Name}}EditorViewModel(document);
        return new {{o.Name}}EditorView { DataContext = vm };
    }

    public object CreateViewModel(ISaveDocument _) => new {{o.Name}}EditorViewModel(document);
}
""";

    public static string EditorViewModel(PluginOptions o) => $$"""
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using System.Globalization;

namespace {{o.Assembly}}.ViewModels;

internal sealed partial class {{o.Name}}EditorViewModel : ObservableObject
{
    private readonly {{o.Name}}Document _document;

    // TODO: add observable properties for editable fields
    [ObservableProperty] private string _saveStatus = "";

    public {{o.Name}}EditorViewModel({{o.Name}}Document document)
    {
        _document = document;
    }

    [RelayCommand]
    private async Task SaveAsync(CancellationToken ct)
    {
        try
        {
            var timestamp = DateTimeOffset.UtcNow.ToString("yyyy-MM-ddTHH-mm-ss", CultureInfo.InvariantCulture);
            var backupDir = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
                "SaveForge", "backups", "editor");
            Directory.CreateDirectory(backupDir);
            var backupPath = Path.Combine(backupDir,
                $"{Path.GetFileName(_document.FilePath)}.{timestamp}.bak");
            File.Copy(_document.FilePath, backupPath, overwrite: false);

            await _document.SaveAsync(ct);
            SaveStatus = $"Saved. Backup: {backupPath}";
        }
        catch (Exception ex)
        {
            SaveStatus = $"Save failed: {ex.Message}";
        }
    }
}
""";

    // In $$"""...""", bare { } are literal — AXAML {Binding ...} syntax is preserved as-is.
    public static string EditorViewAxaml(PluginOptions o) => $$"""
<UserControl xmlns="https://github.com/avaloniaui"
             xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
             xmlns:vm="clr-namespace:{{o.Assembly}}.ViewModels"
             x:Class="{{o.Assembly}}.Views.{{o.Name}}EditorView"
             x:DataType="vm:{{o.Name}}EditorViewModel">

  <Grid RowDefinitions="*,Auto">

    <ScrollViewer Grid.Row="0">
      <StackPanel Margin="12" Spacing="8">
        <!-- TODO: build your editor UI here -->
        <TextBlock Text="TODO: editor UI for {{o.DisplayName}}"
                   Opacity="0.5"
                   HorizontalAlignment="Center"
                   VerticalAlignment="Center" />
      </StackPanel>
    </ScrollViewer>

    <!-- Save bar -->
    <Grid Grid.Row="1" ColumnDefinitions="*,Auto" Margin="8">
      <TextBlock Grid.Column="0"
                 Text="{Binding SaveStatus}"
                 VerticalAlignment="Center"
                 Opacity="0.7"
                 TextTrimming="CharacterEllipsis" />
      <Button Grid.Column="1"
              Content="Save"
              Command="{Binding SaveCommand}"
              Classes="accent" />
    </Grid>

  </Grid>
</UserControl>
""";

    public static string EditorViewCodeBehind(PluginOptions o) => $$"""
using Avalonia.Controls;

namespace {{o.Assembly}}.Views;

public partial class {{o.Name}}EditorView : UserControl
{
    public {{o.Name}}EditorView()
    {
        InitializeComponent();
    }
}
""";
}
