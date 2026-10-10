use std::path::{Path, PathBuf};

/// Errors produced by `saveforge-core`.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("I/O error at {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("JSON error at {}: {source}", path.display())]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid manifest: {0}")]
    Manifest(String),
    #[error("incompatible plugin: {0}")]
    Incompatible(String),
}

impl CoreError {
    pub(crate) fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

pub type Result<T, E = CoreError> = std::result::Result<T, E>;
