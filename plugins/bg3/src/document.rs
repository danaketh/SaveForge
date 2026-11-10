//! The editor: each character's ability scores and experience, plus a
//! read-only overview of the save.

use lslib::package::Package;
use saveforge_sdk::Document;
use saveforge_sdk::ui::{self, El, Event, UiTree};

use crate::globals::Globals;
use crate::party::{self, ABILITIES, Character, MAX_EXPERIENCE, MAX_SCORE, MIN_SCORE};
use crate::save::{PartyMember, SaveSummary};

/// Widget ids.
pub const TABS: &str = "tabs";
pub const CHARACTER: &str = "character";
/// Followed by the ability index, 0 (STR) to 5 (CHA).
pub const ABILITY_PREFIX: &str = "ability.";
pub const EXPERIENCE: &str = "experience";

pub const TAB_TITLES: [&str; 2] = ["Characters", "Save"];

#[derive(Debug)]
pub struct Bg3Document {
    summary: SaveSummary,
    bytes: Vec<u8>,
    /// The character editor, or why this save cannot be edited.
    editor: Result<Editor, String>,
    tab: u32,
    dirty: bool,
    /// The last rejected edit.
    error: Option<String>,
}

#[derive(Debug)]
struct Editor {
    globals: Globals,
    characters: Vec<Character>,
    selected: usize,
}

impl Bg3Document {
    /// Fails only when the save cannot be read at all; a save whose character
    /// data this editor does not understand still opens, read-only.
    pub fn open(bytes: Vec<u8>) -> Result<Self, String> {
        let summary = SaveSummary::read(&bytes)?;
        let editor = Package::read(&bytes)
            .map_err(|e| e.to_string())
            .and_then(|package| Globals::read(&package))
            .and_then(|globals| {
                let characters = party::characters(globals.blob())?;
                Ok(Editor {
                    globals,
                    characters,
                    selected: 0,
                })
            });
        Ok(Self {
            summary,
            bytes,
            editor,
            tab: 0,
            dirty: false,
            error: None,
        })
    }

    pub fn summary(&self) -> &SaveSummary {
        &self.summary
    }

    /// The characters, or why they cannot be edited.
    pub fn characters(&self) -> Result<&[Character], &str> {
        match &self.editor {
            Ok(editor) => Ok(&editor.characters),
            Err(reason) => Err(reason),
        }
    }

    fn characters_page(&self) -> El {
        let editor = match &self.editor {
            Ok(editor) => editor,
            Err(reason) => {
                return ui::column(vec![ui::error_text(format!(
                    "Characters cannot be edited: {reason}"
                ))]);
            }
        };
        let Some(character) = editor.characters.get(editor.selected) else {
            return ui::column(vec![ui::muted("This save has no characters.")]);
        };
        let options = editor
            .characters
            .iter()
            .enumerate()
            .map(|(i, c)| ui::option(i.to_string(), c.name.as_str()))
            .collect();
        let mut page = vec![
            ui::pick(
                CHARACTER,
                "Character",
                options,
                Some(editor.selected.to_string()),
            )
            .into(),
        ];
        if let Some(error) = &self.error {
            page.push(ui::error_text(error.as_str()));
        }

        let mut abilities = vec![ui::muted(
            "Racial and other bonuses stay on top of these scores.",
        )];
        abilities.extend(ABILITIES.iter().zip(character.abilities).enumerate().map(
            |(i, (label, score))| {
                ui::number(format!("{ABILITY_PREFIX}{i}"), *label, f64::from(score))
                    .range(f64::from(MIN_SCORE), f64::from(MAX_SCORE))
                    .decimals(0)
                    .step(1.0)
                    .into()
            },
        ));
        page.push(ui::section("Abilities", abilities));

        let experience = match character.experience {
            Some(xp) => ui::number(EXPERIENCE, "Total XP", f64::from(xp))
                .range(0.0, f64::from(MAX_EXPERIENCE))
                .decimals(0)
                .step(100.0)
                .help("The game levels the character up from this.")
                .into(),
            None => ui::muted("Only party members have experience."),
        };
        page.push(ui::section("Experience", vec![experience]));
        ui::column(page)
    }

