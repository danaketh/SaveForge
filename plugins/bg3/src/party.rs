//! Characters in the save's `NewAge` blob: ability scores and experience.
//!
//! The game rebuilds each character's scores (`StatsComponent`) on load from
//! the points spent at character creation plus racial and other bonuses, so
//! an ability edit changes those points by the same amount as the score. The
//! score in `StatsComponent` is updated too, to keep the save consistent.
//! Experience is stored directly.

use lslib::newage::NewAge;

pub const STATS: &str = "game.stats.v3.StatsComponent";
pub const EXPERIENCE: &str = "game.experience.v0.ExperienceComponent";
pub const CREATION: &str = "game.character_creation.v1.CharacterCreationStatsComponent";

pub const ABILITIES: [&str; 6] = [
    "Strength",
    "Dexterity",
    "Constitution",
    "Intelligence",
    "Wisdom",
    "Charisma",
];
pub const MIN_SCORE: i32 = 1;
pub const MAX_SCORE: i32 = 30;
pub const MAX_EXPERIENCE: i32 = 10_000_000;

const STATS_SIZE: usize = 36;
/// Eight unknown bytes, then STR..CHA, then the proficiency bonus.
const STATS_ABILITIES: usize = 8;
const EXPERIENCE_SIZE: usize = 12;
const CREATION_SIZE: usize = 88;
/// Name: `u64` offset, `u32` length. Abilities: `u64` start and end of an
/// `i32` array indexed None, STR..CHA.
const CREATION_NAME: usize = 56;
const CREATION_ABILITIES: usize = 72;
const CREATION_ABILITY_COUNT: u64 = 7;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Character {
    pub entity: u32,
    pub name: String,
    /// Current scores, STR..CHA.
    pub abilities: [i32; 6],
    /// Total experience; `None` for characters without any, such as
    /// companions who are not in the party.
    pub experience: Option<i32>,
    /// Where the creation ability array starts in the blob.
    creation_abilities: u64,
}

/// Every character with character-creation data and scores, in blob order.
/// Fails when the blob does not have the layout this editor knows.
pub fn characters(blob: &NewAge) -> Result<Vec<Character>, String> {
    let creation = component(blob, CREATION, CREATION_SIZE)?;
    let stats = component(blob, STATS, STATS_SIZE)?;
    let experience = component(blob, EXPERIENCE, EXPERIENCE_SIZE)?;
    let owners = creation.owners().unwrap_or_default();
    let mut characters = Vec::new();
    for (index, &entity) in owners.iter().enumerate() {
        let Some(scores) = stats
            .element_of(entity)
            .and_then(|i| blob.element(stats, i))
        else {
            continue;
        };
        let element = blob.element(creation, index).ok_or("missing element")?;
        let name_offset = u64_at(element, CREATION_NAME);
        let name_len = u64::from(u32_at(element, CREATION_NAME + 8));
        let name = blob
            .bytes_at(name_offset, name_len)
            .map(|name| String::from_utf8_lossy(name).into_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| format!("Character {}", characters.len() + 1));
        let start = u64_at(element, CREATION_ABILITIES);
        let end = u64_at(element, CREATION_ABILITIES + 8);
        if end.checked_sub(start) != Some(CREATION_ABILITY_COUNT * 4)
            || blob.bytes_at(start, CREATION_ABILITY_COUNT * 4).is_none()
        {
            return Err(format!("{name}'s character creation scores are unreadable"));
        }
        characters.push(Character {
            entity,
            name,
            abilities: std::array::from_fn(|i| i32_at(scores, STATS_ABILITIES + 4 * i)),
            experience: experience
                .element_of(entity)
                .and_then(|i| blob.element(experience, i))
                .map(|xp| i32_at(xp, 0)),
            creation_abilities: start,
        });
    }
    Ok(characters)
}

/// Sets `ability` (0 = STR .. 5 = CHA) of `character` to `score`.
pub fn set_ability(
    blob: &mut NewAge,
    character: &Character,
    ability: usize,
    score: i32,
) -> Result<(), String> {
    if !(MIN_SCORE..=MAX_SCORE).contains(&score) {
        return Err(format!("scores range from {MIN_SCORE} to {MAX_SCORE}"));
    }
    let current = *character.abilities.get(ability).ok_or("no such ability")?;
    let delta = score - current;

    let at = character.creation_abilities + 4 * (ability as u64 + 1);
    let points = blob
        .bytes_at(at, 4)
        .map(|b| i32_at(b, 0))
        .ok_or("character creation scores are unreadable")?;
    let points = points
        .checked_add(delta)
        .ok_or("character creation scores are out of range")?;
    blob.write_at(at, &points.to_le_bytes())
        .map_err(|e| e.to_string())?;

    let stats = blob.component(STATS).ok_or("no stats")?;
    let index = stats
        .element_of(character.entity)
        .ok_or("the character has no stats")?;
    let mut scores = blob.element(stats, index).ok_or("no stats")?.to_vec();
    let offset = STATS_ABILITIES + 4 * ability;
    scores[offset..offset + 4].copy_from_slice(&score.to_le_bytes());
    blob.set_element(STATS, index, &scores)
        .map_err(|e| e.to_string())
}

/// Sets the total experience of `entity`.
pub fn set_experience(blob: &mut NewAge, entity: u32, experience: i32) -> Result<(), String> {
    if !(0..=MAX_EXPERIENCE).contains(&experience) {
        return Err(format!("experience ranges from 0 to {MAX_EXPERIENCE}"));
    }
    let component = blob.component(EXPERIENCE).ok_or("no experience")?;
    let index = component
        .element_of(entity)
        .ok_or("the character has no experience")?;
    let mut element = blob
        .element(component, index)
        .ok_or("no experience")?
        .to_vec();
    // The total is stored twice.
    element[..4].copy_from_slice(&experience.to_le_bytes());
    element[4..8].copy_from_slice(&experience.to_le_bytes());
    blob.set_element(EXPERIENCE, index, &element)
        .map_err(|e| e.to_string())
}

fn component<'a>(
    blob: &'a NewAge,
    name: &str,
    size: usize,
) -> Result<&'a lslib::newage::Component, String> {
    match blob.component(name) {
        Some(component) if component.element_size == size => Ok(component),
        Some(component) => Err(format!(
            "{name} is {} bytes, expected {size}: unsupported game version",
            component.element_size
        )),
        None => Err(format!("the save has no {name}: unsupported game version")),
    }
}

fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    let mut word = [0; 8];
    word.copy_from_slice(&bytes[offset..offset + 8]);
    u64::from_le_bytes(word)
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    let mut word = [0; 4];
    word.copy_from_slice(&bytes[offset..offset + 4]);
    u32::from_le_bytes(word)
}

fn i32_at(bytes: &[u8], offset: usize) -> i32 {
    let mut word = [0; 4];
    word.copy_from_slice(&bytes[offset..offset + 4]);
    i32::from_le_bytes(word)
}
