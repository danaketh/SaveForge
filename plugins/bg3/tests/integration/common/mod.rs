//! Synthetic saves: a package with `SaveInfo.json`, `meta.lsf` and a
//! `Globals.lsf` holding a `NewAge` blob laid out like the game's.

use lslib::lsf::LsfFile;
use lslib::newage::HEADER_SIZE;
use lslib::package::{PackageBuilder, PackageVersion};
use lslib::resource::{AttributeValue, Node, Resource};

pub const SAVE_INFO: &str = r#"{
   "Active Party" : { "Characters" : [
      { "Classes" : [ { "Main" : "Wizard", "Sub" : "" } ], "Level" : 2,
        "Origin" : "Generic", "Race" : "Kalashtar" },
      { "Classes" : [ { "Main" : "Cleric", "Sub" : "Trickery" }, { "Main" : "Rogue", "Sub" : "" } ],
        "Level" : 5, "Origin" : "Shadowheart", "Race" : "HalfElf_High" }
   ] },
   "Current Level" : "WLD_Main_A",
   "Difficulty" : [ "DifficultyHard", "RulesetLarian" ],
   "Game Version" : "4.1.1.7631656",
   "Save Name" : "Dank Crypt - 2h 22m"
}"#;

/// A character in the synthetic blob.
pub struct Spec {
    pub name: &'static str,
    /// Points spent at character creation, STR..CHA.
    pub points: [i32; 6],
    /// Scores, STR..CHA.
    pub scores: [i32; 6],
    pub experience: Option<i32>,
}

/// Gale (in the party) and Lae'zel (in camp, no experience).
pub fn party() -> Vec<Spec> {
    vec![
        Spec {
            name: "Gale",
            points: [0, 4, 5, 7, 6, 2],
            scores: [8, 12, 14, 17, 14, 10],
            experience: Some(673),
        },
        Spec {
            name: "Lae'zel",
            points: [7, 6, 5, 0, 2, 4],
            scores: [17, 14, 14, 8, 10, 12],
            experience: None,
        },
    ]
}

