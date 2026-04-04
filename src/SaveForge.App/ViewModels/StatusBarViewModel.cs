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
