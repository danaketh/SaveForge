using System.Text.Json;
using SaveForge.SDK;
using Xunit;

namespace SaveForge.Tests;

public class ManifestParsingTests
{
    private static readonly JsonSerializerOptions Options = new() { PropertyNameCaseInsensitive = true };

    [Fact]
    public void Deserialize_ValidJson_ReturnsManifest()
    {
        var json = """
            {
              "id": "stardew-like",
              "name": "StardewLike Save Editor",
              "version": "1.0.0",
              "author": "Team",
              "minHostVersion": "1.0.0",
              "pluginAssembly": "StardewLike.dll"
            }
            """;

        var manifest = JsonSerializer.Deserialize<PluginManifest>(json, Options)!;

        Assert.Equal("stardew-like", manifest.Id);
        Assert.Equal("1.0.0", manifest.Version);
        Assert.Null(manifest.MaxHostVersion);
    }

    [Fact]
    public void Deserialize_WithMaxVersion_ParsesCorrectly()
    {
        var json = """
            {
              "id": "x", "name": "X", "version": "2.0.0",
              "author": "A", "minHostVersion": "1.0.0",
              "maxHostVersion": "3.0.0",
              "pluginAssembly": "x.dll"
            }
            """;

        var manifest = JsonSerializer.Deserialize<PluginManifest>(json, Options)!;

        Assert.Equal("3.0.0", manifest.MaxHostVersion);
    }

    [Fact]
    public void Deserialize_MissingOptionalDescription_DefaultsToEmpty()
    {
        var json = """
            {
              "id": "x", "name": "X", "version": "1.0.0",
              "author": "A", "minHostVersion": "1.0.0",
              "pluginAssembly": "x.dll"
            }
            """;

        var manifest = JsonSerializer.Deserialize<PluginManifest>(json, Options)!;

        Assert.Equal(string.Empty, manifest.Description);
    }
}
