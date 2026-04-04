using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class TerraTokensData
{
    [JsonPropertyName("terraTokens")]
    public long TerraTokens { get; set; }

    [JsonPropertyName("allTimeTerraTokens")]
    public long AllTimeTerraTokens { get; set; }

    [JsonPropertyName("unlockedGroups")]
    public string UnlockedGroups { get; set; } = "";

    [JsonPropertyName("openedInstanceSeed")]
    public int OpenedInstanceSeed { get; set; }

    [JsonPropertyName("openedInstanceTimeLeft")]
    public double OpenedInstanceTimeLeft { get; set; }
}
