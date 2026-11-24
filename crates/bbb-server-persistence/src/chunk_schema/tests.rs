//! Chunk-document parsing and projection tests.

use super::heightmaps::expected_heightmap_longs;
use super::{Java26ChunkContext, Java26ChunkData, Java26ChunkStatus, BLOCK_ENTRY_COUNT};
use crate::RegionFileStore;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::{NamedCompound, WorldSaveLimits, JAVA_26_1_DATA_VERSION};
use std::fs;
use std::path::{Path, PathBuf};

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn list(values: Vec<NbtSummaryValue>) -> NbtSummaryValue {
    NbtSummaryValue::List(values)
}

fn singleton_blocks() -> NbtSummaryValue {
    compound(vec![entry(
        "palette",
        list(vec![compound(vec![entry(
            "Name",
            NbtSummaryValue::string("minecraft:stone"),
        )])]),
    )])
}

fn singleton_biome() -> NbtSummaryValue {
    compound(vec![entry(
        "palette",
        list(vec![NbtSummaryValue::string(
            "minecraft:plains".to_string(),
        )]),
    )])
}

fn heightmaps() -> NbtSummaryValue {
    compound(
        [
            "OCEAN_FLOOR",
            "WORLD_SURFACE",
            "MOTION_BLOCKING",
            "MOTION_BLOCKING_NO_LEAVES",
        ]
        .into_iter()
        .map(|name| entry(name, NbtSummaryValue::LongArray(vec![0; 37])))
        .collect(),
    )
}

fn heightmaps_with_len(len: usize) -> NbtSummaryValue {
    compound(
        [
            "OCEAN_FLOOR",
            "WORLD_SURFACE",
            "MOTION_BLOCKING",
            "MOTION_BLOCKING_NO_LEAVES",
        ]
        .into_iter()
        .map(|name| entry(name, NbtSummaryValue::LongArray(vec![0; len])))
        .collect(),
    )
}

fn structures() -> NbtSummaryValue {
    compound(vec![
        entry("starts", compound(Vec::new())),
        entry("References", compound(Vec::new())),
    ])
}

fn fixture() -> NamedCompound {
    let mut post_processing = Vec::new();
    for _ in 0..24 {
        post_processing.push(list(Vec::new()));
    }
    NamedCompound::new(
        "",
        vec![
            entry("bbb_unknown", NbtSummaryValue::Int(42)),
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry("xPos", NbtSummaryValue::Int(0)),
            entry("yPos", NbtSummaryValue::Int(-4)),
            entry("zPos", NbtSummaryValue::Int(0)),
            entry("LastUpdate", NbtSummaryValue::Long(99)),
            entry("InhabitedTime", NbtSummaryValue::Long(7)),
            entry("Status", NbtSummaryValue::string("minecraft:full")),
            entry(
                "sections",
                list(vec![compound(vec![
                    entry("Y", NbtSummaryValue::Byte(-4)),
                    entry("block_states", singleton_blocks()),
                    entry("biomes", singleton_biome()),
                ])]),
            ),
            entry("block_entities", list(Vec::new())),
            entry("block_ticks", list(Vec::new())),
            entry("fluid_ticks", list(Vec::new())),
            entry("PostProcessing", list(post_processing)),
            entry("Heightmaps", heightmaps()),
            entry("structures", structures()),
        ],
    )
}

fn context() -> Java26ChunkContext {
    Java26ChunkContext {
        expected_x: 0,
        expected_z: 0,
        min_section_y: -4,
        section_count: 24,
    }
}

fn mutate_field(raw: &mut NamedCompound, name: &str, value: NbtSummaryValue) {
    raw.entries_mut()
        .iter_mut()
        .find(|entry| entry.name == name)
        .unwrap()
        .value = value;
}

fn doubles(values: [f64; 3]) -> NbtSummaryValue {
    list(
        values
            .into_iter()
            .map(|value| NbtSummaryValue::Double(value.to_bits()))
            .collect(),
    )
}

