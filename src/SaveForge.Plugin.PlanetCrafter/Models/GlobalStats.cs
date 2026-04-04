using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class GlobalStats
{
    [JsonPropertyName("craftedObjects")]
    public int CraftedObjects { get; set; }

    [JsonPropertyName("totalSaveFileLoad")]
    public int TotalSaveFileLoad { get; set; }

    [JsonPropertyName("totalSaveFileTime")]
    public int TotalSaveFileTime { get; set; }
}
