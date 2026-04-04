using SaveForge.Plugin.PlanetCrafter.Parser;
using Xunit;

namespace SaveForge.Tests;

public class PlanetCrafterIntegrationTests
{
    private static readonly string SavePath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
        "AppData", "LocalLow", "MijuGames", "Planet Crafter", "Standard-1.json");

    private static bool SaveExists => File.Exists(SavePath);

    [SkippableFact]
    public void Parse_RealSaveFile_DoesNotThrow()
    {
        Skip.IfNot(SaveExists, "Planet Crafter Standard-1.json not present on this machine.");

        var content = File.ReadAllText(SavePath);
        var save = PlanetCrafterSaveParser.Parse(content);

        Assert.NotNull(save.TerraTokens);
        Assert.NotEmpty(save.Planet.PlanetId);
        Assert.NotEmpty(save.Players);
        Assert.NotEmpty(save.WorldObjects);
    }

    [SkippableFact]
    public void RoundTrip_RealSaveFile_PreservesGroupCounts()
    {
        Skip.IfNot(SaveExists, "Planet Crafter Standard-1.json not present on this machine.");

        var content = File.ReadAllText(SavePath);
        var original = PlanetCrafterSaveParser.Parse(content);

        var written = PlanetCrafterSaveWriter.Write(original);
        var reparsed = PlanetCrafterSaveParser.Parse(written);

        Assert.Equal(original.Players.Length, reparsed.Players.Length);
        Assert.Equal(original.WorldObjects.Count, reparsed.WorldObjects.Count);
        Assert.Equal(original.Inventories.Length, reparsed.Inventories.Length);
        Assert.Equal(original.Messages.Length, reparsed.Messages.Length);
        Assert.Equal(original.TerraTokens.TerraTokens, reparsed.TerraTokens.TerraTokens);
    }

    [SkippableFact]
    public void Parse_RealSaveFile_HostPlayerExists()
    {
        Skip.IfNot(SaveExists, "Planet Crafter Standard-1.json not present on this machine.");

        var content = File.ReadAllText(SavePath);
        var save = PlanetCrafterSaveParser.Parse(content);

        Assert.Contains(save.Players, p => p.Host);
    }
}
