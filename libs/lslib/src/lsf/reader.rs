use std::borrow::Cow;
use std::collections::HashMap;

use super::{
    LsfFile, LsfFormat, LsfVersion, MAX_DEPTH, Metadata, MetadataFormat, SIGNATURE, Section,
    effective_engine, translated_string_has_version,
};
use crate::binary::Reader;
use crate::compression::{self, CompressionFlags, CompressionMethod};
use crate::resource::{
    Attribute, AttributeType, AttributeValue, Node, PackedVersion, Resource, TranslatedFSString,
    TranslatedFSStringArgument, TranslatedString,
};
use crate::{LslibError, Result};

/// Deepest nesting of translated format string arguments accepted on read.
const MAX_FS_STRING_DEPTH: usize = 64;

const SHORT_NODE_SIZE: usize = 12;
const LONG_NODE_SIZE: usize = 16;
const SHORT_ATTRIBUTE_SIZE: usize = 12;
const LONG_ATTRIBUTE_SIZE: usize = 16;
const KEY_SIZE: usize = 8;

fn invalid(message: impl Into<String>) -> LslibError {
    LslibError::InvalidLsf(message.into())
}

pub(super) fn read(data: &[u8]) -> Result<LsfFile> {
    let mut r = Reader::new(data, "LSF header");
    if r.array::<4>()? != SIGNATURE {
        return Err(invalid("missing LSOF signature"));
    }
    let raw_version = r.u32()?;
    let version = LsfVersion::from_u32(raw_version)
        .ok_or_else(|| invalid(format!("version {raw_version} is not supported")))?;
    let engine_version = if version >= LsfVersion::Bg3ExtendedHeader {
        PackedVersion::from_u64(r.u64()?)
    } else {
        PackedVersion::from_u32(r.u32()?)
    };
    let meta = read_metadata(&mut r, version)?;
    let format = LsfFormat {
        version,
        metadata_format: meta.format,
        compression: meta.compression,
    };
    let chunked = format.chunked();

    let names = read_names(&read_section(
        &mut r,
        meta.strings,
        meta.compression,
        false,
    )?)?;
    let mut nodes = read_nodes(
        &read_section(&mut r, meta.nodes, meta.compression, chunked)?,
        format.long_entries(),
    )?;
    let attributes = read_section(&mut r, meta.attributes, meta.compression, chunked)?;
    let attributes = if format.long_entries() {
        read_attributes_long(&attributes)?
    } else {
        read_attributes_short(&attributes)?
    };
    let values = read_section(&mut r, meta.values, meta.compression, chunked)?;
    // Unlike LSLib, keys are read whatever the metadata format says.
    let keys = read_section(&mut r, meta.keys, meta.compression, chunked)?;
    read_keys(&keys, &names, &mut nodes)?;

    let values = ValueReader {
        data: &values,
        version,
        engine: effective_engine(version, engine_version),
    };
    let regions = build_tree(&names, &nodes, &attributes, &values)?;
    Ok(LsfFile {
        format,
        resource: Resource {
            engine_version,
            regions,
        },
    })
}

fn read_metadata(r: &mut Reader<'_>, version: LsfVersion) -> Result<Metadata> {
    let mut section = || -> Result<Section> {
        Ok(Section {
            uncompressed: r.u32()?,
            on_disk: r.u32()?,
        })
    };
    let strings = section()?;
    let keys = if version >= LsfVersion::Bg3NodeKeys {
        section()?
    } else {
        Section::default()
    };
    let nodes = section()?;
    let attributes = section()?;
    let values = section()?;
    let compression = CompressionFlags(r.u8()?);
    let _unknown2 = r.u8()?;
    let _unknown3 = r.u16()?;
    let format = MetadataFormat(r.u32()?);
    Ok(Metadata {
        strings,
        keys,
        nodes,
        attributes,
        values,
        compression,
        format,
    })
}

/// The next section, decompressed. A zero size on disk with a non-zero size
/// means the section is stored raw.
fn read_section<'a>(
    r: &mut Reader<'a>,
    section: Section,
    flags: CompressionFlags,
    chunked: bool,
) -> Result<Cow<'a, [u8]>> {
    let size = section.uncompressed as usize;
    if section.on_disk == 0 {
        return r.bytes(size).map(Cow::Borrowed);
    }
    let method = flags.method()?;
    if method == CompressionMethod::None {
        return r.bytes(size).map(Cow::Borrowed);
    }
    let stored = r.bytes(section.on_disk as usize)?;
    if chunked && method == CompressionMethod::Lz4 {
        compression::decompress_lz4_frame(stored, size).map(Cow::Owned)
    } else {
        compression::decompress(stored, size, method).map(Cow::Owned)
    }
}

