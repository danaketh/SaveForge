use std::collections::HashMap;

use super::{
    LsfFile, LsfFormat, LsfVersion, Metadata, MetadataFormat, SIGNATURE, Section, effective_engine,
    translated_string_has_version,
};
use crate::binary::{put_u16, put_u32, put_u64};
use crate::compression::{self, CompressionFlags, CompressionMethod};
use crate::resource::{AttributeValue, Node, PackedVersion, TranslatedFSString};
use crate::{LslibError, Result};

/// Buckets in the name hash table, as in `LSLib`.
const NAME_BUCKETS: usize = 0x200;
/// Attribute value lengths share a 32-bit field with the 6-bit type.
const MAX_VALUE_LENGTH: usize = (1 << 26) - 1;

fn invalid(message: impl Into<String>) -> LslibError {
    LslibError::InvalidLsf(message.into())
}

pub(super) fn write(file: &LsfFile) -> Result<Vec<u8>> {
    let format = file.format;
    let resource = &file.resource;
    let mut w = Writer {
        format,
        values_version: ValueVersion {
            version: format.version,
            engine: effective_engine(format.version, resource.engine_version),
        },
        names: NameTable::default(),
        nodes: Vec::new(),
        attributes: Vec::new(),
        values: Vec::new(),
        keys: Vec::new(),
        node_count: 0,
        attribute_count: 0,
    };
    w.write_siblings(&resource.regions, -1)?;

    let strings = w.names.serialize()?;
    let method = format.compression.method()?;
    let level = format.compression.level();
    let chunked = format.chunked();
    // Key tables only exist from v6; LSLib writes one only with keys and
    // adjacency, BG3 whenever a node has a key.
    let has_keys = format.version >= LsfVersion::Bg3NodeKeys
        && (format.metadata_format == MetadataFormat::KEYS_AND_ADJACENCY || !w.keys.is_empty());

    let stored = |data: &[u8], chunked: bool| -> Vec<u8> {
        if chunked && method == CompressionMethod::Lz4 {
            compression::compress_lz4_frame(data)
        } else {
            compression::compress(data, method, level)
        }
    };
    let strings_stored = stored(&strings, false);
    let nodes_stored = stored(&w.nodes, chunked);
    let attributes_stored = stored(&w.attributes, chunked);
    let values_stored = stored(&w.values, chunked);
    let keys_stored = if has_keys {
        stored(&w.keys, chunked)
    } else {
        Vec::new()
    };

    let section = |raw: &[u8], stored: &[u8]| -> Result<Section> {
        Ok(Section {
            uncompressed: to_u32(raw.len())?,
            on_disk: if method == CompressionMethod::None {
                0
            } else {
                to_u32(stored.len())?
            },
        })
    };
    let meta = Metadata {
        strings: section(&strings, &strings_stored)?,
        keys: if has_keys {
            section(&w.keys, &keys_stored)?
        } else {
            Section::default()
        },
        nodes: section(&w.nodes, &nodes_stored)?,
        attributes: section(&w.attributes, &attributes_stored)?,
        values: section(&w.values, &values_stored)?,
        compression: CompressionFlags::new(method, level),
        format: format.metadata_format,
    };

    let mut out = Vec::new();
    out.extend_from_slice(&SIGNATURE);
    put_u32(&mut out, format.version as u32);
    if format.version >= LsfVersion::Bg3ExtendedHeader {
        put_u64(&mut out, resource.engine_version.to_u64());
    } else {
        put_u32(&mut out, resource.engine_version.to_u32());
    }
    write_metadata(&mut out, &meta, format.version);
    for section in [
        &strings_stored,
        &nodes_stored,
        &attributes_stored,
        &values_stored,
        &keys_stored,
    ] {
        out.extend_from_slice(section);
    }
    Ok(out)
}

fn write_metadata(out: &mut Vec<u8>, meta: &Metadata, version: LsfVersion) {
    let mut section = |s: Section| {
        put_u32(out, s.uncompressed);
        put_u32(out, s.on_disk);
    };
    section(meta.strings);
    if version >= LsfVersion::Bg3NodeKeys {
        section(meta.keys);
    }
    section(meta.nodes);
    section(meta.attributes);
    section(meta.values);
    out.extend([meta.compression.0, 0]);
    put_u16(out, 0);
    put_u32(out, meta.format.0);
}

fn to_u32(len: usize) -> Result<u32> {
    u32::try_from(len).map_err(|_| invalid("section exceeds 4 GiB"))
}

fn to_i32(index: usize) -> Result<i32> {
    i32::try_from(index).map_err(|_| invalid("too many nodes or attributes"))
}

