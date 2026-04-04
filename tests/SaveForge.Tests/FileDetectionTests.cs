using SaveForge.Core.Interfaces;
using SaveForge.Core.Models;
using SaveForge.PluginHost;
using SaveForge.SDK;
using Xunit;

namespace SaveForge.Tests;

public class FileDetectionTests : IDisposable
{
    private readonly string _tempFile = Path.GetTempFileName();

    [Fact]
    public async Task DetectAsync_NoPlugins_ReturnsEmpty()
    {
        var registry = new PluginRegistry();
        var svc = new FileDetectionService(registry);

        var results = await svc.DetectAsync(_tempFile);

        Assert.Empty(results);
    }

    [Fact]
    public async Task DetectAsync_MatchingPlugin_ReturnsIt()
    {
        var registry = new PluginRegistry();
        registry.Register(new InstalledPlugin
        {
            Manifest = MakeManifest("p1"),
            LoadState = PluginLoadState.Loaded,
            AssemblyPath = "",
            Plugin = new AlwaysMatchPlugin("p1", priority: 0)
        });
        var svc = new FileDetectionService(registry);

        var results = await svc.DetectAsync(_tempFile);

        Assert.Single(results);
        Assert.Equal("p1", results[0].Plugin.Manifest.Id);
    }

    [Fact]
    public async Task DetectAsync_MultipleMatches_SortedByPriorityDescending()
    {
        var registry = new PluginRegistry();
        registry.Register(MakeInstalled("low", new AlwaysMatchPlugin("low", priority: 0)));
        registry.Register(MakeInstalled("high", new AlwaysMatchPlugin("high", priority: 10)));
        var svc = new FileDetectionService(registry);

        var results = await svc.DetectAsync(_tempFile);

        Assert.Equal(2, results.Count);
        Assert.Equal("high", results[0].Plugin.Manifest.Id);
    }

    private static PluginManifest MakeManifest(string id) => new()
    {
        Id = id,
        Name = id,
        Version = "1.0.0",
        Author = "Test",
        MinHostVersion = "1.0.0",
        PluginAssembly = "x.dll"
    };

    private static InstalledPlugin MakeInstalled(string id, IGameEditorPlugin plugin) => new()
    {
        Manifest = MakeManifest(id),
        LoadState = PluginLoadState.Loaded,
        AssemblyPath = "",
        Plugin = plugin
    };

    public void Dispose()
    {
        if (File.Exists(_tempFile))
        {
            File.Delete(_tempFile);
        }
    }

    // Test double
    private sealed class AlwaysMatchPlugin : IGameEditorPlugin
    {
        private readonly int _priority;
        public AlwaysMatchPlugin(string id, int priority)
        {
            _priority = priority;
            Manifest = new PluginManifest
            {
                Id = id,
                Name = id,
                Version = "1.0.0",
                Author = "T",
                MinHostVersion = "1.0.0",
                PluginAssembly = "x.dll"
            };
            Capabilities = new SaveCapabilities();
        }
        public PluginManifest Manifest { get; }
        public SaveCapabilities Capabilities { get; }
        public (bool, int) CanOpen(FileInfo f, ReadOnlySpan<byte> h) => (true, _priority);
        public Task<ISaveDocument> OpenAsync(string p, CancellationToken ct) => throw new NotImplementedException();
    }
}