/// The name hash table: buckets of strings, addressed as
/// `(bucket << 16) | index`.
fn read_names(data: &[u8]) -> Result<Vec<Vec<String>>> {
    let mut r = Reader::new(data, "LSF names");
    let buckets = r.u32()?;
    let mut names = Vec::new();
    for _ in 0..buckets {
        let count = r.u16()?;
        let mut bucket = Vec::with_capacity(count.into());
        for _ in 0..count {
            let len = r.u16()?;
            let bytes = r.bytes(len.into())?;
            let name = std::str::from_utf8(bytes).map_err(|_| invalid("name is not UTF-8"))?;
            bucket.push(name.to_owned());
        }
        names.push(bucket);
    }
    Ok(names)
}

fn name(names: &[Vec<String>], packed: u32) -> Result<&str> {
    names
        .get((packed >> 16) as usize)
        .and_then(|bucket| bucket.get((packed & 0xffff) as usize))
        .map(String::as_str)
        .ok_or_else(|| invalid(format!("name reference {packed:#x} is out of range")))
}

#[derive(Debug)]
struct NodeInfo {
    name: u32,
    parent: i32,
    first_attribute: i32,
    key: Option<String>,
}

fn read_nodes(data: &[u8], long: bool) -> Result<Vec<NodeInfo>> {
    let size = if long {
        LONG_NODE_SIZE
    } else {
        SHORT_NODE_SIZE
    };
    if !data.len().is_multiple_of(size) {
        return Err(invalid("node list has a partial entry"));
    }
    data.chunks_exact(size)
        .map(|chunk| {
            let mut r = Reader::new(chunk, "LSF node");
            let name = r.u32()?;
            let (parent, first_attribute) = if long {
                let parent = r.i32()?;
                let _next_sibling = r.i32()?;
                (parent, r.i32()?)
            } else {
                let first_attribute = r.i32()?;
                (r.i32()?, first_attribute)
            };
            Ok(NodeInfo {
                name,
                parent,
                first_attribute,
                key: None,
            })
        })
        .collect()
}

#[derive(Debug)]
struct AttributeInfo {
    name: u32,
    type_id: u32,
    length: u32,
    data_offset: u32,
    next: i32,
}

/// Short entries name their node; each node's attributes are chained in
/// order and their values are laid out back to back.
fn read_attributes_short(data: &[u8]) -> Result<Vec<AttributeInfo>> {
    if !data.len().is_multiple_of(SHORT_ATTRIBUTE_SIZE) {
        return Err(invalid("attribute list has a partial entry"));
    }
    let mut attributes: Vec<AttributeInfo> = Vec::with_capacity(data.len() / SHORT_ATTRIBUTE_SIZE);
    let mut last_of_node: HashMap<i32, usize> = HashMap::new();
    let mut data_offset = 0u32;
    for (index, chunk) in data
        .as_chunks::<SHORT_ATTRIBUTE_SIZE>()
        .0
        .iter()
        .enumerate()
    {
        let mut r = Reader::new(chunk, "LSF attribute");
        let name = r.u32()?;
        let type_and_length = r.u32()?;
        let node = r.i32()?;
        if let Some(previous) = last_of_node.insert(node, index) {
            attributes[previous].next =
                i32::try_from(index).map_err(|_| invalid("too many attributes"))?;
        }
        let length = type_and_length >> 6;
        attributes.push(AttributeInfo {
            name,
            type_id: type_and_length & 0x3f,
            length,
            data_offset,
            next: -1,
        });
        data_offset = data_offset
            .checked_add(length)
            .ok_or_else(|| invalid("values exceed 4 GiB"))?;
    }
    Ok(attributes)
}

/// Long entries carry the next attribute and the value offset themselves.
fn read_attributes_long(data: &[u8]) -> Result<Vec<AttributeInfo>> {
    if !data.len().is_multiple_of(LONG_ATTRIBUTE_SIZE) {
        return Err(invalid("attribute list has a partial entry"));
    }
    data.as_chunks::<LONG_ATTRIBUTE_SIZE>()
        .0
        .iter()
        .map(|chunk| {
            let mut r = Reader::new(chunk, "LSF attribute");
            let name = r.u32()?;
            let type_and_length = r.u32()?;
            Ok(AttributeInfo {
                name,
                type_id: type_and_length & 0x3f,
                length: type_and_length >> 6,
                next: r.i32()?,
                data_offset: r.u32()?,
            })
        })
        .collect()
}

