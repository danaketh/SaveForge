//! Detection and opening, as the host calls them.

use crate::common;

use planet_crafter::document::INVENTORY;
use planet_crafter::{LOCALIZATION_AUX, PRIORITY, PlanetCrafter};
use saveforge_sdk::ui::Widget;
use saveforge_sdk::{AuxFile, Document, Plugin};

fn header(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.len().min(512)]
}

#[test]
fn detects_planet_crafter_saves() {
    let sample = common::sample();
    assert_eq!(
        PlanetCrafter::detect("Standard-1.json", header(&sample)),
        Some(PRIORITY)
    );
    assert_eq!(
        PlanetCrafter::detect("STANDARD-1.JSON", header(&sample)),
        Some(PRIORITY)
    );
}

#[test]
fn ignores_other_files() {
    let sample = common::sample();
    assert_eq!(
        PlanetCrafter::detect("Standard-1.sav", header(&sample)),
        None
    );
    assert_eq!(
        PlanetCrafter::detect("Farm.json", br#"{"player":{}}"#),
        None
    );
    assert_eq!(PlanetCrafter::detect("json", header(&sample)), None);
}

#[test]
fn opens_with_the_localization_aux_file() {
    let aux = vec![AuxFile {
        id: LOCALIZATION_AUX.to_owned(),
        bytes: b"GROUP_NAME_Cobalt**Cobalt Ore*x\n".to_vec(),
    }];
    let doc = PlanetCrafter::open("s.json", common::minimal().into_bytes(), aux).unwrap();
    let tree = doc.view();
    let list = tree
        .nodes
        .iter()
        .find(|n| matches!(&n.widget, Widget::List(l) if l.id == INVENTORY))
        .unwrap();
    let first = &tree.nodes[list.children[0] as usize].widget;
    assert!(matches!(first, Widget::Text(t) if t.text == "Cobalt Ore"));
}

#[test]
fn opening_a_broken_save_reports_why() {
    let err = PlanetCrafter::open("s.json", b"{}@{}".to_vec(), vec![]).unwrap_err();
    assert_eq!(err, "expected at least 11 groups separated by '@', found 2");
}

#[test]
fn current_game_saves_with_a_byte_order_mark_open_and_round_trip() {
    let mut bytes = "\u{feff}".as_bytes().to_vec();
    bytes.extend(common::minimal().into_bytes());
    let header = &bytes[..bytes.len().min(512)];
    assert_eq!(
        PlanetCrafter::detect("Standard-1.json", header),
        Some(PRIORITY)
    );
    let doc = PlanetCrafter::open("Standard-1.json", bytes.clone(), vec![]).unwrap();
    assert_eq!(doc.serialize().unwrap(), bytes);
}

#[test]
fn unmodified_sample_serializes_byte_for_byte() {
    let sample = common::sample();
    let doc = PlanetCrafter::open("Standard-1.json", sample.clone(), vec![]).unwrap();
    assert!(!doc.is_dirty());
    assert_eq!(doc.serialize().unwrap(), sample);
}

#[test]
fn manifest_points_at_the_game_save_folder() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../manifest.json")).unwrap();
    let dirs = &manifest["defaultSaveDirectories"];
    assert_eq!(
        dirs["windows"],
        "%USERPROFILE%/AppData/LocalLow/MijuGames/Planet Crafter"
    );
    assert_eq!(
        dirs["linux"],
        "{steam_compat:1284190}/AppData/LocalLow/MijuGames/Planet Crafter"
    );
    assert!(dirs["macos"].is_null());
}

#[test]
fn icon_is_at_most_256_pixels() {
    let icon = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/icon.png")).unwrap();
    let width = u32::from_be_bytes(icon[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(icon[20..24].try_into().unwrap());
    assert!(width <= 256 && height <= 256, "{width}x{height}");
    assert!(icon.len() < 200_000, "{} bytes", icon.len());
}
