// Test fixtures are tiny; their casts cannot truncate.
#![allow(clippy::cast_possible_truncation)]

use std::io::Write;

use lslib::LslibError;
use lslib::compression::{CompressionFlags, CompressionLevel, CompressionMethod};
use lslib::package::{Package, PackageBuilder, PackageFlags, PackageVersion};
use md5::{Digest, Md5};

const ALL_VERSIONS: [PackageVersion; 7] = [
    PackageVersion::V7,
    PackageVersion::V9,
    PackageVersion::V10,
    PackageVersion::V13,
    PackageVersion::V15,
    PackageVersion::V16,
    PackageVersion::V18,
];

const ALL_METHODS: [CompressionMethod; 4] = [
    CompressionMethod::None,
    CompressionMethod::Zlib,
    CompressionMethod::Lz4,
    CompressionMethod::Zstd,
];

fn sample_files() -> Vec<(String, Vec<u8>)> {
    let text: Vec<u8> = b"Larian ".iter().copied().cycle().take(5000).collect();
    let noise: Vec<u8> = (0u32..3000)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8)
        .collect();
    vec![
        ("Globals.lsf".to_owned(), text),
        ("LevelCache/WLD_Main_A.lsf".to_owned(), noise),
        ("meta.lsf".to_owned(), b"tiny".to_vec()),
        ("Empty.txt".to_owned(), Vec::new()),
    ]
}

fn builder(version: PackageVersion, method: CompressionMethod) -> PackageBuilder {
    let mut builder = PackageBuilder::new(version);
    builder.compression = method;
    for (path, data) in sample_files() {
        builder.add(path, data);
    }
    builder
}

fn expected_method(version: PackageVersion, method: CompressionMethod) -> CompressionMethod {
    if version <= PackageVersion::V9 && method != CompressionMethod::None {
        CompressionMethod::Zlib
    } else {
        method
    }
}

#[test]
fn every_version_and_method_round_trips() {
    for version in ALL_VERSIONS {
        for method in ALL_METHODS {
            let bytes = builder(version, method).build().unwrap();
            let package =
                Package::read(&bytes).unwrap_or_else(|e| panic!("{version:?}/{method:?}: {e}"));

            assert_eq!(package.version(), version);
            assert_eq!(
                package.extract_all().unwrap(),
                sample_files(),
                "{version:?}/{method:?}"
            );
            for file in package.files() {
                let want = if file.size() == 0 {
                    CompressionMethod::None
                } else {
                    expected_method(version, method)
                };
                assert_eq!(
                    file.flags.method().unwrap(),
                    want,
                    "{version:?}/{method:?} {}",
                    file.name
                );
            }
        }
    }
}

#[test]
fn header_fields_round_trip() {
    for version in [
        PackageVersion::V10,
        PackageVersion::V13,
        PackageVersion::V16,
        PackageVersion::V18,
    ] {
        let mut builder = builder(version, CompressionMethod::Zlib);
        builder.flags =
            PackageFlags(PackageFlags::PRELOAD.0 | PackageFlags::ALLOW_MEMORY_MAPPING.0);
        builder.priority = 42;
        let bytes = builder.build().unwrap();
        let package = Package::read(&bytes).unwrap();
        assert_eq!(package.flags(), builder.flags, "{version:?}");
        assert_eq!(package.priority(), 42, "{version:?}");
    }
}

#[test]
fn v18_layout_matches_lslib() {
    let bytes = builder(PackageVersion::V18, CompressionMethod::Zlib)
        .build()
        .unwrap();
    assert_eq!(&bytes[..4], b"LSPK");
    assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 18);
    // Data starts right after signature + header; padding is relative to it.
    let package = Package::read(&bytes).unwrap();
    let offsets: Vec<u64> = package.files().iter().map(|f| f.offset_in_file).collect();
    assert_eq!(offsets[0], 40);
    assert!(offsets.iter().all(|o| (o - 40) % 0x40 == 0), "{offsets:?}");
    assert_eq!(
        bytes[(offsets[0] + package.files()[0].size_on_disk) as usize],
        0xAD
    );
}

#[test]
fn crc_is_written_for_v10_to_v16_only() {
    for version in ALL_VERSIONS {
        let bytes = builder(version, CompressionMethod::Lz4).build().unwrap();
        let package = Package::read(&bytes).unwrap();
        for file in package.files() {
            let stored = &bytes
                [file.offset_in_file as usize..(file.offset_in_file + file.size_on_disk) as usize];
            let want = if version.has_crc() {
                crc32fast::hash(stored)
            } else {
                0
            };
            assert_eq!(file.crc, want, "{version:?} {}", file.name);
        }
    }
}

