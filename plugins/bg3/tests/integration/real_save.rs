//! Real saves: `SAVEFORGE_BG3_SAVE=<path.lsv> cargo test -p bg3 -- --ignored`.

use bg3::document::{ABILITY_PREFIX, EXPERIENCE};
use bg3::{Bg3, SaveSummary};
use lslib::package::Package;
use saveforge_sdk::ui::Event;
use saveforge_sdk::{Document as _, Plugin as _};

fn real_save() -> Vec<u8> {
    let path = std::env::var("SAVEFORGE_BG3_SAVE")
        .expect("set SAVEFORGE_BG3_SAVE to a Baldur's Gate 3 .lsv to run this test");
    std::fs::read(path).unwrap()
}

#[test]
#[ignore = "needs SAVEFORGE_BG3_SAVE"]
fn real_save_has_a_name_and_party() {
    let summary = SaveSummary::read(&real_save()).unwrap();
    assert_ne!(summary.save_name, "");
    assert_ne!(summary.party, []);
    assert_ne!(summary.leader, None);
}

#[test]
#[ignore = "needs SAVEFORGE_BG3_SAVE"]
fn real_save_edits_round_trip() {
    let original = real_save();
    let mut doc = Bg3::open("save.lsv", original.clone(), vec![]).unwrap();
    let characters = doc.characters().unwrap().to_vec();
    assert_ne!(characters, []);
    let first = &characters[0];
    let strength = if first.abilities[0] < 30 {
        first.abilities[0] + 1
    } else {
        29
    };
    doc.update(Event::NumberChanged((
        format!("{ABILITY_PREFIX}0"),
        f64::from(strength),
    )));
    if first.experience.is_some() {
        doc.update(Event::NumberChanged((EXPERIENCE.to_owned(), 12_345.0)));
    }

    let started = std::time::Instant::now();
    let bytes = doc.serialize().unwrap();
    eprintln!("serialize took {:?}", started.elapsed());
    // Unchanged files keep their compressed bytes, so the size barely moves.
    assert!(bytes.len() < original.len() + original.len() / 10);
    assert_ne!(Package::read(&bytes).unwrap().md5(), [0; 16]);

    let reopened = Bg3::open("save.lsv", bytes, vec![]).unwrap();
    let edited = &reopened.characters().unwrap()[0];
    assert_eq!(edited.name, first.name);
    assert_eq!(edited.abilities[0], strength);
    assert_eq!(edited.abilities[1..], first.abilities[1..]);
    if first.experience.is_some() {
        assert_eq!(edited.experience, Some(12_345));
    }
}
