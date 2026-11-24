use super::*;
use crate::level_file::encode_gzip_named_compound;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::{Java26LevelKey, NamedCompound, WorldSaveLimits, WorldSavePaths};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

static NEXT_TEST: AtomicU64 = AtomicU64::new(0);
fn raid(status: Java26RaidStatus) -> Java26Raid {
    Java26Raid {
        started: true,
        active: false,
        ticks_active: 9_876_543_210,
        raid_omen_level: -2,
        groups_spawned: 3,
        cooldown_ticks: 271,
        post_raid_ticks: 17,
        total_health: 125.5,
        group_count: 7,
        status,
        center: BlockPos {
            x: -123,
            y: -12,
            z: 456,
        },
        heroes_of_the_village: BTreeSet::from([
            Uuid::from_u128(0x8123456789abcdef0123456789abcdef),
            Uuid::from_u128(7),
        ]),
    }
}
fn raids() -> Java26Raids {
    Java26Raids {
        raids: BTreeMap::from([(12, raid(Java26RaidStatus::Ongoing))]),
        next_id: 19,
        tick: 97,
    }
}
fn data(document: &mut NamedCompound) -> &mut Vec<NbtSummaryEntry> {
    let field = document
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "data")
        .unwrap();
    let NbtSummaryValue::Compound(fields) = &mut field.value else {
        panic!("data compound")
    };
    fields
}
#[test]
fn raids_codec_roundtrips_every_field_and_all_statuses_without_new_range_clamps() {
    for status in [
        Java26RaidStatus::Ongoing,
        Java26RaidStatus::Victory,
        Java26RaidStatus::Loss,
        Java26RaidStatus::Stopped,
    ] {
        let expected = Java26Raids {
            raids: BTreeMap::from([(-9, raid(status))]),
            next_id: -17,
            tick: -99,
        };
        let document = java_26_1_raids_document(&expected);
        assert_eq!(parse_java_26_1_raids_document(&document).unwrap(), expected);
    }
}

#[test]
fn raids_codec_missing_list_defaults_empty() {
    let mut document = java_26_1_raids_document(&raids());
    data(&mut document).retain(|entry| entry.name != "raids");
    let parsed = parse_java_26_1_raids_document(&document).unwrap();
    assert!(parsed.raids.is_empty());
    assert_eq!((parsed.next_id, parsed.tick), (19, 97));
}

#[test]
fn raids_saved_data_gzip_load_is_dimension_isolated_and_missing_files_default() {
    let serial = NEXT_TEST.fetch_add(1, Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("bbb-raids-codec-{}-{serial}", std::process::id()));
    let paths = WorldSavePaths::new(&root).unwrap();
    let levels = [
        "minecraft:overworld",
        "minecraft:the_nether",
        "test:deep/raid_world",
    ]
    .map(|id| Java26LevelKey::parse(id).unwrap());
    let empty = Java26Raids::default();
    assert!(empty.raids.is_empty());
    assert_eq!((empty.next_id, empty.tick), (1, 0));
    for level in &levels {
        assert_eq!(
            load_java_26_1_raids(&paths, level, WorldSaveLimits::default()).unwrap(),
            empty
        );
    }
    for (index, level) in levels.iter().enumerate() {
        let mut expected = raids();
        expected.tick = 100 + index as i32;
        let path = paths.dimension_dir(level).join("data/minecraft/raids.dat");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let document = java_26_1_raids_document(&expected);
        let gzip =
            encode_gzip_named_compound(&path, &document, WorldSaveLimits::default()).unwrap();
        assert_eq!(&gzip[..2], &[0x1f, 0x8b]);
        std::fs::write(&path, gzip).unwrap();
        assert_eq!(
            load_java_26_1_raids(&paths, level, WorldSaveLimits::default()).unwrap(),
            expected
        );
    }
    for (index, level) in levels.iter().enumerate() {
        assert_eq!(
            load_java_26_1_raids(&paths, level, WorldSaveLimits::default())
                .unwrap()
                .tick,
            100 + index as i32
        );
    }
    let end = Java26LevelKey::parse("minecraft:the_end").unwrap();
    assert_eq!(
        load_java_26_1_raids(&paths, &end, WorldSaveLimits::default()).unwrap(),
        empty
    );
    std::fs::remove_dir_all(root).unwrap();
}
