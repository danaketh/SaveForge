using System.Text;
using SaveForge.Plugin.PlanetCrafter;
using SaveForge.SDK;
using Xunit;

namespace SaveForge.Tests;

public class PlanetCrafterPluginTests : IDisposable
{
    private readonly string _tempFile = Path.GetTempFileName() + ".json";

    [Fact]
    public void CanOpen_FileWithTerraTokensHeader_ReturnsTrue()
    {
        var header = Encoding.UTF8.GetBytes("{\"terraTokens\":100,\"allTimeTerraTokens\":200");
        var plugin = new PlanetCrafterPlugin();

        var (canOpen, priority) = plugin.CanOpen(new FileInfo(_tempFile), header);

        Assert.True(canOpen);
        Assert.Equal(20, priority);
    }

    [Fact]
    public void CanOpen_FileWithoutTerraTokens_ReturnsFalse()
    {
        var header = Encoding.UTF8.GetBytes("{\"player\":{\"name\":\"Farmer\"}}");
        var plugin = new PlanetCrafterPlugin();

        var (canOpen, _) = plugin.CanOpen(new FileInfo(_tempFile), header);

        Assert.False(canOpen);
    }

    [Fact]
    public void CanOpen_NonJsonExtension_ReturnsFalse()
    {
        var header = Encoding.UTF8.GetBytes("{\"terraTokens\":100");
        var plugin = new PlanetCrafterPlugin();

        var (canOpen, _) = plugin.CanOpen(new FileInfo(_tempFile + ".sav"), header);

        Assert.False(canOpen);
    }

    [Fact]
    public void Manifest_HasCorrectId()
    {
        var plugin = new PlanetCrafterPlugin();
        Assert.Equal("planet-crafter", plugin.Manifest.Id);
    }

    public void Dispose()
    {
        if (File.Exists(_tempFile))
        {
            File.Delete(_tempFile);
        }
    }
}
