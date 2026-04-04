using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class InventoryData
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("woIds")]
    public string WoIds { get; set; } = "";

    [JsonPropertyName("size")]
    public int Size { get; set; }
}
