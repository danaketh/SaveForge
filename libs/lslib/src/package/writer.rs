//! Writing LSPK packages (`LSLib` `PackageWriter.cs`).

use md5::{Digest, Md5};

use super::format::{Entry, EntryLayout, Header, HeaderLayout, SIGNATURE};
use super::{PackageFlags, PackageVersion};
use crate::binary::put_u32;
use crate::compression::{self, CompressionFlags, CompressionLevel, CompressionMethod};
use crate::{LslibError, Result};

/// Extensions `LSLib` always stores uncompressed (already compressed media).
const STORED_EXTENSIONS: [&str; 5] = ["gts", "gtp", "wem", "bnk", "bk2"];

/// A file to pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputFile {
    /// The path inside the package; `\` is written as `/`.
    pub path: String,
    pub data: Vec<u8>,
    /// Overrides the builder's compression for this file.
    pub compression: Option<(CompressionMethod, CompressionLevel)>,
}

/// Builds a package in memory. Packages that would need more than one archive
/// part (over 1 GiB up to v15, 4 GiB after) are rejected.
///
/// `LSLib` packs savegames (`.lsv`) with [`CompressionMethod::Zlib`]; to edit an
/// existing save, start from [`super::Package::to_builder`] instead, which
/// keeps each file's original compression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageBuilder {
    pub version: PackageVersion,
    pub compression: CompressionMethod,
    pub level: CompressionLevel,
    pub flags: PackageFlags,
    pub priority: u8,
    /// Compute the archive MD5. Packages before v15 always carry one.
    pub hash: bool,
    pub files: Vec<InputFile>,
}

impl Default for PackageBuilder {
    fn default() -> Self {
        Self::new(PackageVersion::V18)
    }
}

impl PackageBuilder {
    /// An empty, uncompressed package of `version`.
    pub fn new(version: PackageVersion) -> Self {
        Self {
            version,
            compression: CompressionMethod::None,
            level: CompressionLevel::Default,
            flags: PackageFlags::default(),
            priority: 0,
            hash: false,
            files: Vec::new(),
        }
    }

    /// Appends a file using the builder's compression.
    pub fn add(&mut self, path: impl Into<String>, data: Vec<u8>) -> &mut Self {
        self.files.push(InputFile {
            path: path.into(),
            data,
            compression: None,
        });
        self
    }

    /// Replaces the contents of the file at `path` (ignoring ASCII case),
    /// keeping its position and compression, or appends it if absent.
    pub fn set(&mut self, path: &str, data: Vec<u8>) -> &mut Self {
        match self
            .files
            .iter_mut()
            .find(|f| f.path.eq_ignore_ascii_case(path))
        {
            Some(file) => file.data = data,
            None => {
                self.add(path, data);
            }
        }
        self
    }

    /// Writes the package.
    pub fn build(&self) -> Result<Vec<u8>> {
        if self.flags.contains(PackageFlags::SOLID) {
            return Err(LslibError::Unsupported("writing solid packages"));
        }
        let (header_layout, entry_layout) = self.version.layouts();
        let out = match self.version {
            PackageVersion::V7 | PackageVersion::V9 | PackageVersion::V10 => {
                self.build_legacy(header_layout, entry_layout)?
            }
            PackageVersion::V13 => self.build_v13(header_layout, entry_layout)?,
            PackageVersion::V15 | PackageVersion::V16 | PackageVersion::V18 => {
                self.build_v15(header_layout, entry_layout)?
            }
        };
        let limit = self.version.max_package_size();
        if out.len() as u64 > limit {
            return Err(LslibError::TooLarge { limit });
        }
        Ok(out)
    }

    fn header(&self) -> Header {
        Header {
            version: self.version as u32,
            flags: self.flags.0,
            priority: self.priority,
            num_parts: 1,
            ..Header::default()
        }
    }

    /// v7, v9 and v10: header and plain file list up front, data after.
    fn build_legacy(
        &self,
        header_layout: HeaderLayout,
        entry_layout: EntryLayout,
    ) -> Result<Vec<u8>> {
        let mut h = self.header();
        h.num_files = to_u32(self.files.len(), self.version)?;
        h.file_list_size = to_u32(entry_layout.size() * self.files.len(), self.version)?;

        let signature_len = if self.version >= PackageVersion::V10 {
            4
        } else {
            0
        };
        let data_offset = (header_layout.size() + h.file_list_size as usize + signature_len) as u64;
        let data_offset = data_offset.next_multiple_of(self.version.padding_size());
        h.data_offset = to_u32(data_offset, self.version)?;

        // Reserve the headers; they are written once the data is packed.
        let mut out = vec![0; h.data_offset as usize];
        let mut entries = self.pack_files(&mut out)?;
        h.md5 = self.archive_hash();

        let mut front = Vec::with_capacity(h.data_offset as usize);
        if signature_len > 0 {
            put_u32(&mut front, SIGNATURE);
        }
        header_layout.write(&h, &mut front);
        for entry in &mut entries {
            entry.offset_in_file -= data_offset;
            // These versions have no compression level bits.
            entry.flags.0 &= 0x0f;
            entry_layout.write(entry, &mut front)?;
        }
        out[..front.len()].copy_from_slice(&front);
        Ok(out)
    }

