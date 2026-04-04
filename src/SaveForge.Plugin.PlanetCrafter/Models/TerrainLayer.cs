using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class TerrainLayer
{
    [JsonPropertyName("layerId")]
    public string LayerId { get; set; } = "";

    [JsonPropertyName("planet")]
    public long Planet { get; set; }

    [JsonPropertyName("colorBase")]
    public string ColorBase { get; set; } = "";

    [JsonPropertyName("colorCustom")]
    public string ColorCustom { get; set; } = "";

    [JsonPropertyName("colorBaseLerp")]
    public double ColorBaseLerp { get; set; }

    [JsonPropertyName("colorCustomLerp")]
    public double ColorCustomLerp { get; set; }
}
