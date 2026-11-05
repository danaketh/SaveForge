//! `NewAge`: the ECS dump BG3 keeps in the `NewAge` attribute of a savegame's
//! `Globals.lsf` (magic `LSMF`). Most character state lives here: ability
//! scores, experience, health, inventory.
//!
//! Not part of `LSLib`; the layout follows community reverse engineering
//! (`bg3-newage`, `bg3-party-tool`) and was checked against 4.1.1 saves:
//!
//! - a 48-byte header: magic, `u32` version, `u64` checksum, `u64` block
//!   offset, `u64` block size, `u32` name block size, `u16` component count,
//!   `u16`, `u64`. Every offset below is relative to the end of the header;
//! - component data, then the name block at the block offset, then one
//!   48-byte table entry per component: `u64` name offset, `u32` name size,
//!   `u32`, `u64` hash, `u32` element size, `u32` version, `u64` element
//!   count, `u64` data offset;
//! - component `core.v0.Level` holds `u64` entities start/end (16-byte ids)
//!   and owner maps start/end. An owner map (`u64` indices start, end,
//!   component index, count) gives the entity index owning each element of a
//!   component; `u64::MAX` marks an empty map;
//! - the checksum is `h1` of `MurmurHash3` `x64_128` over everything after the
//!   header, seeded with [`CHECKSUM_SEED`]. The game refuses a blob whose
//!   checksum does not match.
//!
//! Elements, and the data they point at, can be overwritten in place;
//! nothing is resized.

use crate::binary::{Reader, slice64};
use crate::{LslibError, Result};

pub const MAGIC: [u8; 4] = *b"LSMF";
pub const HEADER_SIZE: usize = 48;
pub const CHECKSUM_SEED: u64 = 0xf001_5eed_5eed_f001;
pub const LEVEL_COMPONENT: &str = "core.v0.Level";

const TABLE_ENTRY_SIZE: u64 = 48;
const OWNER_MAP_SIZE: u64 = 32;
const ENTITY_SIZE: u64 = 16;
const NO_INDICES: u64 = u64::MAX;

/// One component type: a fixed-size element array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub name: String,
    pub version: u32,
    pub element_size: usize,
    pub count: usize,
    /// Offset of the first element in the body (after the header).
    data_offset: usize,
    /// The owning entity index of each element; `None` for side tables that
    /// other components point into.
    owners: Option<Vec<u32>>,
}

impl Component {
    pub fn owners(&self) -> Option<&[u32]> {
        self.owners.as_deref()
    }

    /// The index of the element owned by `entity`.
    pub fn element_of(&self, entity: u32) -> Option<usize> {
        self.owners.as_ref()?.iter().position(|&e| e == entity)
    }
}

/// A parsed `NewAge` blob. Keeps the original bytes, so writing it back
/// changes only the overwritten elements and the checksum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAge {
    data: Vec<u8>,
    version: u32,
    components: Vec<Component>,
    /// Offset of the entity id array in the body, and its length.
    entities_offset: usize,
    entity_count: usize,
}

impl NewAge {
    pub fn read(data: Vec<u8>) -> Result<Self> {
        let mut header = Reader::new(&data, "NewAge header");
        if header.array::<4>()? != MAGIC {
            return Err(invalid("missing LSMF magic"));
        }
        let version = header.u32()?;
        let _checksum = header.u64()?;
        let block_offset = header.u64()?;
        let _block_size = header.u64()?;
        let name_block_size = header.u32()?;
        let component_count = header.u16()?;
        let body = data
            .get(HEADER_SIZE..)
            .ok_or(LslibError::Truncated("NewAge header"))?;

        let mut components = read_components(body, block_offset, name_block_size, component_count)?;

        let level = components
            .iter()
            .find(|c| c.name == LEVEL_COMPONENT && c.count == 1 && c.element_size == 32)
            .ok_or_else(|| invalid("no core.v0.Level component"))?;
        let mut level = Reader::new(
            slice64(body, level.data_offset as u64, 32, "NewAge level")?,
            "NewAge level",
        );
        let (entities_start, entities_end) = (level.u64()?, level.u64()?);
        let (maps_start, maps_end) = (level.u64()?, level.u64()?);

        let entities_len = range_len(entities_start, entities_end, ENTITY_SIZE, "entities")?;
        slice64(body, entities_start, entities_len, "NewAge entities")?;
        let entity_count = to_usize(entities_len / ENTITY_SIZE)?;

        let maps_len = range_len(maps_start, maps_end, OWNER_MAP_SIZE, "owner maps")?;
        let maps = slice64(body, maps_start, maps_len, "NewAge owner maps")?;
        read_owners(body, maps, &mut components, entity_count)?;

        Ok(Self {
            data,
            version,
            components,
            entities_offset: to_usize(entities_start)?,
            entity_count,
        })
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn components(&self) -> &[Component] {
        &self.components
    }

    pub fn component(&self, name: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.name == name)
    }

