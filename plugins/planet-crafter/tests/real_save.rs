//! Real saves (spec §8): `SAVEFORGE_PC_SAVE=<path> cargo test -p
//! planet-crafter -- --ignored`.

use planet_crafter::save::{Save, int};

fn real_save() -> Vec<u8> {
    let path = std::env::var("SAVEFORGE_PC_SAVE")
        .expect("set SAVEFORGE_PC_SAVE to a Planet Crafter save to run this test");
    std::fs::read(path).unwrap()
}

#[test]
#[ignore = "needs SAVEFORGE_PC_SAVE"]
fn real_save_round_trips_byte_for_byte() {
    let bytes = real_save();
    assert_eq!(Save::parse(&bytes).unwrap().to_bytes(), bytes);
}

#[test]
#[ignore = "needs SAVEFORGE_PC_SAVE"]
fn real_save_players_have_inventories() {
    let save = Save::parse(&real_save()).unwrap();
    assert_ne!(save.players().len(), 0);
    for player in save.players() {
        let inventory = int(player.get("inventoryId")).unwrap();
        assert!(save.inventory_items(inventory).is_some());
    }
}
