using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class StoryEventData
{
    [JsonPropertyName("stringId")]
    public string StringId { get; set; } = "";
}
