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
        {
            FormRenderer = new FormRendererViewModel(def.Schema, document);
        }
        else if (def.Kind == EditorKind.Custom && def.ViewFactory is not null)
        {
            CustomView = def.ViewFactory.CreateView(document);
        }
    }

    [RelayCommand]
    private async Task SaveAsync(CancellationToken ct)
    {
        try
        {
            // 1. Copy original before any write (backup the unchanged file)
            var timestamp = DateTimeOffset.UtcNow.ToString("yyyy-MM-ddTHH-mm-ss", System.Globalization.CultureInfo.InvariantCulture);
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
