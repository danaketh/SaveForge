using System.Text.Json.Serialization;

namespace SaveForge.Store.Dtos;

public sealed class CatalogEntryDto
{
    [JsonPropertyName("id")]
    public required string Id { get; init; }

    [JsonPropertyName("name")]
    public required string Name { get; init; }

    [JsonPropertyName("description")]
    public string Description { get; init; } = string.Empty;

    [JsonPropertyName("author")]
    public required string Author { get; init; }

    [JsonPropertyName("latestVersion")]
    public required string LatestVersion { get; init; }

    [JsonPropertyName("minHostVersion")]
    public required string MinHostVersion { get; init; }

    [JsonPropertyName("maxHostVersion")]
    public string? MaxHostVersion { get; init; }

    [JsonPropertyName("packageUrl")]
    public required string PackageUrl { get; init; }

    [JsonPropertyName("sha256")]
    public required string Sha256 { get; init; }

    [JsonPropertyName("iconUrl")]
    public string? IconUrl { get; init; }

    [JsonPropertyName("screenshotUrls")]
    public string[] ScreenshotUrls { get; init; } = Array.Empty<string>();

    [JsonPropertyName("tags")]
    public string[] Tags { get; init; } = Array.Empty<string>();

    [JsonPropertyName("repositoryUrl")]
    public string? RepositoryUrl { get; init; }
}
