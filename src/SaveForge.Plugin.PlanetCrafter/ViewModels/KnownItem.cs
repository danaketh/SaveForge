namespace SaveForge.Plugin.PlanetCrafter.ViewModels;

/// <summary>A localized item that can be placed in an inventory or equipment slot.</summary>
internal sealed record KnownItem(string GId, string DisplayName)
{
    public override string ToString() => DisplayName;
}
