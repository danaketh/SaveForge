//! Detection and the read-only overview (`SaveInfo.json`, `meta.lsf`).

use bg3::document::TABS;
use bg3::{Bg3, PRIORITY, PartyMember, SaveSummary};
use lslib::package::{PackageBuilder, PackageVersion};
use saveforge_sdk::ui::{Event, Widget};
use saveforge_sdk::{Document as _, Plugin as _};

use crate::common::{editable_save, save};

#[test]
fn detects_lspk_packages() {
    let bytes = save(true, None);
    let header = &bytes[..bytes.len().min(512)];
    assert_eq!(Bg3::detect("save.lsv", header), Some(PRIORITY));
    assert_eq!(Bg3::detect("save.lsv", br#"{"name":"Ada"}"#), None);
}

#[test]
fn reads_save_info_and_meta() {
    let summary = SaveSummary::read(&save(true, None)).unwrap();
    assert_eq!(
        summary,
        SaveSummary {
            save_name: "Dank Crypt - 2h 22m".to_owned(),
            leader: Some("Dan".to_owned()),
            game_version: "4.1.1.7631656".to_owned(),
            level: "WLD_Main_A".to_owned(),
            difficulty: vec!["DifficultyHard".to_owned(), "RulesetLarian".to_owned()],
            party: vec![
                PartyMember {
                    origin: "Generic".to_owned(),
                    race: "Kalashtar".to_owned(),
                    classes: "Wizard".to_owned(),
                    level: Some(2),
                },
                PartyMember {
                    origin: "Shadowheart".to_owned(),
                    race: "HalfElf_High".to_owned(),
                    classes: "Cleric (Trickery), Rogue".to_owned(),
                    level: Some(5),
                },
            ],
            mods: vec!["GustavX".to_owned(), "ImpUI (ImprovedUI)".to_owned()],
        }
    );
}

#[test]
fn meta_is_optional() {
    let summary = SaveSummary::read(&save(false, None)).unwrap();
    assert_eq!(summary.leader, None);
    assert_eq!(summary.mods, Vec::<String>::new());
    assert_eq!(summary.party.len(), 2);
}

#[test]
fn rejects_non_saves() {
    assert!(Bg3::open("x.lsv", b"not a package".to_vec(), vec![]).is_err());
    let empty = PackageBuilder::new(PackageVersion::V18).build().unwrap();
    let err = Bg3::open("x.lsv", empty, vec![]).unwrap_err();
    assert!(err.contains("SaveInfo.json"), "{err}");
}

#[test]
fn save_tab_is_read_only() {
    let mut doc = Bg3::open("save.lsv", editable_save(), vec![]).unwrap();
    doc.update(Event::TabSelected((TABS.to_owned(), 1)));
    let tree = doc.view();
    let fields: Vec<(&str, bool)> = tree
        .nodes
        .iter()
        .filter_map(|node| match &node.widget {
            Widget::Field(field) => Some((field.label.as_str(), field.read_only)),
            _ => None,
        })
        .collect();
    assert_eq!(
        fields,
        [
            ("Save name", true),
            ("Character", true),
            ("Level", true),
            ("Difficulty", true),
            ("Game version", true),
        ]
    );
    let sections: Vec<&str> = tree
        .nodes
        .iter()
        .filter_map(|node| match &node.widget {
            Widget::Section(title) => Some(title.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(sections, ["Save", "Party", "Mods"]);

    doc.update(Event::TextChanged((
        "info.saveName".to_owned(),
        "Edited".to_owned(),
    )));
    assert!(!doc.is_dirty());
}
