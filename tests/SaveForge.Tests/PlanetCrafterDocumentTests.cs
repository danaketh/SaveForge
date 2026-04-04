using SaveForge.Plugin.PlanetCrafter;
using SaveForge.SDK;
using Xunit;

namespace SaveForge.Tests;

public class PlanetCrafterDocumentTests : IDisposable
{
    private readonly string _tempFile = Path.GetTempFileName() + ".json";

    private const string MinimalSave =
        "{\"terraTokens\":100,\"allTimeTerraTokens\":200,\"unlockedGroups\":\"A,B\",\"openedInstanceSeed\":0,\"openedInstanceTimeLeft\":0}@" +
        "{\"planetId\":\"Prime\",\"unitOxygenLevel\":1.0,\"unitHeatLevel\":2.0,\"unitPressureLevel\":3.0,\"unitPlantsLevel\":4.0,\"unitInsectsLevel\":5.0,\"unitAnimalsLevel\":6.0,\"unitPurificationLevel\":-1.0}@" +
        "{\"id\":111,\"name\":\"Alice\",\"inventoryId\":1,\"equipmentId\":2,\"playerPosition\":\"0,0,0\",\"playerRotation\":\"0,0,0,1\",\"playerGaugeOxygen\":100.0,\"playerGaugeThirst\":80.0,\"playerGaugeHealth\":90.0,\"playerGaugeToxic\":0.0,\"host\":true,\"planetId\":\"Prime\"}|\n" +
        "{\"id\":222,\"name\":\"Bob\",\"inventoryId\":3,\"equipmentId\":4,\"playerPosition\":\"1,1,1\",\"playerRotation\":\"0,0,0,1\",\"playerGaugeOxygen\":50.0,\"playerGaugeThirst\":60.0,\"playerGaugeHealth\":70.0,\"playerGaugeToxic\":0.0,\"host\":false,\"planetId\":\"Prime\"}@" +
        "{\"id\":10,\"gId\":\"Cobalt\"}@" +
        "{\"id\":5,\"woIds\":\"10\",\"size\":4}@" +
        "{\"craftedObjects\":50,\"totalSaveFileLoad\":1,\"totalSaveFileTime\":3600}@" +
        "{\"stringId\":\"Msg1\",\"isRead\":true}@" +
        "{\"stringId\":\"StoryEvent1\"}@" +
        "{\"saveDisplayName\":\"TestSave\",\"planetId\":\"Prime\",\"unlockedSpaceTrading\":false,\"unlockedOreExtrators\":false,\"unlockedTeleporters\":false}@" +
        "{\"layerId\":\"Layer1\",\"planet\":-1234,\"colorBase\":\"1-1-1-1\",\"colorCustom\":\"1-1-1-1\",\"colorBaseLerp\":0,\"colorCustomLerp\":0}@" +
        "{\"owner\":0,\"planet\":-1234,\"index\":1,\"seed\":99,\"pos\":\"0,0,0\",\"rot\":\"0,0,0,1\"}@";

    private async Task<PlanetCrafterDocument> OpenDocumentAsync()
    {
        await File.WriteAllTextAsync(_tempFile, MinimalSave);
        var plugin = new PlanetCrafterPlugin();
        return (PlanetCrafterDocument)await plugin.OpenAsync(_tempFile);
    }

    [Fact]
    public async Task GetValue_TerraTokens_ReturnsCorrectValue()
    {
        var doc = await OpenDocumentAsync();
        Assert.Equal(100L, doc.GetValue("tokens.terraTokens"));
    }

    [Fact]
    public async Task GetValue_PlanetId_ReturnsCorrectValue()
    {
        var doc = await OpenDocumentAsync();
        Assert.Equal("Prime", doc.GetValue("planet.planetId"));
    }

    [Fact]
    public async Task GetValue_PlayerName_ReturnsCorrectValue()
    {
        var doc = await OpenDocumentAsync();
        Assert.Equal("Alice", doc.GetValue("players[0].name"));
    }

    [Fact]
    public async Task GetValue_PlayerGaugeHealth_ReturnsCorrectValue()
    {
        var doc = await OpenDocumentAsync();
        Assert.Equal(90.0, doc.GetValue("players[0].playerGaugeHealth"));
    }

    [Fact]
    public async Task SetValue_TerraTokens_MakesDirty()
    {
        var doc = await OpenDocumentAsync();
        doc.SetValue("tokens.terraTokens", 9999L);

        Assert.True(doc.IsDirty);
        Assert.Equal(9999L, doc.GetValue("tokens.terraTokens"));
    }

    [Fact]
    public async Task SetValue_PlayerHealth_UpdatesCorrectPlayer()
    {
        var doc = await OpenDocumentAsync();
        doc.SetValue("players[1].playerGaugeHealth", 42.0);

        Assert.Equal(42.0, doc.GetValue("players[1].playerGaugeHealth"));
    }

    [Fact]
    public async Task SaveAsync_RoundTrip_PreservesChanges()
    {
        var doc = await OpenDocumentAsync();
        doc.SetValue("tokens.terraTokens", 5555L);
        await doc.SaveAsync();
        Assert.False(doc.IsDirty);

        var plugin = new PlanetCrafterPlugin();
        var doc2 = (PlanetCrafterDocument)await plugin.OpenAsync(_tempFile);
        Assert.Equal(5555L, doc2.GetValue("tokens.terraTokens"));
    }

    [Fact]
    public async Task GetEditorDefinition_ReturnsCustomKind()
    {
        var doc = await OpenDocumentAsync();
        var def = doc.GetEditorDefinition();
        Assert.Equal(EditorKind.Custom, def.Kind);
        Assert.NotNull(def.ViewFactory);
    }

    public void Dispose()
    {
        if (File.Exists(_tempFile))
        {
            File.Delete(_tempFile);
        }
    }
}
