using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;

namespace SaveForge.App.Controls;

public partial class TitleBarControl : UserControl
{
    public TitleBarControl()
    {
        InitializeComponent();
    }

    private bool _wired;

    protected override void OnAttachedToVisualTree(VisualTreeAttachmentEventArgs e)
    {
        base.OnAttachedToVisualTree(e);
        if (_wired)
        {
            return;
        }

        _wired = true;
        WireButtons();
    }

    private void WireButtons()
    {
        var window = TopLevel.GetTopLevel(this) as Window;
        if (window is null)
        {
            return;
        }

        if (this.FindControl<Button>("MinimizeButton") is { } min)
        {
            min.Click += (_, _) => window.WindowState = WindowState.Minimized;
        }

        if (this.FindControl<Button>("MaximizeButton") is { } max)
        {
            max.Click += (_, _) =>
                window.WindowState = window.WindowState == WindowState.Maximized
                    ? WindowState.Normal
                    : WindowState.Maximized;
        }

        if (this.FindControl<Button>("CloseButton") is { } close)
        {
            close.Click += (_, _) => window.Close();
        }

        if (this.FindControl<Panel>("DragRegion") is { } drag)
        {
            drag.PointerPressed += (_, args) =>
            {
                if (args.GetCurrentPoint(drag).Properties.IsLeftButtonPressed)
                {
                    window.BeginMoveDrag(args);
                }
            };
        }
    }
}
