use lslib::LslibError;
use lslib::compression::{CompressionFlags, CompressionLevel, CompressionMethod};
use lslib::lsf::{LsfFile, LsfFormat, LsfVersion, MetadataFormat};
use lslib::package::Package;
use lslib::resource::{
    Attribute, AttributeValue, Node, PackedVersion, Resource, TranslatedFSString,
    TranslatedFSStringArgument, TranslatedString, uuid_string,
};

const ALL_VERSIONS: [LsfVersion; 7] = [
    LsfVersion::Initial,
    LsfVersion::ChunkedCompress,
    LsfVersion::ExtendedNodes,
    LsfVersion::Bg3,
    LsfVersion::Bg3ExtendedHeader,
    LsfVersion::Bg3NodeKeys,
    LsfVersion::Bg3Patch3,
];

fn attr(name: &str, value: AttributeValue) -> Attribute {
    Attribute {
        name: name.to_owned(),
        value,
    }
}

/// One attribute of every type. Translated strings carry a version (rather
/// than inline text) with the sample's engine 4.8; format strings only from
/// BG3 files.
fn every_value(version: LsfVersion) -> Vec<Attribute> {
    use AttributeValue as V;

    let bg3 = version >= LsfVersion::Bg3;
    let translated = |handle: &str| TranslatedString {
        version: 3,
        value: None,
        handle: handle.to_owned(),
    };
    let fs = |handle: &str, arguments| TranslatedFSString {
        version: u16::from(bg3),
        value: (!bg3).then(|| "fs text".to_owned()),
        handle: handle.to_owned(),
        arguments,
    };
    let nested = fs(
        "h-outer",
        vec![TranslatedFSStringArgument {
            key: "Arg".to_owned(),
            string: fs("h-inner", Vec::new()),
            value: "42".to_owned(),
        }],
    );

    vec![
        attr("None", V::None),
        attr("Byte", V::Byte(200)),
        attr("Short", V::Short(-1234)),
        attr("UShort", V::UShort(60000)),
        attr("Int", V::Int(-7)),
        attr("UInt", V::UInt(4_000_000_000)),
        attr("Float", V::Float(1.5)),
        attr("Double", V::Double(-2.25)),
        attr("IVec2", V::IVec2([1, -2])),
        attr("IVec3", V::IVec3([1, 2, 3])),
        attr("IVec4", V::IVec4([1, 2, 3, 4])),
        attr("Vec2", V::Vec2([0.5, 1.0])),
        attr("Vec3", V::Vec3([0.5, 1.0, 2.0])),
        attr("Vec4", V::Vec4([0.5, 1.0, 2.0, 4.0])),
        attr("Mat2", V::Mat2([1.0, 0.0, 0.0, 1.0])),
        attr("Mat3", V::Mat3([1.0; 9])),
        attr("Mat3x4", V::Mat3x4([2.0; 12])),
        attr("Mat4x3", V::Mat4x3([3.0; 12])),
        attr("Mat4", V::Mat4([4.0; 16])),
        attr("Bool", V::Bool(true)),
        attr("String", V::String("hello".to_owned())),
        attr("Path", V::Path("Public/Game/x.lsx".to_owned())),
        attr("FixedString", V::FixedString("S_Player_Karlach".to_owned())),
        attr("LSString", V::LSString("Ünïcödé ✓".to_owned())),
        attr("Empty", V::LSString(String::new())),
        attr("ULongLong", V::ULongLong(u64::MAX)),
        attr("ScratchBuffer", V::ScratchBuffer(vec![0, 1, 2, 0xff])),
        attr("Long", V::Long(i64::MIN)),
        attr("Int8", V::Int8(-5)),
        attr("TranslatedString", V::TranslatedString(translated("h1234"))),
        attr("WString", V::WString("wide".to_owned())),
        attr("LSWString", V::LSWString("ls wide".to_owned())),
        attr("Uuid", V::Uuid(*b"0123456789abcdef")),
        attr("Int64", V::Int64(-99)),
        attr("TranslatedFSString", V::TranslatedFSString(nested)),
    ]
}

