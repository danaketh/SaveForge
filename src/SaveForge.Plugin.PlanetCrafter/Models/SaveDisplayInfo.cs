using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class SaveDisplayInfo
{
    [JsonPropertyName("saveDisplayName")]
    public string SaveDisplayName { get; set; } = "";

    [JsonPropertyName("planetId")]
    public string PlanetId { get; set; } = "";

    [JsonPropertyName("unlockedSpaceTrading")]
    public bool UnlockedSpaceTrading { get; set; }

    [JsonPropertyName("unlockedOreExtrators")]
    public bool UnlockedOreExtractors { get; set; }

    [JsonPropertyName("unlockedTeleporters")]
    public bool UnlockedTeleporters { get; set; }
}
