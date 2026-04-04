using System.Text.Json.Nodes;
using SaveForge.Plugin.PlanetCrafter.Models;
using SaveForge.Plugin.PlanetCrafter.Parser;
using Xunit;

namespace SaveForge.Tests;

public class PlanetCrafterParserTests
{
    // Minimal valid save with 2 players and 2 world objects
    private const string MinimalSave =
        "{\"terraTokens\":100,\"allTimeTerraTokens\":200,\"unlockedGroups\":\"A,B\",\"openedInstanceSeed\":0,\"openedInstanceTimeLeft\":0}@" +
        "{\"planetId\":\"Prime\",\"unitOxygenLevel\":1.0,\"unitHeatLevel\":2.0,\"unitPressureLevel\":3.0,\"unitPlantsLevel\":4.0,\"unitInsectsLevel\":5.0,\"unitAnimalsLevel\":6.0,\"unitPurificationLevel\":-1.0}@" +
        "{\"id\":111,\"name\":\"Alice\",\"inventoryId\":1,\"equipmentId\":2,\"playerPosition\":\"0,0,0\",\"playerRotation\":\"0,0,0,1\",\"playerGaugeOxygen\":100.0,\"playerGaugeThirst\":80.0,\"playerGaugeHealth\":90.0,\"playerGaugeToxic\":0.0,\"host\":true,\"planetId\":\"Prime\"}|\n" +
        "{\"id\":222,\"name\":\"Bob\",\"inventoryId\":3,\"equipmentId\":4,\"playerPosition\":\"1,1,1\",\"playerRotation\":\"0,0,0,1\",\"playerGaugeOxygen\":50.0,\"playerGaugeThirst\":60.0,\"playerGaugeHealth\":70.0,\"playerGaugeToxic\":0.0,\"host\":false,\"planetId\":\"Prime\"}@" +
        "{\"id\":10,\"gId\":\"Cobalt\"}|\n" +
        "{\"id\":11,\"gId\":\"Iron\",\"pos\":\"1,2,3\"}@" +
        "{\"id\":5,\"woIds\":\"10,11\",\"size\":4}@" +
        "{\"craftedObjects\":50,\"totalSaveFileLoad\":1,\"totalSaveFileTime\":3600}@" +
        "{\"stringId\":\"Msg1\",\"isRead\":true}|\n" +
        "{\"stringId\":\"Msg2\",\"isRead\":false}@" +
        "{\"stringId\":\"StoryEvent1\"}@" +
        "{\"saveDisplayName\":\"TestSave\",\"planetId\":\"Prime\",\"unlockedSpaceTrading\":false,\"unlockedOreExtrators\":false,\"unlockedTeleporters\":false}@" +
        "{\"layerId\":\"Layer1\",\"planet\":-1234,\"colorBase\":\"1-1-1-1\",\"colorCustom\":\"1-1-1-1\",\"colorBaseLerp\":0}@" +
        "{\"owner\":0,\"planet\":-1234,\"index\":1,\"seed\":99,\"pos\":\"0,0,0\",\"rot\":\"0,0,0,1\"}@";

    [Fact]
    public void Parse_TerraTokens_ReadsCorrectly()
    {
        var save = PlanetCrafterSaveParser.Parse(MinimalSave);

        Assert.Equal(100L, save.TerraTokens.TerraTokens);
        Assert.Equal(200L, save.TerraTokens.AllTimeTerraTokens);
        Assert.Equal("A,B", save.TerraTokens.UnlockedGroups);
    }

    [Fact]
    public void Parse_PlanetData_ReadsCorrectly()
    {
        var save = PlanetCrafterSaveParser.Parse(MinimalSave);

        Assert.Equal("Prime", save.Planet.PlanetId);
        Assert.Equal(1.0, save.Planet.UnitOxygenLevel);
        Assert.Equal(-1.0, save.Planet.UnitPurificationLevel);
    }

    [Fact]
    public void Parse_Players_ReadsAllPlayers()
    {
        var save = PlanetCrafterSaveParser.Parse(MinimalSave);

        Assert.Equal(2, save.Players.Length);
        Assert.Equal("Alice", save.Players[0].Name);
        Assert.Equal(111L, save.Players[0].Id);
        Assert.True(save.Players[0].Host);
        Assert.Equal("Bob", save.Players[1].Name);
        Assert.False(save.Players[1].Host);
    }

    [Fact]
    public void Parse_Players_ReadsGauges()
    {
        var save = PlanetCrafterSaveParser.Parse(MinimalSave);

        Assert.Equal(100.0, save.Players[0].PlayerGaugeOxygen);
        Assert.Equal(90.0, save.Players[0].PlayerGaugeHealth);
        Assert.Equal(70.0, save.Players[1].PlayerGaugeHealth);
    }

