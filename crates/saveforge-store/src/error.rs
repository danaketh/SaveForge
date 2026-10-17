use std::io;
use std::path::PathBuf;

use saveforge_core::CoreError;

/// Errors produced by `saveforge-store`.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("invalid catalog: {0}")]
    InvalidCatalog(String),
    #[error("only https:// and file:// URLs are supported: {0}")]
    UnsupportedUrl(String),
    #[error("could not download {url}: {message}")]
    Http { url: String, message: String },
    #[error("I/O error at {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{url} is larger than {limit} bytes")]
    TooLarge { url: String, limit: u64 },
    #[error("package checksum mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("unsafe package entry: {0}")]
    UnsafePackage(String),
    #[error("package expands to more than 256 MiB")]
    PackageTooLarge,
    #[error("invalid package: {0}")]
    BadPackage(String),
    #[error("package is {found}, catalog says {expected}")]
    ManifestMismatch { expected: String, found: String },
    #[error("{0}")]
    Core(#[from] CoreError),
    #[error("invalid plugin id: {0}")]
    InvalidId(String),
}
