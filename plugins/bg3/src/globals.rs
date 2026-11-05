//! The save's `Globals.lsf` and the `NewAge` blob inside it.

use lslib::lsf::LsfFile;
use lslib::newage::NewAge;
use lslib::package::Package;
use lslib::resource::AttributeValue;

pub const GLOBALS: &str = "Globals.lsf";
const NEW_AGE: &str = "NewAge";

#[derive(Debug, Clone)]
pub struct Globals {
    lsf: LsfFile,
    blob: NewAge,
}

impl Globals {
    pub fn read(package: &Package<'_>) -> Result<Self, String> {
        let file = package
            .find_ignore_case(GLOBALS)
            .ok_or("the save has no Globals.lsf")?;
        let lsf = package
            .contents(file)
            .and_then(|bytes| LsfFile::read(&bytes))
            .map_err(|e| format!("cannot read Globals.lsf: {e}"))?;
        let Some(AttributeValue::ScratchBuffer(raw)) = lsf
            .resource
            .region(NEW_AGE)
            .and_then(|region| region.attribute(NEW_AGE))
        else {
            return Err("Globals.lsf has no character data (NewAge)".to_owned());
        };
        let blob = NewAge::read(raw.clone())
            .map_err(|e| format!("cannot read the character data: {e}"))?;
        Ok(Self { lsf, blob })
    }

    pub fn blob(&self) -> &NewAge {
        &self.blob
    }

    pub fn blob_mut(&mut self) -> &mut NewAge {
        &mut self.blob
    }

    /// The save `original` with this `Globals.lsf` in it. Other files keep
    /// their stored bytes, and the package hash the game checks is renewed.
    pub fn repack(&self, original: &[u8]) -> Result<Vec<u8>, String> {
        let package = Package::read(original).map_err(|e| e.to_string())?;
        let mut lsf = self.lsf.clone();
        let region = lsf
            .resource
            .region_mut(NEW_AGE)
            .ok_or("Globals.lsf has no NewAge region")?;
        region.set_attribute(NEW_AGE, AttributeValue::ScratchBuffer(self.blob.to_bytes()));
        let globals = lsf.write().map_err(|e| e.to_string())?;
        let mut builder = package.to_builder().map_err(|e| e.to_string())?;
        builder.hash = true;
        builder.set(GLOBALS, globals);
        builder.build().map_err(|e| e.to_string())
    }
}
