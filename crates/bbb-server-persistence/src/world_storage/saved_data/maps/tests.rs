use super::*;
use crate::level_file::encode_gzip_named_compound;
use bbb_protocol::encode_named_nbt_summary_java_string;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEST: AtomicU64 = AtomicU64::new(1);

fn test_root(name: &str) -> PathBuf {
    let id = NEXT_TEST.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "bbb-map-saved-data-{name}-{}-{id}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    root
}

fn map_fixture() -> Java26MapSavedData {
    Java26MapSavedData {
        dimension: "minecraft:the_nether".to_string(),
        center_x: -123,
        center_z: 456,
        scale: 4,
        colors: (0..MAP_COLOR_COUNT).map(|value| value as u8).collect(),
        tracking_position: false,
        unlimited_tracking: true,
        locked: true,
        banners: vec![Java26MapBanner {
            pos: [-1, 64, 2],
            color: "red".to_string(),
            name: Some(NbtSummaryValue::string("banner")),
        }],
        frames: vec![Java26MapFrame {
            pos: [3, 65, -4],
            rotation: 270,
            entity_id: -22,
        }],
    }
}

fn data_document(mut fields: Vec<NbtSummaryEntry>) -> NamedCompound {
    if !fields.iter().any(|entry| entry.name == "dimension") {
        fields.insert(
            0,
            entry("dimension", NbtSummaryValue::string("minecraft:overworld")),
        );
    }
    if !fields.iter().any(|entry| entry.name == "xCenter") {
        fields.push(entry("xCenter", NbtSummaryValue::Int(0)));
    }
    if !fields.iter().any(|entry| entry.name == "zCenter") {
        fields.push(entry("zCenter", NbtSummaryValue::Int(0)));
    }
    if !fields.iter().any(|entry| entry.name == "colors") {
        fields.push(entry(
            "colors",
            NbtSummaryValue::ByteArray(vec![0; MAP_COLOR_COUNT]),
        ));
    }
    NamedCompound::new(
        "",
        vec![
            entry("data", NbtSummaryValue::Compound(fields)),
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
        ],
    )
}

