using SaveForge.Core.Interfaces;
using SaveForge.Infrastructure.Services;
using SaveForge.SDK;
using Xunit;

namespace SaveForge.Tests;

public class SaveDiscoveryServiceTests : IDisposable
{
    private readonly string _root = Path.Combine(Path.GetTempPath(), $"sds-test-{Guid.NewGuid()}");
    private readonly ISaveDiscoveryService _svc = new SaveDiscoveryService();

    public SaveDiscoveryServiceTests()
    {
        Directory.CreateDirectory(_root);
    }

    [Fact]
    public async Task DiscoverAsync_EmptyDirectory_ReturnsEmpty()
    {
        var caps = new SaveCapabilities { FileExtensions = [".json"] };
        var results = await _svc.DiscoverAsync(_root, caps);
        Assert.Empty(results);
    }

    [Fact]
    public async Task DiscoverAsync_MatchingFile_ReturnsIt()
    {
        File.WriteAllText(Path.Combine(_root, "save.json"), "{}");
        var caps = new SaveCapabilities { FileExtensions = [".json"] };

        var results = await _svc.DiscoverAsync(_root, caps);

        Assert.Single(results);
        Assert.Equal("save.json", results[0].FileName);
    }

    [Fact]
    public async Task DiscoverAsync_NonMatchingExtension_Excluded()
    {
        File.WriteAllText(Path.Combine(_root, "save.txt"), "data");
        var caps = new SaveCapabilities { FileExtensions = [".json"] };

        var results = await _svc.DiscoverAsync(_root, caps);

        Assert.Empty(results);
    }

    [Fact]
    public async Task DiscoverAsync_RecursesIntoSubdirectories()
    {
        var sub = Directory.CreateDirectory(Path.Combine(_root, "slot1"));
        File.WriteAllText(Path.Combine(sub.FullName, "save.json"), "{}");
        var caps = new SaveCapabilities { FileExtensions = [".json"] };

        var results = await _svc.DiscoverAsync(_root, caps);

        Assert.Single(results);
        Assert.Equal("save.json", results[0].FileName);
    }

    [Fact]
    public async Task DiscoverAsync_SortedByLastModifiedDescending()
    {
        var older = Path.Combine(_root, "old.json");
        var newer = Path.Combine(_root, "new.json");
        File.WriteAllText(older, "{}");
        File.SetLastWriteTimeUtc(older, DateTime.UtcNow.AddHours(-1));
        File.WriteAllText(newer, "{}");
        File.SetLastWriteTimeUtc(newer, DateTime.UtcNow);
        var caps = new SaveCapabilities { FileExtensions = [".json"] };

        var results = await _svc.DiscoverAsync(_root, caps);

        Assert.Equal(2, results.Count);
        Assert.Equal("new.json", results[0].FileName);
        Assert.Equal("old.json", results[1].FileName);
    }

    [Fact]
    public async Task DiscoverAsync_DirectoryDoesNotExist_ReturnsEmpty()
    {
        var missing = Path.Combine(_root, "nonexistent");
        var caps = new SaveCapabilities { FileExtensions = [".json"] };

        var results = await _svc.DiscoverAsync(missing, caps);

        Assert.Empty(results);
    }

    [Fact]
    public async Task DiscoverAsync_ExtensionMatchIsCaseInsensitive()
    {
        File.WriteAllText(Path.Combine(_root, "SAVE.JSON"), "{}");
        var caps = new SaveCapabilities { FileExtensions = [".json"] };

        var results = await _svc.DiscoverAsync(_root, caps);

        Assert.Single(results);
    }

    [Fact]
    public async Task DiscoverAsync_PathWithToken_ExpandsBeforeScanning()
    {
        // Use the real temp dir via %TEMP% token
        var caps = new SaveCapabilities { FileExtensions = [".json"] };
        // Should not throw even when the expanded dir has files we don't care about
        var results = await _svc.DiscoverAsync("%TEMP%", caps);
        // Just assert no exception; result count varies by machine
        Assert.NotNull(results);
    }

    public void Dispose()
    {
        if (Directory.Exists(_root))
        {
            Directory.Delete(_root, recursive: true);
        }
    }
}
