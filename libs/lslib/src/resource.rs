//! The format-independent resource tree (`LSLib` `Resource.cs`,
//! `NodeAttribute.cs`): regions of nested nodes carrying typed attributes.
//!
//! Unlike `LSLib`, nodes keep children and attributes in file order (`LSLib`
//! groups children by name), so a read-modify-write cycle changes only what
//! was edited.

/// A whole resource: its root nodes ("regions").
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Resource {
    /// The engine version that wrote the file.
    pub engine_version: PackedVersion,
    pub regions: Vec<Node>,
}

impl Resource {
    /// The first region named `name`.
    pub fn region(&self, name: &str) -> Option<&Node> {
        self.regions.iter().find(|r| r.name == name)
    }

    pub fn region_mut(&mut self, name: &str) -> Option<&mut Node> {
        self.regions.iter_mut().find(|r| r.name == name)
    }
}

/// A node: a named struct of attributes and child nodes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Node {
    pub name: String,
    /// The attribute that identifies this node among its siblings (BG3).
    pub key_attribute: Option<String>,
    pub attributes: Vec<Attribute>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// The value of the first attribute named `name`.
    pub fn attribute(&self, name: &str) -> Option<&AttributeValue> {
        self.attributes
            .iter()
            .find(|a| a.name == name)
            .map(|a| &a.value)
    }

    pub fn attribute_mut(&mut self, name: &str) -> Option<&mut AttributeValue> {
        self.attributes
            .iter_mut()
            .find(|a| a.name == name)
            .map(|a| &mut a.value)
    }

    /// Replaces the value of the attribute `name`, or appends it.
    pub fn set_attribute(&mut self, name: &str, value: AttributeValue) {
        match self.attribute_mut(name) {
            Some(slot) => *slot = value,
            None => self.attributes.push(Attribute {
                name: name.to_owned(),
                value,
            }),
        }
    }

    /// The first child named `name`.
    pub fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    pub fn child_mut(&mut self, name: &str) -> Option<&mut Node> {
        self.children.iter_mut().find(|c| c.name == name)
    }

    /// Every child named `name`, in order.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children.iter().filter(move |c| c.name == name)
    }
}

/// A named, typed value on a node.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub value: AttributeValue,
}

/// The type ids of attribute values, as stored in files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AttributeType {
    None = 0,
    Byte = 1,
    Short = 2,
    UShort = 3,
    Int = 4,
    UInt = 5,
    Float = 6,
    Double = 7,
    IVec2 = 8,
    IVec3 = 9,
    IVec4 = 10,
    Vec2 = 11,
    Vec3 = 12,
    Vec4 = 13,
    Mat2 = 14,
    Mat3 = 15,
    Mat3x4 = 16,
    Mat4x3 = 17,
    Mat4 = 18,
    Bool = 19,
    String = 20,
    Path = 21,
    FixedString = 22,
    LSString = 23,
    ULongLong = 24,
    ScratchBuffer = 25,
    Long = 26,
    Int8 = 27,
    TranslatedString = 28,
    WString = 29,
    LSWString = 30,
    Uuid = 31,
    Int64 = 32,
    TranslatedFSString = 33,
}

impl AttributeType {
    pub fn from_id(id: u32) -> Option<Self> {
        use AttributeType as T;
        const ALL: [AttributeType; 34] = [
            T::None,
            T::Byte,
            T::Short,
            T::UShort,
            T::Int,
            T::UInt,
            T::Float,
            T::Double,
            T::IVec2,
            T::IVec3,
            T::IVec4,
            T::Vec2,
            T::Vec3,
            T::Vec4,
            T::Mat2,
            T::Mat3,
            T::Mat3x4,
            T::Mat4x3,
            T::Mat4,
            T::Bool,
            T::String,
            T::Path,
            T::FixedString,
            T::LSString,
            T::ULongLong,
            T::ScratchBuffer,
            T::Long,
            T::Int8,
            T::TranslatedString,
            T::WString,
            T::LSWString,
            T::Uuid,
            T::Int64,
            T::TranslatedFSString,
        ];
        ALL.get(usize::try_from(id).ok()?).copied()
    }

    /// The type name `LSLib` uses in LSX files.
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Byte => "uint8",
            Self::Short => "int16",
            Self::UShort => "uint16",
            Self::Int => "int32",
            Self::UInt => "uint32",
            Self::Float => "float",
            Self::Double => "double",
            Self::IVec2 => "ivec2",
            Self::IVec3 => "ivec3",
            Self::IVec4 => "ivec4",
            Self::Vec2 => "fvec2",
            Self::Vec3 => "fvec3",
            Self::Vec4 => "fvec4",
            Self::Mat2 => "mat2x2",
            Self::Mat3 => "mat3x3",
            Self::Mat3x4 => "mat3x4",
            Self::Mat4x3 => "mat4x3",
            Self::Mat4 => "mat4x4",
            Self::Bool => "bool",
            Self::String => "string",
            Self::Path => "path",
            Self::FixedString => "FixedString",
            Self::LSString => "LSString",
            Self::ULongLong => "uint64",
            Self::ScratchBuffer => "ScratchBuffer",
            Self::Long => "old_int64",
            Self::Int8 => "int8",
            Self::TranslatedString => "TranslatedString",
            Self::WString => "WString",
            Self::LSWString => "LSWString",
            Self::Uuid => "guid",
            Self::Int64 => "int64",
            Self::TranslatedFSString => "TranslatedFSString",
        }
    }
}

