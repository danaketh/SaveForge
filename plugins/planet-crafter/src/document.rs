//! The editor (spec §6.1 UI table, parity with `PlanetCrafterEditorView`).

use saveforge_sdk::Document;
use saveforge_sdk::ui::{self, El, Event, PickOption, UiTree};
use serde_json::{Number, Value};

use crate::format::Object;
use crate::localization::Names;
use crate::save::{self, DISPLAY_INFO, STATS, Save, TERRA_TOKENS, int, text};

/// Widget ids.
pub const TABS: &str = "tabs";
pub const CHARACTER: &str = "character";
pub const EQUIPMENT: &str = "equipment";
pub const INVENTORY: &str = "inventory";
pub const TERRA_TOKENS_ID: &str = "tokens.terraTokens";
pub const ALL_TIME_TOKENS_ID: &str = "tokens.allTimeTerraTokens";
pub const UNLOCKED_GROUPS_ID: &str = "tokens.unlockedGroups";
pub const DISPLAY_NAME_ID: &str = "info.saveDisplayName";
pub const PLANET_ID_ID: &str = "info.planetId";
pub const CRAFTED_OBJECTS_ID: &str = "stats.craftedObjects";

pub const TAB_TITLES: [&str; 4] = ["Characters", "Terra Tokens", "Planet State", "Save Info"];

/// Player gauges: key (and widget id after `player.`), label, maximum.
const GAUGES: [(&str, &str, f64); 4] = [
    ("playerGaugeHealth", "Health", 100.0),
    ("playerGaugeOxygen", "Oxygen", 370.0),
    ("playerGaugeThirst", "Thirst", 100.0),
    ("playerGaugeToxic", "Toxic", 100.0),
];

/// Planet levels: key (widget id `planet.{index}.{key}`), label.
const LEVELS: [(&str, &str); 6] = [
    ("unitOxygenLevel", "Oxygen"),
    ("unitHeatLevel", "Heat"),
    ("unitPressureLevel", "Pressure"),
    ("unitPlantsLevel", "Plants"),
    ("unitInsectsLevel", "Insects"),
    ("unitAnimalsLevel", "Animals"),
];

const DISPLAY_NAME_MAX: usize = 64;

/// Terra Tokens are an `int` in the game; larger values would not load.
const TERRA_TOKENS_MAX: f64 = 2_147_483_647.0;

#[derive(Debug)]
pub struct PlanetCrafterDocument {
    save: Save,
    names: Names,
    /// Options for the add-item pick-lists, computed once at open.
    known_items: Vec<PickOption>,
    tab: u32,
    player: usize,
    dirty: bool,
}

impl PlanetCrafterDocument {
    pub fn new(save: Save, names: Names) -> Self {
        let known_items = names
            .known_items(save.gids())
            .into_iter()
            .map(|(gid, name)| ui::option(gid, name))
            .collect();
        Self {
            save,
            names,
            known_items,
            tab: 0,
            player: 0,
            dirty: false,
        }
    }

    pub fn save(&self) -> &Save {
        &self.save
    }

    fn characters_page(&self) -> El {
        let players = self.save.players();
        let Some(player) = players.get(self.player) else {
            return ui::column(vec![ui::muted("This save has no characters.")]);
        };
        let options = players
            .iter()
            .enumerate()
            .map(|(i, p)| ui::option(i.to_string(), player_name(p)))
            .collect();
        let gauges = GAUGES
            .iter()
            .map(|(key, label, max)| {
                ui::number(format!("player.{key}"), *label, float(player.get(key)))
                    .range(0.0, *max)
                    .decimals(1)
                    .into()
            })
            .collect();
        ui::column(vec![
            ui::pick(
                CHARACTER,
                "Character",
                options,
                Some(self.player.to_string()),
            )
            .into(),
            ui::section("Stats", gauges),
            ui::section("Equipment", self.items_view(EQUIPMENT, player)),
            ui::section("Inventory", self.items_view(INVENTORY, player)),
        ])
    }

