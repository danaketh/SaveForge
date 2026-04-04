namespace SaveForge.Core.Models;

public sealed class CatalogEntry
{
    public required string Id { get; init; }
    public required string Name { get; init; }
    public string Description { get; init; } = string.Empty;
    public required string Author { get; init; }
    public required string LatestVersion { get; init; }
    public required string MinHostVersion { get; init; }
    public string? MaxHostVersion { get; init; }
    public required string PackageUrl { get; init; }
    public required string Sha256 { get; init; }
    public string? IconUrl { get; init; }
    public string[] ScreenshotUrls { get; init; } = Array.Empty<string>();
    public string[] Tags { get; init; } = Array.Empty<string>();
    public string? RepositoryUrl { get; init; }

    // Computed at load time by comparing against installed plugins
    public bool IsInstalled { get; init; }
    public bool HasUpdate { get; init; }
}