fn floats(values: [f32; 2]) -> NbtSummaryValue {
    list(
        values
            .into_iter()
            .map(|value| NbtSummaryValue::Float(value.to_bits()))
            .collect(),
    )
}

fn proto_entity(uuid: i32, x: f64, z: f64) -> NbtSummaryValue {
    compound(vec![
        entry("id", NbtSummaryValue::string("minecraft:pig")),
        entry("UUID", NbtSummaryValue::IntArray(vec![0, 0, 0, uuid])),
        entry("Pos", doubles([x, 64.0, z])),
        entry("Motion", doubles([0.0, 0.0, 0.0])),
        entry("Rotation", floats([0.0, 0.0])),
        entry("fall_distance", NbtSummaryValue::Double(0.0_f64.to_bits())),
        entry("Fire", NbtSummaryValue::Short(0)),
        entry("Air", NbtSummaryValue::Short(300)),
        entry("OnGround", NbtSummaryValue::Byte(1)),
        entry("Invulnerable", NbtSummaryValue::Byte(0)),
        entry("PortalCooldown", NbtSummaryValue::Int(0)),
    ])
}

#[test]
fn exact_full_chunk_projects_typed_fields_and_preserves_raw() {
    let mut raw = fixture();
    raw.entries_mut().push(entry(
        "Heightmaps",
        compound(vec![entry("bbb:future_heightmap", NbtSummaryValue::Int(7))]),
    ));
    let duplicate_heightmaps = raw.entries_mut().pop().unwrap();
    let NbtSummaryValue::Compound(unknown) = duplicate_heightmaps.value else {
        unreachable!()
    };
    let NbtSummaryValue::Compound(heightmaps) = &mut raw
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "Heightmaps")
        .unwrap()
        .value
    else {
        unreachable!()
    };
    heightmaps.extend(unknown);
    let chunk = Java26ChunkData::parse(raw.clone(), context()).unwrap();
    assert_eq!((chunk.x(), chunk.min_section_y(), chunk.z()), (0, -4, 0));
    assert_eq!(chunk.last_update(), 99);
    assert_eq!(chunk.inhabited_time(), 7);
    assert_eq!(chunk.status(), Java26ChunkStatus::Full);
    assert_eq!(
        chunk.sections()[0]
            .block_states
            .as_ref()
            .unwrap()
            .bits_per_entry,
        0
    );
    assert!(chunk.block_entities().is_empty());
    assert!(chunk.entities().is_none());
    assert!(chunk.block_ticks().is_empty());
    assert!(chunk.fluid_ticks().is_empty());
    assert_eq!(chunk.post_processing().len(), 24);
    assert_eq!(chunk.heightmaps().len(), 4);
    assert!(!chunk.heightmaps().contains_key("bbb:future_heightmap"));
}