    pub fn entity_count(&self) -> usize {
        self.entity_count
    }

    /// `len` bytes at `offset` in the body: the offsets that elements store
    /// to point at arrays, strings and other data outside the components.
    pub fn bytes_at(&self, offset: u64, len: u64) -> Option<&[u8]> {
        slice64(&self.data[HEADER_SIZE..], offset, len, "NewAge data").ok()
    }

    /// Overwrites the bytes at body `offset` (see [`NewAge::bytes_at`]).
    pub fn write_at(&mut self, offset: u64, bytes: &[u8]) -> Result<()> {
        let len = bytes.len() as u64;
        if self.bytes_at(offset, len).is_none() {
            return Err(invalid(format!(
                "{len} bytes at {offset} are outside the blob"
            )));
        }
        let start = HEADER_SIZE + to_usize(offset)?;
        self.data[start..start + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    /// The stored 16-byte id of entity `index`, as the blob keeps it.
    pub fn entity(&self, index: u32) -> Option<[u8; 16]> {
        if index as usize >= self.entity_count {
            return None;
        }
        let start = HEADER_SIZE + self.entities_offset + index as usize * 16;
        self.data.get(start..start + 16)?.try_into().ok()
    }

    /// Element `index` of `component`.
    pub fn element(&self, component: &Component, index: usize) -> Option<&[u8]> {
        if index >= component.count {
            return None;
        }
        let start = HEADER_SIZE + component.data_offset + index * component.element_size;
        self.data.get(start..start + component.element_size)
    }

    /// Overwrites element `index` of the component named `name`. `bytes`
    /// must be exactly one element long.
    pub fn set_element(&mut self, name: &str, index: usize, bytes: &[u8]) -> Result<()> {
        let component = self
            .component(name)
            .ok_or_else(|| invalid(format!("no component {name}")))?;
        if bytes.len() != component.element_size {
            return Err(invalid(format!(
                "{name} elements are {} bytes, got {}",
                component.element_size,
                bytes.len()
            )));
        }
        if index >= component.count {
            return Err(invalid(format!(
                "{name} has {} elements, no element {index}",
                component.count
            )));
        }
        let start = HEADER_SIZE + component.data_offset + index * component.element_size;
        self.data[start..start + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }

    /// The checksum stored in the header.
    pub fn stored_checksum(&self) -> u64 {
        let mut checksum = [0; 8];
        checksum.copy_from_slice(&self.data[8..16]);
        u64::from_le_bytes(checksum)
    }

    /// The checksum of the current contents.
    pub fn checksum(&self) -> u64 {
        murmur3_x64_128(&self.data[HEADER_SIZE..], CHECKSUM_SEED).0
    }

    /// The blob with its checksum updated.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.data.clone();
        out[8..16].copy_from_slice(&self.checksum().to_le_bytes());
        out
    }
}

/// The component table and names; owners are filled in by [`read_owners`].
fn read_components(
    body: &[u8],
    block_offset: u64,
    name_block_size: u32,
    count: u16,
) -> Result<Vec<Component>> {
    let names = slice64(body, block_offset, name_block_size.into(), "NewAge names")?;
    let table_offset = block_offset
        .checked_add(name_block_size.into())
        .ok_or_else(|| invalid("component table offset overflows"))?;
    let mut table = Reader::new(
        slice64(
            body,
            table_offset,
            u64::from(count) * TABLE_ENTRY_SIZE,
            "NewAge component table",
        )?,
        "NewAge component table",
    );
    let mut components = Vec::with_capacity(count.into());
    for _ in 0..count {
        let name_offset = table.u64()?;
        let name_size = table.u32()?;
        let _unknown = table.u32()?;
        let _hash = table.u64()?;
        let element_size = table.u32()?;
        let version = table.u32()?;
        let count = table.u64()?;
        let data_offset = table.u64()?;
        let name = slice64(
            names,
            name_offset,
            name_size.into(),
            "NewAge component name",
        )?;
        let name = String::from_utf8_lossy(name).into_owned();
        let bytes = u64::from(element_size)
            .checked_mul(count)
            .ok_or_else(|| invalid(format!("component {name} is too large")))?;
        slice64(body, data_offset, bytes, "NewAge component data")?;
        components.push(Component {
            name,
            version,
            element_size: to_usize(element_size.into())?,
            count: to_usize(count)?,
            data_offset: to_usize(data_offset)?,
            owners: None,
        });
    }
    Ok(components)
}

/// Sets each component's owners from the owner maps in `maps`.
fn read_owners(
    body: &[u8],
    maps: &[u8],
    components: &mut [Component],
    entity_count: usize,
) -> Result<()> {
    let mut maps = Reader::new(maps, "NewAge owner maps");
    while maps.remaining() > 0 {
        let (start, end, component, count) = (maps.u64()?, maps.u64()?, maps.u64()?, maps.u64()?);
        // Unused slots have every field set to `u64::MAX`.
        if start == NO_INDICES {
            continue;
        }
        let component = usize::try_from(component)
            .ok()
            .and_then(|i| components.get_mut(i))
            .ok_or_else(|| invalid(format!("owner map for missing component {component}")))?;
        let len = range_len(start, end, 4, "owner indices")?;
        let mut indices = Reader::new(
            slice64(body, start, len, "NewAge owner indices")?,
            "NewAge owner indices",
        );
        let mut owners = Vec::with_capacity(component.count);
        for _ in 0..len / 4 {
            let entity = indices.u32()?;
            if entity as usize >= entity_count {
                return Err(invalid(format!(
                    "{} element owned by missing entity {entity}",
                    component.name
                )));
            }
            owners.push(entity);
        }
        if owners.len() as u64 != count || owners.len() != component.count {
            return Err(invalid(format!(
                "{} has {} elements but {} owners",
                component.name,
                component.count,
                owners.len()
            )));
        }
        component.owners = Some(owners);
    }
    Ok(())
}

/// `MurmurHash3` x64 128-bit: `(h1, h2)`.
pub fn murmur3_x64_128(data: &[u8], seed: u64) -> (u64, u64) {
    const C1: u64 = 0x87c3_7b91_1142_53d5;
    const C2: u64 = 0x4cf5_ad43_2745_937f;
    let mix_k1 = |k: u64| k.wrapping_mul(C1).rotate_left(31).wrapping_mul(C2);
    let mix_k2 = |k: u64| k.wrapping_mul(C2).rotate_left(33).wrapping_mul(C1);

    let (mut h1, mut h2) = (seed, seed);
    let (blocks, tail) = data.as_chunks::<16>();
    for block in blocks {
        let (halves, _) = block.as_chunks::<8>();
        let (k1, k2) = (u64::from_le_bytes(halves[0]), u64::from_le_bytes(halves[1]));
        h1 ^= mix_k1(k1);
        h1 = h1
            .rotate_left(27)
            .wrapping_add(h2)
            .wrapping_mul(5)
            .wrapping_add(0x52dc_e729);
        h2 ^= mix_k2(k2);
        h2 = h2
            .rotate_left(31)
            .wrapping_add(h1)
            .wrapping_mul(5)
            .wrapping_add(0x3849_5ab5);
    }

    let word = |bytes: &[u8]| {
        bytes
            .iter()
            .rev()
            .fold(0u64, |acc, &b| (acc << 8) | u64::from(b))
    };
    if tail.len() > 8 {
        h2 ^= mix_k2(word(&tail[8..]));
    }
    if !tail.is_empty() {
        h1 ^= mix_k1(word(&tail[..tail.len().min(8)]));
    }

    let len = data.len() as u64;
    h1 ^= len;
    h2 ^= len;
    h1 = h1.wrapping_add(h2);
    h2 = h2.wrapping_add(h1);
    h1 = fmix64(h1);
    h2 = fmix64(h2);
    h1 = h1.wrapping_add(h2);
    h2 = h2.wrapping_add(h1);
    (h1, h2)
}

fn fmix64(mut k: u64) -> u64 {
    k ^= k >> 33;
    k = k.wrapping_mul(0xff51_afd7_ed55_8ccd);
    k ^= k >> 33;
    k = k.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    k ^ (k >> 33)
}

/// `end - start`, checked to be a whole number of `unit`-sized records.
fn range_len(start: u64, end: u64, unit: u64, what: &str) -> Result<u64> {
    end.checked_sub(start)
        .filter(|len| len % unit == 0)
        .ok_or_else(|| invalid(format!("bad {what} range {start}..{end}")))
}

fn to_usize(value: u64) -> Result<usize> {
    usize::try_from(value).map_err(|_| invalid(format!("offset {value} does not fit in memory")))
}

fn invalid(message: impl Into<String>) -> LslibError {
    LslibError::InvalidNewAge(message.into())
}
