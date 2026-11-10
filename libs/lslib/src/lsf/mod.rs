//! LSF, the binary resource format (`LSLib` `LSFReader.cs`, `LSFWriter.cs`,
//! `LSFCommon.cs`). Savegames store their state in LSF files such as
//! `Globals.lsf` and `LevelCache/*.lsf`.
//!
//! [`LsfFile::read`] returns the [`Resource`] tree along with the
//! [`LsfFormat`] it was stored in, and [`LsfFile::write`] stores it the same
//! way, so an edited file keeps its version and compression.
//!
//! Differences from `LSLib`:
//! - Node key tables are kept whenever present. BG3 saves carry them even
//!   with [`MetadataFormat::NONE`], where `LSLib` drops them.
//! - The writer encodes translated strings with the same version test as the
//!   reader (`LSLib`'s reader and writer disagree for some D:OS 2 files).
//! - Name hash buckets come from a deterministic hash rather than .NET's
//!   per-process one; readers only follow the stored indices.

mod reader;
mod writer;

use crate::Result;
use crate::compression::{CompressionFlags, CompressionLevel, CompressionMethod};
use crate::resource::{PackedVersion, Resource};

/// `"LSOF"`.
const SIGNATURE: [u8; 4] = *b"LSOF";

/// Deepest node nesting accepted on read; real files stay far below it.
const MAX_DEPTH: usize = 256;

/// LSF format versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LsfVersion {
    /// Initial version.
    Initial = 1,
    /// Chunked (LZ4 frame) compression for sections (D:OS EE).
    ChunkedCompress = 2,
    /// Extended node descriptors (D:OS 2).
    ExtendedNodes = 3,
    /// Baldur's Gate 3, renumbered only.
    Bg3 = 4,
    /// BG3 with a 64-bit engine version.
    Bg3ExtendedHeader = 5,
    /// BG3 with node key names.
    Bg3NodeKeys = 6,
    /// BG3 patch 3.
    Bg3Patch3 = 7,
}

impl LsfVersion {
    pub fn from_u32(version: u32) -> Option<Self> {
        Some(match version {
            1 => Self::Initial,
            2 => Self::ChunkedCompress,
            3 => Self::ExtendedNodes,
            4 => Self::Bg3,
            5 => Self::Bg3ExtendedHeader,
            6 => Self::Bg3NodeKeys,
            7 => Self::Bg3Patch3,
            _ => return None,
        })
    }
}

/// Which node/attribute layout a file uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MetadataFormat(pub u32);

impl MetadataFormat {
    /// Compact nodes and attributes.
    pub const NONE: Self = Self(0);
    /// Nodes with sibling links and attributes with explicit value offsets
    /// (from [`LsfVersion::ExtendedNodes`]).
    pub const KEYS_AND_ADJACENCY: Self = Self(1);
    /// Behaves like [`MetadataFormat::NONE`].
    pub const NONE2: Self = Self(2);
}

/// How a resource is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LsfFormat {
    pub version: LsfVersion,
    pub metadata_format: MetadataFormat,
    pub compression: CompressionFlags,
}

impl Default for LsfFormat {
    /// What BG3 writes into its savegames.
    fn default() -> Self {
        Self {
            version: LsfVersion::Bg3Patch3,
            metadata_format: MetadataFormat::NONE,
            compression: CompressionFlags::new(CompressionMethod::Lz4, CompressionLevel::Default),
        }
    }
}

impl LsfFormat {
    /// Whether the long node and attribute entries are used.
    fn long_entries(self) -> bool {
        self.version >= LsfVersion::ExtendedNodes
            && self.metadata_format == MetadataFormat::KEYS_AND_ADJACENCY
    }

    /// Whether sections other than the string table are LZ4 frames.
    fn chunked(self) -> bool {
        self.version >= LsfVersion::ChunkedCompress
    }
}

/// An LSF resource with its storage format.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LsfFile {
    pub format: LsfFormat,
    pub resource: Resource,
}

impl LsfFile {
    pub fn read(data: &[u8]) -> Result<Self> {
        reader::read(data)
    }

    pub fn write(&self) -> Result<Vec<u8>> {
        writer::write(self)
    }
}

/// Whether translated strings carry a version instead of inline text: always
/// from [`LsfVersion::Bg3`], and from engine 4.0.0.26 in older files.
fn translated_string_has_version(version: LsfVersion, engine: PackedVersion) -> bool {
    version >= LsfVersion::Bg3
        || engine.major > 4
        || (engine.major == 4 && engine.revision > 0)
        || (engine.major == 4 && engine.revision == 0 && engine.build >= 0x1a)
}

/// The engine version used for format decisions. Merged files lack one; `LSLib`
/// assumes 4.0.9.0 for them.
fn effective_engine(version: LsfVersion, engine: PackedVersion) -> PackedVersion {
    if version >= LsfVersion::Bg3ExtendedHeader && engine.major == 0 {
        PackedVersion {
            major: 4,
            minor: 0,
            revision: 9,
            build: 0,
        }
    } else {
        engine
    }
}

/// The on-disk sizes of the sections, in file order (keys last).
#[derive(Debug, Clone, Copy, Default)]
struct Section {
    uncompressed: u32,
    on_disk: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct Metadata {
    strings: Section,
    keys: Section,
    nodes: Section,
    attributes: Section,
    values: Section,
    compression: CompressionFlags,
    format: MetadataFormat,
}
