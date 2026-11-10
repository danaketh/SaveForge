//! LSPK packages: `.pak` game data and `.lsv` savegames (`LSLib`
//! `PackageReader.cs`, `PackageWriter.cs`, `PackageCommon.cs`).
//!
//! [`Package`] reads a package from bytes; [`PackageBuilder`] writes one.

mod format;
mod writer;

use std::borrow::Cow;

use crate::binary::{slice, slice64, u32_at};
use crate::compression::{self, CompressionFlags, CompressionMethod};
use crate::{LslibError, Result};
use format::{Entry, EntryLayout, Header, HeaderLayout, SIGNATURE};

pub use writer::{InputFile, PackageBuilder};

/// A package format version and the game that introduced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PackageVersion {
    /// Divinity: Original Sin.
    V7 = 7,
    /// Divinity: Original Sin Enhanced Edition.
    V9 = 9,
    /// Divinity: Original Sin 2.
    V10 = 10,
    /// Divinity: Original Sin 2 Definitive Edition.
    V13 = 13,
    /// Baldur's Gate 3 early access.
    V15 = 15,
    /// Baldur's Gate 3 early access, patch 4.
    V16 = 16,
    /// Baldur's Gate 3 release.
    V18 = 18,
}

impl PackageVersion {
    pub fn from_u32(version: u32) -> Option<Self> {
        Some(match version {
            7 => Self::V7,
            9 => Self::V9,
            10 => Self::V10,
            13 => Self::V13,
            15 => Self::V15,
            16 => Self::V16,
            18 => Self::V18,
            _ => return None,
        })
    }

    /// Whether file entries carry a CRC-32 of the stored bytes.
    pub fn has_crc(self) -> bool {
        (Self::V10..=Self::V16).contains(&self)
    }

    /// The largest size of one archive part.
    pub fn max_package_size(self) -> u64 {
        if self <= Self::V15 {
            0x4000_0000
        } else {
            0x1_0000_0000
        }
    }

    /// The alignment of file data in the archive.
    pub fn padding_size(self) -> u64 {
        if self <= Self::V9 { 0x1000 } else { 0x40 }
    }

    fn layouts(self) -> (HeaderLayout, EntryLayout) {
        match self {
            Self::V7 | Self::V9 => (HeaderLayout::V7, EntryLayout::V7),
            Self::V10 => (HeaderLayout::V10, EntryLayout::V10),
            Self::V13 => (HeaderLayout::V13, EntryLayout::V10),
            Self::V15 => (HeaderLayout::V15, EntryLayout::V15),
            Self::V16 => (HeaderLayout::V16, EntryLayout::V15),
            Self::V18 => (HeaderLayout::V16, EntryLayout::V18),
        }
    }
}

/// Package-wide flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PackageFlags(pub u8);

impl PackageFlags {
    /// Allow memory-mapped access to the files in this archive.
    pub const ALLOW_MEMORY_MAPPING: Self = Self(0x02);
    /// All files are compressed into a single LZ4 frame.
    pub const SOLID: Self = Self(0x04);
    /// Archive contents should be preloaded on game startup.
    pub const PRELOAD: Self = Self(0x08);

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

/// A file listed in a package. Read its bytes with [`Package::contents`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagedFile {
    /// The path inside the package, with `/` separators.
    pub name: String,
    pub archive_part: u32,
    /// CRC-32 of the stored bytes (v10 to v16; 0 otherwise).
    pub crc: u32,
    pub flags: CompressionFlags,
    pub offset_in_file: u64,
    pub size_on_disk: u64,
    /// 0 for uncompressed files in the file list; use [`PackagedFile::size`].
    pub uncompressed_size: u64,
    /// Offset into the decompressed frame of a solid archive.
    solid_offset: Option<u64>,
}

