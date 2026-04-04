using SaveForge.SamplePlugin.StardewLike;
using SaveForge.SDK;
using Xunit;

namespace SaveForge.Tests;

public class StardewPluginTests : IDisposable
{
    private readonly string _tempFile = Path.GetTempFileName() + ".json";

    private const string SampleSave = """
        {
          "player": {
            "name": "Farmer",
            "level": 5,
            "money": 1000,
            "stats": { "health": 100, "energy": 80 }
          },
          "inventory": [
            { "id": "wood", "count": 50 }
          ]
        }
        """;

    [Fact]
    public async Task OpenAsync_LoadsPlayerName()
    {
        await File.WriteAllTextAsync(_tempFile, SampleSave);
        var plugin = new StardewPlugin();
        var doc = await plugin.OpenAsync(_tempFile);

        Assert.Equal("Farmer", doc.GetValue("player.name"));
    }

    [Fact]
    public async Task SetValue_MakesDocumentDirty()
    {
        await File.WriteAllTextAsync(_tempFile, SampleSave);
        var plugin = new StardewPlugin();
        var doc = await plugin.OpenAsync(_tempFile);

        doc.SetValue("player.name", "NewFarmer");

        Assert.True(doc.IsDirty);
    }

    [Fact]
    public async Task SaveAsync_RoundTrip_PreservesChanges()
    {
        await File.WriteAllTextAsync(_tempFile, SampleSave);
        var plugin = new StardewPlugin();
        var doc = await plugin.OpenAsync(_tempFile);

        doc.SetValue("player.money", 9999);
        await doc.SaveAsync();

        var doc2 = await plugin.OpenAsync(_tempFile);
        var money = Convert.ToInt32(doc2.GetValue("player.money"));
        Assert.Equal(9999, money);
    }

    [Fact]
    public async Task CanOpen_JsonWithPlayerKey_ReturnsTrue()
    {
        await File.WriteAllTextAsync(_tempFile, SampleSave);
        var plugin = new StardewPlugin();
        var header = System.Text.Encoding.UTF8.GetBytes(SampleSave[..Math.Min(256, SampleSave.Length)]);

        var (canOpen, _) = plugin.CanOpen(new FileInfo(_tempFile), header);

        Assert.True(canOpen);
    }

    [Fact]
    public async Task CanOpen_NonJsonFile_ReturnsFalse()
    {
        var txtFile = _tempFile + ".txt";
        await File.WriteAllTextAsync(txtFile, "hello");
        var plugin = new StardewPlugin();
        var header = System.Text.Encoding.UTF8.GetBytes("hello");

        var (canOpen, _) = plugin.CanOpen(new FileInfo(txtFile), header);

        Assert.False(canOpen);
        if (File.Exists(txtFile))
        {
            File.Delete(txtFile);
        }
    }

    [Fact]
    public async Task GetEditorDefinition_ReturnsSchema()
    {
        await File.WriteAllTextAsync(_tempFile, SampleSave);
        var plugin = new StardewPlugin();
        var doc = await plugin.OpenAsync(_tempFile);
        var def = doc.GetEditorDefinition();

        Assert.Equal(EditorKind.Schema, def.Kind);
        Assert.NotNull(def.Schema);
    }

    public void Dispose()
    {
        if (File.Exists(_tempFile))
        {
            File.Delete(_tempFile);
        }
    }
}
