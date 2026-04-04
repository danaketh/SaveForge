using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class PlanetInstance
{
    [JsonPropertyName("owner")]
    public int Owner { get; set; }

    [JsonPropertyName("planet")]
    public long Planet { get; set; }

    [JsonPropertyName("index")]
    public int Index { get; set; }

    [JsonPropertyName("seed")]
    public int Seed { get; set; }

    [JsonPropertyName("pos")]
    public string Pos { get; set; } = "";

    [JsonPropertyName("rot")]
    public string Rot { get; set; } = "";
}
