# Planet Crafter Plugin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a SaveForge plugin for Planet Crafter saves that parses the game's custom multi-group JSON format and exposes key fields for editing (terra tokens, planet state, player gauges, messages).

**Architecture:** The plugin sits in a new project `SaveForge.Plugin.PlanetCrafter` alongside the existing `SaveForge.SamplePlugin.StardewLike`. It follows the same three-file pattern: plugin entry point, document handle, and schema builder. The custom save format (groups separated by `@`, objects within a group separated by `|`) is handled by a dedicated parser/writer pair that converts between the raw format and strongly-typed C# models. The document then wraps those models and exposes them through the SDK's dot-path `GetValue`/`SetValue` API.

**Tech Stack:** .NET 8, `System.Text.Json` (built-in), xUnit (existing test project), SaveForge.SDK

---

## File Format Reference

The Planet Crafter save format (e.g. `Standard-1.json`) is **not** standard JSON. It is a sequence of 12 groups separated by `@`. Within each group, individual JSON objects are separated by `|` (plus a newline). The last object in each group has no trailing `|`.

Parsing algorithm:
1. `content.Split('@')` → 12 group strings (last is empty/trailing)
2. For each group: `group.Split('|').Select(s => s.Trim()).Where(s => s.Length > 0)`
3. Each item is a valid `System.Text.Json`-parseable JSON object

### Group Index Map (from `Standard-1.json` analysis)

| Index | Count | Type | Key Fields |
|-------|-------|------|-----------|
| 0 | 1 | TerraTokensData | `terraTokens`, `allTimeTerraTokens`, `unlockedGroups` |
| 1 | 1 | PlanetData | `planetId`, `unitOxygenLevel`, `unitHeatLevel`, `unitPressureLevel`, `unitPlantsLevel`, `unitInsectsLevel`, `unitAnimalsLevel`, `unitPurificationLevel` |
| 2 | N | PlayerData[] | `id`, `name`, `inventoryId`, `equipmentId`, `playerPosition`, `playerRotation`, `playerGaugeOxygen`, `playerGaugeThirst`, `playerGaugeHealth`, `playerGaugeToxic`, `host`, `planetId` |
| 3 | N | WorldObject[] | `id`, `gId`, `pos`, `rot`, `planet`, + many optional fields |
| 4 | N | InventoryData[] | `id`, `woIds`, `size` |
| 5 | 1 | GlobalStats | `craftedObjects`, `totalSaveFileLoad`, `totalSaveFileTime` |
| 6 | N | MessageData[] | `stringId`, `isRead` |
| 7 | N | StoryEventData[] | `stringId` |
| 8 | 1 | SaveDisplayInfo | `saveDisplayName`, `planetId`, `unlockedSpaceTrading`, `unlockedOreExtrators`, `unlockedTeleporters` |
| 9 | N | TerrainLayer[] | `layerId`, `planet`, `colorBase`, `colorCustom`, `colorBaseLerp` |
| 10 | N | PlanetInstance[] | `owner`, `planet`, `index`, `seed`, `pos`, `rot` |
| 11 | 0 | (empty trailing) | — |

---

## File Map

### New project: `src/SaveForge.Plugin.PlanetCrafter/`

| File | Responsibility |
|------|---------------|
| `SaveForge.Plugin.PlanetCrafter.csproj` | Project file, references SDK |
| `manifest.json` | Plugin metadata (id, version, assembly name) |
| `PlanetCrafterPlugin.cs` | `IGameEditorPlugin` — detects `.json` files with the `@`-group format |
| `PlanetCrafterDocument.cs` | `ISaveDocument` — wraps `PlanetCrafterSave`, handles dot-path routing |
| `PlanetCrafterSchema.cs` | Builds `FormSchema` for all editable fields |
| `Parser/PlanetCrafterSaveParser.cs` | `Parse(string) → PlanetCrafterSave` |
| `Parser/PlanetCrafterSaveWriter.cs` | `Write(PlanetCrafterSave) → string` |
| `Models/PlanetCrafterSave.cs` | Top-level aggregate; holds all group arrays |
| `Models/TerraTokensData.cs` | Group 0 model |
| `Models/PlanetData.cs` | Group 1 model |
| `Models/PlayerData.cs` | Group 2 model (one per player) |
| `Models/WorldObject.cs` | Group 3 model (raw JsonObject passthrough — too varied to fully type) |
| `Models/InventoryData.cs` | Group 4 model |
| `Models/GlobalStats.cs` | Group 5 model |
| `Models/MessageData.cs` | Group 6 model |
| `Models/StoryEventData.cs` | Group 7 model |
| `Models/SaveDisplayInfo.cs` | Group 8 model |
| `Models/TerrainLayer.cs` | Group 9 model |
| `Models/PlanetInstance.cs` | Group 10 model |

### Modified: `tests/SaveForge.Tests/`

| File | Responsibility |
|------|---------------|
| `PlanetCrafterParserTests.cs` | Unit tests for parser and writer |
| `PlanetCrafterDocumentTests.cs` | Tests for ISaveDocument dot-path API |
| `PlanetCrafterPluginTests.cs` | Tests for CanOpen detection logic |
| `SaveForge.Tests.csproj` | Add ProjectReference to new plugin project |

### Modified: `SaveForge.sln`
Add new plugin project to solution.

---

## Task 1: Project scaffold + model classes

**Files:**
- Create: `src/SaveForge.Plugin.PlanetCrafter/SaveForge.Plugin.PlanetCrafter.csproj`
- Create: `src/SaveForge.Plugin.PlanetCrafter/manifest.json`
- Create: `src/SaveForge.Plugin.PlanetCrafter/Models/TerraTokensData.cs`
- Create: `src/SaveForge.Plugin.PlanetCrafter/Models/PlanetData.cs`
- Create: `src/SaveForge.Plugin.PlanetCrafter/Models/PlayerData.cs`
- Create: `src/SaveForge.Plugin.PlanetCrafter/Models/WorldObject.cs`
- Create: `src/SaveForge.Plugin.PlanetCrafter/Models/InventoryData.cs`
- Create: `src/SaveForge.Plugin.PlanetCrafter/Models/GlobalStats.cs`
- Create: `src/SaveForge.Plugin.PlanetCrafter/Models/MessageData.cs`
- Create: `src/SaveForge.Plugin.PlanetCrafter/Models/StoryEventData.cs`
- Create: `src/SaveForge.Plugin.PlanetCrafter/Models/SaveDisplayInfo.cs`
- Create: `src/SaveForge.Plugin.PlanetCrafter/Models/TerrainLayer.cs`
- Create: `src/SaveForge.Plugin.PlanetCrafter/Models/PlanetInstance.cs`
- Create: `src/SaveForge.Plugin.PlanetCrafter/Models/PlanetCrafterSave.cs`

- [ ] **Step 1: Create the project file**

`src/SaveForge.Plugin.PlanetCrafter/SaveForge.Plugin.PlanetCrafter.csproj`:
```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
    <LangVersion>latest</LangVersion>
    <RootNamespace>SaveForge.Plugin.PlanetCrafter</RootNamespace>
  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="..\SaveForge.SDK\SaveForge.SDK.csproj" />
  </ItemGroup>
</Project>
```

- [ ] **Step 2: Create manifest.json**