#[test]
fn already_compressed_media_and_empty_files_are_stored() {
    let mut builder = PackageBuilder::new(PackageVersion::V18);
    builder.compression = CompressionMethod::Zlib;
    builder
        .add("Sound/Voice.WEM", vec![7; 1000])
        .add("Empty.lsf", Vec::new())
        .add("Data.lsf", vec![7; 1000]);
    let bytes = builder.build().unwrap();
    let package = Package::read(&bytes).unwrap();

    let flags: Vec<CompressionFlags> = package.files().iter().map(|f| f.flags).collect();
    let zlib = CompressionFlags::new(CompressionMethod::Zlib, CompressionLevel::Default);
    assert_eq!(
        flags,
        [CompressionFlags::NONE, CompressionFlags::NONE, zlib]
    );
    assert_eq!(
        package.contents(&package.files()[0]).unwrap().as_ref(),
        &[7; 1000]
    );
}

#[test]
fn backslashes_are_written_as_slashes() {
    let mut builder = PackageBuilder::new(PackageVersion::V18);
    builder.add("Mods\\Gustav\\meta.lsx", b"x".to_vec());
    let bytes = builder.build().unwrap();
    assert_eq!(
        Package::read(&bytes).unwrap().files()[0].name,
        "Mods/Gustav/meta.lsx"
    );
}

#[test]
fn archive_hash_is_md5_plus_one() {
    let files = sample_files();
    let digest = |files: &[(String, Vec<u8>)]| -> [u8; 16] {
        let mut md5 = Md5::new();
        for (_, data) in files {
            md5.update(data);
        }
        let mut hash: [u8; 16] = md5.finalize().into();
        for byte in &mut hash {
            *byte = byte.wrapping_add(1);
        }
        hash
    };

    // v15+ hash in file order, and only on request.
    let mut v18 = builder(PackageVersion::V18, CompressionMethod::Zlib);
    assert_eq!(Package::read(&v18.build().unwrap()).unwrap().md5(), [0; 16]);
    v18.hash = true;
    assert_eq!(
        Package::read(&v18.build().unwrap()).unwrap().md5(),
        digest(&files)
    );

    // Older versions always hash, sorted by path.
    let mut sorted = files.clone();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let v13 = builder(PackageVersion::V13, CompressionMethod::Zlib)
        .build()
        .unwrap();
    assert_eq!(Package::read(&v13).unwrap().md5(), digest(&sorted));
}

#[test]
fn to_builder_keeps_per_file_compression() {
    let mut original = PackageBuilder::new(PackageVersion::V18);
    original.compression = CompressionMethod::Zstd;
    original
        .add("a.lsf", vec![1; 500])
        .add("b.lsf", vec![2; 500]);
    original.files[1].compression = Some((CompressionMethod::Lz4, CompressionLevel::Max));
    let bytes = original.build().unwrap();
    let package = Package::read(&bytes).unwrap();

    let mut rebuilt = package.to_builder().unwrap();
    rebuilt.set("A.LSF", vec![9; 10]);
    let bytes = rebuilt.build().unwrap();
    let package = Package::read(&bytes).unwrap();

    let summary: Vec<(&str, CompressionMethod)> = package
        .files()
        .iter()
        .map(|f| (f.name.as_str(), f.flags.method().unwrap()))
        .collect();
    assert_eq!(
        summary,
        [
            ("a.lsf", CompressionMethod::Zstd),
            ("b.lsf", CompressionMethod::Lz4)
        ]
    );
    assert_eq!(package.files()[1].flags.level(), CompressionLevel::Max);
    assert_eq!(package.extract_all().unwrap()[0].1, vec![9; 10]);
}

