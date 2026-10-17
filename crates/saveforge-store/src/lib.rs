//! The plugin store (spec §3.3, §5.2): catalog loading and package
//! installation. Everything here is synchronous; the app runs it on blocking
//! tasks.

mod catalog;
mod error;

pub use catalog::{Catalog, CatalogEntry, EntryStatus, Update};
pub use error::StoreError;