impl PackagedFile {
    fn from_entry(e: Entry) -> Self {
        Self {
            name: e.name,
            archive_part: e.archive_part,
            crc: e.crc,
            flags: e.flags,
            offset_in_file: e.offset_in_file,
            size_on_disk: e.size_on_disk,
            uncompressed_size: e.uncompressed_size,
            solid_offset: None,
        }
    }

    /// The size of the file's contents.
    pub fn size(&self) -> u64 {
        match self.flags.method() {
            Ok(CompressionMethod::None) => self.size_on_disk,
            _ => self.uncompressed_size,
        }
    }

    /// Whether this entry marks a file deleted by a patch package.
    pub fn is_deletion(&self) -> bool {
        self.offset_in_file & 0x0000_ffff_ffff_ffff == 0xbeef_dead_beef
    }
}

/// A parsed package. It borrows the archive bytes and decompresses files on
/// demand.
#[derive(Debug, Clone)]
pub struct Package<'a> {
    parts: Vec<&'a [u8]>,
    version: PackageVersion,
    flags: PackageFlags,
    priority: u8,
    md5: [u8; 16],
    files: Vec<PackagedFile>,
    /// The decompressed frame of a solid archive.
    solid: Option<Vec<u8>>,
}

impl<'a> Package<'a> {
    /// Reads a single-part package, such as any savegame.
    pub fn read(data: &'a [u8]) -> Result<Self> {
        Self::read_parts(vec![data])
    }

    /// Reads a package split over several parts: `parts[0]` is the main file
    /// (`Game.pak`), `parts[n]` its `Game_n.pak` sibling.
    pub fn read_parts(parts: Vec<&'a [u8]>) -> Result<Self> {
        let main = *parts.first().ok_or(LslibError::NotAPackage)?;
        let (version, offset) = detect(main)?;
        let (header_layout, entry_layout) = version.layouts();
        let header =
            header_layout.read(slice(main, offset, header_layout.size(), "package header")?)?;

        let num_parts = header.num_parts.max(1);
        if num_parts as usize > parts.len() {
            return Err(LslibError::MissingPart(
                parts.len().try_into().unwrap_or(u32::MAX),
            ));
        }

        let mut package = Self {
            parts,
            version,
            flags: PackageFlags(header.flags),
            priority: header.priority,
            md5: header.md5,
            files: Vec::new(),
            solid: None,
        };

        let mut header = header;
        if version > PackageVersion::V10 {
            header.data_offset = u32::try_from(offset + header_layout.size())
                .map_err(|_| LslibError::InvalidPackage("header offset out of range".into()))?;
            package.files = read_compressed_file_list(main, &header, version, entry_layout)?;
        } else {
            package.files =
                read_file_list(main, &header, offset + header_layout.size(), entry_layout)?;
        }

        if let Some(file) = package
            .files
            .iter()
            .find(|f| f.archive_part >= num_parts && !f.is_deletion())
        {
            return Err(LslibError::MissingPart(file.archive_part));
        }

        if package.flags.contains(PackageFlags::SOLID) && !package.files.is_empty() {
            package.unpack_solid(main, u64::from(header.data_offset))?;
        }
        Ok(package)
    }

    pub fn version(&self) -> PackageVersion {
        self.version
    }

    pub fn flags(&self) -> PackageFlags {
        self.flags
    }

    pub fn priority(&self) -> u8 {
        self.priority
    }

    /// The archive hash from the header (all zeros when not computed).
    pub fn md5(&self) -> [u8; 16] {
        self.md5
    }

    pub fn files(&self) -> &[PackagedFile] {
        &self.files
    }

    /// The file named exactly `name`.
    pub fn find(&self, name: &str) -> Option<&PackagedFile> {
        self.files.iter().find(|f| f.name == name)
    }

    /// The file named `name`, ignoring ASCII case.
    pub fn find_ignore_case(&self, name: &str) -> Option<&PackagedFile> {
        self.files
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
    }

