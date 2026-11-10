//! On-disk LSPK headers and file entries (`LSLib` `PackageFormat.cs`). All
//! structures are packed and little-endian.

use crate::binary::{Reader, put_u16, put_u32, put_u64};
use crate::compression::{CompressionFlags, CompressionLevel, CompressionMethod};
use crate::{LslibError, Result};

/// `"LSPK"`.
pub(crate) const SIGNATURE: u32 = 0x4B50_534C;
const NAME_LEN: usize = 256;

/// The version-independent header fields.
#[derive(Debug, Clone, Default)]
pub(crate) struct Header {
    pub version: u32,
    pub file_list_offset: u64,
    /// Size of the file list (legacy and v13 packages).
    pub file_list_size: u32,
    /// Number of files (packages up to v10).
    pub num_files: u32,
    pub num_parts: u32,
    /// Offset of the packed data in part 0 (packages up to v10).
    pub data_offset: u32,
    pub flags: u8,
    pub priority: u8,
    pub md5: [u8; 16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeaderLayout {
    V7,
    V10,
    V13,
    V15,
    V16,
}

impl HeaderLayout {
    pub(crate) const fn size(self) -> usize {
        match self {
            Self::V7 => 21,
            Self::V10 => 20,
            Self::V13 => 32,
            Self::V15 => 34,
            Self::V16 => 36,
        }
    }

    pub(crate) fn read(self, data: &[u8]) -> Result<Header> {
        let mut r = Reader::new(data, "package header");
        let mut h = Header {
            version: r.u32()?,
            ..Header::default()
        };
        match self {
            Self::V7 => {
                h.data_offset = r.u32()?;
                h.num_parts = r.u32()?;
                h.file_list_size = r.u32()?;
                let _little_endian = r.u8()?;
                h.num_files = r.u32()?;
                h.file_list_offset = Self::V7.size() as u64;
            }
            Self::V10 => {
                h.data_offset = r.u32()?;
                h.file_list_size = r.u32()?;
                h.num_parts = r.u16()?.into();
                h.flags = r.u8()?;
                h.priority = r.u8()?;
                h.num_files = r.u32()?;
                h.file_list_offset = Self::V7.size() as u64;
            }
            Self::V13 => {
                h.file_list_offset = r.u32()?.into();
                h.file_list_size = r.u32()?;
                h.num_parts = r.u16()?.into();
                h.flags = r.u8()?;
                h.priority = r.u8()?;
                h.md5 = r.array()?;
            }
            Self::V15 | Self::V16 => {
                h.file_list_offset = r.u64()?;
                h.file_list_size = r.u32()?;
                h.flags = r.u8()?;
                h.priority = r.u8()?;
                h.md5 = r.array()?;
                h.num_parts = if self == Self::V16 {
                    r.u16()?.into()
                } else {
                    1
                };
            }
        }
        Ok(h)
    }

    /// Appends `h` in this layout. Fields wider on disk than in the layout
    /// are truncated, as `LSLib` does.
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn write(self, h: &Header, out: &mut Vec<u8>) {
        put_u32(out, h.version);
        match self {
            Self::V7 => {
                put_u32(out, h.data_offset);
                put_u32(out, h.num_parts);
                put_u32(out, h.file_list_size);
                out.push(0);
                put_u32(out, h.num_files);
            }
            Self::V10 => {
                put_u32(out, h.data_offset);
                put_u32(out, h.file_list_size);
                put_u16(out, h.num_parts as u16);
                out.extend([h.flags, h.priority]);
                put_u32(out, h.num_files);
            }
            Self::V13 => {
                put_u32(out, h.file_list_offset as u32);
                put_u32(out, h.file_list_size);
                put_u16(out, h.num_parts as u16);
                out.extend([h.flags, h.priority]);
                out.extend_from_slice(&h.md5);
            }
            Self::V15 | Self::V16 => {
                put_u64(out, h.file_list_offset);
                put_u32(out, h.file_list_size);
                out.extend([h.flags, h.priority]);
                out.extend_from_slice(&h.md5);
                if self == Self::V16 {
                    put_u16(out, h.num_parts as u16);
                }
            }
        }
    }
}

/// The version-independent fields of a file list entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Entry {
    pub name: String,
    pub archive_part: u32,
    pub crc: u32,
    pub flags: CompressionFlags,
    pub offset_in_file: u64,
    pub size_on_disk: u64,
    pub uncompressed_size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryLayout {
    V7,
    V10,
    V15,
    V18,
}

impl EntryLayout {
    pub(crate) const fn size(self) -> usize {
        match self {
            Self::V7 | Self::V18 => 272,
            Self::V10 => 280,
            Self::V15 => 296,
        }
    }

    pub(crate) fn read(self, data: &[u8]) -> Result<Entry> {
        let mut r = Reader::new(data, "file entry");
        let mut e = Entry {
            name: read_name(r.bytes(NAME_LEN)?)?,
            ..Entry::default()
        };
        match self {
            Self::V7 => {
                e.offset_in_file = r.u32()?.into();
                e.size_on_disk = r.u32()?.into();
                e.uncompressed_size = r.u32()?.into();
                e.archive_part = r.u32()?;
                // v7 entries have no flags; a non-zero size means zlib.
                if e.uncompressed_size > 0 {
                    e.flags =
                        CompressionFlags::new(CompressionMethod::Zlib, CompressionLevel::Default);
                }
            }
            Self::V10 => {
                e.offset_in_file = r.u32()?.into();
                e.size_on_disk = r.u32()?.into();
                e.uncompressed_size = r.u32()?.into();
                e.archive_part = r.u32()?;
                e.flags = flags_from_u32(r.u32()?);
                e.crc = r.u32()?;
            }
            Self::V15 => {
                e.offset_in_file = r.u64()?;
                e.size_on_disk = r.u64()?;
                e.uncompressed_size = r.u64()?;
                e.archive_part = r.u32()?;
                e.flags = flags_from_u32(r.u32()?);
                e.crc = r.u32()?;
                let _unknown = r.u32()?;
            }
            Self::V18 => {
                let low = r.u32()?;
                let high = r.u16()?;
                e.offset_in_file = u64::from(low) | (u64::from(high) << 32);
                e.archive_part = r.u8()?.into();
                e.flags = CompressionFlags(r.u8()?);
                e.size_on_disk = r.u32()?.into();
                e.uncompressed_size = r.u32()?.into();
            }
        }
        Ok(e)
    }

    /// Appends `e` in this layout. The uncompressed size of an uncompressed
    /// file is stored as 0; fields wider than the layout are truncated, as
    /// `LSLib` does (the writer bounds sizes per version beforehand).
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn write(self, e: &Entry, out: &mut Vec<u8>) -> Result<()> {
        write_name(&e.name, out)?;
        let uncompressed = if e.flags.method()? == CompressionMethod::None {
            0
        } else {
            e.uncompressed_size
        };
        match self {
            Self::V7 | Self::V10 => {
                put_u32(out, e.offset_in_file as u32);
                put_u32(out, e.size_on_disk as u32);
                put_u32(out, uncompressed as u32);
                put_u32(out, e.archive_part);
                if self == Self::V10 {
                    put_u32(out, e.flags.0.into());
                    put_u32(out, e.crc);
                }
            }
            Self::V15 => {
                put_u64(out, e.offset_in_file);
                put_u64(out, e.size_on_disk);
                put_u64(out, uncompressed);
                put_u32(out, e.archive_part);
                put_u32(out, e.flags.0.into());
                put_u32(out, e.crc);
                put_u32(out, 0);
            }
            Self::V18 => {
                put_u32(out, (e.offset_in_file & 0xffff_ffff) as u32);
                put_u16(out, ((e.offset_in_file >> 32) & 0xffff) as u16);
                out.push(e.archive_part as u8);
                out.push(e.flags.0);
                put_u32(out, e.size_on_disk as u32);
                put_u32(out, uncompressed as u32);
            }
        }
        Ok(())
    }
}

/// `LSLib` keeps only the low byte of the 32-bit flags field.
#[allow(clippy::cast_possible_truncation)]
fn flags_from_u32(flags: u32) -> CompressionFlags {
    CompressionFlags(flags as u8)
}

fn read_name(bytes: &[u8]) -> Result<String> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8(bytes[..end].to_vec()).map_err(|_| {
        LslibError::InvalidPackage(format!(
            "file name {:?} is not UTF-8",
            String::from_utf8_lossy(&bytes[..end])
        ))
    })
}

/// A NUL-terminated name padded to 256 bytes.
fn write_name(name: &str, out: &mut Vec<u8>) -> Result<()> {
    if name.len() >= NAME_LEN {
        return Err(LslibError::NameTooLong(name.to_owned()));
    }
    out.extend_from_slice(name.as_bytes());
    out.resize(out.len() + NAME_LEN - name.len(), 0);
    Ok(())
}