`src/SaveForge.Plugin.PlanetCrafter/manifest.json`:
```json
{
  "id": "planet-crafter",
  "name": "Planet Crafter Save Editor",
  "description": "Edit saves for Planet Crafter.",
  "author": "SaveForge Team",
  "version": "1.0.0",
  "minHostVersion": "1.0.0",
  "maxHostVersion": null,
  "pluginAssembly": "SaveForge.Plugin.PlanetCrafter.dll"
}
```

- [ ] **Step 3: Create model classes**

`src/SaveForge.Plugin.PlanetCrafter/Models/TerraTokensData.cs`:
```csharp
using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class TerraTokensData
{
    [JsonPropertyName("terraTokens")]
    public long TerraTokens { get; set; }

    [JsonPropertyName("allTimeTerraTokens")]
    public long AllTimeTerraTokens { get; set; }

    [JsonPropertyName("unlockedGroups")]
    public string UnlockedGroups { get; set; } = "";

    [JsonPropertyName("openedInstanceSeed")]
    public int OpenedInstanceSeed { get; set; }

    [JsonPropertyName("openedInstanceTimeLeft")]
    public double OpenedInstanceTimeLeft { get; set; }
}
```

`src/SaveForge.Plugin.PlanetCrafter/Models/PlanetData.cs`:
```csharp
using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class PlanetData
{
    [JsonPropertyName("planetId")]
    public string PlanetId { get; set; } = "";

    [JsonPropertyName("unitOxygenLevel")]
    public double UnitOxygenLevel { get; set; }

    [JsonPropertyName("unitHeatLevel")]
    public double UnitHeatLevel { get; set; }

    [JsonPropertyName("unitPressureLevel")]
    public double UnitPressureLevel { get; set; }

    [JsonPropertyName("unitPlantsLevel")]
    public double UnitPlantsLevel { get; set; }

    [JsonPropertyName("unitInsectsLevel")]
    public double UnitInsectsLevel { get; set; }

    [JsonPropertyName("unitAnimalsLevel")]
    public double UnitAnimalsLevel { get; set; }

    [JsonPropertyName("unitPurificationLevel")]
    public double UnitPurificationLevel { get; set; }
}
```

`src/SaveForge.Plugin.PlanetCrafter/Models/PlayerData.cs`:
```csharp
using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class PlayerData
{
    [JsonPropertyName("id")]
    public long Id { get; set; }

    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("inventoryId")]
    public int InventoryId { get; set; }

    [JsonPropertyName("equipmentId")]
    public int EquipmentId { get; set; }

    [JsonPropertyName("playerPosition")]
    public string PlayerPosition { get; set; } = "";

    [JsonPropertyName("playerRotation")]
    public string PlayerRotation { get; set; } = "";

    [JsonPropertyName("playerGaugeOxygen")]
    public double PlayerGaugeOxygen { get; set; }

    [JsonPropertyName("playerGaugeThirst")]
    public double PlayerGaugeThirst { get; set; }

    [JsonPropertyName("playerGaugeHealth")]
    public double PlayerGaugeHealth { get; set; }

    [JsonPropertyName("playerGaugeToxic")]
    public double PlayerGaugeToxic { get; set; }

    [JsonPropertyName("host")]
    public bool Host { get; set; }

    [JsonPropertyName("planetId")]
    public string PlanetId { get; set; } = "";
}
```

`src/SaveForge.Plugin.PlanetCrafter/Models/WorldObject.cs`:
```csharp
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

/// <summary>
/// World objects are highly varied (thousands of different gId types with different fields).
/// We store the raw JsonObject to preserve all fields and avoid data loss.
/// </summary>
public sealed class WorldObject
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("gId")]
    public string GId { get; set; } = "";

    /// <summary>Raw JSON node for all additional optional fields (pos, rot, planet, text, etc.).</summary>
    [JsonIgnore]
    public JsonObject? RawNode { get; set; }
}
```

`src/SaveForge.Plugin.PlanetCrafter/Models/InventoryData.cs`:
```csharp
using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class InventoryData
{
    [JsonPropertyName("id")]
    public int Id { get; set; }

    [JsonPropertyName("woIds")]
    public string WoIds { get; set; } = "";

    [JsonPropertyName("size")]
    public int Size { get; set; }
}
```

`src/SaveForge.Plugin.PlanetCrafter/Models/GlobalStats.cs`:
```csharp
using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class GlobalStats
{
    [JsonPropertyName("craftedObjects")]
    public int CraftedObjects { get; set; }

    [JsonPropertyName("totalSaveFileLoad")]
    public int TotalSaveFileLoad { get; set; }

    [JsonPropertyName("totalSaveFileTime")]
    public int TotalSaveFileTime { get; set; }
}
```

`src/SaveForge.Plugin.PlanetCrafter/Models/MessageData.cs`:
```csharp
using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class MessageData
{
    [JsonPropertyName("stringId")]
    public string StringId { get; set; } = "";

    [JsonPropertyName("isRead")]
    public bool IsRead { get; set; }
}
```

`src/SaveForge.Plugin.PlanetCrafter/Models/StoryEventData.cs`:
```csharp
using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class StoryEventData
{
    [JsonPropertyName("stringId")]
    public string StringId { get; set; } = "";
}
```

`src/SaveForge.Plugin.PlanetCrafter/Models/SaveDisplayInfo.cs`:
```csharp
using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class SaveDisplayInfo
{
    [JsonPropertyName("saveDisplayName")]
    public string SaveDisplayName { get; set; } = "";

    [JsonPropertyName("planetId")]
    public string PlanetId { get; set; } = "";

    [JsonPropertyName("unlockedSpaceTrading")]
    public bool UnlockedSpaceTrading { get; set; }

    [JsonPropertyName("unlockedOreExtrators")]
    public bool UnlockedOreExtractors { get; set; }

    [JsonPropertyName("unlockedTeleporters")]
    public bool UnlockedTeleporters { get; set; }
}
```

`src/SaveForge.Plugin.PlanetCrafter/Models/TerrainLayer.cs`:
```csharp
using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class TerrainLayer
{
    [JsonPropertyName("layerId")]
    public string LayerId { get; set; } = "";

    [JsonPropertyName("planet")]
    public long Planet { get; set; }

    [JsonPropertyName("colorBase")]
    public string ColorBase { get; set; } = "";

    [JsonPropertyName("colorCustom")]
    public string ColorCustom { get; set; } = "";

    [JsonPropertyName("colorBaseLerp")]
    public string ColorBaseLerp { get; set; } = "";
}
```

`src/SaveForge.Plugin.PlanetCrafter/Models/PlanetInstance.cs`:
```csharp
using System.Text.Json.Serialization;

namespace SaveForge.Plugin.PlanetCrafter.Models;

public sealed class PlanetInstance
{
    [JsonPropertyName("owner")]
    public int Owner { get; set; }

    [JsonPropertyName("planet")]
    public long Planet { get; set; }

    [JsonPropertyName("index")]
    public int Index { get; set; }

    [JsonPropertyName("seed")]
    public int Seed { get; set; }

    [JsonPropertyName("pos")]
    public string Pos { get; set; } = "";

    [JsonPropertyName("rot")]
    public string Rot { get; set; } = "";
}
```

