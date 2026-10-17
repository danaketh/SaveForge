//! The plugin store (spec §3.3, §5.2): catalog loading and package
//! installation. Everything here is synchronous; the app runs it on blocking
//! tasks.

pub mod cache;
mod catalog;
mod error;
pub mod transport;

pub use catalog::{Catalog, CatalogEntry, EntryStatus, Update};
pub use error::StoreError;
