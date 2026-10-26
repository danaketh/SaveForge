//! The compression methods used inside Larian files (`LSLib` `Compression.cs`).
//!
//! Output is format compatible with `LSLib` but not byte identical: LZ4 always
//! uses the fast (non-HC) compressor, and Zstandard always uses the fastest
//! level, since the pure-Rust encoders implement no others.

use std::io::{Read, Write};

use crate::{LslibError, Result};

/// A compression algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompressionMethod {
    None,
    Zlib,
    Lz4,
    Zstd,
}

/// A compression effort, recorded in the high nibble of [`CompressionFlags`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CompressionLevel {
    Fast,
    #[default]
    Default,
    Max,
}

/// The per-file compression byte: method in the low nibble, level in the high.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CompressionFlags(pub u8);

impl CompressionFlags {
    pub const NONE: Self = Self(0);

    /// The flags for `method` at `level`; no level bits without compression.
    pub fn new(method: CompressionMethod, level: CompressionLevel) -> Self {
        let method_bits = match method {
            CompressionMethod::None => return Self::NONE,
            CompressionMethod::Zlib => 1,
            CompressionMethod::Lz4 => 2,
            CompressionMethod::Zstd => 3,
        };
        let level_bits = match level {
            CompressionLevel::Fast => 0x10,
            CompressionLevel::Default => 0x20,
            CompressionLevel::Max => 0x40,
        };
        Self(method_bits | level_bits)
    }

    pub fn method(self) -> Result<CompressionMethod> {
        match self.0 & 0x0f {
            0 => Ok(CompressionMethod::None),
            1 => Ok(CompressionMethod::Zlib),
            2 => Ok(CompressionMethod::Lz4),
            3 => Ok(CompressionMethod::Zstd),
            other => Err(LslibError::UnsupportedCompression(other)),
        }
    }

    /// The level; unknown bits read as [`CompressionLevel::Default`], since
    /// they do not affect decompression.
    pub fn level(self) -> CompressionLevel {
        match self.0 & 0xf0 {
            0x10 => CompressionLevel::Fast,
            0x40 => CompressionLevel::Max,
            _ => CompressionLevel::Default,
        }
    }
}

/// Compresses `data`. LZ4 produces a raw block (no frame), Zlib a stream with
/// the zlib header, Zstandard a single frame.
///
/// # Panics
///
/// Never in practice: the compressors only fail on output errors, and they
/// write to memory.
pub fn compress(data: &[u8], method: CompressionMethod, level: CompressionLevel) -> Vec<u8> {
    match method {
        CompressionMethod::None => data.to_vec(),
        CompressionMethod::Zlib => {
            let zlib_level = match level {
                CompressionLevel::Fast => 1,
                CompressionLevel::Default => 6,
                CompressionLevel::Max => 9,
            };
            let mut encoder =
                flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(zlib_level));
            // Writing to a Vec cannot fail.
            encoder.write_all(data).expect("in-memory zlib write");
            encoder.finish().expect("in-memory zlib finish")
        }
        CompressionMethod::Lz4 => lz4_flex::block::compress(data),
        CompressionMethod::Zstd => {
            ruzstd::encoding::compress_to_vec(data, ruzstd::encoding::CompressionLevel::Fastest)
        }
    }
}

/// Decompresses `data` that unpacks to exactly `size` bytes.
pub fn decompress(data: &[u8], size: usize, method: CompressionMethod) -> Result<Vec<u8>> {
    let out = match method {
        CompressionMethod::None => return Ok(data.to_vec()),
        CompressionMethod::Zlib => read_capped(flate2::read::ZlibDecoder::new(data), size, method)?,
        CompressionMethod::Lz4 => {
            // An LZ4 block expands at most ~255:1; refuse absurd sizes from
            // untrusted headers before allocating.
            if size > data.len().saturating_mul(255).saturating_add(16) {
                return Err(LslibError::decompression(
                    method,
                    &format!("{} bytes cannot unpack to {size}", data.len()),
                ));
            }
            let mut out = vec![0; size];
            let written = lz4_flex::block::decompress_into(data, &mut out)
                .map_err(|e| LslibError::decompression(method, &e))?;
            out.truncate(written);
            out
        }
        CompressionMethod::Zstd => decompress_zstd(data, size)?,
    };
    if out.len() == size {
        Ok(out)
    } else {
        Err(LslibError::decompression(
            method,
            &format!("expected {size} bytes, got {}", out.len()),
        ))
    }
}

/// Decompresses an LZ4 frame (the solid-archive format) to `size` bytes. A
/// missing end mark is tolerated, as the frame may be cut at the last file.
pub(crate) fn decompress_lz4_frame(data: &[u8], size: usize) -> Result<Vec<u8>> {
    let method = CompressionMethod::Lz4;
    let mut decoder = lz4_flex::frame::FrameDecoder::new(data);
    let mut out = Vec::new();
    let mut chunk = vec![0u8; 0x10000];
    while out.len() < size {
        let want = chunk.len().min(size - out.len());
        let read = decoder
            .read(&mut chunk[..want])
            .map_err(|e| LslibError::decompression(method, &e))?;
        if read == 0 {
            return Err(LslibError::decompression(
                method,
                &format!("frame ended after {} of {size} bytes", out.len()),
            ));
        }
        out.extend_from_slice(&chunk[..read]);
    }
    Ok(out)
}

/// Decodes concatenated Zstandard frames, as `LSLib`'s stream decoder does.
fn decompress_zstd(mut data: &[u8], size: usize) -> Result<Vec<u8>> {
    let method = CompressionMethod::Zstd;
    let mut out = Vec::new();
    while !data.is_empty() && out.len() <= size {
        let decoder = ruzstd::decoding::StreamingDecoder::new(&mut data)
            .map_err(|e| LslibError::decompression(method, &e))?;
        out.extend(read_capped(decoder, size - out.len(), method)?);
    }
    Ok(out)
}

/// Reads `reader` to the end, but at most one byte past `size`, so a lying
/// header cannot make a small input expand without bound.
fn read_capped(reader: impl Read, size: usize, method: CompressionMethod) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    reader
        .take(size as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|e| LslibError::decompression(method, &e))?;
    Ok(out)
}