`src/SaveForge.Plugin.PlanetCrafter/Models/PlanetCrafterSave.cs`:
```csharp
namespace SaveForge.Plugin.PlanetCrafter.Models;

/// <summary>Top-level aggregate model for a Planet Crafter save file.</summary>
public sealed class PlanetCrafterSave
{
    public required TerraTokensData TerraTokens { get; init; }
    public required PlanetData Planet { get; init; }
    public required PlayerData[] Players { get; init; }
    public required WorldObject[] WorldObjects { get; init; }
    public required InventoryData[] Inventories { get; init; }
    public required GlobalStats Stats { get; init; }
    public required MessageData[] Messages { get; init; }
    public required StoryEventData[] StoryEvents { get; init; }
    public required SaveDisplayInfo DisplayInfo { get; init; }
    public required TerrainLayer[] TerrainLayers { get; init; }
    public required PlanetInstance[] PlanetInstances { get; init; }
}
```

- [ ] **Step 4: Add to solution**

```bash
cd /I/Projects/danaketh/SaveForge
dotnet sln SaveForge.sln add src/SaveForge.Plugin.PlanetCrafter/SaveForge.Plugin.PlanetCrafter.csproj
```

Expected output: `Project ... added to the solution.`

- [ ] **Step 5: Verify project builds (models only)**

```bash
cd /I/Projects/danaketh/SaveForge
dotnet build src/SaveForge.Plugin.PlanetCrafter/SaveForge.Plugin.PlanetCrafter.csproj 2>&1 | tail -5
```

Expected: `Build succeeded.`

- [ ] **Step 6: Commit**

```bash
cd /I/Projects/danaketh/SaveForge
git add src/SaveForge.Plugin.PlanetCrafter/ SaveForge.sln
git commit -m "feat(planet-crafter): scaffold project with model classes for all 11 save groups"
```

---

## Task 2: Parser — parse raw save content into typed models

**Files:**
- Create: `src/SaveForge.Plugin.PlanetCrafter/Parser/PlanetCrafterSaveParser.cs`
- Modify: `tests/SaveForge.Tests/SaveForge.Tests.csproj` (add ProjectReference)
- Create: `tests/SaveForge.Tests/PlanetCrafterParserTests.cs`

The parser must handle the custom format: `content.Split('@')` gives groups; each group split by `'|'` then trimmed/filtered gives JSON strings.

**WorldObject handling:** Group 3 objects are too varied to deserialize field-by-field (they have dozens of optional keys). The parser will parse each as a `JsonObject`, extract `id` and `gId`, and store the full `JsonObject` as `RawNode` for lossless round-trip.

- [ ] **Step 1: Write the failing tests**

`tests/SaveForge.Tests/PlanetCrafterParserTests.cs`:
```csharp
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
        "{\"id\":111,\"name\":\"Alice\",\"inventoryId\":1,\"equipmentId\":2,\"playerPosition\":\"0,0,0\",\"playerRotation\":\"0,0,0,1\",\"playerGaugeOxygen\":100.0,\"playerGaugeThirst\":80.0,\"playerGaugeHealth\":90.0,\"playerGaugeToxic\":0.0,\"host\":true,\"planetId\":\"Prime\"}|" + "\n" +
        "{\"id\":222,\"name\":\"Bob\",\"inventoryId\":3,\"equipmentId\":4,\"playerPosition\":\"1,1,1\",\"playerRotation\":\"0,0,0,1\",\"playerGaugeOxygen\":50.0,\"playerGaugeThirst\":60.0,\"playerGaugeHealth\":70.0,\"playerGaugeToxic\":0.0,\"host\":false,\"planetId\":\"Prime\"}@" +
        "{\"id\":10,\"gId\":\"Cobalt\"}|" + "\n" +
        "{\"id\":11,\"gId\":\"Iron\",\"pos\":\"1,2,3\"}@" +
        "{\"id\":5,\"woIds\":\"10,11\",\"size\":4}@" +
        "{\"craftedObjects\":50,\"totalSaveFileLoad\":1,\"totalSaveFileTime\":3600}@" +
        "{\"stringId\":\"Msg1\",\"isRead\":true}|" + "\n" +
        "{\"stringId\":\"Msg2\",\"isRead\":false}@" +
        "{\"stringId\":\"StoryEvent1\"}@" +
        "{\"saveDisplayName\":\"TestSave\",\"planetId\":\"Prime\",\"unlockedSpaceTrading\":false,\"unlockedOreExtrators\":false,\"unlockedTeleporters\":false}@" +
        "{\"layerId\":\"Layer1\",\"planet\":-1234,\"colorBase\":\"1-1-1-1\",\"colorCustom\":\"1-1-1-1\",\"colorBaseLerp\":\"0-0-0-0\"}@" +
        "{\"owner\":0,\"planet\":-1234,\"index\":1,\"seed\":99,\"pos\":\"0,0,0\",\"rot\":\"0,0,0,1\"}@";

    [Fact]
    public void Parse_TerraTokens_ReadsCorrectly()
    {
        var save = PlanetCrafterSaveParser.Parse(MinimalSave);

        Assert.Equal(100, save.TerraTokens.TerraTokens);
        Assert.Equal(200, save.TerraTokens.AllTimeTerraTokens);
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
        Assert.Equal(111, save.Players[0].Id);
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

        Assert.Equal(2, save.WorldObjects.Length);
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
```

- [ ] **Step 2: Add project reference to test project**

Edit `tests/SaveForge.Tests/SaveForge.Tests.csproj` — add inside the `<ItemGroup>` with other ProjectReferences:
```xml
<ProjectReference Include="..\..\src\SaveForge.Plugin.PlanetCrafter\SaveForge.Plugin.PlanetCrafter.csproj" />
```

- [ ] **Step 3: Run tests to verify they fail with "type not found"**

```bash
cd /I/Projects/danaketh/SaveForge
dotnet test tests/SaveForge.Tests/SaveForge.Tests.csproj --filter "FullyQualifiedName~PlanetCrafterParser" 2>&1 | tail -10
```

Expected: Build failure — `PlanetCrafterSaveParser` does not exist yet.

- [ ] **Step 4: Implement the parser**