    [Fact]
    public void Parse_WorldObjects_ReadsIdAndGId()
    {
        var save = PlanetCrafterSaveParser.Parse(MinimalSave);

        Assert.Equal(2, save.WorldObjects.Count);
        Assert.Equal(10, save.WorldObjects[0].Id);
        Assert.Equal("Cobalt", save.WorldObjects[0].GId);
        Assert.Equal(11, save.WorldObjects[1].Id);
        Assert.Equal("Iron", save.WorldObjects[1].GId);
    }

    [Fact]
    public void Parse_WorldObjects_PreservesRawNode()
    {
        var save = PlanetCrafterSaveParser.Parse(MinimalSave);

        // Iron object has optional "pos" field
        var iron = save.WorldObjects[1];
        Assert.NotNull(iron.RawNode);
        Assert.Equal("1,2,3", iron.RawNode!["pos"]?.GetValue<string>());
    }

    [Fact]
    public void Parse_Inventories_ReadsCorrectly()
    {
        var save = PlanetCrafterSaveParser.Parse(MinimalSave);

        Assert.Single(save.Inventories);
        Assert.Equal(5, save.Inventories[0].Id);
        Assert.Equal("10,11", save.Inventories[0].WoIds);
        Assert.Equal(4, save.Inventories[0].Size);
    }

    [Fact]
    public void Parse_GlobalStats_ReadsCorrectly()
    {
        var save = PlanetCrafterSaveParser.Parse(MinimalSave);

        Assert.Equal(50, save.Stats.CraftedObjects);
        Assert.Equal(3600, save.Stats.TotalSaveFileTime);
    }

    [Fact]
    public void Parse_Messages_ReadsAllMessages()
    {
        var save = PlanetCrafterSaveParser.Parse(MinimalSave);

        Assert.Equal(2, save.Messages.Length);
        Assert.Equal("Msg1", save.Messages[0].StringId);
        Assert.True(save.Messages[0].IsRead);
        Assert.Equal("Msg2", save.Messages[1].StringId);
        Assert.False(save.Messages[1].IsRead);
    }

    [Fact]
    public void Parse_StoryEvents_ReadsAll()
    {
        var save = PlanetCrafterSaveParser.Parse(MinimalSave);

        Assert.Single(save.StoryEvents);
        Assert.Equal("StoryEvent1", save.StoryEvents[0].StringId);
    }

    [Fact]
    public void Parse_DisplayInfo_ReadsCorrectly()
    {
        var save = PlanetCrafterSaveParser.Parse(MinimalSave);

        Assert.Equal("TestSave", save.DisplayInfo.SaveDisplayName);
        Assert.Equal("Prime", save.DisplayInfo.PlanetId);
    }

    [Fact]
    public void Parse_TerrainLayers_ReadsCorrectly()
    {
        var save = PlanetCrafterSaveParser.Parse(MinimalSave);

        Assert.Single(save.TerrainLayers);
        Assert.Equal("Layer1", save.TerrainLayers[0].LayerId);
    }

    [Fact]
    public void Parse_PlanetInstances_ReadsCorrectly()
    {
        var save = PlanetCrafterSaveParser.Parse(MinimalSave);

        Assert.Single(save.PlanetInstances);
        Assert.Equal(99, save.PlanetInstances[0].Seed);
    }

    [Fact]
    public void Parse_ThrowsOnNull()
    {
        Assert.Throws<ArgumentNullException>(() => PlanetCrafterSaveParser.Parse(null!));
    }

    [Fact]
    public void Parse_ThrowsOnTooFewGroups()
    {
        Assert.Throws<InvalidDataException>(() => PlanetCrafterSaveParser.Parse("{}@{}"));
    }
}