#[test]
fn sparse_chunk_reads_with_vanilla_defaults() {
    let mut words = vec![0_i64; BLOCK_ENTRY_COUNT.div_ceil(16)];
    words[0] = (1 << 4) | (2 << 8) | (15 << 12);
    let raw = NamedCompound::new(
        "",
        vec![
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry("xPos", NbtSummaryValue::Int(0)),
            entry("zPos", NbtSummaryValue::Int(0)),
            entry("Status", NbtSummaryValue::string("minecraft:full")),
            entry(
                "sections",
                list(vec![compound(vec![
                    entry("Y", NbtSummaryValue::Byte(0)),
                    entry(
                        "block_states",
                        compound(vec![
                            entry(
                                "palette",
                                list(vec![
                                    compound(vec![entry("Name", NbtSummaryValue::string("stone"))]),
                                    compound(vec![
                                        entry("Name", NbtSummaryValue::string("minecraft:lever")),
                                        entry(
                                            "Properties",
                                            compound(vec![
                                                entry("facing", NbtSummaryValue::string("east")),
                                                entry("powered", NbtSummaryValue::string("maybe")),
                                                entry("future", NbtSummaryValue::string("x")),
                                            ]),
                                        ),
                                    ]),
                                    compound(vec![entry(
                                        "Name",
                                        NbtSummaryValue::string("mod:unknown"),
                                    )]),
                                ]),
                            ),
                            entry("data", NbtSummaryValue::LongArray(words)),
                        ]),
                    ),
                ])]),
            ),
        ],
    );
    let chunk = Java26ChunkData::parse(raw.clone(), context()).unwrap();
    assert_eq!((chunk.last_update(), chunk.inhabited_time()), (0, 0));
    assert!(chunk.block_ticks().is_empty() && chunk.block_entities().is_empty());
    assert_eq!(chunk.post_processing().len(), 24);
    assert!(chunk.heightmaps().is_empty());
    let section = &chunk.sections()[0];
    let blocks = section.block_states.as_ref().unwrap();
    assert_eq!(blocks.value_at(0, 0, 0).unwrap().name, "minecraft:stone");
    let lever = blocks.value_at(1, 0, 0).unwrap();
    assert_eq!(lever.name, "minecraft:lever");
    assert_eq!(
        lever.properties,
        [("face", "wall"), ("facing", "east"), ("powered", "false")]
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .into(),
    );
    assert_eq!(blocks.value_at(2, 0, 0).unwrap().name, "minecraft:air");
    // An index past the palette reads the first entry.
    assert_eq!(blocks.value_at(3, 0, 0).unwrap().name, "minecraft:stone");
    assert_eq!(
        section.biomes.as_ref().unwrap().value_at(0, 0, 0).unwrap(),
        "minecraft:plains"
    );
}

#[test]
fn heightmap_lengths_follow_world_height_and_simple_bit_storage_rows() {
    for (section_count, expected) in [(8, 32), (24, 37), (32, 43)] {
        let dynamic_context = Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: -(section_count as i32 / 2),
            section_count,
        };
        assert_eq!(expected_heightmap_longs(dynamic_context), expected);

        let mut raw = fixture();
        mutate_field(
            &mut raw,
            "yPos",
            NbtSummaryValue::Int(dynamic_context.min_section_y),
        );
        mutate_field(
            &mut raw,
            "sections",
            list(vec![compound(vec![
                entry(
                    "Y",
                    NbtSummaryValue::Byte(dynamic_context.min_section_y as i8),
                ),
                entry("block_states", singleton_blocks()),
                entry("biomes", singleton_biome()),
            ])]),
        );
        mutate_field(
            &mut raw,
            "PostProcessing",
            list(vec![list(Vec::new()); section_count]),
        );
        mutate_field(&mut raw, "Heightmaps", heightmaps_with_len(expected));
        assert_eq!(
            Java26ChunkData::parse(raw.clone(), dynamic_context)
                .unwrap()
                .heightmaps()
                .len(),
            4
        );

        mutate_field(
            &mut raw,
            "Heightmaps",
            heightmaps_with_len(expected.saturating_sub(1)),
        );
        // A mis-sized heightmap is primed again on load.
        assert!(Java26ChunkData::parse(raw, dynamic_context)
            .unwrap()
            .heightmaps()
            .is_empty());
    }
}

#[test]
fn proto_entities_are_kept_raw() {
    let mut valid = fixture();
    mutate_field(
        &mut valid,
        "Status",
        NbtSummaryValue::string("minecraft:noise"),
    );
    mutate_field(
        &mut valid,
        "Heightmaps",
        compound(vec![
            entry("OCEAN_FLOOR_WG", NbtSummaryValue::LongArray(vec![0; 37])),
            entry("WORLD_SURFACE_WG", NbtSummaryValue::LongArray(vec![0; 37])),
        ]),
    );
    let NbtSummaryValue::Compound(mut root) = proto_entity(1, 0.5, 0.5) else {
        unreachable!()
    };
    root.push(entry(
        "Passengers",
        list(vec![proto_entity(2, 128.5, 128.5)]),
    ));
    valid
        .entries_mut()
        .push(entry("entities", list(vec![compound(root)])));
    let chunk = Java26ChunkData::parse(valid, context()).unwrap();
    assert_eq!(chunk.entities().unwrap().len(), 1);
}