`src/SaveForge.Plugin.PlanetCrafter/Parser/PlanetCrafterSaveParser.cs`:
```csharp
using System.Text.Json;
using System.Text.Json.Nodes;
using SaveForge.Plugin.PlanetCrafter.Models;

namespace SaveForge.Plugin.PlanetCrafter.Parser;

/// <summary>
/// Parses the Planet Crafter custom save format into a <see cref="PlanetCrafterSave"/>.
///
/// Format: Groups are separated by '@'. Within each group, JSON objects are separated
/// by '|' (with optional surrounding whitespace/newlines). The last object in a group
/// has no trailing '|'. There are always at least 11 groups (index 0–10); a trailing
/// empty group (index 11) is normal.
/// </summary>
public static class PlanetCrafterSaveParser
{
    private static readonly JsonSerializerOptions _opts = new()
    {
        PropertyNameCaseInsensitive = false
    };

    public static PlanetCrafterSave Parse(string content)
    {
        ArgumentNullException.ThrowIfNull(content);

        var groups = content.Split('@');
        if (groups.Length < 11)
            throw new InvalidDataException(
                $"Expected at least 11 groups separated by '@', found {groups.Length}.");

        return new PlanetCrafterSave
        {
            TerraTokens  = ParseSingle<TerraTokensData>(groups[0]),
            Planet       = ParseSingle<PlanetData>(groups[1]),
            Players      = ParseMany<PlayerData>(groups[2]),
            WorldObjects = ParseWorldObjects(groups[3]),
            Inventories  = ParseMany<InventoryData>(groups[4]),
            Stats        = ParseSingle<GlobalStats>(groups[5]),
            Messages     = ParseMany<MessageData>(groups[6]),
            StoryEvents  = ParseMany<StoryEventData>(groups[7]),
            DisplayInfo  = ParseSingle<SaveDisplayInfo>(groups[8]),
            TerrainLayers   = ParseMany<TerrainLayer>(groups[9]),
            PlanetInstances = ParseMany<PlanetInstance>(groups[10]),
        };
    }

    private static T ParseSingle<T>(string group)
    {
        var json = group.Trim();
        return JsonSerializer.Deserialize<T>(json, _opts)
            ?? throw new InvalidDataException($"Failed to deserialize group as {typeof(T).Name}: {json[..Math.Min(80, json.Length)]}");
    }

    private static T[] ParseMany<T>(string group)
    {
        var items = SplitObjects(group);
        if (items.Length == 0) return Array.Empty<T>();

        var result = new T[items.Length];
        for (int i = 0; i < items.Length; i++)
        {
            result[i] = JsonSerializer.Deserialize<T>(items[i], _opts)
                ?? throw new InvalidDataException($"Failed to deserialize item {i} as {typeof(T).Name}.");
        }
        return result;
    }

    private static WorldObject[] ParseWorldObjects(string group)
    {
        var items = SplitObjects(group);
        if (items.Length == 0) return Array.Empty<WorldObject>();

        var result = new WorldObject[items.Length];
        for (int i = 0; i < items.Length; i++)
        {
            var node = JsonNode.Parse(items[i]) as JsonObject
                ?? throw new InvalidDataException($"World object {i} is not a JSON object.");

            result[i] = new WorldObject
            {
                Id      = node["id"]?.GetValue<int>() ?? 0,
                GId     = node["gId"]?.GetValue<string>() ?? "",
                RawNode = node,
            };
        }
        return result;
    }

    /// <summary>Splits a group string into individual JSON object strings.</summary>
    private static string[] SplitObjects(string group) =>
        group.Split('|')
             .Select(s => s.Trim())
             .Where(s => s.Length > 0)
             .ToArray();
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd /I/Projects/danaketh/SaveForge
dotnet test tests/SaveForge.Tests/SaveForge.Tests.csproj --filter "FullyQualifiedName~PlanetCrafterParser" -v n 2>&1 | tail -20
```

Expected: All 15 tests pass.

- [ ] **Step 6: Commit**

```bash
cd /I/Projects/danaketh/SaveForge
git add src/SaveForge.Plugin.PlanetCrafter/Parser/ tests/SaveForge.Tests/
git commit -m "feat(planet-crafter): implement PlanetCrafterSaveParser with full test coverage"
```

---

## Task 3: Writer — serialize typed models back to the custom format

**Files:**
- Create: `src/SaveForge.Plugin.PlanetCrafter/Parser/PlanetCrafterSaveWriter.cs`
- Modify: `tests/SaveForge.Tests/PlanetCrafterParserTests.cs` (add round-trip tests)

The writer must produce output that the parser can read back without data loss. The format: groups joined by `@`, objects within a group joined by `|\n`, with no trailing `|` on the last object.

**WorldObject serialization:** Use the `RawNode` JsonObject directly — call `.ToJsonString()` — so all fields are preserved exactly.

- [ ] **Step 1: Write the failing round-trip tests**

Add to `tests/SaveForge.Tests/PlanetCrafterParserTests.cs`:
```csharp
public class PlanetCrafterWriterTests
{
    private const string MinimalSave =
        "{\"terraTokens\":100,\"allTimeTerraTokens\":200,\"unlockedGroups\":\"A,B\",\"openedInstanceSeed\":0,\"openedInstanceTimeLeft\":0}@" +
        "{\"planetId\":\"Prime\",\"unitOxygenLevel\":1.0,\"unitHeatLevel\":2.0,\"unitPressureLevel\":3.0,\"unitPlantsLevel\":4.0,\"unitInsectsLevel\":5.0,\"unitAnimalsLevel\":6.0,\"unitPurificationLevel\":-1.0}@" +
        "{\"id\":111,\"name\":\"Alice\",\"inventoryId\":1,\"equipmentId\":2,\"playerPosition\":\"0,0,0\",\"playerRotation\":\"0,0,0,1\",\"playerGaugeOxygen\":100.0,\"playerGaugeThirst\":80.0,\"playerGaugeHealth\":90.0,\"playerGaugeToxic\":0.0,\"host\":true,\"planetId\":\"Prime\"}|" + "\n" +
        "{\"id\":222,\"name\":\"Bob\",\"inventoryId\":3,\"equipmentId\":4,\"playerPosition\":\"1,1,1\",\"playerRotation\":\"0,0,0,1\",\"playerGaugeOxygen\":50.0,\"playerGaugeThirst\":60.0,\"playerGaugeHealth\":70.0,\"playerGaugeToxic\":0.0,\"host\":false,\"planetId\":\"Prime\"}@" +
        "{\"id\":10,\"gId\":\"Cobalt\"}|" + "\n" +
        "{\"id\":11,\"gId\":\"Iron\",\"pos\":\"1,2,3\"}@" +
        "{\"id\":5,\"woIds\":\"10,11\",\"size\":4}@" +
        "{\"craftedObjects\":50,\"totalSaveFileLoad\":1,\"totalSaveFileTime\":3600}@" +
        "{\"stringId\":\"Msg1\",\"isRead\":true}|" + "\n" +
        "{\"stringId\":\"Msg2\",\"isRead\":false}@" +
        "{\"stringId\":\"StoryEvent1\"}@" +
        "{\"saveDisplayName\":\"TestSave\",\"planetId\":\"Prime\",\"unlockedSpaceTrading\":false,\"unlockedOreExtrators\":false,\"unlockedTeleporters\":false}@" +
        "{\"layerId\":\"Layer1\",\"planet\":-1234,\"colorBase\":\"1-1-1-1\",\"colorCustom\":\"1-1-1-1\",\"colorBaseLerp\":\"0-0-0-0\"}@" +
        "{\"owner\":0,\"planet\":-1234,\"index\":1,\"seed\":99,\"pos\":\"0,0,0\",\"rot\":\"0,0,0,1\"}@";

    [Fact]
    public void Write_ThenParse_TerraTokens_RoundTrips()
    {
        var original = PlanetCrafterSaveParser.Parse(MinimalSave);
        original.TerraTokens.TerraTokens = 9999;

        var written = PlanetCrafterSaveWriter.Write(original);
        var reparsed = PlanetCrafterSaveParser.Parse(written);

        Assert.Equal(9999, reparsed.TerraTokens.TerraTokens);
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

        Assert.Equal(2, reparsed.WorldObjects.Length);
        // "pos" is an optional field on Iron — must survive round-trip
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

        // Players group (index 2): two players separated by "|\n", last has no "|"
        var groups = written.Split('@');
        var playersGroup = groups[2];
        Assert.Contains("|\n", playersGroup);
        // Last player entry must not end with |
        var lastPlayer = playersGroup.Split("|\n").Last().Trim();
        Assert.DoesNotMatch(@"\|$", lastPlayer);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /I/Projects/danaketh/SaveForge
dotnet test tests/SaveForge.Tests/SaveForge.Tests.csproj --filter "FullyQualifiedName~PlanetCrafterWriter" 2>&1 | tail -10
```

Expected: Build failure — `PlanetCrafterSaveWriter` does not exist yet.

