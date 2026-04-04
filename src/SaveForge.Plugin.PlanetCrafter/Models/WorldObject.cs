using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

/// <summary>
/// World objects are highly varied (thousands of different gId types with different fields).
/// We store the raw JsonObject to preserve all fields and avoid data loss.
/// </summary>
public sealed class WorldObject
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("gId")]
    public string GId { get; set; } = "";

    /// <summary>Raw JSON node for all additional optional fields (pos, rot, planet, text, etc.).</summary>
    [JsonIgnore]
    public JsonObject? RawNode { get; set; }
}
