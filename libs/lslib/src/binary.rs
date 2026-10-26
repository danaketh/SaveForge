//! Little-endian reading and writing over byte slices. Every read is bounds
//! checked, so malformed input yields an error instead of a panic.

use crate::{LslibError, Result};

/// A cursor over a byte slice.
#[derive(Debug)]
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    what: &'static str,
}

impl<'a> Reader<'a> {
    /// A reader over `data`; `what` names the structure in truncation errors.
    pub(crate) fn new(data: &'a [u8], what: &'static str) -> Self {
        Self { data, pos: 0, what }
    }

    pub(crate) fn bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        let bytes = slice(self.data, self.pos, len, self.what)?;
        self.pos += len;
        Ok(bytes)
    }

    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut out = [0; N];
        out.copy_from_slice(self.bytes(N)?);
        Ok(out)
    }

    pub(crate) fn u8(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16> {
        self.array().map(u16::from_le_bytes)
    }

    pub(crate) fn u32(&mut self) -> Result<u32> {
        self.array().map(u32::from_le_bytes)
    }

    pub(crate) fn u64(&mut self) -> Result<u64> {
        self.array().map(u64::from_le_bytes)
    }
}

/// `data[offset..offset + len]`, or a truncation error naming `what`.
pub(crate) fn slice<'a>(
    data: &'a [u8],
    offset: usize,
    len: usize,
    what: &'static str,
) -> Result<&'a [u8]> {
    offset
        .checked_add(len)
        .and_then(|end| data.get(offset..end))
        .ok_or(LslibError::Truncated(what))
}

/// [`slice`] with offsets as stored in the file.
pub(crate) fn slice64<'a>(
    data: &'a [u8],
    offset: u64,
    len: u64,
    what: &'static str,
) -> Result<&'a [u8]> {
    let offset = usize::try_from(offset).map_err(|_| LslibError::Truncated(what))?;
    let len = usize::try_from(len).map_err(|_| LslibError::Truncated(what))?;
    slice(data, offset, len, what)
}

pub(crate) fn u32_at(data: &[u8], offset: usize, what: &'static str) -> Result<u32> {
    let bytes = slice(data, offset, 4, what)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

pub(crate) fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}
