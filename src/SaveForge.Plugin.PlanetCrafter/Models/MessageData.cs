using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class MessageData
{
    [JsonPropertyName("stringId")]
    public string StringId { get; set; } = "";

    [JsonPropertyName("isRead")]
    public bool IsRead { get; set; }
}
