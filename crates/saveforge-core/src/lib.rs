//! Domain logic for the `SaveForge` host: plugin manifests, path tokens, Steam
//! library lookup, save discovery, safe file writes and settings.

pub mod backup;
pub mod discovery;
mod error;
pub mod manifest;
pub mod paths;
pub mod platform;
pub mod settings;
pub mod steam;
pub mod tokens;

pub use error::{CoreError, Result};