    /// The decompressed contents of `file`, which must come from this package.
    pub fn contents(&self, file: &PackagedFile) -> Result<Cow<'_, [u8]>> {
        if file.is_deletion() {
            return Err(LslibError::DeletedFile(file.name.clone()));
        }
        if let (Some(solid), Some(start)) = (&self.solid, file.solid_offset) {
            return slice64(solid, start, file.uncompressed_size, "solid file").map(Cow::Borrowed);
        }

        let part = usize::try_from(file.archive_part)
            .ok()
            .and_then(|p| self.parts.get(p))
            .ok_or(LslibError::MissingPart(file.archive_part))?;
        let stored = slice64(part, file.offset_in_file, file.size_on_disk, "file data")?;
        let method = file.flags.method()?;
        if stored.is_empty() || method == CompressionMethod::None {
            return Ok(Cow::Borrowed(stored));
        }
        let size = usize::try_from(file.uncompressed_size)
            .map_err(|_| LslibError::decompression(method, &"file too large"))?;
        compression::decompress(stored, size, method).map(Cow::Owned)
    }

    /// The bytes of `file` as stored on disk, when they can be written back
    /// unchanged (not for solid archives).
    fn stored(&self, file: &PackagedFile) -> Option<&[u8]> {
        if self.solid.is_some() {
            return None;
        }
        let part = self.parts.get(usize::try_from(file.archive_part).ok()?)?;
        slice64(part, file.offset_in_file, file.size_on_disk, "file data").ok()
    }

    /// Every file (deletion markers excluded) with its contents, in package
    /// order.
    pub fn extract_all(&self) -> Result<Vec<(String, Vec<u8>)>> {
        self.files
            .iter()
            .filter(|f| !f.is_deletion())
            .map(|f| Ok((f.name.clone(), self.contents(f)?.into_owned())))
            .collect()
    }

    /// A builder that recreates this package: same version, flags (except
    /// solid, which cannot be written), priority and files, each keeping its
    /// compression.
    ///
    /// A v15+ archive hash is recomputed when this package has one, so a
    /// resaved BG3 savegame passes the game's integrity check.
    pub fn to_builder(&self) -> Result<PackageBuilder> {
        let mut builder = PackageBuilder::new(self.version);
        builder.hash = self.md5 != [0; 16];
        builder.flags = PackageFlags(self.flags.0 & !PackageFlags::SOLID.0);
        builder.priority = self.priority;
        for file in self.files.iter().filter(|f| !f.is_deletion()) {
            let compression = (file.flags.method()?, file.flags.level());
            builder.files.push(InputFile {
                path: file.name.clone(),
                data: self.contents(file)?.into_owned(),
                compression: Some(compression),
                stored: self.stored(file).map(<[u8]>::to_vec),
            });
        }
        Ok(builder)
    }

    /// Decompresses the single LZ4 frame holding every file of a solid
    /// archive, then points each file into it.
    fn unpack_solid(&mut self, main: &[u8], data_offset: u64) -> Result<()> {
        let first = self
            .files
            .iter()
            .map(|f| f.offset_in_file)
            .min()
            .unwrap_or(0);
        let last = self
            .files
            .iter()
            .map(|f| f.offset_in_file.saturating_add(f.size_on_disk))
            .max()
            .unwrap_or(0);
        let on_disk = self
            .files
            .iter()
            .map(|f| f.size_on_disk)
            .fold(0u64, u64::saturating_add);
        let total = self
            .files
            .iter()
            .map(|f| f.uncompressed_size)
            .fold(0u64, u64::saturating_add);

        // The 7 bytes after the data offset are the LZ4 frame header.
        if first != data_offset + 7 || last - first != on_disk {
            return Err(LslibError::InvalidPackage(format!(
                "incorrectly compressed solid archive; offsets {first}/{last}, bytes {on_disk}"
            )));
        }

        let frame = slice64(main, data_offset, last - data_offset, "solid frame")?;
        let total = usize::try_from(total).map_err(|_| {
            LslibError::decompression(CompressionMethod::Lz4, &"solid archive too large")
        })?;
        let solid = compression::decompress_lz4_frame(frame, total)?;

        let mut offset = first;
        let mut solid_offset = 0;
        for file in &mut self.files {
            if file.offset_in_file != offset {
                return Err(LslibError::InvalidPackage(
                    "file list in solid archive not contiguous".into(),
                ));
            }
            file.solid_offset = Some(solid_offset);
            offset += file.size_on_disk;
            solid_offset += file.uncompressed_size;
        }
        self.solid = Some(solid);
        Ok(())
    }
}

