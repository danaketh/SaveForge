//! `StardewLike` sample plugin: the reference for the schema-driven pattern
//! (spec §6.2). Everything game-specific is the schema below.

use saveforge_sdk::form::{
    FormDocument, FormPage, FormSchema, FormSection, JsonCodec, VisibilityOp, VisibilityRule,
    checkbox, choice, list, number, object, text,
};
use saveforge_sdk::{AuxFile, Plugin, export_plugin};

#[derive(Debug)]
pub struct Stardew;

pub type StardewDocument = FormDocument<JsonCodec>;

/// Detection priority; Planet Crafter's more specific detector uses 20.
pub const PRIORITY: u32 = 5;

impl Plugin for Stardew {
    type Doc = StardewDocument;

    fn detect(_file_name: &str, header: &[u8]) -> Option<u32> {
        header
            .windows(br#""player""#.len())
            .any(|w| w == br#""player""#)
            .then_some(PRIORITY)
    }

    fn open(
        _file_name: &str,
        bytes: Vec<u8>,
        _aux: Vec<AuxFile>,
    ) -> Result<StardewDocument, String> {
        StardewDocument::open(schema(), &bytes)
    }
}

/// Port of the .NET `StardewSchema`, plus a conditional spouse field showing
/// a `VisibilityRule`.
pub fn schema() -> FormSchema {
    FormSchema::new()
        .page(
            FormPage::new("Player")
                .section(
                    FormSection::new()
                        .heading("Identity")
                        .field(
                            text("player.name", "Name")
                                .help("The farmer's name.")
                                .required()
                                .max_len(50),
                        )
                        .field(number("player.level", "Level").range(1.0, 100.0))
                        .field(number("player.money", "Money").min(0.0))
                        .field(choice(
                            "farm.type",
                            "Farm type",
                            &[
                                ("standard", "Standard"),
                                ("riverland", "Riverland"),
                                ("forest", "Forest"),
                            ],
                        ))
                        .field(checkbox("player.married", "Married"))
                        .field(text("player.spouse", "Spouse").required().visible_when(
                            VisibilityRule::new("player.married", VisibilityOp::Equals, true),
                        )),
                )
                .section(FormSection::new().field(object(
                    "player.stats",
                    "Stats",
                    vec![
                        number("health", "Health").range(0.0, 100.0),
                        number("energy", "Energy").range(0.0, 100.0),
                    ],
                ))),
        )
        .page(
            FormPage::new("Inventory").section(FormSection::new().field(list(
                "inventory",
                "Items",
                "item",
                vec![
                    text("id", "Item ID").required(),
                    number("count", "Count").min(0.0),
                ],
            ))),
        )
}

export_plugin!(Stardew);
