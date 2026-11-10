use crate::compression::CompressionMethod;

/// Errors produced by `lslib`.
#[derive(Debug, thiserror::Error)]
pub enum LslibError {
    #[error("not an LSPK package: no valid signature found")]
    NotAPackage,
    #[error("package version v{0} is not supported")]
    UnsupportedVersion(u32),
    #[error("unexpected end of data while reading {0}")]
    Truncated(&'static str),
    #[error("invalid package: {0}")]
    InvalidPackage(String),
    #[error("invalid LSF resource: {0}")]
    InvalidLsf(String),
    #[error("invalid NewAge blob: {0}")]
    InvalidNewAge(String),
    #[error("unsupported compression method {0}")]
    UnsupportedCompression(u8),
    #[error("{method:?} decompression failed: {message}")]
    Decompression {
        method: CompressionMethod,
        message: String,
    },
    #[error("file name {0:?} is longer than 255 bytes")]
    NameTooLong(String),
    #[error("{0:?} is a deletion marker and has no contents")]
    DeletedFile(String),
    #[error("the package spans archive part {0}, which was not supplied")]
    MissingPart(u32),
    #[error("the package exceeds the {limit}-byte size limit of its version")]
    TooLarge { limit: u64 },
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
}

impl LslibError {
    pub(crate) fn decompression(
        method: CompressionMethod,
        message: &dyn std::fmt::Display,
    ) -> Self {
        Self::Decompression {
            method,
            message: message.to_string(),
        }
    }
}

pub type Result<T, E = LslibError> = std::result::Result<T, E>;