/// The package version and header offset, by `LSLib`'s signature probing.
fn detect(data: &[u8]) -> Result<(PackageVersion, usize)> {
    // v13 keeps its header at the end, followed by its size and the signature.
    if data.len() >= 8 && u32_at(data, data.len() - 4, "signature")? == SIGNATURE {
        let header_size = u32_at(data, data.len() - 8, "header size")? as usize;
        let offset = data
            .len()
            .checked_sub(header_size)
            .ok_or(LslibError::Truncated("package header"))?;
        return Ok((PackageVersion::V13, offset));
    }

    // v10 and later lead with the signature.
    if data.len() >= 8 && u32_at(data, 0, "signature")? == SIGNATURE {
        let version = u32_at(data, 4, "version")?;
        return match PackageVersion::from_u32(version) {
            Some(
                v @ (PackageVersion::V10
                | PackageVersion::V15
                | PackageVersion::V16
                | PackageVersion::V18),
            ) => Ok((v, 4)),
            _ => Err(LslibError::UnsupportedVersion(version)),
        };
    }

    // v7 and v9 have no signature at all.
    match u32_at(data, 0, "version") {
        Ok(7) => Ok((PackageVersion::V7, 0)),
        Ok(9) => Ok((PackageVersion::V9, 0)),
        _ => Err(LslibError::NotAPackage),
    }
}

/// The plain file list of packages up to v10. Offsets in part 0 are relative
/// to the data offset.
fn read_file_list(
    main: &[u8],
    header: &Header,
    offset: usize,
    layout: EntryLayout,
) -> Result<Vec<PackagedFile>> {
    let count = header.num_files as usize;
    let len = count
        .checked_mul(layout.size())
        .ok_or(LslibError::Truncated("file list"))?;
    let list = slice(main, offset, len, "file list")?;
    list.chunks_exact(layout.size())
        .map(|chunk| {
            let mut file = PackagedFile::from_entry(layout.read(chunk)?);
            if file.archive_part == 0 {
                file.offset_in_file += u64::from(header.data_offset);
            }
            Ok(file)
        })
        .collect()
}

/// The LZ4-compressed file list of v13 and later packages.
fn read_compressed_file_list(
    main: &[u8],
    header: &Header,
    version: PackageVersion,
    layout: EntryLayout,
) -> Result<Vec<PackagedFile>> {
    let offset =
        usize::try_from(header.file_list_offset).map_err(|_| LslibError::Truncated("file list"))?;
    let count = u32_at(main, offset, "file count")? as usize;
    let compressed = if version > PackageVersion::V13 {
        let size = u32_at(main, offset + 4, "file list size")? as usize;
        slice(main, offset + 8, size, "file list")?
    } else {
        let size = (header.file_list_size as usize)
            .checked_sub(4)
            .ok_or(LslibError::Truncated("file list"))?;
        slice(main, offset + 4, size, "file list")?
    };

    let len = count
        .checked_mul(layout.size())
        .ok_or(LslibError::Truncated("file list"))?;
    let list = compression::decompress(compressed, len, CompressionMethod::Lz4)?;
    list.chunks_exact(layout.size())
        .map(|chunk| layout.read(chunk).map(PackagedFile::from_entry))
        .collect()
}
