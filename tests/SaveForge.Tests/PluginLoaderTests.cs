using System.Text.Json;
using Microsoft.Extensions.Logging.Abstractions;
using SaveForge.Core.Models;
using SaveForge.PluginHost;
using Xunit;

namespace SaveForge.Tests;

public class PluginLoaderTests : IDisposable
{
    private readonly string _tempDir = Path.Combine(Path.GetTempPath(), "gse-loader-" + Guid.NewGuid());

    public PluginLoaderTests() => Directory.CreateDirectory(_tempDir);

    [Fact]
    public void Load_MissingManifest_ReturnsFailed()
    {
        var loader = new PluginLoader(NullLogger<PluginLoader>.Instance, "1.0.0");
        var (result, _) = loader.Load(_tempDir);

        Assert.IsType<PluginLoadResult.Failed>(result);
    }

    [Fact]
    public void Load_IncompatibleMinVersion_ReturnsIncompatible()
    {
        WriteManifest(new
        {
            id = "x",
            name = "X",
            version = "1.0.0",
            author = "A",
            minHostVersion = "99.0.0",
            pluginAssembly = "x.dll"
        });

        var loader = new PluginLoader(NullLogger<PluginLoader>.Instance, "1.0.0");
        var (result, _) = loader.Load(_tempDir);

        Assert.IsType<PluginLoadResult.Incompatible>(result);
    }

    [Fact]
    public void Load_IncompatibleMaxVersion_ReturnsIncompatible()
    {
        WriteManifest(new
        {
            id = "x",
            name = "X",
            version = "1.0.0",
            author = "A",
            minHostVersion = "1.0.0",
            maxHostVersion = "1.0.0",
            pluginAssembly = "x.dll"
        });

        var loader = new PluginLoader(NullLogger<PluginLoader>.Instance, "2.0.0");
        var (result, _) = loader.Load(_tempDir);

        Assert.IsType<PluginLoadResult.Incompatible>(result);
    }

    [Fact]
    public void Load_MissingAssembly_ReturnsFailed()
    {
        WriteManifest(new
        {
            id = "x",
            name = "X",
            version = "1.0.0",
            author = "A",
            minHostVersion = "1.0.0",
            pluginAssembly = "missing.dll"
        });

        var loader = new PluginLoader(NullLogger<PluginLoader>.Instance, "1.0.0");
        var (result, _) = loader.Load(_tempDir);

        Assert.IsType<PluginLoadResult.Failed>(result);
    }

    private void WriteManifest(object manifest)
    {
        var json = JsonSerializer.Serialize(manifest);
        File.WriteAllText(Path.Combine(_tempDir, "manifest.json"), json);
    }

    public void Dispose() => Directory.Delete(_tempDir, recursive: true);
}
