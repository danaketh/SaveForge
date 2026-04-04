using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class PlanetData
{
    [JsonPropertyName("planetId")]
    public string PlanetId { get; set; } = "";

    [JsonPropertyName("unitOxygenLevel")]
    public double UnitOxygenLevel { get; set; }

    [JsonPropertyName("unitHeatLevel")]
    public double UnitHeatLevel { get; set; }

    [JsonPropertyName("unitPressureLevel")]
    public double UnitPressureLevel { get; set; }

    [JsonPropertyName("unitPlantsLevel")]
    public double UnitPlantsLevel { get; set; }

    [JsonPropertyName("unitInsectsLevel")]
    public double UnitInsectsLevel { get; set; }

    [JsonPropertyName("unitAnimalsLevel")]
    public double UnitAnimalsLevel { get; set; }

    [JsonPropertyName("unitPurificationLevel")]
    public double UnitPurificationLevel { get; set; }
}
