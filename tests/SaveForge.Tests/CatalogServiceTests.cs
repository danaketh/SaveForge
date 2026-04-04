using System.Text.Json;
using SaveForge.Infrastructure.Services;
using SaveForge.Store;
using SaveForge.Store.Dtos;
using Xunit;

namespace SaveForge.Tests;

public class CatalogServiceTests : IDisposable
{
    private readonly string _tempDir = Path.Combine(Path.GetTempPath(), "gse-catalog-" + Guid.NewGuid());

    public CatalogServiceTests() => Directory.CreateDirectory(_tempDir);

    [Fact]
    public async Task GetCatalogAsync_LocalFile_ReturnsParsedEntries()
    {
        var catalogPath = Path.Combine(_tempDir, "catalog.json");
        var dto = new CatalogManifestDto
        {
            SchemaVersion = "1.0",
            GeneratedAt = DateTime.UtcNow,
            Plugins = [new CatalogEntryDto
            {
                Id = "test-plugin", Name = "Test", Author = "A",
                LatestVersion = "1.0.0", MinHostVersion = "1.0.0",
                PackageUrl = "http://example.com/p.zip", Sha256 = "abc"
            }]
        };
        await File.WriteAllTextAsync(catalogPath, JsonSerializer.Serialize(dto));

        var url = new Uri(catalogPath).AbsoluteUri;
        var cachePath = Path.Combine(_tempDir, "cache.json");
        var downloader = new HttpDownloadService(new HttpClient());
        var svc = new CatalogService(downloader, url, cachePath, installedPluginIds: new HashSet<string>());

        var entries = await svc.GetCatalogAsync();

        Assert.Single(entries);
        Assert.Equal("test-plugin", entries[0].Id);
    }

    [Fact]
    public async Task GetCatalogAsync_MarksInstalledPlugin()
    {
        var catalogPath = Path.Combine(_tempDir, "catalog2.json");
        var dto = new CatalogManifestDto
        {
            Plugins = [new CatalogEntryDto
            {
                Id = "installed-plugin", Name = "Installed", Author = "A",
                LatestVersion = "1.0.0", MinHostVersion = "1.0.0",
                PackageUrl = "http://example.com/p.zip", Sha256 = "abc"
            }]
        };
        await File.WriteAllTextAsync(catalogPath, JsonSerializer.Serialize(dto));

        var url = new Uri(catalogPath).AbsoluteUri;
        var cachePath = Path.Combine(_tempDir, "cache2.json");
        var downloader = new HttpDownloadService(new HttpClient());
        var svc = new CatalogService(downloader, url, cachePath,
            installedPluginIds: new HashSet<string> { "installed-plugin" });

        var entries = await svc.GetCatalogAsync();

        Assert.True(entries[0].IsInstalled);
    }

    public void Dispose() => Directory.Delete(_tempDir, recursive: true);
}