fn put(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

struct Component {
    name: &'static str,
    size: u32,
    elements: Vec<Vec<u8>>,
    owners: Option<Vec<u32>>,
}

/// A blob with one entity per character plus one without components. Layout:
/// level, entities, owner indices, owner maps, names and creation scores,
/// component data, component names, table.
pub fn newage(characters: &[Spec]) -> Vec<u8> {
    let mut components = components(characters);
    let owner_bytes: usize = components
        .iter()
        .filter_map(|c| c.owners.as_ref())
        .map(|owners| owners.len() * 4)
        .sum();
    let maps = components.iter().filter(|c| c.owners.is_some()).count() + 1;
    let entity_count = characters.len() + 1;
    let heap_start = 32 + 16 * entity_count + owner_bytes + 32 * maps;
    let (heap, creation) = creation(characters, heap_start);
    components[1].elements = creation;
    assemble(&components, entity_count, &heap)
}

fn components(characters: &[Spec]) -> Vec<Component> {
    let all: Vec<u32> = (0..u32::try_from(characters.len()).unwrap()).collect();
    let with_xp: Vec<u32> = all
        .iter()
        .copied()
        .filter(|&e| characters[e as usize].experience.is_some())
        .collect();
    let stats = characters
        .iter()
        .map(|c| {
            let mut out = vec![4, 0, 64, 56, 0, 0, 0, 0];
            for score in c.scores.into_iter().chain([2]) {
                put32(&mut out, score);
            }
            out
        })
        .collect();
    let experience = with_xp
        .iter()
        .map(|&e| {
            let xp = characters[e as usize].experience.unwrap();
            let mut out = Vec::new();
            for value in [xp, xp, 0] {
                put32(&mut out, value);
            }
            out
        })
        .collect();
    vec![
        Component {
            name: "core.v0.Level",
            size: 32,
            elements: vec![vec![0; 32]],
            owners: None,
        },
        Component {
            name: "game.character_creation.v1.CharacterCreationStatsComponent",
            size: 88,
            elements: Vec::new(),
            owners: Some(all.clone()),
        },
        Component {
            name: "game.stats.v3.StatsComponent",
            size: 36,
            elements: stats,
            owners: Some(all),
        },
        Component {
            name: "game.experience.v0.ExperienceComponent",
            size: 12,
            elements: experience,
            owners: Some(with_xp),
        },
    ]
}

/// Names and creation scores (placed at `heap_start`), and the creation
/// elements pointing at them.
fn creation(characters: &[Spec], heap_start: usize) -> (Vec<u8>, Vec<Vec<u8>>) {
    let mut heap = Vec::new();
    let mut elements = Vec::new();
    for c in characters {
        let name_at = (heap_start + heap.len()) as u64;
        heap.extend_from_slice(c.name.as_bytes());
        let scores_at = (heap_start + heap.len()) as u64;
        for points in [0].into_iter().chain(c.points) {
            put32(&mut heap, points);
        }
        let mut element = vec![0xab; 56];
        put(&mut element, name_at);
        element.extend_from_slice(&u32::try_from(c.name.len()).unwrap().to_le_bytes());
        element.extend_from_slice(&0x7ff4u32.to_le_bytes());
        put(&mut element, scores_at);
        put(&mut element, scores_at + 28);
        elements.push(element);
    }
    (heap, elements)
}

fn assemble(components: &[Component], entity_count: usize, heap: &[u8]) -> Vec<u8> {
    let mut body = vec![0; 32];
    for e in 0..entity_count {
        body.extend_from_slice(&[u8::try_from(e).unwrap() + 1; 16]);
    }
    let entities_end = body.len() as u64;
    let mut maps = Vec::new();
    for (i, component) in components.iter().enumerate() {
        if let Some(owners) = &component.owners {
            let start = body.len() as u64;
            for owner in owners {
                body.extend_from_slice(&owner.to_le_bytes());
            }
            maps.push([start, body.len() as u64, i as u64, owners.len() as u64]);
        }
    }
    maps.push([u64::MAX; 4]);
    let maps_start = body.len() as u64;
    for field in maps.iter().flatten() {
        put(&mut body, *field);
    }
    let maps_end = body.len() as u64;
    body.extend_from_slice(heap);

    let mut level = Vec::new();
    for field in [32, entities_end, maps_start, maps_end] {
        put(&mut level, field);
    }
    body[..32].copy_from_slice(&level);

    let mut data_offsets = vec![0];
    for component in &components[1..] {
        data_offsets.push(body.len() as u64);
        for element in &component.elements {
            assert_eq!(element.len(), component.size as usize);
            body.extend_from_slice(element);
        }
    }
    let block_offset = body.len() as u64;
    let mut names = vec![0u8];
    let mut name_offsets = Vec::new();
    for component in components {
        name_offsets.push(names.len() as u64);
        names.extend_from_slice(component.name.as_bytes());
    }
    body.extend_from_slice(&names);
    for (i, component) in components.iter().enumerate() {
        put(&mut body, name_offsets[i]);
        body.extend_from_slice(&u32::try_from(component.name.len()).unwrap().to_le_bytes());
        body.extend_from_slice(&101u32.to_le_bytes());
        put(&mut body, 0);
        body.extend_from_slice(&component.size.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        put(&mut body, component.elements.len() as u64);
        put(&mut body, data_offsets[i]);
    }

    let mut out = b"LSMF".to_vec();
    out.extend_from_slice(&0x0800_0101u32.to_le_bytes());
    put(&mut out, 0);
    put(&mut out, block_offset);
    put(&mut out, body.len() as u64 - block_offset);
    out.extend_from_slice(&u32::try_from(names.len()).unwrap().to_le_bytes());
    out.extend_from_slice(&u16::try_from(components.len()).unwrap().to_le_bytes());
    out.extend_from_slice(&[0; 10]);
    assert_eq!(out.len(), HEADER_SIZE);
    out.extend_from_slice(&body);
    out
}

fn lsf(regions: Vec<Node>) -> Vec<u8> {
    LsfFile {
        resource: Resource {
            regions,
            ..Resource::default()
        },
        ..LsfFile::default()
    }
    .write()
    .unwrap()
}

fn module(name: &str) -> Node {
    let mut node = Node::new("ModuleShortDesc");
    node.set_attribute("Name", AttributeValue::LSString(name.to_owned()));
    node
}

pub fn meta_lsf() -> Vec<u8> {
    let mut mods = Node::new("Mods");
    mods.children = vec![module("GustavX"), module("ImpUI (ImprovedUI)")];
    let mut settings = Node::new("ModuleSettings");
    settings.children.push(mods);
    let mut meta = Node::new("MetaData");
    meta.set_attribute("LeaderName", AttributeValue::LSString("Dan".to_owned()));
    meta.children.push(settings);
    let mut region = Node::new("MetaData");
    region.children.push(meta);
    lsf(vec![region])
}

pub fn globals_lsf(blob: Vec<u8>) -> Vec<u8> {
    let mut region = Node::new("NewAge");
    region.set_attribute("NewAge", AttributeValue::ScratchBuffer(blob));
    let mut other = Node::new("Story");
    other.set_attribute("Kept", AttributeValue::Bool(true));
    lsf(vec![region, other])
}

/// A hashed v18 package like the game writes.
pub fn save(with_meta: bool, globals: Option<Vec<u8>>) -> Vec<u8> {
    let mut builder = PackageBuilder::new(PackageVersion::V18);
    builder.compression = lslib::compression::CompressionMethod::Zstd;
    builder.hash = true;
    builder.add("SaveInfo.json", SAVE_INFO.as_bytes().to_vec());
    if with_meta {
        builder.add("meta.lsf", meta_lsf());
    }
    if let Some(globals) = globals {
        builder.add("Globals.lsf", globals);
    }
    builder.add("StorySave.bin", b"story".repeat(100));
    builder.build().unwrap()
}

/// A save with [`party`] in its `Globals.lsf`.
pub fn editable_save() -> Vec<u8> {
    save(true, Some(globals_lsf(newage(&party()))))
}