- [ ] **Step 3: Implement the writer**

`src/SaveForge.Plugin.PlanetCrafter/Parser/PlanetCrafterSaveWriter.cs`:
```csharp
using System.Text;
using System.Text.Json;
using SaveForge.Plugin.PlanetCrafter.Models;

namespace SaveForge.Plugin.PlanetCrafter.Parser;

/// <summary>
/// Serializes a <see cref="PlanetCrafterSave"/> back to the Planet Crafter custom format.
///
/// Output format: 12 groups joined by '@'. Within each group, objects are joined by "|\n".
/// The 12th group is always empty (trailing '@' at end of file).
/// </summary>
public static class PlanetCrafterSaveWriter
{
    private static readonly JsonSerializerOptions _opts = new()
    {
        WriteIndented = false
    };

    public static string Write(PlanetCrafterSave save)
    {
        var sb = new StringBuilder();

        sb.Append(Serialize(save.TerraTokens));
        sb.Append('@');
        sb.Append(Serialize(save.Planet));
        sb.Append('@');
        sb.Append(SerializeMany(save.Players.Cast<object>().ToArray()));
        sb.Append('@');
        sb.Append(SerializeWorldObjects(save.WorldObjects));
        sb.Append('@');
        sb.Append(SerializeMany(save.Inventories.Cast<object>().ToArray()));
        sb.Append('@');
        sb.Append(Serialize(save.Stats));
        sb.Append('@');
        sb.Append(SerializeMany(save.Messages.Cast<object>().ToArray()));
        sb.Append('@');
        sb.Append(SerializeMany(save.StoryEvents.Cast<object>().ToArray()));
        sb.Append('@');
        sb.Append(Serialize(save.DisplayInfo));
        sb.Append('@');
        sb.Append(SerializeMany(save.TerrainLayers.Cast<object>().ToArray()));
        sb.Append('@');
        sb.Append(SerializeMany(save.PlanetInstances.Cast<object>().ToArray()));
        sb.Append('@');  // trailing empty group

        return sb.ToString();
    }

    private static string Serialize<T>(T obj) =>
        JsonSerializer.Serialize(obj, _opts);

    private static string SerializeMany(object[] items)
    {
        if (items.Length == 0) return "";
        if (items.Length == 1) return Serialize(items[0]);

        var parts = items.Select(Serialize).ToArray();
        return string.Join("|\n", parts);
    }

    private static string SerializeWorldObjects(WorldObject[] objects)
    {
        if (objects.Length == 0) return "";
        if (objects.Length == 1)
            return objects[0].RawNode?.ToJsonString(_opts) ?? Serialize(objects[0]);

        var parts = objects.Select(wo =>
            wo.RawNode?.ToJsonString(_opts) ?? Serialize(wo));
        return string.Join("|\n", parts);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd /I/Projects/danaketh/SaveForge
dotnet test tests/SaveForge.Tests/SaveForge.Tests.csproj --filter "FullyQualifiedName~PlanetCrafter" -v n 2>&1 | tail -20
```

Expected: All 20 tests pass (15 parser + 5 writer).

- [ ] **Step 5: Commit**

```bash
cd /I/Projects/danaketh/SaveForge
git add src/SaveForge.Plugin.PlanetCrafter/Parser/PlanetCrafterSaveWriter.cs \
        tests/SaveForge.Tests/PlanetCrafterParserTests.cs
git commit -m "feat(planet-crafter): implement PlanetCrafterSaveWriter with round-trip tests"
```

---

## Task 4: Integration test with real save file

**Files:**
- Create: `tests/SaveForge.Tests/PlanetCrafterIntegrationTests.cs`

This test reads the actual `Standard-1.json` from the game's save folder (skip if not present) and verifies the parser handles it without errors.

- [ ] **Step 1: Write the integration tests**

`tests/SaveForge.Tests/PlanetCrafterIntegrationTests.cs`:
```csharp
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
        Assert.Equal(original.WorldObjects.Length, reparsed.WorldObjects.Length);
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
```

- [ ] **Step 2: Add xunit.skippable NuGet to test project**

Edit `tests/SaveForge.Tests/SaveForge.Tests.csproj` — add to `<ItemGroup>`:
```xml
<PackageReference Include="Xunit.SkippableFact" Version="1.4.13" />
```

- [ ] **Step 3: Run integration tests**

```bash
cd /I/Projects/danaketh/SaveForge
dotnet test tests/SaveForge.Tests/SaveForge.Tests.csproj --filter "FullyQualifiedName~PlanetCrafterIntegration" -v n 2>&1 | tail -15
```

Expected: All 3 tests pass (or skip if save file absent).

- [ ] **Step 4: Run full test suite to confirm nothing broke**

```bash
cd /I/Projects/danaketh/SaveForge
dotnet test SaveForge.sln -v n 2>&1 | tail -10
```

Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
cd /I/Projects/danaketh/SaveForge
git add tests/SaveForge.Tests/PlanetCrafterIntegrationTests.cs \
        tests/SaveForge.Tests/SaveForge.Tests.csproj
git commit -m "test(planet-crafter): add integration tests against real save file"
```

---

## Task 5: Plugin entry point and ISaveDocument

**Files:**
- Create: `src/SaveForge.Plugin.PlanetCrafter/PlanetCrafterPlugin.cs`
- Create: `src/SaveForge.Plugin.PlanetCrafter/PlanetCrafterDocument.cs`
- Create: `tests/SaveForge.Tests/PlanetCrafterDocumentTests.cs`
- Create: `tests/SaveForge.Tests/PlanetCrafterPluginTests.cs`

**Dot-path routing in PlanetCrafterDocument:**
The document must route `GetValue`/`SetValue` calls to the correct model field.

Key supported paths:
| Dot-path | Type | Maps to |
|----------|------|---------|
| `tokens.terraTokens` | long | `TerraTokens.TerraTokens` |
| `tokens.allTimeTerraTokens` | long | `TerraTokens.AllTimeTerraTokens` |
| `tokens.unlockedGroups` | string | `TerraTokens.UnlockedGroups` |
| `planet.planetId` | string | `Planet.PlanetId` |
| `planet.unitOxygenLevel` | double | `Planet.UnitOxygenLevel` |
| `planet.unitHeatLevel` | double | `Planet.UnitHeatLevel` |
| `planet.unitPressureLevel` | double | `Planet.UnitPressureLevel` |
| `planet.unitPlantsLevel` | double | `Planet.UnitPlantsLevel` |
| `planet.unitInsectsLevel` | double | `Planet.UnitInsectsLevel` |
| `planet.unitAnimalsLevel` | double | `Planet.UnitAnimalsLevel` |
| `players[N].name` | string | `Players[N].Name` |
| `players[N].playerGaugeHealth` | double | `Players[N].PlayerGaugeHealth` |
| `players[N].playerGaugeOxygen` | double | `Players[N].PlayerGaugeOxygen` |
| `players[N].playerGaugeThirst` | double | `Players[N].PlayerGaugeThirst` |
| `players[N].playerGaugeToxic` | double | `Players[N].PlayerGaugeToxic` |
| `displayInfo.saveDisplayName` | string | `DisplayInfo.SaveDisplayName` |
| `stats.craftedObjects` | int | `Stats.CraftedObjects` |

**CanOpen detection:** Planet Crafter saves have `.json` extension AND contain `"terraTokens"` in the first 512 bytes. Priority 20 (higher than StardewLike's 5).

- [ ] **Step 1: Write the failing tests**

`tests/SaveForge.Tests/PlanetCrafterPluginTests.cs`:
```csharp
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

        var (canOpen, priority) = plugin.CanOpen(new FileInfo(_tempFile + ".json"), header);

        Assert.True(canOpen);
        Assert.Equal(20, priority);
    }

    [Fact]
    public void CanOpen_FileWithoutTerraTokens_ReturnsFalse()
    {
        var header = Encoding.UTF8.GetBytes("{\"player\":{\"name\":\"Farmer\"}}");
        var plugin = new PlanetCrafterPlugin();

        var (canOpen, _) = plugin.CanOpen(new FileInfo(_tempFile + ".json"), header);

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
        if (File.Exists(_tempFile)) File.Delete(_tempFile);
    }
}
```

`tests/SaveForge.Tests/PlanetCrafterDocumentTests.cs`:
```csharp
using SaveForge.Plugin.PlanetCrafter;
using SaveForge.SDK;
using Xunit;