#[derive(Debug)]
struct Writer {
    format: LsfFormat,
    values_version: ValueVersion,
    names: NameTable,
    nodes: Vec<u8>,
    attributes: Vec<u8>,
    values: Vec<u8>,
    keys: Vec<u8>,
    node_count: usize,
    attribute_count: usize,
}

impl Writer {
    /// Writes `nodes` (siblings) and their subtrees: node entries in
    /// pre-order, attributes in post-order (a node's after its subtree's), the
    /// layout BG3 itself writes.
    fn write_siblings(&mut self, nodes: &[Node], parent: i32) -> Result<()> {
        for (i, node) in nodes.iter().enumerate() {
            let next_sibling = if i + 1 < nodes.len() {
                to_i32(self.node_count + subtree_size(node))?
            } else {
                -1
            };
            self.write_node(node, parent, next_sibling)?;
        }
        Ok(())
    }

    fn write_node(&mut self, node: &Node, parent: i32, next_sibling: i32) -> Result<()> {
        let index = to_i32(self.node_count)?;
        let name = self.names.add(&node.name)?;

        // The first attribute index is patched in once it is known.
        put_u32(&mut self.nodes, name);
        let first_attribute_at = if self.format.long_entries() {
            put_i32(&mut self.nodes, parent);
            put_i32(&mut self.nodes, next_sibling);
            put_i32(&mut self.nodes, -1);
            self.nodes.len() - 4
        } else {
            put_i32(&mut self.nodes, -1);
            put_i32(&mut self.nodes, parent);
            self.nodes.len() - 8
        };

        if let Some(key) = &node.key_attribute
            && self.format.version >= LsfVersion::Bg3NodeKeys
        {
            let key = self.names.add(key)?;
            put_i32(&mut self.keys, index);
            put_u32(&mut self.keys, key);
        }

        self.node_count += 1;
        self.write_siblings(&node.children, index)?;

        if !node.attributes.is_empty() {
            let first_attribute = to_i32(self.attribute_count)?;
            self.nodes[first_attribute_at..first_attribute_at + 4]
                .copy_from_slice(&first_attribute.to_le_bytes());
            self.write_attributes(node, index)?;
        }
        Ok(())
    }

    fn write_attributes(&mut self, node: &Node, node_index: i32) -> Result<()> {
        for (i, attribute) in node.attributes.iter().enumerate() {
            let offset = self.values.len();
            self.values_version
                .write(&mut self.values, &attribute.value)?;
            let length = self.values.len() - offset;
            if length > MAX_VALUE_LENGTH {
                return Err(invalid(format!(
                    "value of {:?} is too long",
                    attribute.name
                )));
            }
            let type_and_length = u32::from(attribute.value.kind() as u8) | (to_u32(length)? << 6);

            put_u32(&mut self.attributes, self.names.add(&attribute.name)?);
            put_u32(&mut self.attributes, type_and_length);
            if self.format.long_entries() {
                let next = if i + 1 < node.attributes.len() {
                    to_i32(self.attribute_count + 1)?
                } else {
                    -1
                };
                put_i32(&mut self.attributes, next);
                put_u32(&mut self.attributes, to_u32(offset)?);
            } else {
                put_i32(&mut self.attributes, node_index);
            }
            self.attribute_count += 1;
        }
        Ok(())
    }
}

fn subtree_size(node: &Node) -> usize {
    1 + node.children.iter().map(subtree_size).sum::<usize>()
}

/// The name hash table. Bucket choice is arbitrary for readers, which only
/// follow the stored `(bucket << 16) | index` references.
#[derive(Debug)]
struct NameTable {
    buckets: Vec<Vec<String>>,
    index: HashMap<String, u32>,
}

impl Default for NameTable {
    fn default() -> Self {
        Self {
            buckets: vec![Vec::new(); NAME_BUCKETS],
            index: HashMap::new(),
        }
    }
}

