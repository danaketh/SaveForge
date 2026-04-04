using System.Text.Json.Serialization;

namespace SaveForge.Store.Dtos;

public sealed class CatalogManifestDto
{
    [JsonPropertyName("schemaVersion")]
    public string SchemaVersion { get; init; } = "1.0";

    [JsonPropertyName("generatedAt")]
    public DateTime GeneratedAt { get; init; }

    [JsonPropertyName("plugins")]
    public CatalogEntryDto[] Plugins { get; init; } = Array.Empty<CatalogEntryDto>();
}