namespace SaveForge.Tests;

public class PlanetCrafterDocumentTests : IDisposable
{
    private readonly string _tempFile = Path.GetTempFileName() + ".json";

    // Minimal 11-group save (same as MinimalSave in parser tests)
    private const string MinimalSave =
        "{\"terraTokens\":100,\"allTimeTerraTokens\":200,\"unlockedGroups\":\"A,B\",\"openedInstanceSeed\":0,\"openedInstanceTimeLeft\":0}@" +
        "{\"planetId\":\"Prime\",\"unitOxygenLevel\":1.0,\"unitHeatLevel\":2.0,\"unitPressureLevel\":3.0,\"unitPlantsLevel\":4.0,\"unitInsectsLevel\":5.0,\"unitAnimalsLevel\":6.0,\"unitPurificationLevel\":-1.0}@" +
        "{\"id\":111,\"name\":\"Alice\",\"inventoryId\":1,\"equipmentId\":2,\"playerPosition\":\"0,0,0\",\"playerRotation\":\"0,0,0,1\",\"playerGaugeOxygen\":100.0,\"playerGaugeThirst\":80.0,\"playerGaugeHealth\":90.0,\"playerGaugeToxic\":0.0,\"host\":true,\"planetId\":\"Prime\"}|" + "\n" +
        "{\"id\":222,\"name\":\"Bob\",\"inventoryId\":3,\"equipmentId\":4,\"playerPosition\":\"1,1,1\",\"playerRotation\":\"0,0,0,1\",\"playerGaugeOxygen\":50.0,\"playerGaugeThirst\":60.0,\"playerGaugeHealth\":70.0,\"playerGaugeToxic\":0.0,\"host\":false,\"planetId\":\"Prime\"}@" +
        "{\"id\":10,\"gId\":\"Cobalt\"}@" +
        "{\"id\":5,\"woIds\":\"10\",\"size\":4}@" +
        "{\"craftedObjects\":50,\"totalSaveFileLoad\":1,\"totalSaveFileTime\":3600}@" +
        "{\"stringId\":\"Msg1\",\"isRead\":true}@" +
        "{\"stringId\":\"StoryEvent1\"}@" +
        "{\"saveDisplayName\":\"TestSave\",\"planetId\":\"Prime\",\"unlockedSpaceTrading\":false,\"unlockedOreExtrators\":false,\"unlockedTeleporters\":false}@" +
        "{\"layerId\":\"Layer1\",\"planet\":-1234,\"colorBase\":\"1-1-1-1\",\"colorCustom\":\"1-1-1-1\",\"colorBaseLerp\":\"0-0-0-0\"}@" +
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

        var plugin = new PlanetCrafterPlugin();
        var doc2 = (PlanetCrafterDocument)await plugin.OpenAsync(_tempFile);
        Assert.Equal(5555L, doc2.GetValue("tokens.terraTokens"));
    }

    [Fact]
    public async Task GetEditorDefinition_ReturnsSchemaKind()
    {
        var doc = await OpenDocumentAsync();
        var def = doc.GetEditorDefinition();
        Assert.Equal(EditorKind.Schema, def.Kind);
        Assert.NotNull(def.Schema);
    }

    public void Dispose()
    {
        if (File.Exists(_tempFile)) File.Delete(_tempFile);
    }
}
```

- [ ] **Step 2: Implement PlanetCrafterPlugin.cs**

`src/SaveForge.Plugin.PlanetCrafter/PlanetCrafterPlugin.cs`:
```csharp
using System.Text;
using SaveForge.Plugin.PlanetCrafter.Parser;
using SaveForge.SDK;

namespace SaveForge.Plugin.PlanetCrafter;

public sealed class PlanetCrafterPlugin : IGameEditorPlugin
{
    public PluginManifest Manifest { get; } = new()
    {
        Id = "planet-crafter",
        Name = "Planet Crafter Save Editor",
        Version = "1.0.0",
        Author = "SaveForge Team",
        Description = "Edit saves for Planet Crafter.",
        MinHostVersion = "1.0.0",
        PluginAssembly = "SaveForge.Plugin.PlanetCrafter.dll"
    };

    public SaveCapabilities Capabilities { get; } = new()
    {
        FileExtensions = [".json"],
        DetectionPriority = 20
    };

    public (bool CanOpen, int Priority) CanOpen(FileInfo file, ReadOnlySpan<byte> header)
    {
        if (!file.Extension.Equals(".json", StringComparison.OrdinalIgnoreCase))
            return (false, 0);

        var text = Encoding.UTF8.GetString(header[..Math.Min(512, header.Length)]);
        return text.Contains("\"terraTokens\"", StringComparison.Ordinal)
            ? (true, Capabilities.DetectionPriority)
            : (false, 0);
    }

    public async Task<ISaveDocument> OpenAsync(string filePath, CancellationToken ct = default)
    {
        var content = await File.ReadAllTextAsync(filePath, ct);
        var save = PlanetCrafterSaveParser.Parse(content);
        return new PlanetCrafterDocument(filePath, save);
    }
}
```

- [ ] **Step 3: Implement PlanetCrafterDocument.cs**

`src/SaveForge.Plugin.PlanetCrafter/PlanetCrafterDocument.cs`:
```csharp
using System.Text.RegularExpressions;
using SaveForge.Plugin.PlanetCrafter.Models;
using SaveForge.Plugin.PlanetCrafter.Parser;
using SaveForge.SDK;
using SaveForge.SDK.Schema;

namespace SaveForge.Plugin.PlanetCrafter;

public sealed class PlanetCrafterDocument : ISaveDocument
{
    private readonly PlanetCrafterSave _save;
    private bool _isDirty;

    private static readonly Regex _indexedPath = new(@"^(\w+)\[(\d+)\]\.(.+)$", RegexOptions.Compiled);

    public PlanetCrafterDocument(string filePath, PlanetCrafterSave save)
    {
        FilePath = filePath;
        _save = save;
    }

    public string FilePath { get; }
    public bool IsDirty => _isDirty;

    public object? GetValue(string dotPath)
    {
        var m = _indexedPath.Match(dotPath);
        if (m.Success)
            return GetIndexed(m.Groups[1].Value, int.Parse(m.Groups[2].Value), m.Groups[3].Value);

        var parts = dotPath.Split('.', 2);
        if (parts.Length < 2) return null;

