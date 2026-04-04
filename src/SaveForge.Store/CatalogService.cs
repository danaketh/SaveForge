using System.Text.Json;
using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.Infrastructure.Services;
using SaveForge.Store.Dtos;

namespace SaveForge.Store;

public sealed class CatalogService : ICatalogService
{
    private readonly HttpDownloadService _downloader;
    private readonly string _catalogUrl;
    private readonly string _cachePath;
    private readonly IReadOnlySet<string> _installedPluginIds;
    private IReadOnlyList<CatalogEntry>? _cache;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true
    };

    public CatalogService(
        HttpDownloadService downloader,
        string catalogUrl,
        string cachePath,
        IReadOnlySet<string> installedPluginIds)
    {
        _downloader = downloader;
        _catalogUrl = catalogUrl;
        _cachePath = cachePath;
        _installedPluginIds = installedPluginIds;
    }

    public async Task<IReadOnlyList<CatalogEntry>> GetCatalogAsync(
        bool forceRefresh = false, CancellationToken ct = default)
    {
        if (_cache is not null && !forceRefresh)
        {
            return _cache;
        }

        CatalogManifestDto? dto = null;

        try
        {
            var json = await _downloader.FetchTextAsync(_catalogUrl, ct);
            dto = JsonSerializer.Deserialize<CatalogManifestDto>(json, JsonOptions);
            // Cache to disk
            Directory.CreateDirectory(Path.GetDirectoryName(_cachePath)!);
            await File.WriteAllTextAsync(_cachePath, json, ct);
        }
        catch
        {
            // Fall back to disk cache
            if (File.Exists(_cachePath))
            {
                try
                {
                    var json = await File.ReadAllTextAsync(_cachePath, ct);
                    dto = JsonSerializer.Deserialize<CatalogManifestDto>(json, JsonOptions);
                }
                catch { /* swallow */ }
            }
        }

        if (dto is null)
        {
            return Array.Empty<CatalogEntry>();
        }

        _cache = dto.Plugins.Select(ToEntry).ToList();
        return _cache;
    }

    private CatalogEntry ToEntry(CatalogEntryDto dto)
    {
        var isInstalled = _installedPluginIds.Contains(dto.Id);
        return new CatalogEntry
        {
            Id = dto.Id,
            Name = dto.Name,
            Description = dto.Description,
            Author = dto.Author,
            LatestVersion = dto.LatestVersion,
            MinHostVersion = dto.MinHostVersion,
            MaxHostVersion = dto.MaxHostVersion,
            PackageUrl = dto.PackageUrl,
            Sha256 = dto.Sha256,
            IconUrl = dto.IconUrl,
            ScreenshotUrls = dto.ScreenshotUrls,
            Tags = dto.Tags,
            RepositoryUrl = dto.RepositoryUrl,
            IsInstalled = isInstalled,
            HasUpdate = false // UpdateChecker sets this separately
        };
    }
}
