namespace SaveForge.Plugin.PlanetCrafter.Models;

/// <summary>Top-level aggregate model for a Planet Crafter save file.</summary>
public sealed class PlanetCrafterSave
{
    public required TerraTokensData TerraTokens { get; init; }
    public required PlanetData Planet { get; init; }
    public required PlayerData[] Players { get; init; }
    public required List<WorldObject> WorldObjects { get; init; }
    public required InventoryData[] Inventories { get; init; }
    public required GlobalStats Stats { get; init; }
    public required MessageData[] Messages { get; init; }
    public required StoryEventData[] StoryEvents { get; init; }
    public required SaveDisplayInfo DisplayInfo { get; init; }
    public required TerrainLayer[] TerrainLayers { get; init; }
    public required PlanetInstance[] PlanetInstances { get; init; }
}