#[test]
fn java26_legacy_generation_payloads_are_typed_losslessly() {
    let mut raw = fixture();
    mutate_field(
        &mut raw,
        "Status",
        NbtSummaryValue::string("minecraft:features"),
    );
    raw.entries_mut().extend([
        entry("entities", list(Vec::new())),
        entry("carving_mask", NbtSummaryValue::LongArray(Vec::new())),
        entry(
            "UpgradeData",
            compound(vec![
                entry("Sides", NbtSummaryValue::Byte(-127)),
                entry(
                    "Indices",
                    compound(vec![
                        entry("0", NbtSummaryValue::IntArray(vec![0x0123, 0x0fff])),
                        entry("23", NbtSummaryValue::IntArray(Vec::new())),
                    ]),
                ),
                entry(
                    "neighbor_block_ticks",
                    list(vec![compound(vec![
                        entry("i", NbtSummaryValue::string("minecraft:stone")),
                        entry("x", NbtSummaryValue::Int(16)),
                        entry("y", NbtSummaryValue::Int(-64)),
                        entry("z", NbtSummaryValue::Int(0)),
                        entry("t", NbtSummaryValue::Int(2)),
                        entry("p", NbtSummaryValue::Int(-1)),
                    ])]),
                ),
                entry("neighbor_fluid_ticks", list(Vec::new())),
            ]),
        ),
        entry(
            "below_zero_retrogen",
            compound(vec![
                entry("target_status", NbtSummaryValue::string("minecraft:full")),
                entry(
                    "missing_bedrock",
                    NbtSummaryValue::LongArray(vec![1, i64::MIN]),
                ),
            ]),
        ),
    ]);
    let parsed = Java26ChunkData::parse(raw.clone(), context()).unwrap();
    let upgrade = parsed.upgrade_data().unwrap();
    assert_eq!(upgrade.sides(), 129);
    assert_eq!(upgrade.indices().unwrap()[&0], [0x0123, 0x0fff]);
    assert_eq!(upgrade.indices().unwrap()[&23], Vec::<i32>::new());
    assert_eq!(upgrade.neighbor_block_ticks().unwrap().len(), 1);
    assert_eq!(upgrade.neighbor_fluid_ticks(), Some(&[][..]));
    let retrogen = parsed.below_zero_retrogen().unwrap();
    assert_eq!(retrogen.target_status(), Java26ChunkStatus::Full);
    assert_eq!(
        retrogen.missing_bedrock_words(),
        Some(&[1, 1_u64 << 63][..])
    );
    assert!(retrogen.has_bedrock_hole(0, 0));
    assert!(retrogen.has_bedrock_hole(15, 7));

    let mut default_sides = raw.clone();
    let NbtSummaryValue::Compound(fields) = &mut default_sides
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "UpgradeData")
        .expect("the fixture contains UpgradeData")
        .value
    else {
        unreachable!("the fixture contains UpgradeData")
    };
    fields.retain(|entry| entry.name != "Sides");
    assert_eq!(
        Java26ChunkData::parse(default_sides, context())
            .unwrap()
            .upgrade_data()
            .unwrap()
            .sides(),
        0
    );
}

