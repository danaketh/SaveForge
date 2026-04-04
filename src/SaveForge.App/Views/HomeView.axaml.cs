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
            {
                vm.RequestFolderPath = OpenFolderPickerAsync;
            }
        };
    }

    private async Task<string?> OpenFolderPickerAsync()
    {
        var topLevel = TopLevel.GetTopLevel(this);
        if (topLevel is null)
        {
            return null;
        }

        var folders = await topLevel.StorageProvider.OpenFolderPickerAsync(
            new FolderPickerOpenOptions
            {
                Title = "Select Save Directory",
                AllowMultiple = false
            });

        return folders.Count > 0 ? folders[0].Path.LocalPath : null;
    }

    private void SaveItem_Click(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not HomeViewModel vm)
        {
            return;
        }

        if (sender is Button { DataContext: SaveFileInfo save })
        {
            vm.OpenSaveCommand.Execute(save);
        }
    }
}