    fn save_page(&self) -> El {
        ui::column(vec![
            self.save_section(),
            self.party_section(),
            self.mods_section(),
        ])
    }

    fn save_section(&self) -> El {
        let s = &self.summary;
        let field = |id: &str, label: &str, value: &str| -> El {
            ui::text_input(id, label, value).read_only(true).into()
        };
        ui::section(
            "Save",
            vec![
                field("info.saveName", "Save name", &s.save_name),
                field(
                    "info.leader",
                    "Character",
                    s.leader.as_deref().unwrap_or_default(),
                ),
                field("info.level", "Level", &s.level),
                field("info.difficulty", "Difficulty", &s.difficulty.join(", ")),
                field("info.gameVersion", "Game version", &s.game_version),
            ],
        )
    }

    fn party_section(&self) -> El {
        let rows = if self.summary.party.is_empty() {
            vec![ui::muted("No party members recorded")]
        } else {
            self.summary.party.iter().map(member_row).collect()
        };
        ui::section("Party", rows)
    }

    fn mods_section(&self) -> El {
        let rows = if self.summary.mods.is_empty() {
            vec![ui::muted("No mods")]
        } else {
            self.summary.mods.iter().map(ui::text).collect()
        };
        ui::section("Mods", rows)
    }

    fn set_number(&mut self, id: &str, value: f64) {
        let Ok(editor) = &mut self.editor else {
            return;
        };
        let Some(character) = editor.characters.get(editor.selected).cloned() else {
            return;
        };
        let Some(value) = whole(value) else {
            return;
        };
        let blob = editor.globals.blob_mut();
        let result = if id == EXPERIENCE {
            party::set_experience(blob, character.entity, value)
        } else if let Some(ability) = id
            .strip_prefix(ABILITY_PREFIX)
            .and_then(|i| i.parse::<usize>().ok())
            .filter(|&i| i < ABILITIES.len())
        {
            if character.abilities[ability] == value {
                return;
            }
            party::set_ability(blob, &character, ability, value)
        } else {
            return;
        };
        match result.and_then(|()| party::characters(editor.globals.blob())) {
            Ok(characters) => {
                editor.characters = characters;
                self.dirty = true;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
}

/// `value` as a whole `i32`, if it is one after rounding.
fn whole(value: f64) -> Option<i32> {
    let value = value.round();
    (value.is_finite() && value >= f64::from(i32::MIN) && value <= f64::from(i32::MAX)).then(|| {
        #[expect(clippy::cast_possible_truncation, reason = "range checked above")]
        let value = value as i32;
        value
    })
}

fn member_row(member: &PartyMember) -> El {
    let level = member
        .level
        .map_or_else(String::new, |level| format!(", level {level}"));
    ui::row(vec![
        ui::text(member.origin.as_str()),
        ui::muted(format!("{} {}{level}", member.race, member.classes)),
    ])
}

impl Document for Bg3Document {
    fn view(&self) -> UiTree {
        let titles = TAB_TITLES.iter().map(|t| (*t).to_owned()).collect();
        let selected = self.tab.min(1);
        let page = if selected == 0 {
            self.characters_page()
        } else {
            self.save_page()
        };
        ui::tabs(TABS, titles, selected, ui::scrollable(page)).into_tree()
    }

    fn update(&mut self, event: Event) {
        match event {
            Event::TabSelected((id, index)) if id == TABS => self.tab = index,
            Event::Picked((id, value)) if id == CHARACTER => {
                if let Ok(editor) = &mut self.editor
                    && let Ok(index) = value.parse::<usize>()
                    && index < editor.characters.len()
                {
                    editor.selected = index;
                    self.error = None;
                }
            }
            Event::NumberChanged((id, value)) => self.set_number(&id, value),
            _ => {}
        }
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn serialize(&self) -> Result<Vec<u8>, String> {
        match &self.editor {
            Ok(editor) if self.dirty => editor.globals.repack(&self.bytes),
            _ => Ok(self.bytes.clone()),
        }
    }

    fn mark_saved(&mut self) {
        self.dirty = false;
    }
}
