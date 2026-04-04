using SaveForge.SDK.Schema;

namespace SaveForge.SamplePlugin.StardewLike;

internal static class StardewSchema
{
    public static FormSchema Build() => new()
    {
        Pages =
        [
            new FormPage
            {
                Title = "Player",
                Sections =
                [
                    new FormSection
                    {
                        Heading = "Identity",
                        Fields =
                        [
                            new TextField
                            {
                                Key = "player.name",
                                Label = "Name",
                                Description = "The farmer's name.",
                                IsRequired = true,
                                MaxLength = 50
                            },
                            new NumericField
                            {
                                Key = "player.level",
                                Label = "Level",
                                Min = 1,
                                Max = 100,
                                DecimalPlaces = 0
                            },
                            new NumericField
                            {
                                Key = "player.money",
                                Label = "Money",
                                Min = 0,
                                DecimalPlaces = 0
                            }
                        ]
                    },
                    new FormSection
                    {
                        Heading = "Stats",
                        Fields =
                        [
                            new NumericField
                            {
                                Key = "player.stats.health",
                                Label = "Health",
                                Min = 0,
                                Max = 100,
                                DecimalPlaces = 0
                            },
                            new NumericField
                            {
                                Key = "player.stats.energy",
                                Label = "Energy",
                                Min = 0,
                                Max = 100,
                                DecimalPlaces = 0
                            }
                        ]
                    }
                ]
            },
            new FormPage
            {
                Title = "Inventory",
                Sections =
                [
                    new FormSection
                    {
                        Fields =
                        [
                            new ListField
                            {
                                Key = "inventory",
                                Label = "Items",
                                ItemSchema = new ObjectSectionField
                                {
                                    Key = "",
                                    Label = "Item",
                                    Fields =
                                    [
                                        new TextField { Key = "id", Label = "Item ID", IsRequired = true },
                                        new NumericField { Key = "count", Label = "Count", Min = 0, DecimalPlaces = 0 }
                                    ]
                                }
                            }
                        ]
                    }
                ]
            }
        ]
    };
}