/// An attribute value. Matrices are stored flat in file order (column by
/// column); UUIDs as their 16 bytes on disk.
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    None,
    Byte(u8),
    Short(i16),
    UShort(u16),
    Int(i32),
    UInt(u32),
    Float(f32),
    Double(f64),
    IVec2([i32; 2]),
    IVec3([i32; 3]),
    IVec4([i32; 4]),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Mat2([f32; 4]),
    Mat3([f32; 9]),
    Mat3x4([f32; 12]),
    Mat4x3([f32; 12]),
    Mat4([f32; 16]),
    Bool(bool),
    String(String),
    Path(String),
    FixedString(String),
    LSString(String),
    ULongLong(u64),
    ScratchBuffer(Vec<u8>),
    Long(i64),
    Int8(i8),
    TranslatedString(TranslatedString),
    WString(String),
    LSWString(String),
    Uuid([u8; 16]),
    Int64(i64),
    TranslatedFSString(TranslatedFSString),
}

impl AttributeValue {
    pub fn kind(&self) -> AttributeType {
        use AttributeType as T;
        match self {
            Self::None => T::None,
            Self::Byte(_) => T::Byte,
            Self::Short(_) => T::Short,
            Self::UShort(_) => T::UShort,
            Self::Int(_) => T::Int,
            Self::UInt(_) => T::UInt,
            Self::Float(_) => T::Float,
            Self::Double(_) => T::Double,
            Self::IVec2(_) => T::IVec2,
            Self::IVec3(_) => T::IVec3,
            Self::IVec4(_) => T::IVec4,
            Self::Vec2(_) => T::Vec2,
            Self::Vec3(_) => T::Vec3,
            Self::Vec4(_) => T::Vec4,
            Self::Mat2(_) => T::Mat2,
            Self::Mat3(_) => T::Mat3,
            Self::Mat3x4(_) => T::Mat3x4,
            Self::Mat4x3(_) => T::Mat4x3,
            Self::Mat4(_) => T::Mat4,
            Self::Bool(_) => T::Bool,
            Self::String(_) => T::String,
            Self::Path(_) => T::Path,
            Self::FixedString(_) => T::FixedString,
            Self::LSString(_) => T::LSString,
            Self::ULongLong(_) => T::ULongLong,
            Self::ScratchBuffer(_) => T::ScratchBuffer,
            Self::Long(_) => T::Long,
            Self::Int8(_) => T::Int8,
            Self::TranslatedString(_) => T::TranslatedString,
            Self::WString(_) => T::WString,
            Self::LSWString(_) => T::LSWString,
            Self::Uuid(_) => T::Uuid,
            Self::Int64(_) => T::Int64,
            Self::TranslatedFSString(_) => T::TranslatedFSString,
        }
    }

    /// The text of any of the plain string types.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s)
            | Self::Path(s)
            | Self::FixedString(s)
            | Self::LSString(s)
            | Self::WString(s)
            | Self::LSWString(s) => Some(s),
            _ => None,
        }
    }
}

/// The text form of a stored UUID, as the game and `LSLib` print it: .NET
/// GUID order (first three groups little-endian) with bytes 8..16 swapped in
/// pairs, e.g. `GustavX`'s `cb555efe-2d9e-131f-8195-a89329d218ea`.
pub fn uuid_string(bytes: &[u8; 16]) -> String {
    use std::fmt::Write as _;
    const ORDER: [usize; 16] = [3, 2, 1, 0, 5, 4, 7, 6, 9, 8, 11, 10, 13, 12, 15, 14];
    ORDER
        .iter()
        .enumerate()
        .fold(String::with_capacity(36), |mut out, (i, &b)| {
            if matches!(i, 4 | 6 | 8 | 10) {
                out.push('-');
            }
            let _ = write!(out, "{:02x}", bytes[b]);
            out
        })
}

/// A localized string reference.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TranslatedString {
    /// Handle version (newer files).
    pub version: u16,
    /// Inline text (older files only; newer files store a version instead).
    pub value: Option<String>,
    pub handle: String,
}

/// A localized format string with arguments.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TranslatedFSString {
    pub version: u16,
    pub value: Option<String>,
    pub handle: String,
    pub arguments: Vec<TranslatedFSStringArgument>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TranslatedFSStringArgument {
    pub key: String,
    pub string: TranslatedFSString,
    pub value: String,
}

/// An engine version, packed into 32 bits (older files) or 64 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PackedVersion {
    pub major: u32,
    pub minor: u32,
    pub revision: u32,
    pub build: u32,
}

#[allow(clippy::cast_possible_truncation)]
impl PackedVersion {
    pub fn from_u64(packed: u64) -> Self {
        Self {
            major: ((packed >> 55) & 0x7f) as u32,
            minor: ((packed >> 47) & 0xff) as u32,
            revision: ((packed >> 31) & 0xffff) as u32,
            build: (packed & 0x7fff_ffff) as u32,
        }
    }

    pub fn from_u32(packed: u32) -> Self {
        Self {
            major: (packed >> 28) & 0x0f,
            minor: (packed >> 24) & 0x0f,
            revision: (packed >> 16) & 0xff,
            build: packed & 0xffff,
        }
    }

    pub fn to_u64(self) -> u64 {
        (u64::from(self.major) & 0x7f) << 55
            | (u64::from(self.minor) & 0xff) << 47
            | (u64::from(self.revision) & 0xffff) << 31
            | (u64::from(self.build) & 0x7fff_ffff)
    }

    pub fn to_u32(self) -> u32 {
        (self.major & 0x0f) << 28
            | (self.minor & 0x0f) << 24
            | (self.revision & 0xff) << 16
            | (self.build & 0xffff)
    }
}