fn read_keys(data: &[u8], names: &[Vec<String>], nodes: &mut [NodeInfo]) -> Result<()> {
    if !data.len().is_multiple_of(KEY_SIZE) {
        return Err(invalid("key list has a partial entry"));
    }
    for chunk in data.as_chunks::<KEY_SIZE>().0 {
        let mut r = Reader::new(chunk, "LSF key");
        let node = r.u32()? as usize;
        let key = name(names, r.u32()?)?.to_owned();
        nodes
            .get_mut(node)
            .ok_or_else(|| invalid(format!("key for node {node}, which does not exist")))?
            .key = Some(key);
    }
    Ok(())
}

/// Assembles the node list into trees. Parents always precede their
/// children, so walking backwards attaches every subtree before its parent
/// is moved, without recursion.
fn build_tree(
    names: &[Vec<String>],
    infos: &[NodeInfo],
    attributes: &[AttributeInfo],
    values: &ValueReader<'_>,
) -> Result<Vec<Node>> {
    let mut parents = Vec::with_capacity(infos.len());
    let mut depths: Vec<usize> = Vec::with_capacity(infos.len());
    let mut nodes = Vec::with_capacity(infos.len());
    for (index, info) in infos.iter().enumerate() {
        let parent = match info.parent {
            -1 => None,
            p => Some(
                usize::try_from(p)
                    .ok()
                    .filter(|&p| p < index)
                    .ok_or_else(|| invalid(format!("node {index} has invalid parent {p}")))?,
            ),
        };
        let depth = parent.map_or(0, |p| depths[p] + 1);
        if depth > MAX_DEPTH {
            return Err(invalid("nodes are nested too deeply"));
        }
        parents.push(parent);
        depths.push(depth);
        nodes.push(Some(Node {
            name: name(names, info.name)?.to_owned(),
            key_attribute: info.key.clone(),
            attributes: read_node_attributes(names, info, attributes, values)?,
            children: Vec::new(),
        }));
    }

    let mut regions = Vec::new();
    for index in (0..nodes.len()).rev() {
        let mut node = nodes[index].take().expect("each node is taken once");
        node.children.reverse();
        match parents[index] {
            Some(parent) => nodes[parent]
                .as_mut()
                .expect("parents precede children")
                .children
                .push(node),
            None => regions.push(node),
        }
    }
    regions.reverse();
    Ok(regions)
}

fn read_node_attributes(
    names: &[Vec<String>],
    info: &NodeInfo,
    attributes: &[AttributeInfo],
    values: &ValueReader<'_>,
) -> Result<Vec<Attribute>> {
    let mut out = Vec::new();
    let mut next = info.first_attribute;
    while next != -1 {
        // A well-formed chain visits each attribute at most once.
        if out.len() >= attributes.len() {
            return Err(invalid("attribute chain loops"));
        }
        let attribute = usize::try_from(next)
            .ok()
            .and_then(|i| attributes.get(i))
            .ok_or_else(|| invalid(format!("attribute index {next} is out of range")))?;
        let kind = AttributeType::from_id(attribute.type_id)
            .ok_or_else(|| invalid(format!("unknown attribute type {}", attribute.type_id)))?;
        out.push(Attribute {
            name: name(names, attribute.name)?.to_owned(),
            value: values.read(
                kind,
                attribute.data_offset as usize,
                attribute.length as usize,
            )?,
        });
        next = attribute.next;
    }
    Ok(out)
}

/// Decodes attribute values from the value section.
#[derive(Debug)]
struct ValueReader<'a> {
    data: &'a [u8],
    version: LsfVersion,
    engine: PackedVersion,
}

