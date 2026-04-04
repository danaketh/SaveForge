using Avalonia.Controls;
using Avalonia.Interactivity;
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
        {
            _ = vm.RunPostLoadTasksAsync();
        }
    }

    private void PluginIcon_Click(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainViewModel vm)
        {
            return;
        }

        if (sender is not Button { DataContext: PluginListItem item })
        {
            return;
        }

        vm.NavigateToCommand.Execute("home");
        vm.Home.SelectPluginCommand.Execute(item.Plugin);
    }
}