fn write_gzip(paths: &WorldSavePaths, id: i32, document: &NamedCompound) {
    fs::create_dir_all(paths.maps_dir()).unwrap();
    let path = paths.map_data_dat(id);
    let bytes = encode_gzip_named_compound(&path, document, WorldSaveLimits::default()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn write_raw(paths: &WorldSavePaths, id: i32, document: &NamedCompound) -> usize {
    fs::create_dir_all(paths.maps_dir()).unwrap();
    let bytes = encode_named_nbt_summary_java_string(
        document.root_name(),
        &document.to_summary(),
        WorldSaveLimits::default().max_decompressed_nbt_bytes,
    )
    .unwrap();
    fs::write(paths.map_data_dat(id), &bytes).unwrap();
    bytes.len()
}

#[test]
fn map_saved_data_round_trips_every_codec_field() {
    let expected = map_fixture();
    let document = java_26_1_map_document(&expected);
    assert_eq!(parse_map_document(&document).unwrap().unwrap(), expected);
}

#[test]
fn map_file_ignores_the_root_name() {
    let root = test_root("raw-root");
    let paths = WorldSavePaths::new(&root).unwrap();
    let expected = map_fixture();
    let document = java_26_1_map_document(&expected);
    let named = NamedCompound::new("abc", document.entries().to_vec());
    write_raw(&paths, 0, &named);
    let path = paths.map_data_dat(0);
    let mut bytes = fs::read(&path).unwrap();
    bytes[3..6].copy_from_slice(&[0xff, 0xff, 0xff]);
    fs::write(&path, bytes).unwrap();
    assert_eq!(
        load_java_26_1_maps(&paths, WorldSaveLimits::default())
            .unwrap()
            .maps[&0],
        expected
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn map_saved_data_applies_defaults_scale_clamp_and_color_normalization() {
    let defaults = parse_map_document(&data_document(Vec::new()))
        .unwrap()
        .unwrap();
    assert_eq!(defaults.scale, 0);
    assert!(defaults.tracking_position);
    assert!(!defaults.unlimited_tracking);
    assert!(!defaults.locked);
    assert!(defaults.banners.is_empty());
    assert!(defaults.frames.is_empty());

    let clamped = parse_map_document(&data_document(vec![
        entry("scale", NbtSummaryValue::Byte(100)),
        entry("colors", NbtSummaryValue::ByteArray(vec![-1; 7])),
        entry("trackingPosition", NbtSummaryValue::Byte(0)),
        entry("unlimitedTracking", NbtSummaryValue::Byte(-1)),
        entry("locked", NbtSummaryValue::Byte(2)),
    ]))
    .unwrap()
    .unwrap();
    assert_eq!(clamped.scale, 4);
    assert_eq!(clamped.colors, vec![0; MAP_COLOR_COUNT]);
    assert!(!clamped.tracking_position);
    assert!(clamped.unlimited_tracking);
    assert!(clamped.locked);

    let low = parse_map_document(&data_document(vec![entry(
        "scale",
        NbtSummaryValue::Byte(-5),
    )]))
    .unwrap()
    .unwrap();
    assert_eq!(low.scale, 0);
}

#[test]
fn map_saved_data_banner_lenient_fields_and_frame_shape_match_codec() {
    let document = data_document(vec![
        entry(
            "banners",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry("pos", NbtSummaryValue::IntArray(vec![1, 2, 3])),
                entry("color", NbtSummaryValue::Int(14)),
                entry("name", NbtSummaryValue::Int(4)),
            ])]),
        ),
        entry(
            "frames",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry("pos", NbtSummaryValue::IntArray(vec![-1, 2, 9])),
                entry("rotation", NbtSummaryValue::Int(-90)),
                entry("entity_id", NbtSummaryValue::Int(42)),
            ])]),
        ),
    ]);
    let map = parse_map_document(&document).unwrap().unwrap();
    assert_eq!(map.banners[0].color, "white");
    assert_eq!(map.banners[0].name, None);
    assert_eq!(map.frames[0].pos, [-1, 2, 9]);
    assert_eq!(map.frames[0].rotation, -90);

    let invalid = data_document(vec![entry(
        "frames",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            entry("pos", NbtSummaryValue::IntArray(vec![1, 2])),
            entry("rotation", NbtSummaryValue::Int(0)),
            entry("entity_id", NbtSummaryValue::Int(1)),
        ])]),
    )]);
    assert!(parse_map_document(&invalid)
        .unwrap()
        .unwrap()
        .frames
        .is_empty());
}

#[test]
fn map_saved_data_discovers_signed_canonical_gzip_and_raw_files_only() {
    let root = test_root("discovery");
    let paths = WorldSavePaths::new(&root).unwrap();
    let positive = java_26_1_map_document(&map_fixture());
    let mut negative_map = map_fixture();
    negative_map.center_x = 99;
    let negative = java_26_1_map_document(&negative_map);
    write_gzip(&paths, 7, &positive);
    write_raw(&paths, -3, &negative);
    // MapItemSavedData.CODEC requires colors; the map reads as absent.
    let mut undecodable = positive.clone();
    let NbtSummaryValue::Compound(data) = &mut undecodable.entries_mut()[0].value else {
        unreachable!()
    };
    data.retain(|entry| entry.name != "colors");
    write_gzip(&paths, 5, &undecodable);
    for ignored in ["01.dat", "+1.dat", "map_7.dat", "7.dat.old"] {
        fs::write(paths.maps_dir().join(ignored), b"untouched").unwrap();
    }
    let index = encode_gzip_named_compound(
        &paths.map_index_dat(),
        &java_26_1_map_index_document(8),
        WorldSaveLimits::default(),
    )
    .unwrap();
    fs::write(paths.map_index_dat(), index).unwrap();
    let loaded = load_java_26_1_maps(&paths, WorldSaveLimits::default()).unwrap();
    assert_eq!(loaded.maps.keys().copied().collect::<Vec<_>>(), vec![-3, 7]);
    assert_eq!(loaded.maps[&-3].center_x, 99);
    assert_eq!(loaded.last_id, 8);
    fs::remove_dir_all(root).unwrap();
}