#[test]
fn reads_solid_archives() {
    let contents: [&[u8]; 3] = [b"first file contents", b"second", b"third and last file"];
    let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
    encoder.write_all(&contents.concat()).unwrap();
    let frame = encoder.finish().unwrap();

    // Signature + v16 header, then the frame; its 7-byte header belongs to no
    // file, and the rest is split between the files.
    let data_offset = 40u64;
    let body = frame.len() as u64 - 7;
    let sizes = [body / 3, body / 3, body - 2 * (body / 3)];
    let mut entries = Vec::new();
    let mut offset = data_offset + 7;
    for (i, (data, size)) in contents.iter().zip(sizes).enumerate() {
        let mut name = format!("file{i}.lsf").into_bytes();
        name.resize(256, 0);
        entries.extend_from_slice(&name);
        entries.extend_from_slice(&(offset as u32).to_le_bytes());
        entries.extend_from_slice(&0u16.to_le_bytes());
        entries.extend([0, 0x22]);
        entries.extend_from_slice(&(size as u32).to_le_bytes());
        entries.extend_from_slice(&(data.len() as u32).to_le_bytes());
        offset += size;
    }
    let list = lz4_flex::block::compress(&entries);

    let mut bytes = b"LSPK".to_vec();
    bytes.extend_from_slice(&18u32.to_le_bytes());
    let file_list_offset = data_offset + frame.len() as u64;
    bytes.extend_from_slice(&file_list_offset.to_le_bytes());
    bytes.extend_from_slice(&(8 + list.len() as u32).to_le_bytes());
    bytes.extend([PackageFlags::SOLID.0, 0]);
    bytes.extend_from_slice(&[0; 16]);
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&frame);
    bytes.extend_from_slice(&3u32.to_le_bytes());
    bytes.extend_from_slice(&(list.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&list);

    let package = Package::read(&bytes).unwrap();
    let extracted: Vec<Vec<u8>> = package
        .extract_all()
        .unwrap()
        .into_iter()
        .map(|(_, d)| d)
        .collect();
    assert_eq!(extracted, contents.map(<[u8]>::to_vec));

    // A solid package is rebuilt as a regular one.
    let rebuilt = package.to_builder().unwrap();
    assert!(!rebuilt.flags.contains(PackageFlags::SOLID));
    assert_eq!(
        Package::read(&rebuilt.build().unwrap())
            .unwrap()
            .extract_all()
            .unwrap(),
        package.extract_all().unwrap()
    );
}

#[test]
fn writing_solid_packages_is_refused() {
    let mut builder = builder(PackageVersion::V18, CompressionMethod::Lz4);
    builder.flags = PackageFlags::SOLID;
    assert!(matches!(builder.build(), Err(LslibError::Unsupported(_))));
}

#[test]
fn rejects_long_names() {
    let mut builder = PackageBuilder::new(PackageVersion::V18);
    builder.add("a".repeat(256), Vec::new());
    assert!(matches!(builder.build(), Err(LslibError::NameTooLong(_))));
}

#[test]
fn rejects_non_packages() {
    assert!(matches!(Package::read(b""), Err(LslibError::NotAPackage)));
    assert!(matches!(
        Package::read(b"PK\x03\x04 a zip file"),
        Err(LslibError::NotAPackage)
    ));
    let mut v12 = b"LSPK".to_vec();
    v12.extend_from_slice(&12u32.to_le_bytes());
    v12.resize(64, 0);
    assert!(matches!(
        Package::read(&v12),
        Err(LslibError::UnsupportedVersion(12))
    ));
}

/// Every truncation and single-byte corruption of a valid package must fail
/// cleanly or read; never panic.
#[test]
fn malformed_packages_do_not_panic() {
    for version in ALL_VERSIONS {
        for method in [
            CompressionMethod::Zlib,
            CompressionMethod::Lz4,
            CompressionMethod::Zstd,
        ] {
            let mut builder = PackageBuilder::new(version);
            builder.compression = method;
            builder
                .add("a.lsf", b"hello hello hello hello".to_vec())
                .add("b.lsf", vec![3; 100]);
            let bytes = builder.build().unwrap();

            let exercise = |data: &[u8]| {
                if let Ok(package) = Package::read(data) {
                    for file in package.files() {
                        let _ = package.contents(file);
                    }
                }
            };
            for len in 0..bytes.len() {
                exercise(&bytes[..len]);
            }
            let mut corrupt = bytes.clone();
            for i in 0..bytes.len() {
                corrupt[i] ^= 0xff;
                exercise(&corrupt);
                corrupt[i] = bytes[i];
            }
        }
    }
}

/// `SAVEFORGE_BG3_SAVE=<path to .lsv> cargo test -p lslib -- --ignored`
#[test]
#[ignore = "needs a real savegame"]
fn real_savegame_round_trips() {
    let path = std::env::var("SAVEFORGE_BG3_SAVE").expect("set SAVEFORGE_BG3_SAVE to an .lsv file");
    let bytes = std::fs::read(path).unwrap();
    let package = Package::read(&bytes).unwrap();
    assert!(
        package.find_ignore_case("Globals.lsf").is_some(),
        "not a savegame"
    );
    let files = package.extract_all().unwrap();

    let repacked = package.to_builder().unwrap().build().unwrap();
    let reread = Package::read(&repacked).unwrap();
    assert_eq!(reread.version(), package.version());
    assert_eq!(
        (reread.flags(), reread.priority()),
        (package.flags(), package.priority())
    );
    assert_eq!(reread.extract_all().unwrap(), files);
    for (old, new) in package.files().iter().zip(reread.files()) {
        assert_eq!((&old.name, old.flags), (&new.name, new.flags));
    }
}
