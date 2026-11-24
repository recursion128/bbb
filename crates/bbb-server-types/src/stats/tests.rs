use super::*;

use bbb_protocol::stat_types::{
    VANILLA_STAT_TYPE_BLOCK_MINED_ID, VANILLA_STAT_TYPE_ITEM_BROKEN_ID,
    VANILLA_STAT_TYPE_ITEM_USED_ID,
};

fn stone() -> i32 {
    bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:stone")
        .expect("the generated catalog carries stone")
        .id
}

fn pickaxe() -> i32 {
    bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:diamond_pickaxe")
        .expect("the generated catalog carries a diamond pickaxe")
        .id
}

fn play_time() -> i32 {
    vanilla_custom_stat_by_resource_id("minecraft:play_time")
        .expect("the generated catalog carries play_time")
        .id
}

/// The JSON shape is the one `ServerStatsCounter.toJson` builds
/// (`ServerStatsCounter.java:115-120`): a two-level `stats` object keyed by the
/// plain colon form, plus the current data version.
#[test]
fn the_json_shape_matches_the_vanilla_writer() {
    let mut counter = ServerStatsCounter::new();
    counter.increment(StatKey::new(VANILLA_STAT_TYPE_BLOCK_MINED_ID, stone()), 42);
    counter.increment(StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, pickaxe()), 7);
    counter.increment(StatKey::custom(play_time()), 1234);
    let document = counter.to_json();
    assert_eq!(
        document,
        serde_json::json!({
            "stats": {
                "minecraft:mined": { "minecraft:stone": 42 },
                "minecraft:used": { "minecraft:diamond_pickaxe": 7 },
                "minecraft:custom": { "minecraft:play_time": 1234 },
            },
            "DataVersion": crate::save::JAVA_26_1_DATA_VERSION,
        })
    );
}

/// The dotted `Stat.buildName` form (`Stat.java:27-33`) is the scoreboard
/// criterion name and must not appear in the file.
#[test]
fn the_json_keys_are_not_the_scoreboard_criterion_form() {
    let mut counter = ServerStatsCounter::new();
    counter.increment(StatKey::new(VANILLA_STAT_TYPE_BLOCK_MINED_ID, stone()), 1);
    let text = String::from_utf8(counter.to_file_bytes()).unwrap();
    assert!(text.contains("\"minecraft:mined\""), "{text}");
    assert!(!text.contains("minecraft.mined"), "{text}");
}

#[test]
fn a_written_document_reads_back_identically() {
    let mut counter = ServerStatsCounter::new();
    counter.increment(StatKey::new(VANILLA_STAT_TYPE_BLOCK_MINED_ID, stone()), 3);
    counter.increment(StatKey::new(VANILLA_STAT_TYPE_ITEM_BROKEN_ID, pickaxe()), 1);
    counter.increment(StatKey::custom(play_time()), 20);
    let bytes = counter.to_file_bytes();
    let read = ServerStatsCounter::parse_bytes(&bytes).unwrap();
    assert_eq!(
        read.value(StatKey::new(VANILLA_STAT_TYPE_BLOCK_MINED_ID, stone())),
        3
    );
    assert_eq!(
        read.value(StatKey::new(VANILLA_STAT_TYPE_ITEM_BROKEN_ID, pickaxe())),
        1
    );
    assert_eq!(read.value(StatKey::custom(play_time())), 20);
    // A parsed counter is clean: nothing has been awarded since it was read.
    assert_eq!(read.clone().take_dirty(), Vec::new());
    assert_eq!(read.to_json(), counter.to_json());
}

/// `StatsCounter.increment` sums in `long` and saturates
/// (`StatsCounter.java:16`), so a counter at the cap stays there instead of
/// wrapping negative.
#[test]
fn a_counter_saturates_at_the_java_int_maximum() {
    let stat = StatKey::custom(play_time());
    let mut counter = ServerStatsCounter::new();
    counter.set_value(stat, i32::MAX - 1);
    counter.increment(stat, 100);
    assert_eq!(counter.value(stat), i32::MAX);
}

/// A missing key reads as zero (`StatsCounter.java:12`) and is not written.
#[test]
fn an_unawarded_stat_reads_zero_and_is_not_written() {
    let counter = ServerStatsCounter::new();
    assert_eq!(counter.value(StatKey::custom(play_time())), 0);
    assert!(counter.is_empty());
    assert_eq!(
        counter.to_json(),
        serde_json::json!({
            "stats": {},
            "DataVersion": crate::save::JAVA_26_1_DATA_VERSION,
        })
    );
}

/// Every award marks the stat dirty, and draining the set clears it
/// (`ServerStatsCounter.java:93-102`).
#[test]
fn the_dirty_set_is_drained_by_the_send() {
    let stat = StatKey::new(VANILLA_STAT_TYPE_BLOCK_MINED_ID, stone());
    let mut counter = ServerStatsCounter::new();
    counter.increment(stat, 1);
    assert_eq!(counter.take_dirty(), vec![(stat, 1)]);
    assert_eq!(counter.take_dirty(), Vec::new());
    // `markAllDirty` puts every counted stat back (`:122-124`).
    counter.mark_all_dirty();
    assert_eq!(counter.take_dirty(), vec![(stat, 1)]);
}

