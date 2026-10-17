//! Catalog loading with an offline fallback to the last good copy.

use std::fs;
use std::path::Path;

use saveforge_core::backup::write_atomic;

use crate::transport::{MAX_CATALOG_BYTES, read_url};
use crate::{Catalog, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogSource {
    Online,
    Cached,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatalogLoad {
    pub catalog: Catalog,
    pub source: CatalogSource,
}

/// Fetches and parses the catalog at `url`, caching the raw text at
/// `cache_path`. On any fetch or parse failure, the cache is used instead;
/// with no usable cache, the original error is returned.
pub fn load_catalog(url: &str, cache_path: &Path) -> Result<CatalogLoad, StoreError> {
    let error = match fetch(url) {
        Ok((catalog, text)) => {
            if let Err(e) = write_cache(cache_path, &text) {
                tracing::warn!(path = %cache_path.display(), %e, "could not cache the catalog");
            }
            return Ok(CatalogLoad {
                catalog,
                source: CatalogSource::Online,
            });
        }
        Err(e) => e,
    };
    tracing::warn!(url, %error, "catalog fetch failed; trying the cache");
    match fs::read_to_string(cache_path)
        .ok()
        .and_then(|text| Catalog::parse(&text).ok())
    {
        Some(catalog) => Ok(CatalogLoad {
            catalog,
            source: CatalogSource::Cached,
        }),
        None => Err(error),
    }
}

fn write_cache(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    write_atomic(path, text.as_bytes()).map_err(|e| e.to_string())
}

fn fetch(url: &str) -> Result<(Catalog, String), StoreError> {
    let mut bytes = Vec::new();
    read_url(url, MAX_CATALOG_BYTES, &mut bytes, &mut |_, _| {})?;
    let text =
        String::from_utf8(bytes).map_err(|_| StoreError::InvalidCatalog("not UTF-8".into()))?;
    let catalog = Catalog::parse(&text)?;
    Ok((catalog, text))
}