fn sample(version: LsfVersion) -> Resource {
    let keys = version >= LsfVersion::Bg3NodeKeys;
    let leaf = |name: &str, id: u32| {
        let mut node = Node::new(name);
        node.set_attribute("ID", AttributeValue::UInt(id));
        node.key_attribute = keys.then(|| "ID".to_owned());
        node
    };
    // Children with interleaved names must keep their order.
    let mut items = Node::new("Items");
    items.children = vec![leaf("Item", 1), leaf("Gold", 2), leaf("Item", 3)];
    items.set_attribute("Count", AttributeValue::Int(3));

    let mut values = Node::new("Values");
    values.attributes = every_value(version);

    Resource {
        engine_version: PackedVersion {
            major: 4,
            minor: 8,
            revision: 0,
            build: 700,
        },
        regions: vec![
            Node {
                children: vec![items, values],
                ..Node::new("Globals")
            },
            Node::new("EmptyRegion"),
        ],
    }
}

fn formats() -> Vec<LsfFormat> {
    let mut out = Vec::new();
    for version in ALL_VERSIONS {
        for metadata_format in [
            MetadataFormat::NONE,
            MetadataFormat::KEYS_AND_ADJACENCY,
            MetadataFormat::NONE2,
        ] {
            for method in [
                CompressionMethod::None,
                CompressionMethod::Zlib,
                CompressionMethod::Lz4,
                CompressionMethod::Zstd,
            ] {
                out.push(LsfFormat {
                    version,
                    metadata_format,
                    compression: CompressionFlags::new(method, CompressionLevel::Default),
                });
            }
        }
    }
    out
}

#[test]
fn every_format_round_trips() {
    for format in formats() {
        let file = LsfFile {
            format,
            resource: sample(format.version),
        };
        let bytes = file.write().unwrap_or_else(|e| panic!("{format:?}: {e}"));
        let back = LsfFile::read(&bytes).unwrap_or_else(|e| panic!("{format:?}: {e}"));
        assert_eq!(back, file, "{format:?}");
    }
}

#[test]
fn engine_version_packs_per_version() {
    let engine = PackedVersion {
        major: 4,
        minor: 8,
        revision: 0,
        build: 700,
    };
    assert_eq!(PackedVersion::from_u64(engine.to_u64()), engine);
    assert_eq!(PackedVersion::from_u32(engine.to_u32()), engine);

    let file = |version| LsfFile {
        format: LsfFormat {
            version,
            ..LsfFormat::default()
        },
        resource: Resource {
            engine_version: engine,
            regions: Vec::new(),
        },
    };
    let v3 = file(LsfVersion::ExtendedNodes).write().unwrap();
    let v7 = file(LsfVersion::Bg3Patch3).write().unwrap();
    assert_eq!(&v3[..4], b"LSOF");
    assert_eq!(
        u32::from_le_bytes(v3[8..12].try_into().unwrap()),
        engine.to_u32()
    );
    assert_eq!(
        u64::from_le_bytes(v7[8..16].try_into().unwrap()),
        engine.to_u64()
    );
}

#[test]
fn node_helpers_find_and_edit() {
    let mut resource = sample(LsfVersion::Bg3Patch3);
    let globals = resource.region_mut("Globals").unwrap();
    let items = globals.child_mut("Items").unwrap();
    assert_eq!(items.children_named("Item").count(), 2);
    items.set_attribute("Count", AttributeValue::Int(4));
    items.set_attribute("New", AttributeValue::Bool(false));

    let items = resource.region("Globals").unwrap().child("Items").unwrap();
    assert_eq!(items.attribute("Count"), Some(&AttributeValue::Int(4)));
    assert_eq!(items.attributes.len(), 2);
    let values = resource.region("Globals").unwrap().child("Values").unwrap();
    assert_eq!(
        values
            .attribute("FixedString")
            .and_then(AttributeValue::as_str),
        Some("S_Player_Karlach")
    );
}

