using System.Reflection;
using CommunityToolkit.Mvvm.ComponentModel;

namespace SaveForge.App.ViewModels;

public sealed class AboutViewModel : ObservableObject
{
    public string AppVersion { get; } =
        Assembly.GetExecutingAssembly().GetName().Version?.ToString() ?? "0.0.0";

    public string HostVersion { get; } = "1.0.0";
}
