using Avalonia.Controls;
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
        {
            _ = vm.LoadAsync(CancellationToken.None);
        }
    }
}
