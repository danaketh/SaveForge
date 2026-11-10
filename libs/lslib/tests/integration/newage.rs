use lslib::LslibError;
use lslib::lsf::LsfFile;
use lslib::newage::{CHECKSUM_SEED, HEADER_SIZE, LEVEL_COMPONENT, NewAge, murmur3_x64_128};
use lslib::package::Package;
use lslib::resource::AttributeValue;

const STATS: &str = "game.stats.v3.StatsComponent";
const SIDE: &str = "game.side.v0.Table";

struct Spec {
    name: &'static str,
    element_size: u32,
    elements: Vec<Vec<u8>>,
    owners: Option<Vec<u32>>,
}

fn put(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// A blob laid out like the game's: level element, entities, owner indices,
/// owner maps (with an unused slot), component data, names, table. The
/// checksum is left zero.
fn build(entities: &[[u8; 16]], specs: &[Spec]) -> Vec<u8> {
    let mut body = vec![0; 32];
    let entities_start = body.len() as u64;
    for entity in entities {
        body.extend_from_slice(entity);
    }
    let entities_end = body.len() as u64;

    let mut maps = Vec::new();
    for (index, spec) in specs.iter().enumerate() {
        if let Some(owners) = &spec.owners {
            let start = body.len() as u64;
            for owner in owners {
                body.extend_from_slice(&owner.to_le_bytes());
            }
            maps.push([start, body.len() as u64, index as u64, owners.len() as u64]);
        }
    }
    maps.push([u64::MAX; 4]);
    let maps_start = body.len() as u64;
    for map in &maps {
        for field in map {
            put(&mut body, *field);
        }
    }
    let maps_end = body.len() as u64;

    let mut level = Vec::new();
    for field in [entities_start, entities_end, maps_start, maps_end] {
        put(&mut level, field);
    }
    body[..32].copy_from_slice(&level);

    let mut data_offsets = Vec::new();
    for spec in specs {
        if spec.name == LEVEL_COMPONENT {
            data_offsets.push(0);
            continue;
        }
        data_offsets.push(body.len() as u64);
        for element in &spec.elements {
            assert_eq!(element.len(), spec.element_size as usize);
            body.extend_from_slice(element);
        }
    }

    let block_offset = body.len() as u64;
    let mut names = vec![0u8];
    let mut name_offsets = Vec::new();
    for spec in specs {
        name_offsets.push(names.len() as u64);
        names.extend_from_slice(spec.name.as_bytes());
    }
    body.extend_from_slice(&names);
    for (i, spec) in specs.iter().enumerate() {
        put(&mut body, name_offsets[i]);
        body.extend_from_slice(&u32::try_from(spec.name.len()).unwrap().to_le_bytes());
        body.extend_from_slice(&101u32.to_le_bytes());
        put(&mut body, 0xdead_beef);
        body.extend_from_slice(&spec.element_size.to_le_bytes());
        body.extend_from_slice(&3u32.to_le_bytes());
        put(&mut body, spec.elements.len() as u64);
        put(&mut body, data_offsets[i]);
    }
    let block_size = body.len() as u64 - block_offset;

    let mut out = b"LSMF".to_vec();
    out.extend_from_slice(&0x0800_0101u32.to_le_bytes());
    put(&mut out, 0);
    put(&mut out, block_offset);
    put(&mut out, block_size);
    out.extend_from_slice(&u32::try_from(names.len()).unwrap().to_le_bytes());
    out.extend_from_slice(&u16::try_from(specs.len()).unwrap().to_le_bytes());
    out.extend_from_slice(&[0; 10]);
    assert_eq!(out.len(), HEADER_SIZE);
    out.extend_from_slice(&body);
    out
}

fn stats(abilities: [i32; 6]) -> Vec<u8> {
    let mut out = vec![0; 8];
    for value in abilities.into_iter().chain([2]) {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

fn sample() -> Vec<u8> {
    build(
        &[[1; 16], [2; 16], [3; 16]],
        &[
            Spec {
                name: LEVEL_COMPONENT,
                element_size: 32,
                elements: vec![vec![0; 32]],
                owners: None,
            },
            Spec {
                name: STATS,
                element_size: 36,
                elements: vec![stats([8, 12, 14, 17, 14, 10]), stats([10; 6])],
                owners: Some(vec![2, 0]),
            },
            Spec {
                name: SIDE,
                element_size: 4,
                elements: vec![vec![1; 4], vec![2; 4], vec![3; 4]],
                owners: None,
            },
        ],
    )
}

#[test]
fn murmur3_matches_reference_vectors() {
    assert_eq!(murmur3_x64_128(b"", 0), (0, 0));
    assert_eq!(
        murmur3_x64_128(b"hello", 0),
        (0xcbd8_a7b3_41bd_9b02, 0x5b1e_906a_48ae_1d19)
    );
    assert_eq!(
        murmur3_x64_128(b"The quick brown fox jumps over the lazy dog", 0),
        (0xe34b_bc7b_bc07_1b6c, 0x7a43_3ca9_c49a_9347)
    );
    let bytes: Vec<u8> = (0..31).collect();
    assert_eq!(
        murmur3_x64_128(&bytes, CHECKSUM_SEED),
        (0x483b_3e1f_1393_ad41, 0x0bd8_27aa_63dc_95a4)
    );
}

#[test]
fn reads_components_entities_and_owners() {
    let blob = NewAge::read(sample()).unwrap();
    assert_eq!(blob.version(), 0x0800_0101);
    assert_eq!(blob.entity_count(), 3);
    assert_eq!(blob.entity(2), Some([3; 16]));
    assert_eq!(blob.entity(3), None);

    let names: Vec<&str> = blob.components().iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, [LEVEL_COMPONENT, STATS, SIDE]);

    let stats = blob.component(STATS).unwrap();
    assert_eq!((stats.element_size, stats.count, stats.version), (36, 2, 3));
    assert_eq!(stats.owners(), Some(&[2, 0][..]));
    assert_eq!(stats.element_of(0), Some(1));
    assert_eq!(stats.element_of(1), None);
    assert_eq!(
        blob.element(stats, 0).unwrap(),
        &self::stats([8, 12, 14, 17, 14, 10])[..]
    );
    assert_eq!(blob.element(stats, 2), None);

    let side = blob.component(SIDE).unwrap();
    assert_eq!(side.owners(), None);
    assert_eq!(blob.element(side, 2).unwrap(), &[3; 4]);
}

#[test]
fn set_element_changes_only_that_element_and_the_checksum() {
    let original = sample();
    let mut blob = NewAge::read(original.clone()).unwrap();
    assert_ne!(blob.stored_checksum(), blob.checksum());

    blob.set_element(STATS, 1, &stats([20; 6])).unwrap();
    let written = blob.to_bytes();
    let reread = NewAge::read(written.clone()).unwrap();
    assert_eq!(reread.stored_checksum(), reread.checksum());
    let component = reread.component(STATS).unwrap();
    assert_eq!(reread.element(component, 1).unwrap(), &stats([20; 6])[..]);
    assert_eq!(
        reread.element(component, 0).unwrap(),
        &stats([8, 12, 14, 17, 14, 10])[..]
    );

    let changed: Vec<usize> = (0..original.len())
        .filter(|&i| original[i] != written[i])
        .collect();
    assert!(
        changed
            .iter()
            .all(|&i| (8..16).contains(&i) || i >= HEADER_SIZE)
    );
    // Six abilities, 10 -> 20: one byte each.
    assert_eq!(changed.iter().filter(|&&i| i >= HEADER_SIZE).count(), 6);
}

#[test]
fn raw_bytes_read_and_write_at_body_offsets() {
    let mut blob = NewAge::read(sample()).unwrap();
    // The level element starts the body: entities start right after it.
    assert_eq!(blob.bytes_at(0, 8).unwrap(), &32u64.to_le_bytes());
    assert_eq!(blob.bytes_at(32, 16).unwrap(), &[1; 16]);
    blob.write_at(32, &[9; 4]).unwrap();
    assert_eq!(blob.entity(0).unwrap()[..5], [9, 9, 9, 9, 1]);

    let len = (sample().len() - HEADER_SIZE) as u64;
    assert_eq!(blob.bytes_at(len - 1, 1).map(<[u8]>::len), Some(1));
    assert_eq!(blob.bytes_at(len, 1), None);
    assert_eq!(blob.bytes_at(u64::MAX, 2), None);
    assert!(matches!(
        blob.write_at(len - 1, &[0; 2]),
        Err(LslibError::InvalidNewAge(_))
    ));
}

#[test]
fn set_element_rejects_wrong_size_index_or_name() {
    let mut blob = NewAge::read(sample()).unwrap();
    for result in [
        blob.set_element(STATS, 0, &[0; 4]),
        blob.set_element(STATS, 2, &stats([1; 6])),
        blob.set_element("game.missing.v0.Component", 0, &[0; 4]),
    ] {
        assert!(matches!(result, Err(LslibError::InvalidNewAge(_))));
    }
}

#[test]
fn malformed_blobs_are_errors_not_panics() {
    let bytes = sample();
    assert!(matches!(
        NewAge::read(b"LSPK".to_vec()),
        Err(LslibError::InvalidNewAge(_) | LslibError::Truncated(_))
    ));
    for len in 0..bytes.len() {
        assert!(NewAge::read(bytes[..len].to_vec()).is_err(), "len {len}");
    }
    // Corrupt each byte of the header and table in turn.
    for i in 0..bytes.len() {
        let mut corrupt = bytes.clone();
        corrupt[i] ^= 0xff;
        let _ = NewAge::read(corrupt);
    }
}

/// `SAVEFORGE_BG3_SAVE=<path to .lsv> cargo test -p lslib -- --ignored`
#[test]
#[ignore = "needs a real savegame"]
fn real_savegame_newage_round_trips() {
    let path = std::env::var("SAVEFORGE_BG3_SAVE").expect("set SAVEFORGE_BG3_SAVE to an .lsv file");
    let bytes = std::fs::read(path).unwrap();
    let package = Package::read(&bytes).unwrap();
    let globals = package.find_ignore_case("Globals.lsf").unwrap();
    let globals = LsfFile::read(&package.contents(globals).unwrap()).unwrap();
    let Some(AttributeValue::ScratchBuffer(raw)) = globals
        .resource
        .region("NewAge")
        .and_then(|r| r.attribute("NewAge"))
    else {
        panic!("no NewAge blob");
    };

    let blob = NewAge::read(raw.clone()).unwrap();
    assert_eq!(blob.stored_checksum(), blob.checksum());
    assert_eq!(&blob.to_bytes(), raw);

    let players = blob.component("game.v0.PlayerComponent").unwrap();
    let experience = blob
        .component("game.experience.v0.ExperienceComponent")
        .unwrap();
    let stats = blob.component(STATS).unwrap();
    assert_ne!(players.count, 0);
    for &player in players.owners().unwrap() {
        assert!(experience.element_of(player).is_some());
        assert!(stats.element_of(player).is_some());
    }
}
