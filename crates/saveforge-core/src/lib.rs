//! Domain logic for the `SaveForge` host: plugin manifests, path tokens, Steam
//! library lookup, save discovery, safe file writes and settings.

mod error;
pub mod manifest;
pub mod platform;
pub mod steam;
pub mod tokens;

pub use error::{CoreError, Result};
