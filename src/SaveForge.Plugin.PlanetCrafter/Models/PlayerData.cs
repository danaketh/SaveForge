using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class PlayerData
{
    [JsonPropertyName("id")]
    public long Id { get; set; }

    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("inventoryId")]
    public int InventoryId { get; set; }

    [JsonPropertyName("equipmentId")]
    public int EquipmentId { get; set; }

    [JsonPropertyName("playerPosition")]
    public string PlayerPosition { get; set; } = "";

    [JsonPropertyName("playerRotation")]
    public string PlayerRotation { get; set; } = "";

    [JsonPropertyName("playerGaugeOxygen")]
    public double PlayerGaugeOxygen { get; set; }

    [JsonPropertyName("playerGaugeThirst")]
    public double PlayerGaugeThirst { get; set; }

    [JsonPropertyName("playerGaugeHealth")]
    public double PlayerGaugeHealth { get; set; }

    [JsonPropertyName("playerGaugeToxic")]
    public double PlayerGaugeToxic { get; set; }

    [JsonPropertyName("host")]
    public bool Host { get; set; }

    [JsonPropertyName("planetId")]
    public string PlanetId { get; set; } = "";
}