    fn items_view(&self, list_id: &str, player: &Object) -> Vec<El> {
        let Some(items) =
            int(player.get(inventory_key(list_id))).and_then(|id| self.save.inventory_items(id))
        else {
            return vec![ui::muted("This inventory is not in the save.")];
        };
        let rows = items
            .iter()
            .map(|item| ui::text(self.names.name(&item.gid)))
            .collect();
        vec![
            ui::list(list_id, rows)
                .removable(true)
                .add_options(self.known_items.clone(), Some("Add item…".to_owned()))
                .into(),
        ]
    }

    fn terra_tokens_page(&self) -> El {
        let tokens = self.save.single(TERRA_TOKENS);
        ui::column(vec![
            ui::number(
                TERRA_TOKENS_ID,
                "Terra Tokens",
                float(tokens.get("terraTokens")),
            )
            .range(0.0, TERRA_TOKENS_MAX)
            .into(),
            ui::number(
                ALL_TIME_TOKENS_ID,
                "All-Time Terra Tokens",
                float(tokens.get("allTimeTerraTokens")),
            )
            .read_only(true)
            .into(),
            ui::text_input(
                UNLOCKED_GROUPS_ID,
                "Unlocked Groups",
                display(tokens.get("unlockedGroups")),
            )
            .read_only(true)
            .into(),
        ])
    }

    fn planet_page(&self) -> El {
        let planets = self.save.planets();
        if planets.is_empty() {
            return ui::column(vec![ui::muted("This save has no planets.")]);
        }
        ui::column(
            planets
                .iter()
                .enumerate()
                .map(|(index, planet)| {
                    let levels = LEVELS
                        .iter()
                        .map(|(key, label)| {
                            ui::number(
                                format!("planet.{index}.{key}"),
                                *label,
                                float(planet.get(key)),
                            )
                            .min(0.0)
                            .decimals(2)
                            .into()
                        })
                        .collect();
                    let name = text(planet.get("planetId"));
                    let title = if name.is_empty() {
                        format!("Planet {}", index + 1)
                    } else {
                        name.to_owned()
                    };
                    ui::section(title, levels)
                })
                .collect(),
        )
    }

    fn save_info_page(&self) -> El {
        let info = self.save.single(DISPLAY_INFO);
        let stats = self.save.single(STATS);
        ui::column(vec![
            ui::text_input(
                DISPLAY_NAME_ID,
                "Display Name",
                display(info.get("saveDisplayName")),
            )
            .max_len(u32::try_from(DISPLAY_NAME_MAX).unwrap_or(u32::MAX))
            .into(),
            ui::text_input(PLANET_ID_ID, "Planet ID", display(info.get("planetId")))
                .read_only(true)
                .into(),
            ui::text_input(
                CRAFTED_OBJECTS_ID,
                "Crafted Objects",
                display(stats.get("craftedObjects")),
            )
            .read_only(true)
            .into(),
        ])
    }

    fn set_number(&mut self, id: &str, value: f64) -> bool {
        if !value.is_finite() || value < 0.0 {
            return false;
        }
        if let Some(key) = id.strip_prefix("player.") {
            let Some((key, _, max)) = GAUGES.iter().find(|(k, ..)| *k == key) else {
                return false;
            };
            let player = self.player;
            return value <= *max
                && self
                    .save
                    .player_mut(player)
                    .is_some_and(|p| set_float(p, key, value));
        }
        if let Some((index, key)) = id
            .strip_prefix("planet.")
            .and_then(|rest| rest.split_once('.'))
        {
            return LEVELS.iter().any(|(k, _)| *k == key)
                && index
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| self.save.planet_mut(index))
                    .is_some_and(|planet| set_float(planet, key, value));
        }
        if id == TERRA_TOKENS_ID && value <= TERRA_TOKENS_MAX {
            return set_whole(self.save.single_mut(TERRA_TOKENS), "terraTokens", value);
        }
        false
    }

    fn set_text(&mut self, id: &str, value: &str) -> bool {
        if id != DISPLAY_NAME_ID {
            return false;
        }
        // The game's loader may split on `@` and `|` even inside strings, so
        // they are never written.
        let value: String = value
            .chars()
            .filter(|c| !matches!(c, '@' | '|'))
            .take(DISPLAY_NAME_MAX)
            .collect();
        self.save
            .single_mut(DISPLAY_INFO)
            .set("saveDisplayName", Value::String(value))
    }

    fn selected_inventory(&self, list_id: &str) -> Option<i64> {
        let key = match list_id {
            EQUIPMENT | INVENTORY => inventory_key(list_id),
            _ => return None,
        };
        int(self.save.players().get(self.player)?.get(key))
    }

    fn add_item(&mut self, list_id: &str, gid: &str) -> bool {
        if gid.is_empty() {
            return false;
        }
        self.selected_inventory(list_id)
            .and_then(|inventory| self.save.add_item(inventory, gid))
            .is_some()
    }

    fn remove_item(&mut self, list_id: &str, index: u32) -> bool {
        let Some(inventory) = self.selected_inventory(list_id) else {
            return false;
        };
        self.save.remove_item_at(inventory, index as usize)
    }
}