/// An entry naming an id outside the built-in registries is dropped and the
/// rest of the file loads (`ServerStatsCounter.java:110`, `resultOrPartial`).
#[test]
fn an_unknown_key_is_dropped() {
    let document = serde_json::json!({
        "stats": {
            "minecraft:mined": { "minecraft:not_a_block": 1, "minecraft:stone": 3 },
            "minecraft:not_a_stat_type": { "minecraft:stone": 1 },
        },
        "DataVersion": crate::save::JAVA_26_1_DATA_VERSION,
    });
    let counter = ServerStatsCounter::parse(&document).unwrap();
    assert_eq!(
        counter.value(StatKey::new(VANILLA_STAT_TYPE_BLOCK_MINED_ID, stone())),
        3
    );
}

/// A file from another data version is refused: this crate has no datafixer,
/// and the version default vanilla reads for a file that carries none is the
/// 1.12.2 one (`ServerStatsCounter.java:106`).
#[test]
fn a_foreign_data_version_is_refused() {
    let document = serde_json::json!({ "stats": {}, "DataVersion": 1343 });
    assert!(matches!(
        ServerStatsCounter::parse(&document),
        Err(StatsParseError::DataVersion(1343))
    ));
    let document = serde_json::json!({ "stats": {} });
    assert!(matches!(
        ServerStatsCounter::parse(&document),
        Err(StatsParseError::DataVersion(1343))
    ));
}

/// A file with no `stats` member at all is empty rather than broken
/// (`ServerStatsCounter.java:109`, `orElseEmptyMap`).
#[test]
fn a_document_without_a_stats_member_is_empty() {
    let document = serde_json::json!({ "DataVersion": crate::save::JAVA_26_1_DATA_VERSION });
    let counter = ServerStatsCounter::parse(&document).unwrap();
    assert!(counter.is_empty());
}

/// Every custom statistic this crate names by hand is a registry id
/// `Stats.java` declares with `makeCustomStat`, so a rename fails here rather
/// than in a panic at award time.
#[test]
fn the_named_custom_stats_are_declared_by_vanilla() {
    let named = [
        CUSTOM_STAT_LEAVE_GAME,
        CUSTOM_STAT_PLAY_TIME,
        CUSTOM_STAT_TOTAL_WORLD_TIME,
        CUSTOM_STAT_TIME_SINCE_DEATH,
        CUSTOM_STAT_TIME_SINCE_REST,
        CUSTOM_STAT_SNEAK_TIME,
        CUSTOM_STAT_DEATHS,
    ];
    for resource_id in named {
        assert!(
            vanilla_custom_stat_by_resource_id(resource_id).is_some(),
            "the generated catalog has no {resource_id}"
        );
    }
}

#[test]
fn mod_stats_round_trip_with_content_registry() {
    use bbb_protocol::content_registry::{
        ContentRegistryBuilder, ModBlockRegistration, ModEntityRegistration, ModItemRegistration,
    };

    let mut builder = ContentRegistryBuilder::new();
    builder.register_block(ModBlockRegistration {
        resource_id: "modtest:ore".to_string(),
        state_count: 1,
        light_emission: 0,
        destroy_speed: 3.0,
        full_cube_collision: true,
        dynamic_light_rgb: None,
    });
    builder.register_item(ModItemRegistration {
        resource_id: "modtest:gem".to_string(),
        max_stack_size: 64,
        is_block_item: false,
    });
    builder.register_entity(ModEntityRegistration {
        resource_id: "modtest:golem".to_string(),
        ..Default::default()
    });
    let registry = builder.freeze_for_session().unwrap();
    let block_stat = StatKey::new(
        VANILLA_STAT_TYPE_BLOCK_MINED_ID,
        registry
            .block_by_resource_id("modtest:ore")
            .unwrap()
            .block_id,
    );
    let item_stat = StatKey::new(
        VANILLA_STAT_TYPE_ITEM_BROKEN_ID,
        registry.item_by_resource_id("modtest:gem").unwrap().id,
    );
    let entity_stat = StatKey::new(
        bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
        registry.entity_by_resource_id("modtest:golem").unwrap().id,
    );

    let mut counter = ServerStatsCounter::new();
    counter.increment(block_stat, 4);
    counter.increment(item_stat, 2);
    counter.increment(entity_stat, 1);
    let bytes = counter.to_file_bytes_with_registry(&registry);
    let read = ServerStatsCounter::parse_bytes_with_registry(&bytes, &registry).unwrap();
    assert_eq!(read.value(block_stat), 4);
    assert_eq!(read.value(item_stat), 2);
    assert_eq!(read.value(entity_stat), 1);
}
