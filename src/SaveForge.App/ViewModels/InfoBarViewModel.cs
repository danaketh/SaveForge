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
