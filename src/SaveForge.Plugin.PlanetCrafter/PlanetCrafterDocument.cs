using System.Text.RegularExpressions;
using SaveForge.Plugin.PlanetCrafter.Models;
using SaveForge.Plugin.PlanetCrafter.Parser;
using SaveForge.SDK;
using SaveForge.SDK.Schema; // FormSchema still needed by CustomEditorDefinition interface

namespace SaveForge.Plugin.PlanetCrafter;

public sealed class PlanetCrafterDocument : ISaveDocument
{
    private readonly PlanetCrafterSave _save;
    private bool _isDirty;

    private static readonly Regex _indexedPath =
        new(@"^(\w+)\[(\d+)\]\.(.+)$", RegexOptions.Compiled);

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
        {
            return GetIndexed(m.Groups[1].Value, int.Parse(m.Groups[2].Value), m.Groups[3].Value);
        }

        var sep = dotPath.IndexOf('.');
        if (sep < 0)
        {
            return null;
        }

        var prefix = dotPath[..sep];
        var field = dotPath[(sep + 1)..];

        return prefix switch
        {
            "tokens" => GetTokenField(field),
            "planet" => GetPlanetField(field),
            "displayInfo" => GetDisplayInfoField(field),
            "stats" => GetStatsField(field),
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

        var sep = dotPath.IndexOf('.');
        if (sep < 0)
        {
            return;
        }

        var prefix = dotPath[..sep];
        var field = dotPath[(sep + 1)..];

        switch (prefix)
        {
            case "tokens":
                SetTokenField(field, value);
                break;
            case "planet":
                SetPlanetField(field, value);
                break;
            case "displayInfo":
                SetDisplayInfoField(field, value);
                break;
            case "stats":
                SetStatsField(field, value);
                break;
        }
    }

    public IEditorDefinition GetEditorDefinition() =>
        new CustomEditorDefinition(new PlanetCrafterEditorViewFactory(this, LocalizationService.TryLoad()));

    internal PlanetCrafterSave GetSave() => _save;
    internal void MarkDirty() => _isDirty = true;

    public async Task SaveAsync(CancellationToken ct = default)
    {
        var content = PlanetCrafterSaveWriter.Write(_save);
        await File.WriteAllTextAsync(FilePath, content, ct);
        _isDirty = false;
    }

    // --- Getters ---

    private object? GetTokenField(string field) => field switch
    {
        "terraTokens" => _save.TerraTokens.TerraTokens,
        "allTimeTerraTokens" => _save.TerraTokens.AllTimeTerraTokens,
        "unlockedGroups" => _save.TerraTokens.UnlockedGroups,
        _ => null
    };

    private object? GetPlanetField(string field) => field switch
    {
        "planetId" => _save.Planet.PlanetId,
        "unitOxygenLevel" => _save.Planet.UnitOxygenLevel,
        "unitHeatLevel" => _save.Planet.UnitHeatLevel,
        "unitPressureLevel" => _save.Planet.UnitPressureLevel,
        "unitPlantsLevel" => _save.Planet.UnitPlantsLevel,
        "unitInsectsLevel" => _save.Planet.UnitInsectsLevel,
        "unitAnimalsLevel" => _save.Planet.UnitAnimalsLevel,
        "unitPurificationLevel" => _save.Planet.UnitPurificationLevel,
        _ => null
    };

    private object? GetDisplayInfoField(string field) => field switch
    {
        "saveDisplayName" => _save.DisplayInfo.SaveDisplayName,
        "planetId" => _save.DisplayInfo.PlanetId,
        "unlockedSpaceTrading" => _save.DisplayInfo.UnlockedSpaceTrading,
        _ => null
    };

    private object? GetStatsField(string field) => field switch
    {
        "craftedObjects" => _save.Stats.CraftedObjects,
        "totalSaveFileTime" => _save.Stats.TotalSaveFileTime,
        "totalSaveFileLoad" => _save.Stats.TotalSaveFileLoad,
        _ => null
    };

