//! Larian Studios file formats, ported from Norbyte's
//! [LSLib](https://github.com/Norbyte/lslib) (MIT).
//!
//! Covers:
//! - [`package`]: LSPK packages, the container of `.pak` game data and `.lsv`
//!   savegames, in every version `LSLib` supports (v7 D:OS 1 through v18 BG3);
//! - [`lsf`]: LSF binary resources (such as a save's `Globals.lsf`), read into
//!   the [`resource`] tree;
//! - [`newage`]: the BG3 ECS blob in a save's `Globals.lsf` (character
//!   stats and experience live there). Not part of `LSLib`.
//!
//! Everything works on byte slices so the crate runs inside a sandboxed
//! plugin: the caller reads and writes the files.
//!
//! ```no_run
//! # fn demo(bytes: &[u8]) -> lslib::Result<()> {
//! use lslib::lsf::LsfFile;
//! use lslib::package::Package;
//! use lslib::resource::AttributeValue;
//!
//! let package = Package::read(bytes)?;
//! let globals = package.find_ignore_case("Globals.lsf").expect("not a savegame");
//! let mut lsf = LsfFile::read(&package.contents(globals)?)?;
//! if let Some(region) = lsf.resource.region_mut("Story") {
//!     region.set_attribute("Edited", AttributeValue::Bool(true));
//! }
//!
//! let mut builder = package.to_builder()?;
//! builder.set("Globals.lsf", lsf.write()?);
//! let repacked: Vec<u8> = builder.build()?;
//! # Ok(()) }
//! ```

mod binary;
pub mod compression;
mod error;
pub mod lsf;
pub mod newage;
pub mod package;
pub mod resource;

pub use error::{LslibError, Result};