        return parts[0] switch
        {
            "tokens"      => GetTokenField(parts[1]),
            "planet"      => GetPlanetField(parts[1]),
            "displayInfo" => GetDisplayInfoField(parts[1]),
            "stats"       => GetStatsField(parts[1]),
            _ => null
        };
    }

    public void SetValue(string dotPath, object? value)
    {
        var m = _indexedPath.Match(dotPath);
        if (m.Success)
        {
            SetIndexed(m.Groups[1].Value, int.Parse(m.Groups[2].Value), m.Groups[3].Value, value);
            return;
        }

        var parts = dotPath.Split('.', 2);
        if (parts.Length < 2) return;

        switch (parts[0])
        {
            case "tokens":      SetTokenField(parts[1], value); break;
            case "planet":      SetPlanetField(parts[1], value); break;
            case "displayInfo": SetDisplayInfoField(parts[1], value); break;
            case "stats":       SetStatsField(parts[1], value); break;
        }
    }

    public IEditorDefinition GetEditorDefinition() =>
        new SchemaEditorDefinition(PlanetCrafterSchema.Build(_save));

    public async Task SaveAsync(CancellationToken ct = default)
    {
        var content = PlanetCrafterSaveWriter.Write(_save);
        await File.WriteAllTextAsync(FilePath, content, ct);
        _isDirty = false;
    }

    // --- Getters ---

    private object? GetTokenField(string field) => field switch
    {
        "terraTokens"        => _save.TerraTokens.TerraTokens,
        "allTimeTerraTokens" => _save.TerraTokens.AllTimeTerraTokens,
        "unlockedGroups"     => _save.TerraTokens.UnlockedGroups,
        _ => null
    };

    private object? GetPlanetField(string field) => field switch
    {
        "planetId"            => _save.Planet.PlanetId,
        "unitOxygenLevel"     => _save.Planet.UnitOxygenLevel,
        "unitHeatLevel"       => _save.Planet.UnitHeatLevel,
        "unitPressureLevel"   => _save.Planet.UnitPressureLevel,
        "unitPlantsLevel"     => _save.Planet.UnitPlantsLevel,
        "unitInsectsLevel"    => _save.Planet.UnitInsectsLevel,
        "unitAnimalsLevel"    => _save.Planet.UnitAnimalsLevel,
        "unitPurificationLevel" => _save.Planet.UnitPurificationLevel,
        _ => null
    };

    private object? GetDisplayInfoField(string field) => field switch
    {
        "saveDisplayName"      => _save.DisplayInfo.SaveDisplayName,
        "planetId"             => _save.DisplayInfo.PlanetId,
        "unlockedSpaceTrading" => _save.DisplayInfo.UnlockedSpaceTrading,
        _ => null
    };

    private object? GetStatsField(string field) => field switch
    {
        "craftedObjects"    => _save.Stats.CraftedObjects,
        "totalSaveFileTime" => _save.Stats.TotalSaveFileTime,
        "totalSaveFileLoad" => _save.Stats.TotalSaveFileLoad,
        _ => null
    };

    private object? GetIndexed(string group, int index, string field) => group switch
    {
        "players" when index < _save.Players.Length => GetPlayerField(_save.Players[index], field),
        _ => null
    };

    private static object? GetPlayerField(PlayerData p, string field) => field switch
    {
        "id"                 => p.Id,
        "name"               => p.Name,
        "playerGaugeOxygen"  => p.PlayerGaugeOxygen,
        "playerGaugeThirst"  => p.PlayerGaugeThirst,
        "playerGaugeHealth"  => p.PlayerGaugeHealth,
        "playerGaugeToxic"   => p.PlayerGaugeToxic,
        "host"               => p.Host,
        "planetId"           => p.PlanetId,
        _ => null
    };

    // --- Setters ---

    private void SetTokenField(string field, object? value)
    {
        switch (field)
        {
            case "terraTokens":        _save.TerraTokens.TerraTokens = ToLong(value); break;
            case "allTimeTerraTokens": _save.TerraTokens.AllTimeTerraTokens = ToLong(value); break;
            case "unlockedGroups":     _save.TerraTokens.UnlockedGroups = value?.ToString() ?? ""; break;
            default: return;
        }
        _isDirty = true;
    }

    private void SetPlanetField(string field, object? value)
    {
        switch (field)
        {
            case "unitOxygenLevel":     _save.Planet.UnitOxygenLevel = ToDouble(value); break;
            case "unitHeatLevel":       _save.Planet.UnitHeatLevel = ToDouble(value); break;
            case "unitPressureLevel":   _save.Planet.UnitPressureLevel = ToDouble(value); break;
            case "unitPlantsLevel":     _save.Planet.UnitPlantsLevel = ToDouble(value); break;
            case "unitInsectsLevel":    _save.Planet.UnitInsectsLevel = ToDouble(value); break;
            case "unitAnimalsLevel":    _save.Planet.UnitAnimalsLevel = ToDouble(value); break;
            default: return;
        }
        _isDirty = true;
    }

    private void SetDisplayInfoField(string field, object? value)
    {
        switch (field)
        {
            case "saveDisplayName": _save.DisplayInfo.SaveDisplayName = value?.ToString() ?? ""; break;
            default: return;
        }
        _isDirty = true;
    }

    private void SetStatsField(string field, object? value)
    {
        switch (field)
        {
            case "craftedObjects": _save.Stats.CraftedObjects = ToInt(value); break;
            default: return;
        }
        _isDirty = true;
    }

    private void SetIndexed(string group, int index, string field, object? value)
    {
        if (group == "players" && index < _save.Players.Length)
        {
            SetPlayerField(_save.Players[index], field, value);
        }
    }

    private void SetPlayerField(PlayerData p, string field, object? value)
    {
        switch (field)
        {
            case "name":               p.Name = value?.ToString() ?? ""; break;
            case "playerGaugeOxygen":  p.PlayerGaugeOxygen = ToDouble(value); break;
            case "playerGaugeThirst":  p.PlayerGaugeThirst = ToDouble(value); break;
            case "playerGaugeHealth":  p.PlayerGaugeHealth = ToDouble(value); break;
            case "playerGaugeToxic":   p.PlayerGaugeToxic = ToDouble(value); break;
            default: return;
        }
        _isDirty = true;
    }

    private static long   ToLong(object? v)   => v is long l ? l : Convert.ToInt64(v);
    private static double ToDouble(object? v)  => v is double d ? d : Convert.ToDouble(v);
    private static int    ToInt(object? v)     => v is int i ? i : Convert.ToInt32(v);
}

internal sealed class SchemaEditorDefinition : IEditorDefinition
{
    public SchemaEditorDefinition(FormSchema schema) => Schema = schema;
    public EditorKind Kind => EditorKind.Schema;
    public FormSchema? Schema { get; }
    public ICustomEditorViewFactory? ViewFactory => null;
}
```

- [ ] **Step 4: Run failing tests to check they now compile and fail on logic**

```bash
cd /I/Projects/danaketh/SaveForge
dotnet test tests/SaveForge.Tests/SaveForge.Tests.csproj \
  --filter "FullyQualifiedName~PlanetCrafterPlugin|FullyQualifiedName~PlanetCrafterDocument" \
  -v n 2>&1 | tail -20
```

Expected: Build succeeds, tests fail on `PlanetCrafterSchema.Build` (not implemented yet).

- [ ] **Step 5: Commit stub (before schema)**

```bash
cd /I/Projects/danaketh/SaveForge
git add src/SaveForge.Plugin.PlanetCrafter/PlanetCrafterPlugin.cs \
        src/SaveForge.Plugin.PlanetCrafter/PlanetCrafterDocument.cs \
        tests/SaveForge.Tests/
