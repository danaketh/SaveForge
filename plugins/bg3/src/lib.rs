//! Baldur's Gate 3 savegames (`.lsv` LSPK packages): edits each character's
//! ability scores and experience, and shows an overview of the save, its party
//! and its mods. The save list groups saves by character through the
//! manifest's `saveGrouping`.

pub mod document;
pub mod globals;
pub mod party;
pub mod save;

pub use document::Bg3Document;
pub use party::Character;
pub use save::{PartyMember, SaveSummary};

#[derive(Debug)]
pub struct Bg3;

/// Detection priority. When several plugins claim a file, the highest wins.
pub const PRIORITY: u32 = 10;

/// LSPK packages from v10 on (BG3 saves are v18) start with this.
const SIGNATURE: &[u8] = b"LSPK";

impl saveforge_sdk::Plugin for Bg3 {
    type Doc = Bg3Document;

    fn detect(_file_name: &str, header: &[u8]) -> Option<u32> {
        header.starts_with(SIGNATURE).then_some(PRIORITY)
    }

    fn open(
        _file_name: &str,
        bytes: Vec<u8>,
        _aux: Vec<saveforge_sdk::AuxFile>,
    ) -> Result<Bg3Document, String> {
        Bg3Document::open(bytes)
    }
}

saveforge_sdk::export_plugin!(Bg3);