impl ValueReader<'_> {
    fn read(&self, kind: AttributeType, offset: usize, length: usize) -> Result<AttributeValue> {
        use AttributeValue as V;
        let mut r = Reader::new(self.data, "LSF value");
        r.seek(offset);
        Ok(match kind {
            AttributeType::None => V::None,
            AttributeType::Byte => V::Byte(r.u8()?),
            AttributeType::Short => V::Short(r.i16()?),
            AttributeType::UShort => V::UShort(r.u16()?),
            AttributeType::Int => V::Int(r.i32()?),
            AttributeType::UInt => V::UInt(r.u32()?),
            AttributeType::Float => V::Float(r.f32()?),
            AttributeType::Double => V::Double(r.f64()?),
            AttributeType::IVec2 => V::IVec2(ints(&mut r)?),
            AttributeType::IVec3 => V::IVec3(ints(&mut r)?),
            AttributeType::IVec4 => V::IVec4(ints(&mut r)?),
            AttributeType::Vec2 => V::Vec2(floats(&mut r)?),
            AttributeType::Vec3 => V::Vec3(floats(&mut r)?),
            AttributeType::Vec4 => V::Vec4(floats(&mut r)?),
            AttributeType::Mat2 => V::Mat2(floats(&mut r)?),
            AttributeType::Mat3 => V::Mat3(floats(&mut r)?),
            AttributeType::Mat3x4 => V::Mat3x4(floats(&mut r)?),
            AttributeType::Mat4x3 => V::Mat4x3(floats(&mut r)?),
            AttributeType::Mat4 => V::Mat4(floats(&mut r)?),
            AttributeType::Bool => V::Bool(r.u8()? != 0),
            AttributeType::String => V::String(string(&mut r, length)?),
            AttributeType::Path => V::Path(string(&mut r, length)?),
            AttributeType::FixedString => V::FixedString(string(&mut r, length)?),
            AttributeType::LSString => V::LSString(string(&mut r, length)?),
            AttributeType::WString => V::WString(string(&mut r, length)?),
            AttributeType::LSWString => V::LSWString(string(&mut r, length)?),
            AttributeType::ULongLong => V::ULongLong(r.u64()?),
            AttributeType::ScratchBuffer => V::ScratchBuffer(r.bytes(length)?.to_vec()),
            AttributeType::Long => V::Long(r.i64()?),
            AttributeType::Int64 => V::Int64(r.i64()?),
            AttributeType::Int8 => V::Int8(r.i8()?),
            AttributeType::Uuid => V::Uuid(r.array()?),
            AttributeType::TranslatedString => V::TranslatedString(self.translated_string(&mut r)?),
            AttributeType::TranslatedFSString => {
                V::TranslatedFSString(self.translated_fs_string(&mut r, 0)?)
            }
        })
    }

    fn translated_string(&self, r: &mut Reader<'_>) -> Result<TranslatedString> {
        let (version, value) = if translated_string_has_version(self.version, self.engine) {
            (r.u16()?, None)
        } else {
            (0, Some(string_with_length(r)?))
        };
        Ok(TranslatedString {
            version,
            value,
            handle: string_with_length(r)?,
        })
    }

    fn translated_fs_string(&self, r: &mut Reader<'_>, depth: usize) -> Result<TranslatedFSString> {
        if depth > MAX_FS_STRING_DEPTH {
            return Err(invalid("translated string arguments are nested too deeply"));
        }
        // LSLib reads format strings with the plain version test only.
        let (version, value) = if self.version >= LsfVersion::Bg3 {
            (r.u16()?, None)
        } else {
            (0, Some(string_with_length(r)?))
        };
        let handle = string_with_length(r)?;
        let count = r.u32()? as usize;
        // Each argument takes at least 12 bytes; reject counts the data
        // cannot hold before allocating.
        if count > r.remaining() / 12 {
            return Err(LslibError::Truncated("translated string arguments"));
        }
        let mut arguments = Vec::with_capacity(count);
        for _ in 0..count {
            let key = string_with_length(r)?;
            let string = self.translated_fs_string(r, depth + 1)?;
            let value = string_with_length(r)?;
            arguments.push(TranslatedFSStringArgument { key, string, value });
        }
        Ok(TranslatedFSString {
            version,
            value,
            handle,
            arguments,
        })
    }
}

fn ints<const N: usize>(r: &mut Reader<'_>) -> Result<[i32; N]> {
    let mut out = [0; N];
    for item in &mut out {
        *item = r.i32()?;
    }
    Ok(out)
}

fn floats<const N: usize>(r: &mut Reader<'_>) -> Result<[f32; N]> {
    let mut out = [0.0; N];
    for item in &mut out {
        *item = r.f32()?;
    }
    Ok(out)
}

/// A NUL-terminated string of `length` bytes (terminator included); trailing
/// NULs before the terminator are dropped, as `LSLib` does.
fn string(r: &mut Reader<'_>, length: usize) -> Result<String> {
    let body_len = length
        .checked_sub(1)
        .ok_or_else(|| invalid("string has no terminator"))?;
    let body = r.bytes(body_len)?;
    if r.u8()? != 0 {
        return Err(invalid("string is not NUL-terminated"));
    }
    let end = body.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    std::str::from_utf8(&body[..end])
        .map(str::to_owned)
        .map_err(|_| invalid("string is not UTF-8"))
}

/// A 32-bit length followed by that many bytes of NUL-terminated string.
fn string_with_length(r: &mut Reader<'_>) -> Result<String> {
    let length = r.u32()? as usize;
    string(r, length)
}