git commit -m "feat(planet-crafter): implement PlanetCrafterPlugin and PlanetCrafterDocument"
```

---

## Task 6: Schema builder

**Files:**
- Create: `src/SaveForge.Plugin.PlanetCrafter/PlanetCrafterSchema.cs`

The schema exposes all editable fields organized across pages: Tokens, Planet, Players, Save Info.

- [ ] **Step 1: Run tests to confirm they fail at schema**

```bash
cd /I/Projects/danaketh/SaveForge
dotnet test tests/SaveForge.Tests/SaveForge.Tests.csproj \
  --filter "FullyQualifiedName~PlanetCrafterDocument" -v n 2>&1 | tail -10
```

Expected: Build failure — `PlanetCrafterSchema` does not exist.

- [ ] **Step 2: Implement the schema builder**

`src/SaveForge.Plugin.PlanetCrafter/PlanetCrafterSchema.cs`:
```csharp
using SaveForge.Plugin.PlanetCrafter.Models;
using SaveForge.SDK.Schema;

namespace SaveForge.Plugin.PlanetCrafter;

internal static class PlanetCrafterSchema
{
    public static FormSchema Build(PlanetCrafterSave save) => new()
    {
        Pages =
        [
            BuildTokensPage(),
            BuildPlanetPage(),
            BuildPlayersPage(save.Players),
            BuildSaveInfoPage(),
        ]
    };

    private static FormPage BuildTokensPage() => new()
    {
        Title = "Terra Tokens",
        Sections =
        [
            new FormSection
            {
                Heading = "Currency",
                Fields =
                [
                    new NumericField
                    {
                        Key = "tokens.terraTokens",
                        Label = "Terra Tokens",
                        Description = "Current spendable terra tokens.",
                        Min = 0,
                        DecimalPlaces = 0
                    },
                    new NumericField
                    {
                        Key = "tokens.allTimeTerraTokens",
                        Label = "All-Time Terra Tokens",
                        Description = "Total tokens earned (used for unlock tracking).",
                        Min = 0,
                        DecimalPlaces = 0,
                        IsReadOnly = true
                    },
                ]
            },
            new FormSection
            {
                Heading = "Unlocks",
                Fields =
                [
                    new ReadOnlyField
                    {
                        Key = "tokens.unlockedGroups",
                        Label = "Unlocked Groups",
                        Description = "Comma-separated list of unlocked item groups.",
                    }
                ]
            }
        ]
    };

    private static FormPage BuildPlanetPage() => new()
    {
        Title = "Planet State",
        Sections =
        [
            new FormSection
            {
                Heading = "Terraforming Levels",
                Fields =
                [
                    new NumericField { Key = "planet.unitOxygenLevel",   Label = "Oxygen",   Min = 0, DecimalPlaces = 2 },
                    new NumericField { Key = "planet.unitHeatLevel",      Label = "Heat",     Min = 0, DecimalPlaces = 2 },
                    new NumericField { Key = "planet.unitPressureLevel",  Label = "Pressure", Min = 0, DecimalPlaces = 2 },
                    new NumericField { Key = "planet.unitPlantsLevel",    Label = "Plants",   Min = 0, DecimalPlaces = 2 },
                    new NumericField { Key = "planet.unitInsectsLevel",   Label = "Insects",  Min = 0, DecimalPlaces = 2 },
                    new NumericField { Key = "planet.unitAnimalsLevel",   Label = "Animals",  Min = 0, DecimalPlaces = 2 },
                ]
            }
        ]
    };

    private static FormPage BuildPlayersPage(PlayerData[] players)
    {
        var sections = players.Select((p, i) => new FormSection
        {
            Heading = $"{p.Name}{(p.Host ? " (Host)" : "")}",
            Fields =
            [
                new ReadOnlyField { Key = $"players[{i}].name",  Label = "Name" },
                new NumericField  { Key = $"players[{i}].playerGaugeHealth", Label = "Health", Min = 0, Max = 100, DecimalPlaces = 1 },
                new NumericField  { Key = $"players[{i}].playerGaugeOxygen", Label = "Oxygen", Min = 0, Max = 370, DecimalPlaces = 1 },
                new NumericField  { Key = $"players[{i}].playerGaugeThirst", Label = "Thirst", Min = 0, Max = 100, DecimalPlaces = 1 },
                new NumericField  { Key = $"players[{i}].playerGaugeToxic",  Label = "Toxic",  Min = 0, Max = 100, DecimalPlaces = 1 },
            ]
        }).ToArray();

        return new FormPage { Title = "Players", Sections = sections };
    }

    private static FormPage BuildSaveInfoPage() => new()
    {
        Title = "Save Info",
        Sections =
        [
            new FormSection
            {
                Heading = "General",
                Fields =
                [
                    new TextField     { Key = "displayInfo.saveDisplayName", Label = "Save Display Name", MaxLength = 64 },
                    new ReadOnlyField { Key = "planet.planetId",             Label = "Planet ID" },
                    new ReadOnlyField { Key = "stats.craftedObjects",        Label = "Crafted Objects" },
                ]
            }
        ]
    };
}
```

- [ ] **Step 3: Run all PlanetCrafter tests**

```bash
cd /I/Projects/danaketh/SaveForge
dotnet test tests/SaveForge.Tests/SaveForge.Tests.csproj \
  --filter "FullyQualifiedName~PlanetCrafter" -v n 2>&1 | tail -20
```

Expected: All ~28 tests pass.

- [ ] **Step 4: Run full suite**

```bash
cd /I/Projects/danaketh/SaveForge
dotnet test SaveForge.sln -v n 2>&1 | tail -10
```

Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
cd /I/Projects/danaketh/SaveForge
git add src/SaveForge.Plugin.PlanetCrafter/PlanetCrafterSchema.cs
git commit -m "feat(planet-crafter): implement schema with tokens, planet, players, and save info pages"
```

---

## Self-Review

### Spec coverage
- ✅ Default save path documented (format reference section)
- ✅ Sample save file analyzed and format understood
- ✅ Parser for multi-group `@`/`|` format — Task 2
- ✅ Writer for round-trip — Task 3
- ✅ Tests for parser — Task 2
- ✅ Integration test with real save — Task 4
- ✅ Plugin entry point (IGameEditorPlugin) — Task 5
- ✅ ISaveDocument with dot-path routing — Task 5
- ✅ FormSchema for UI — Task 6
- ✅ Plan is written down and can be updated each iteration

### Placeholder scan
- No TBD/TODO markers found
- All code blocks contain full implementations
- All types referenced in later tasks are defined in earlier tasks

### Type consistency
- `PlanetCrafterSave` defined in Task 1, used in Tasks 2, 3, 5, 6 ✅
- `PlanetCrafterSaveParser.Parse(string)` defined in Task 2, used in Tasks 3, 4, 5 ✅
- `PlanetCrafterSaveWriter.Write(PlanetCrafterSave)` defined in Task 3, used in Tasks 4, 5 ✅
- `PlanetCrafterSchema.Build(PlanetCrafterSave)` defined in Task 6, referenced in Task 5 (document tests run after Task 6) ✅
- `SchemaEditorDefinition` defined in Task 5 — note: same name exists in StardewSaveDocument.cs but is `internal`, no conflict ✅