impl Document for PlanetCrafterDocument {
    fn view(&self) -> UiTree {
        let titles = TAB_TITLES.iter().map(|t| (*t).to_owned()).collect();
        let selected = self.tab.min(3);
        let page = match selected {
            0 => self.characters_page(),
            1 => self.terra_tokens_page(),
            2 => self.planet_page(),
            _ => self.save_info_page(),
        };
        ui::tabs(TABS, titles, selected, page).into_tree()
    }

    fn update(&mut self, event: Event) {
        let changed = match event {
            Event::TabSelected((_, index)) => {
                self.tab = index;
                false
            }
            Event::Picked((id, value)) => {
                if id == CHARACTER
                    && let Ok(index) = value.parse::<usize>()
                    && index < self.save.players().len()
                {
                    self.player = index;
                }
                false
            }
            Event::NumberChanged((id, value)) => self.set_number(&id, value),
            Event::TextChanged((id, value)) => self.set_text(&id, &value),
            Event::ListAdd((id, gid)) => self.add_item(&id, &gid),
            Event::ListRemove((id, index)) => self.remove_item(&id, index),
            Event::Toggled(_) | Event::Pressed(_) => false,
        };
        self.dirty |= changed;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn serialize(&self) -> Result<Vec<u8>, String> {
        Ok(self.save.to_bytes())
    }

    fn mark_saved(&mut self) {
        self.dirty = false;
    }
}

fn inventory_key(list_id: &str) -> &'static str {
    if list_id == EQUIPMENT {
        "equipmentId"
    } else {
        "inventoryId"
    }
}

/// `name`, or `name (Host)` for the host.
fn player_name(player: &Object) -> String {
    let name = text(player.get("name"));
    if player.get("host").and_then(Value::as_bool) == Some(true) {
        format!("{name} (Host)")
    } else {
        name.to_owned()
    }
}

fn float(value: Option<&Value>) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(0.0)
}

/// How a value shows in a text field: strings as-is, numbers as written.
fn display(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// Sets a float field unless it already holds `value`, so numbers the
/// player did not change keep their exact text (`100.0` stays `100.0`).
fn set_float(object: &mut Object, key: &str, value: f64) -> bool {
    #[allow(clippy::float_cmp)] // exactly the same number means no edit
    if object.get(key).and_then(Value::as_f64) == Some(value) {
        return false;
    }
    Number::from_f64(value).is_some_and(|n| object.set(key, Value::Number(n)))
}

/// Sets an integer field, rounding `value` to a whole number.
fn set_whole(object: &mut Object, key: &str, value: f64) -> bool {
    const MAX_EXACT: f64 = 9_007_199_254_740_992.0;
    let value = value.round();
    if value.abs() > MAX_EXACT {
        return false;
    }
    #[allow(clippy::cast_possible_truncation)] // whole and within ±2^53
    let value = value as i64;
    if save::int(object.get(key)) == Some(value) {
        return false;
    }
    object.set(key, Value::from(value))
}
