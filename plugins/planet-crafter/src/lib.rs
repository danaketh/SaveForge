//! Planet Crafter save editor (spec §6.1). The save is parsed losslessly
//! ([`format`]), read and edited through [`save`], and shown by
//! [`document::PlanetCrafterDocument`].

pub mod document;
pub mod format;
pub mod localization;
pub mod save;

use std::path::Path;

use saveforge_sdk::{AuxFile, Plugin, export_plugin};

use crate::document::PlanetCrafterDocument;
use crate::localization::Names;
use crate::save::Save;

#[derive(Debug)]
pub struct PlanetCrafter;

/// Detection priority; above the generic sample plugin's 5.
pub const PRIORITY: u32 = 20;

/// Id of the optional localization aux file in `manifest.json`.
pub const LOCALIZATION_AUX: &str = "localization";

impl Plugin for PlanetCrafter {
    type Doc = PlanetCrafterDocument;

    fn detect(file_name: &str, header: &[u8]) -> Option<u32> {
        let is_json = Path::new(file_name)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json"));
        let header = &header[..header.len().min(512)];
        let marker = br#""terraTokens""#;
        (is_json && header.windows(marker.len()).any(|w| w == marker)).then_some(PRIORITY)
    }

    fn open(
        _file_name: &str,
        bytes: Vec<u8>,
        aux: Vec<AuxFile>,
    ) -> Result<PlanetCrafterDocument, String> {
        let save = Save::parse(&bytes)?;
        let names = aux
            .iter()
            .find(|file| file.id == LOCALIZATION_AUX)
            .map_or_else(Names::default, |file| Names::parse(&file.bytes));
        Ok(PlanetCrafterDocument::new(save, names))
    }
}

export_plugin!(PlanetCrafter);