impl NameTable {
    fn add(&mut self, name: &str) -> Result<u32> {
        if let Some(&packed) = self.index.get(name) {
            return Ok(packed);
        }
        // FNV-1a, folded to 9 bits the way LSLib folds .NET's hash code.
        let hash = name.bytes().fold(0x811c_9dc5u32, |h, b| {
            (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
        });
        let bucket = (hash & 0x1ff)
            ^ ((hash >> 9) & 0x1ff)
            ^ ((hash >> 18) & 0x1ff)
            ^ ((hash >> 27) & 0x1ff);
        let chain = &mut self.buckets[bucket as usize];
        let position = u32::try_from(chain.len())
            .ok()
            .filter(|&p| p <= 0xffff)
            .ok_or_else(|| invalid("too many names"))?;
        chain.push(name.to_owned());
        let packed = (bucket << 16) | position;
        self.index.insert(name.to_owned(), packed);
        Ok(packed)
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        put_u32(&mut out, to_u32(self.buckets.len())?);
        for bucket in &self.buckets {
            put_u16(
                &mut out,
                u16::try_from(bucket.len()).map_err(|_| invalid("too many names"))?,
            );
            for name in bucket {
                put_u16(
                    &mut out,
                    u16::try_from(name.len()).map_err(|_| invalid("name too long"))?,
                );
                out.extend_from_slice(name.as_bytes());
            }
        }
        Ok(out)
    }
}

/// Encodes attribute values; translated strings depend on the version.
#[derive(Debug, Clone, Copy)]
struct ValueVersion {
    version: LsfVersion,
    engine: PackedVersion,
}

impl ValueVersion {
    fn write(self, out: &mut Vec<u8>, value: &AttributeValue) -> Result<()> {
        use AttributeValue as V;
        match value {
            V::None => {}
            V::Byte(v) => out.push(*v),
            V::Short(v) => out.extend_from_slice(&v.to_le_bytes()),
            V::UShort(v) => out.extend_from_slice(&v.to_le_bytes()),
            V::Int(v) => out.extend_from_slice(&v.to_le_bytes()),
            V::UInt(v) => out.extend_from_slice(&v.to_le_bytes()),
            V::Float(v) => out.extend_from_slice(&v.to_le_bytes()),
            V::Double(v) => out.extend_from_slice(&v.to_le_bytes()),
            V::IVec2(v) => put_ints(out, v),
            V::IVec3(v) => put_ints(out, v),
            V::IVec4(v) => put_ints(out, v),
            V::Vec2(v) => put_floats(out, v),
            V::Vec3(v) => put_floats(out, v),
            V::Vec4(v) | V::Mat2(v) => put_floats(out, v),
            V::Mat3(v) => put_floats(out, v),
            V::Mat3x4(v) | V::Mat4x3(v) => put_floats(out, v),
            V::Mat4(v) => put_floats(out, v),
            V::Bool(v) => out.push(u8::from(*v)),
            V::String(s)
            | V::Path(s)
            | V::FixedString(s)
            | V::LSString(s)
            | V::WString(s)
            | V::LSWString(s) => put_string(out, s),
            V::ULongLong(v) => out.extend_from_slice(&v.to_le_bytes()),
            V::ScratchBuffer(v) => out.extend_from_slice(v),
            V::Long(v) | V::Int64(v) => out.extend_from_slice(&v.to_le_bytes()),
            V::Int8(v) => out.extend_from_slice(&v.to_le_bytes()),
            V::Uuid(v) => out.extend_from_slice(v),
            V::TranslatedString(ts) => {
                if translated_string_has_version(self.version, self.engine) {
                    put_u16(out, ts.version);
                } else {
                    put_string_with_length(out, ts.value.as_deref().unwrap_or_default())?;
                }
                put_string_with_length(out, &ts.handle)?;
            }
            V::TranslatedFSString(fs) => self.write_fs_string(out, fs)?,
        }
        Ok(())
    }

    fn write_fs_string(self, out: &mut Vec<u8>, fs: &TranslatedFSString) -> Result<()> {
        // Matches the reader's test (LSLib's writer uses a different one).
        if self.version >= LsfVersion::Bg3 {
            put_u16(out, fs.version);
        } else {
            put_string_with_length(out, fs.value.as_deref().unwrap_or_default())?;
        }
        put_string_with_length(out, &fs.handle)?;
        put_u32(out, to_u32(fs.arguments.len())?);
        for argument in &fs.arguments {
            put_string_with_length(out, &argument.key)?;
            self.write_fs_string(out, &argument.string)?;
            put_string_with_length(out, &argument.value)?;
        }
        Ok(())
    }
}

fn put_ints(out: &mut Vec<u8>, items: &[i32]) {
    for item in items {
        out.extend_from_slice(&item.to_le_bytes());
    }
}

fn put_floats(out: &mut Vec<u8>, items: &[f32]) {
    for item in items {
        out.extend_from_slice(&item.to_le_bytes());
    }
}

fn put_string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(s.as_bytes());
    out.push(0);
}

fn put_string_with_length(out: &mut Vec<u8>, s: &str) -> Result<()> {
    put_u32(out, to_u32(s.len() + 1)?);
    put_string(out, s);
    Ok(())
}

fn put_i32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}