    /// v13: data first, then the compressed file list, the header, its size
    /// and the signature at the very end.
    fn build_v13(&self, header_layout: HeaderLayout, entry_layout: EntryLayout) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let entries = self.pack_files(&mut out)?;

        let mut h = self.header();
        h.file_list_offset = out.len() as u64;
        self.write_file_list(&entries, entry_layout, &mut out)?;
        h.file_list_size = to_u32(out.len() as u64 - h.file_list_offset, self.version)?;
        h.md5 = self.archive_hash();

        header_layout.write(&h, &mut out);
        put_u32(&mut out, to_u32(8 + header_layout.size(), self.version)?);
        put_u32(&mut out, SIGNATURE);
        Ok(out)
    }

    /// v15 and later: signature and header up front, data, then the
    /// compressed file list.
    fn build_v15(&self, header_layout: HeaderLayout, entry_layout: EntryLayout) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        put_u32(&mut out, SIGNATURE);
        out.resize(4 + header_layout.size(), 0);
        let entries = self.pack_files(&mut out)?;

        let mut h = self.header();
        h.file_list_offset = out.len() as u64;
        self.write_file_list(&entries, entry_layout, &mut out)?;
        h.file_list_size = to_u32(out.len() as u64 - h.file_list_offset, self.version)?;
        if self.hash {
            h.md5 = self.archive_hash();
        }

        let mut header = Vec::with_capacity(header_layout.size());
        header_layout.write(&h, &mut header);
        out[4..4 + header.len()].copy_from_slice(&header);
        Ok(out)
    }

    /// Compresses and appends every file, padding after each, and returns
    /// their entries with absolute offsets.
    fn pack_files(&self, out: &mut Vec<u8>) -> Result<Vec<Entry>> {
        self.files
            .iter()
            .map(|file| {
                let (method, level) = self.compression_for(file);
                let stored = compression::compress(&file.data, method, level);

                let offset = out.len() as u64;
                if offset + stored.len() as u64 > self.version.max_package_size() {
                    return Err(LslibError::TooLarge {
                        limit: self.version.max_package_size(),
                    });
                }
                out.extend_from_slice(&stored);
                self.write_padding(out);

                Ok(Entry {
                    name: file.path.replace('\\', "/"),
                    archive_part: 0,
                    crc: if self.version.has_crc() {
                        crc32fast::hash(&stored)
                    } else {
                        0
                    },
                    flags: CompressionFlags::new(method, level),
                    offset_in_file: offset,
                    size_on_disk: stored.len() as u64,
                    uncompressed_size: file.data.len() as u64,
                })
            })
            .collect()
    }

    fn compression_for(&self, file: &InputFile) -> (CompressionMethod, CompressionLevel) {
        let extension = file
            .path
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_ascii_lowercase());
        let stored = extension.is_some_and(|ext| STORED_EXTENSIONS.contains(&ext.as_str()));
        if stored || file.data.is_empty() {
            return (CompressionMethod::None, CompressionLevel::Fast);
        }
        let (method, level) = file.compression.unwrap_or((self.compression, self.level));
        // v7 and v9 entries can only express zlib. (`LSLib` maps LZ4 to zlib
        // here; zstd is mapped too, as it would be unreadable.)
        if self.version <= PackageVersion::V9 && method != CompressionMethod::None {
            return (CompressionMethod::Zlib, level);
        }
        (method, level)
    }

    /// Pads with `0xAD` to the version's alignment. From v16 the alignment is
    /// relative to the end of the header.
    fn write_padding(&self, out: &mut Vec<u8>) {
        let pad = self.version.padding_size();
        let base = if self.version >= PackageVersion::V16 {
            (4 + HeaderLayout::V16.size()) as u64
        } else {
            0
        };
        let position = out.len() as u64 - base;
        let padded = position.next_multiple_of(pad) - position;
        // Under 0x1000 by construction.
        out.resize(out.len() + usize::try_from(padded).unwrap_or(0), 0xAD);
    }

    fn write_file_list(
        &self,
        entries: &[Entry],
        layout: EntryLayout,
        out: &mut Vec<u8>,
    ) -> Result<()> {
        let mut list = Vec::with_capacity(entries.len() * layout.size());
        for entry in entries {
            layout.write(entry, &mut list)?;
        }
        let compressed =
            compression::compress(&list, CompressionMethod::Lz4, CompressionLevel::Default);

        put_u32(out, to_u32(entries.len(), self.version)?);
        if self.version > PackageVersion::V13 {
            put_u32(out, to_u32(compressed.len(), self.version)?);
        }
        out.extend_from_slice(&compressed);
        Ok(())
    }

    /// MD5 over the uncompressed contents of every file (sorted by path before
    /// v15), with every byte incremented by one.
    fn archive_hash(&self) -> [u8; 16] {
        let mut files: Vec<&InputFile> = self.files.iter().collect();
        if self.version < PackageVersion::V15 {
            files.sort_by(|a, b| a.path.cmp(&b.path));
        }
        let mut md5 = Md5::new();
        for file in files {
            md5.update(&file.data);
        }
        let mut hash: [u8; 16] = md5.finalize().into();
        for byte in &mut hash {
            *byte = byte.wrapping_add(1);
        }
        hash
    }
}

fn to_u32(value: impl TryInto<u32>, version: PackageVersion) -> Result<u32> {
    value.try_into().map_err(|_| LslibError::TooLarge {
        limit: version.max_package_size(),
    })
}