#[test]
fn uuids_print_like_the_game() {
    // GustavX's module UUID as stored in a BG3 save's meta.lsf.
    let stored = [
        0xfe, 0x5e, 0x55, 0xcb, 0x9e, 0x2d, 0x1f, 0x13, 0x95, 0x81, 0x93, 0xa8, 0xd2, 0x29, 0xea,
        0x18,
    ];
    assert_eq!(uuid_string(&stored), "cb555efe-2d9e-131f-8195-a89329d218ea");
}

#[test]
fn rejects_non_lsf_data() {
    assert!(matches!(LsfFile::read(b""), Err(LslibError::Truncated(_))));
    assert!(matches!(
        LsfFile::read(b"LSPK\x07\0\0\0"),
        Err(LslibError::InvalidLsf(_))
    ));
    assert!(matches!(
        LsfFile::read(b"LSOF\x08\0\0\0"),
        Err(LslibError::InvalidLsf(_))
    ));
}

#[test]
fn rejects_excessive_nesting() {
    let mut node = Node::new("Leaf");
    for _ in 0..300 {
        node = Node {
            children: vec![node],
            ..Node::new("Wrapper")
        };
    }
    let file = LsfFile {
        format: LsfFormat::default(),
        resource: Resource {
            engine_version: PackedVersion::default(),
            regions: vec![node],
        },
    };
    let bytes = file.write().unwrap();
    assert!(matches!(
        LsfFile::read(&bytes),
        Err(LslibError::InvalidLsf(_))
    ));
}

/// Every truncation and single-byte corruption must fail cleanly or read;
/// never panic or hang.
#[test]
fn malformed_files_do_not_panic() {
    for version in [
        LsfVersion::ChunkedCompress,
        LsfVersion::ExtendedNodes,
        LsfVersion::Bg3Patch3,
    ] {
        for metadata_format in [MetadataFormat::NONE, MetadataFormat::KEYS_AND_ADJACENCY] {
            for method in [CompressionMethod::None, CompressionMethod::Lz4] {
                let mut resource = sample(version);
                // Keep the file small: the full value set makes this slow.
                resource.regions[0].children[1].attributes.truncate(4);
                resource.regions[0].children[1]
                    .attributes
                    .extend(every_value(version).into_iter().skip(28).take(2));
                let file = LsfFile {
                    format: LsfFormat {
                        version,
                        metadata_format,
                        compression: CompressionFlags::new(method, CompressionLevel::Default),
                    },
                    resource,
                };
                let bytes = file.write().unwrap();
                for len in 0..bytes.len() {
                    let _ = LsfFile::read(&bytes[..len]);
                }
                let mut corrupt = bytes.clone();
                for i in 0..bytes.len() {
                    for flip in [0xff, 0x80, 0x01] {
                        corrupt[i] ^= flip;
                        let _ = LsfFile::read(&corrupt);
                        corrupt[i] = bytes[i];
                    }
                }
            }
        }
    }
}

/// `SAVEFORGE_BG3_SAVE=<path to .lsv> cargo test -p lslib -- --ignored`:
/// every LSF in the save reads, and writes back to the same tree.
#[test]
#[ignore = "needs a real savegame"]
fn real_savegame_resources_round_trip() {
    let path = std::env::var("SAVEFORGE_BG3_SAVE").expect("set SAVEFORGE_BG3_SAVE to an .lsv file");
    let bytes = std::fs::read(path).unwrap();
    let package = Package::read(&bytes).unwrap();

    let mut builder = package.to_builder().unwrap();
    let mut seen = 0;
    for file in package
        .files()
        .iter()
        .filter(|f| f.name.to_ascii_lowercase().ends_with(".lsf"))
    {
        let lsf = LsfFile::read(&package.contents(file).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", file.name));
        let written = lsf.write().unwrap();
        assert_eq!(LsfFile::read(&written).unwrap(), lsf, "{}", file.name);
        builder.set(&file.name, written);
        seen += 1;
    }
    assert_ne!(seen, 0, "no LSF files in the save");

    // The repacked save holds the rewritten resources.
    let repacked = builder.build().unwrap();
    let reread = Package::read(&repacked).unwrap();
    let globals = LsfFile::read(
        &reread
            .contents(reread.find_ignore_case("Globals.lsf").unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_ne!(globals.resource.regions.len(), 0);
}