    private object? GetIndexed(string group, int index, string field) => group switch
    {
        "players" when index < _save.Players.Length =>
            GetPlayerField(_save.Players[index], field),
        _ => null
    };

    private static object? GetPlayerField(PlayerData p, string field) => field switch
    {
        "id" => p.Id,
        "name" => p.Name,
        "playerGaugeOxygen" => p.PlayerGaugeOxygen,
        "playerGaugeThirst" => p.PlayerGaugeThirst,
        "playerGaugeHealth" => p.PlayerGaugeHealth,
        "playerGaugeToxic" => p.PlayerGaugeToxic,
        "host" => p.Host,
        "planetId" => p.PlanetId,
        _ => null
    };

    // --- Setters ---

    private void SetTokenField(string field, object? value)
    {
        switch (field)
        {
            case "terraTokens":
                _save.TerraTokens.TerraTokens = ToLong(value);
                break;
            case "allTimeTerraTokens":
                _save.TerraTokens.AllTimeTerraTokens = ToLong(value);
                break;
            case "unlockedGroups":
                _save.TerraTokens.UnlockedGroups = value?.ToString() ?? "";
                break;
            default:
                return;
        }
        _isDirty = true;
    }

    private void SetPlanetField(string field, object? value)
    {
        switch (field)
        {
            case "planetId":
                _save.Planet.PlanetId = value?.ToString() ?? "";
                break;
            case "unitOxygenLevel":
                _save.Planet.UnitOxygenLevel = ToDouble(value);
                break;
            case "unitHeatLevel":
                _save.Planet.UnitHeatLevel = ToDouble(value);
                break;
            case "unitPressureLevel":
                _save.Planet.UnitPressureLevel = ToDouble(value);
                break;
            case "unitPlantsLevel":
                _save.Planet.UnitPlantsLevel = ToDouble(value);
                break;
            case "unitInsectsLevel":
                _save.Planet.UnitInsectsLevel = ToDouble(value);
                break;
            case "unitAnimalsLevel":
                _save.Planet.UnitAnimalsLevel = ToDouble(value);
                break;
            case "unitPurificationLevel":
                _save.Planet.UnitPurificationLevel = ToDouble(value);
                break;
            default:
                return;
        }
        _isDirty = true;
    }

    private void SetDisplayInfoField(string field, object? value)
    {
        switch (field)
        {
            case "saveDisplayName":
                _save.DisplayInfo.SaveDisplayName = value?.ToString() ?? "";
                break;
            default:
                return;
        }
        _isDirty = true;
    }

    private void SetStatsField(string field, object? value)
    {
        switch (field)
        {
            case "craftedObjects":
                _save.Stats.CraftedObjects = ToInt(value);
                break;
            default:
                return;
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
            case "name":
                p.Name = value?.ToString() ?? "";
                break;
            case "playerGaugeOxygen":
                p.PlayerGaugeOxygen = ToDouble(value);
                break;
            case "playerGaugeThirst":
                p.PlayerGaugeThirst = ToDouble(value);
                break;
            case "playerGaugeHealth":
                p.PlayerGaugeHealth = ToDouble(value);
                break;
            case "playerGaugeToxic":
                p.PlayerGaugeToxic = ToDouble(value);
                break;
            default:
                return;
        }
        _isDirty = true;
    }

    private static long ToLong(object? v) => v is long l ? l : Convert.ToInt64(v);
    private static double ToDouble(object? v) => v is double d ? d : Convert.ToDouble(v);
    private static int ToInt(object? v) => v is int i ? i : Convert.ToInt32(v);
}

internal sealed class CustomEditorDefinition(ICustomEditorViewFactory factory) : IEditorDefinition
{
    public EditorKind Kind => EditorKind.Custom;
    public FormSchema? Schema => null;
    public ICustomEditorViewFactory? ViewFactory => factory;
}
