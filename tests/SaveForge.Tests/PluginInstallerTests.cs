using System.IO.Compression;
using System.Security.Cryptography;
using System.Text.Json;
using Microsoft.Extensions.Logging.Abstractions;
using SaveForge.Core.Models;
using SaveForge.Infrastructure.Services;
using SaveForge.PluginHost;
using SaveForge.Store;
using SaveForge.Store.Dtos;
using Xunit;

namespace SaveForge.Tests;

public class PluginInstallerTests : IDisposable
{
    private readonly string _tempDir = Path.Combine(Path.GetTempPath(), "gse-install-" + Guid.NewGuid());
    private readonly string _pluginDir;
    private readonly string _catalogDir;

    public PluginInstallerTests()
    {
        Directory.CreateDirectory(_tempDir);
        _pluginDir = Path.Combine(_tempDir, "plugins");
        _catalogDir = Path.Combine(_tempDir, "catalog");
        Directory.CreateDirectory(_catalogDir);
    }

    [Fact]
    public async Task InstallAsync_ValidPackage_ExtractsToPluginDir()
    {
        var (zipPath, sha256) = CreateValidPluginZip("my-plugin", "1.0.0");
        var catalogUrl = CreateCatalog("my-plugin", "1.0.0", new Uri(zipPath).AbsoluteUri, sha256);

        var installer = CreateInstaller(catalogUrl);
        await installer.InstallAsync("my-plugin");

        var expectedDir = Path.Combine(_pluginDir, "my-plugin", "1.0.0");
        Assert.True(Directory.Exists(expectedDir));
        Assert.True(File.Exists(Path.Combine(expectedDir, "manifest.json")));
    }

    [Fact]
    public async Task InstallAsync_BadHash_ThrowsAndDoesNotInstall()
    {
        var (zipPath, _) = CreateValidPluginZip("bad-plugin", "1.0.0");
        var catalogUrl = CreateCatalog("bad-plugin", "1.0.0", new Uri(zipPath).AbsoluteUri, "0000000000000000000000000000000000000000000000000000000000000000");

        var installer = CreateInstaller(catalogUrl);

        await Assert.ThrowsAnyAsync<Exception>(() => installer.InstallAsync("bad-plugin"));
        Assert.False(Directory.Exists(Path.Combine(_pluginDir, "bad-plugin")));
    }

    private PluginInstaller CreateInstaller(string catalogUrl)
    {
        var downloader = new HttpDownloadService(new HttpClient());
        var registry = new PluginRegistry();
        var loader = new PluginLoader(NullLogger<PluginLoader>.Instance, "1.0.0");
        var cachePath = Path.Combine(_tempDir, "cache.json");
        var catalogService = new CatalogService(downloader, catalogUrl, cachePath, new HashSet<string>());
        return new PluginInstaller(downloader, catalogService, registry, loader,
            NullLogger<PluginInstaller>.Instance, _pluginDir);
    }

    private (string ZipPath, string Sha256) CreateValidPluginZip(string id, string version)
    {
        // Locate the StardewLike DLL from test output directory so plugin loading succeeds
        const string dllName = "SaveForge.SamplePlugin.StardewLike.dll";
        var stardewDll = Path.Combine(AppContext.BaseDirectory, dllName);

        var zipPath = Path.Combine(_tempDir, $"{id}-{version}.zip");
        using (var zip = ZipFile.Open(zipPath, ZipArchiveMode.Create))
        {
            var assemblyEntry = File.Exists(stardewDll) ? dllName : "dummy.dll";
            var manifest = new
            {
                id,
                name = id,
                version,
                author = "Test",
                minHostVersion = "1.0.0",
                pluginAssembly = assemblyEntry
            };
            var mEntry = zip.CreateEntry("manifest.json");
            using (var w = new StreamWriter(mEntry.Open()))
            {
                w.Write(JsonSerializer.Serialize(manifest));
            }

            // Include the real DLL if available so the load step succeeds.
            // Copy first since the file may be locked by the current test process.
            if (File.Exists(stardewDll))
            {
                var tempDll = Path.Combine(_tempDir, dllName);
                File.Copy(stardewDll, tempDll, overwrite: true);
                zip.CreateEntryFromFile(tempDll, dllName);
            }
        }
        var sha256 = ComputeSha256(zipPath);
        return (zipPath, sha256);
    }

    private string CreateCatalog(string id, string version, string packageUrl, string sha256)
    {
        var dto = new CatalogManifestDto
        {
            Plugins = [new CatalogEntryDto { Id = id, Name = id, Author = "T",
                LatestVersion = version, MinHostVersion = "1.0.0",
                PackageUrl = packageUrl, Sha256 = sha256 }]
        };
        var path = Path.Combine(_catalogDir, "catalog.json");
        File.WriteAllText(path, JsonSerializer.Serialize(dto));
        return new Uri(path).AbsoluteUri;
    }

    private static string ComputeSha256(string path)
    {
        using var sha = SHA256.Create();
        using var stream = File.OpenRead(path);
        return Convert.ToHexString(sha.ComputeHash(stream)).ToLowerInvariant();
    }

    public void Dispose()
    {
        // Force GC to unload any collectible AssemblyLoadContexts holding plugin DLLs
        GC.Collect();
        GC.WaitForPendingFinalizers();
        GC.Collect();
        try
        { Directory.Delete(_tempDir, recursive: true); }
        catch { /* ignore locked files */ }
    }
}
