#![allow(dead_code)] // each test file uses a subset

//! A small save in the game's layout: each group wrapped in `\r`, objects
//! separated by `|\n`, two planets, and a trailing empty group.

use std::path::Path;

/// Groups of the minimal save, in file order.
pub const GROUPS: [&[&str]; 11] = [
    &[r#"{"terraTokens":100,"allTimeTerraTokens":200,"unlockedGroups":"A,B"}"#],
    &[
        r#"{"planetId":"Prime","unitOxygenLevel":1.0,"unitHeatLevel":2.5,"unitPressureLevel":0.0,"unitPlantsLevel":44554280960.0,"unitInsectsLevel":0.0,"unitAnimalsLevel":0.0,"unitPurificationLevel":-1.0}"#,
        r#"{"planetId":"Humble","unitOxygenLevel":166326.765625,"unitHeatLevel":25745.0390625,"unitPressureLevel":14958.927734375,"unitPlantsLevel":0.0,"unitInsectsLevel":0.0,"unitAnimalsLevel":0.0,"unitPurificationLevel":-1.0}"#,
    ],
    &[
        r#"{"id":76561198047777538,"name":"Alice","inventoryId":5,"equipmentId":6,"playerGaugeOxygen":100.0,"playerGaugeThirst":37.343666076660159,"playerGaugeHealth":90.0,"playerGaugeToxic":0.0,"host":true}"#,
        r#"{"id":111,"name":"Bob","inventoryId":7,"equipmentId":8,"playerGaugeOxygen":50.0,"playerGaugeThirst":60.0,"playerGaugeHealth":70.0,"playerGaugeToxic":5.0,"host":false}"#,
    ],
    &[
        r#"{"id":10,"gId":"Cobalt","pos":"1,2,3"}"#,
        r#"{"id":11,"gId":"Iron"}"#,
        r#"{"id":12,"gId":"Titanium"}"#,
    ],
    &[
        r#"{"id":5,"woIds":"10,11,99,x","size":4}"#,
        r#"{"id":6,"woIds":"","size":2}"#,
        r#"{"id":7,"woIds":"12","size":4}"#,
    ],
    &[r#"{"craftedObjects":50,"totalSaveFileLoad":0,"totalSaveFileTime":3600}"#],
    &[
        r#"{"stringId":"Msg1","isRead":true}"#,
        r#"{"stringId":"Msg2","isRead":false}"#,
    ],
    &[r#"{"stringId":"StoryEvent1"}"#],
    &[r#"{"saveDisplayName":"TestSave","planetId":"Prime"}"#],
    &[r#"{"layerId":"Layer1"}"#],
    &[r#"{"owner":0,"seed":1004335971}"#],
];

/// Builds a save from groups of object texts.
pub fn build(groups: &[&[&str]]) -> String {
    let mut text = groups
        .iter()
        .map(|objects| format!("\r{}\r", objects.join("|\n")))
        .collect::<Vec<_>>()
        .join("@");
    text.push('@');
    text
}

pub fn minimal() -> String {
    build(&GROUPS)
}

/// `docs/sample-saves/PlanetCrafter-Standard-1.json`, a real save.
pub fn sample() -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/sample-saves/PlanetCrafter-Standard-1.json"),
    )
    .unwrap()
}