public class PlanetCrafterWriterTests
{
    private const string MinimalSave =
        "{\"terraTokens\":100,\"allTimeTerraTokens\":200,\"unlockedGroups\":\"A,B\",\"openedInstanceSeed\":0,\"openedInstanceTimeLeft\":0}@" +
        "{\"planetId\":\"Prime\",\"unitOxygenLevel\":1.0,\"unitHeatLevel\":2.0,\"unitPressureLevel\":3.0,\"unitPlantsLevel\":4.0,\"unitInsectsLevel\":5.0,\"unitAnimalsLevel\":6.0,\"unitPurificationLevel\":-1.0}@" +
        "{\"id\":111,\"name\":\"Alice\",\"inventoryId\":1,\"equipmentId\":2,\"playerPosition\":\"0,0,0\",\"playerRotation\":\"0,0,0,1\",\"playerGaugeOxygen\":100.0,\"playerGaugeThirst\":80.0,\"playerGaugeHealth\":90.0,\"playerGaugeToxic\":0.0,\"host\":true,\"planetId\":\"Prime\"}|\n" +
        "{\"id\":222,\"name\":\"Bob\",\"inventoryId\":3,\"equipmentId\":4,\"playerPosition\":\"1,1,1\",\"playerRotation\":\"0,0,0,1\",\"playerGaugeOxygen\":50.0,\"playerGaugeThirst\":60.0,\"playerGaugeHealth\":70.0,\"playerGaugeToxic\":0.0,\"host\":false,\"planetId\":\"Prime\"}@" +
        "{\"id\":10,\"gId\":\"Cobalt\"}|\n" +
        "{\"id\":11,\"gId\":\"Iron\",\"pos\":\"1,2,3\"}@" +
        "{\"id\":5,\"woIds\":\"10,11\",\"size\":4}@" +
        "{\"craftedObjects\":50,\"totalSaveFileLoad\":1,\"totalSaveFileTime\":3600}@" +
        "{\"stringId\":\"Msg1\",\"isRead\":true}|\n" +
        "{\"stringId\":\"Msg2\",\"isRead\":false}@" +
        "{\"stringId\":\"StoryEvent1\"}@" +
        "{\"saveDisplayName\":\"TestSave\",\"planetId\":\"Prime\",\"unlockedSpaceTrading\":false,\"unlockedOreExtrators\":false,\"unlockedTeleporters\":false}@" +
        "{\"layerId\":\"Layer1\",\"planet\":-1234,\"colorBase\":\"1-1-1-1\",\"colorCustom\":\"1-1-1-1\",\"colorBaseLerp\":0}@" +
        "{\"owner\":0,\"planet\":-1234,\"index\":1,\"seed\":99,\"pos\":\"0,0,0\",\"rot\":\"0,0,0,1\"}@";

    [Fact]
    public void Write_ThenParse_TerraTokens_RoundTrips()
    {
        var original = PlanetCrafterSaveParser.Parse(MinimalSave);
        original.TerraTokens.TerraTokens = 9999;

        var written = PlanetCrafterSaveWriter.Write(original);
        var reparsed = PlanetCrafterSaveParser.Parse(written);

        Assert.Equal(9999L, reparsed.TerraTokens.TerraTokens);
        Assert.Equal("A,B", reparsed.TerraTokens.UnlockedGroups);
    }

    [Fact]
    public void Write_ThenParse_Players_RoundTrips()
    {
        var original = PlanetCrafterSaveParser.Parse(MinimalSave);
        original.Players[0].PlayerGaugeHealth = 42.5;

        var written = PlanetCrafterSaveWriter.Write(original);
        var reparsed = PlanetCrafterSaveParser.Parse(written);

        Assert.Equal(2, reparsed.Players.Length);
        Assert.Equal(42.5, reparsed.Players[0].PlayerGaugeHealth, precision: 5);
        Assert.Equal("Alice", reparsed.Players[0].Name);
    }

    [Fact]
    public void Write_ThenParse_WorldObjects_PreservesRawFields()
    {
        var original = PlanetCrafterSaveParser.Parse(MinimalSave);

        var written = PlanetCrafterSaveWriter.Write(original);
        var reparsed = PlanetCrafterSaveParser.Parse(written);

        Assert.Equal(2, reparsed.WorldObjects.Count);
        Assert.Equal("1,2,3", reparsed.WorldObjects[1].RawNode!["pos"]?.GetValue<string>());
    }

    [Fact]
    public void Write_ProducesCorrectGroupCount()
    {
        var original = PlanetCrafterSaveParser.Parse(MinimalSave);
        var written = PlanetCrafterSaveWriter.Write(original);

        // 11 data groups + 1 trailing empty = 12 groups when split by @
        Assert.Equal(12, written.Split('@').Length);
    }

    [Fact]
    public void Write_ObjectsWithinGroupSeparatedByPipeNewline()
    {
        var original = PlanetCrafterSaveParser.Parse(MinimalSave);
        var written = PlanetCrafterSaveWriter.Write(original);

        // Players group (index 2): two players separated by "|\n"
        var groups = written.Split('@');
        var playersGroup = groups[2];
        Assert.Contains("|\n", playersGroup);
        // Last player entry must not end with |
        var lastPlayer = playersGroup.Split("|\n").Last().Trim();
        Assert.DoesNotMatch(@"\|$", lastPlayer);
    }
}
