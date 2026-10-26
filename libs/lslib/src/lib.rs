//! Larian Studios file formats, ported from Norbyte's
//! [LSLib](https://github.com/Norbyte/lslib) (MIT).
//!
//! Currently covers LSPK packages, the container used by `.pak` game data and
//! `.lsv` savegames, in every version `LSLib` supports (v7 D:OS 1 through v18
//! BG3). Everything works on byte slices so the crate runs inside a sandboxed
//! plugin: the caller reads and writes the files.
//!
//! ```no_run
//! # fn demo(bytes: &[u8]) -> lslib::Result<()> {
//! use lslib::package::Package;
//!
//! let package = Package::read(bytes)?;
//! let globals = package.find_ignore_case("Globals.lsf").expect("not a savegame");
//! let mut contents = package.contents(globals)?.into_owned();
//! contents[0] ^= 0; // edit
//!
//! let mut builder = package.to_builder()?;
//! builder.set("Globals.lsf", contents);
//! let repacked: Vec<u8> = builder.build()?;
//! # Ok(()) }
//! ```

mod binary;
pub mod compression;
mod error;
pub mod package;

pub use error::{LslibError, Result};
