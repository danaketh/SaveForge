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
                    new NumericField { Key = "planet.unitOxygenLevel", Label = "Oxygen", Min = 0, DecimalPlaces = 2 },
                    new NumericField { Key = "planet.unitHeatLevel", Label = "Heat", Min = 0, DecimalPlaces = 2 },
                    new NumericField
                    {
                        Key = "planet.unitPressureLevel", Label = "Pressure", Min = 0, DecimalPlaces = 2
                    },
                    new NumericField { Key = "planet.unitPlantsLevel", Label = "Plants", Min = 0, DecimalPlaces = 2 },
                    new NumericField { Key = "planet.unitInsectsLevel", Label = "Insects", Min = 0, DecimalPlaces = 2 },
                    new NumericField { Key = "planet.unitAnimalsLevel", Label = "Animals", Min = 0, DecimalPlaces = 2 },
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
                new ReadOnlyField { Key = $"players[{i}].name", Label = "Name" },
                new NumericField
                {
                    Key = $"players[{i}].playerGaugeHealth",
                    Label = "Health",
                    Min = 0,
                    Max = 100,
                    DecimalPlaces = 1
                },
                new NumericField
                {
                    Key = $"players[{i}].playerGaugeOxygen",
                    Label = "Oxygen",
                    Min = 0,
                    Max = 370,
                    DecimalPlaces = 1
                },
                new NumericField
                {
                    Key = $"players[{i}].playerGaugeThirst",
                    Label = "Thirst",
                    Min = 0,
                    Max = 100,
                    DecimalPlaces = 1
                },
                new NumericField
                {
                    Key = $"players[{i}].playerGaugeToxic",
                    Label = "Toxic",
                    Min = 0,
                    Max = 100,
                    DecimalPlaces = 1
                },
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
                    new TextField { Key = "displayInfo.saveDisplayName", Label = "Save Display Name", MaxLength = 64 },
                    new ReadOnlyField { Key = "planet.planetId", Label = "Planet ID" },
                    new ReadOnlyField { Key = "stats.craftedObjects", Label = "Crafted Objects" },
                ]
            }
        ]
    };
}