#[test]
fn java26_blending_data_typed_round_trip() {
    let absent = Java26ChunkData::parse(fixture(), context()).unwrap();
    assert!(absent.blending_data().is_none());
    assert!(!absent.legacy_generation_metadata().has_blending_data());

    let packed = |heights: Option<Vec<NbtSummaryValue>>| {
        let mut fields = vec![
            entry("min_section", NbtSummaryValue::Int(-4)),
            entry("max_section", NbtSummaryValue::Int(20)),
        ];
        if let Some(heights) = heights {
            fields.push(entry("heights", list(heights)));
        }
        compound(fields)
    };

    let mut without_heights = fixture();
    without_heights
        .entries_mut()
        .push(entry("blending_data", packed(None)));
    let without_heights = Java26ChunkData::parse(without_heights, context()).unwrap();
    let blending = without_heights.blending_data().unwrap();
    assert_eq!(blending.min_section(), -4);
    assert_eq!(blending.max_section(), 20);
    assert!(blending.height_bits().is_none());
    assert!(blending.heights().is_none());
    assert!(without_heights
        .legacy_generation_metadata()
        .has_blending_data());

    let bits = [
        0x0000_0000_0000_0000,
        0x8000_0000_0000_0000,
        0x3ff0_0000_0000_0000,
        0xbff0_0000_0000_0000,
        0x4008_0000_0000_0000,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff8_0000_0000_0001,
        0x0010_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x3fd0_0000_0000_0000,
        0x3fe0_0000_0000_0000,
        0x4010_0000_0000_0000,
        0x4024_0000_0000_0000,
        0xc024_0000_0000_0000,
        0x7fe0_0000_0000_0000,
    ];
    let mut with_heights = fixture();
    with_heights.entries_mut().push(entry(
        "blending_data",
        packed(Some(
            bits.into_iter().map(NbtSummaryValue::Double).collect(),
        )),
    ));
    let with_heights = Java26ChunkData::parse(with_heights, context()).unwrap();
    let blending = with_heights.blending_data().unwrap();
    assert_eq!(blending.height_bits(), Some(&bits));
    assert_eq!(
        blending.heights().unwrap().map(|height| height.to_bits()),
        bits
    );
}

#[test]
#[ignore = "requires BBB_TEST_JAVA_26_1_WORLD official live-gate artifact"]
fn verified_official_java_26_1_chunk_schema() {
    let world = PathBuf::from(
        std::env::var_os("BBB_TEST_JAVA_26_1_WORLD")
            .expect("BBB_TEST_JAVA_26_1_WORLD must point at an official Java 26.1 world"),
    );
    let region_dir = world
        .join("dimensions")
        .join("minecraft")
        .join("overworld")
        .join("region");
    let mut checked_regions = 0;
    let mut checked_chunks = 0;
    for path in region_paths(&region_dir) {
        let (region_x, region_z) = parse_region_coords(&path);
        let occupied = occupied_slots(&path);
        assert!(
            !occupied.is_empty(),
            "official terrain region {} is empty",
            path.display()
        );
        let store = RegionFileStore::new(&path, &region_dir, WorldSaveLimits::default()).unwrap();
        for local in occupied {
            let global_x = region_x * 32 + (local % 32) as i32;
            let global_z = region_z * 32 + (local / 32) as i32;
            let raw = store.read_chunk(global_x, global_z).unwrap().unwrap();
            Java26ChunkData::parse(
                raw,
                Java26ChunkContext {
                    expected_x: global_x,
                    expected_z: global_z,
                    min_section_y: -4,
                    section_count: 24,
                },
            )
            .unwrap_or_else(|error| panic!("official chunk ({global_x}, {global_z}): {error}"));
            checked_chunks += 1;
        }
        checked_regions += 1;
    }
    assert_eq!(checked_regions, 4);
    assert!(checked_chunks > 0);
}

fn region_paths(region_dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(region_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "mca"))
        .collect();
    paths.sort();
    paths
}

fn parse_region_coords(path: &Path) -> (i32, i32) {
    let name = path.file_name().unwrap().to_str().unwrap();
    let mut parts = name.split('.');
    assert_eq!(parts.next(), Some("r"));
    let x = parts.next().unwrap().parse().unwrap();
    let z = parts.next().unwrap().parse().unwrap();
    assert_eq!(parts.next(), Some("mca"));
    (x, z)
}

fn occupied_slots(path: &Path) -> Vec<usize> {
    let header = fs::read(path).unwrap();
    assert!(header.len() >= 4096);
    header[..4096]
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| (*entry != [0, 0, 0, 0]).then_some(index))
        .collect()
}
