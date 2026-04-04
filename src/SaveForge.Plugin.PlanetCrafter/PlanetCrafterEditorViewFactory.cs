using Avalonia.Controls;
using SaveForge.Plugin.PlanetCrafter.ViewModels;
using SaveForge.Plugin.PlanetCrafter.Views;
using SaveForge.SDK;

namespace SaveForge.Plugin.PlanetCrafter;

internal sealed class PlanetCrafterEditorViewFactory(
    PlanetCrafterDocument document,
    LocalizationService localization) : ICustomEditorViewFactory
{
    public Control CreateView(ISaveDocument _)
    {
        var vm = new PlanetCrafterEditorViewModel(document, localization);
        return new PlanetCrafterEditorView { DataContext = vm };
    }

    public object CreateViewModel(ISaveDocument _) =>
        new PlanetCrafterEditorViewModel(document, localization);
}
