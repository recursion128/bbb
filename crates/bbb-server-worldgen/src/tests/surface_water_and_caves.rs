//! Disk, fungus, pile, vegetation, fallen-tree, bamboo, ocean, ice,
//! dripstone, magma and lake Feature.place tests.

use super::super::*;
use super::{
    basalt_state, default_state, origin, place_feature, retained_from_json, still_lava,
    still_water, RecordingLevel,
};
use crate::random::WorldgenRandom;
use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::packets::BlockPos;
use bbb_server_datapack::worldgen::{ConfiguredFeature, FloatProvider};

fn disk_json(state: &str, target_json: &str, radius_json: &str, half_height: i32) -> String {
    format!(
        r#"{{"type":"minecraft:disk","config":{{"state_provider":{{"type":"simple_state_provider","state":{{"Name":"{state}"}}}},"target":{target_json},"radius":{radius_json},"half_height":{half_height}}}}}"#
    )
}

fn disk_retained(
    state: &str,
    target_json: &str,
    radius_json: &str,
    half_height: i32,
) -> ConfiguredFeature {
    retained_from_json(disk_json(state, target_json, radius_json, half_height).as_bytes())
}

#[test]
fn place_configured_feature_disk_true_predicate_places_the_origin() {
    let retained = disk_retained("sandstone", r#"{"type":"true"}"#, "0", 0);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let sandstone = default_state("minecraft:sandstone");
    let mut level = RecordingLevel::admitting();
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(*level.sets.borrow(), [(origin, sandstone, UPDATE_CLIENTS)]);
    assert!(level.postprocess.borrow().is_empty());
}

#[test]
fn place_configured_feature_disk_circle_skips_corners() {
    let retained = disk_retained("sandstone", r#"{"type":"true"}"#, "1", 0);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let sandstone = default_state("minecraft:sandstone");
    let mut level = RecordingLevel::admitting();
    assert!(place_feature(&retained, &mut level, origin));
    let cell = |x, z| (BlockPos { x, y: origin.y, z }, sandstone, UPDATE_CLIENTS);
    assert_eq!(
        *level.sets.borrow(),
        [cell(0, -1), cell(-1, 0), cell(0, 0), cell(1, 0), cell(0, 1),]
    );
}

#[test]
fn place_configured_feature_disk_half_height_walks_the_column() {
    let retained = disk_retained("sandstone", r#"{"type":"true"}"#, "0", 1);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let sandstone = default_state("minecraft:sandstone");
    let mut level = RecordingLevel::admitting();
    assert!(place_feature(&retained, &mut level, origin));
    let cell = |y| {
        (
            BlockPos {
                x: origin.x,
                y,
                z: origin.z,
            },
            sandstone,
            UPDATE_CLIENTS,
        )
    };
    assert_eq!(*level.sets.borrow(), [cell(65), cell(64), cell(63)]);
}

#[test]
fn place_configured_feature_disk_matching_blocks_is_selective() {
    let retained = disk_retained(
        "sandstone",
        r#"{"type":"matching_blocks","blocks":["minecraft:stone"]}"#,
        "0",
        1,
    );
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let stone = default_state("minecraft:stone");
    let sandstone = default_state("minecraft:sandstone");
    let mut level = RecordingLevel::admitting();
    level
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z), stone);
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(*level.sets.borrow(), [(origin, sandstone, UPDATE_CLIENTS)]);
}

#[test]
fn place_configured_feature_disk_marks_above_for_postprocessing() {
    let retained = disk_retained("sandstone", r#"{"type":"true"}"#, "0", 0);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let stone = default_state("minecraft:stone");
    let above = BlockPos {
        x: origin.x,
        y: origin.y + 1,
        z: origin.z,
    };
    let above_two = BlockPos {
        x: origin.x,
        y: origin.y + 2,
        z: origin.z,
    };

    let mut both = RecordingLevel::filled(stone);
    assert!(place_feature(&retained, &mut both, origin));
    assert_eq!(*both.postprocess.borrow(), [above, above_two]);

    let mut first_only = RecordingLevel::admitting();
    first_only
        .cells
        .borrow_mut()
        .insert((above.x, above.y, above.z), stone);
    assert!(place_feature(&retained, &mut first_only, origin));
    assert_eq!(*first_only.postprocess.borrow(), [above]);

    let mut air_then_stone = RecordingLevel::admitting();
    air_then_stone
        .cells
        .borrow_mut()
        .insert((above_two.x, above_two.y, above_two.z), stone);
    assert!(place_feature(&retained, &mut air_then_stone, origin));
    assert!(air_then_stone.postprocess.borrow().is_empty());
}

#[test]
fn place_configured_feature_disk_resets_placed_above_between_gaps() {
    let retained = disk_retained(
        "sandstone",
        r#"{"type":"matching_blocks","blocks":["minecraft:dirt"]}"#,
        "0",
        2,
    );
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let dirt = default_state("minecraft:dirt");
    let stone = default_state("minecraft:stone");
    let mut level = RecordingLevel::filled(dirt);
    level
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z), stone);
    assert!(place_feature(&retained, &mut level, origin));
    let mark = |y| BlockPos {
        x: origin.x,
        y,
        z: origin.z,
    };
    assert_eq!(
        *level.postprocess.borrow(),
        [mark(67), mark(68), mark(64), mark(65),]
    );
}

#[test]
fn place_configured_feature_disk_rule_without_fallback_skips_the_cell() {
    let json = r#"{"type":"minecraft:disk","config":{"state_provider":{"type":"rule_based_state_provider","rules":[]},"target":{"type":"true"},"radius":0,"half_height":0}}"#;
    let retained = retained_from_json(json.as_bytes());
    let mut level = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut level, origin()));
    assert!(level.sets.borrow().is_empty());
}

fn huge_fungus_json(predicate_json: &str, planted: bool) -> String {
    format!(
        r#"{{"type":"minecraft:huge_fungus","config":{{"valid_base_block":{{"Name":"crimson_nylium"}},"stem_state":{{"Name":"crimson_stem"}},"hat_state":{{"Name":"nether_wart_block"}},"decor_state":{{"Name":"shroomlight"}},"replaceable_blocks":{predicate_json},"planted":{planted}}}}}"#
    )
}

fn huge_fungus_retained(predicate_json: &str, planted: bool) -> ConfiguredFeature {
    retained_from_json(huge_fungus_json(predicate_json, planted).as_bytes())
}

fn prepare_fungus_base(level: &mut RecordingLevel, origin: BlockPos) {
    level.cells.borrow_mut().insert(
        (origin.x, origin.y - 1, origin.z),
        default_state("minecraft:crimson_nylium"),
    );
}

#[test]
fn place_configured_feature_huge_fungus_requires_valid_base() {
    let retained = huge_fungus_retained(r#"{"type":"true"}"#, false);
    let mut level = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut level, origin()));
    assert!(level.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_huge_fungus_places_stem_and_hat() {
    let retained = huge_fungus_retained(r#"{"type":"true"}"#, false);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_fungus_base(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    let air = default_state("minecraft:air");
    let sets = level.sets.borrow().clone();
    assert_eq!(sets[0], (origin, air, UPDATE_NONE));
    assert!(sets.iter().any(|(pos, state, flags)| {
        *pos == origin && block_is(*state, "minecraft:crimson_stem") && *flags == FEATURE_SET_BLOCK
    }));
    assert!(sets.iter().any(|(_, state, flags)| {
        block_is(*state, "minecraft:nether_wart_block") && *flags == FEATURE_SET_BLOCK
    }));
    assert!(block_is(level.get_block(origin), "minecraft:crimson_stem"));
}

#[test]
fn place_configured_feature_huge_fungus_refuses_when_unplanted_height_exceeds_gen_depth() {
    let retained = huge_fungus_retained(r#"{"type":"true"}"#, false);
    let origin = BlockPos { x: 0, y: 380, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_fungus_base(&mut level, origin);
    assert!(!place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_huge_fungus_planted_skips_gen_depth() {
    let retained = huge_fungus_retained(r#"{"type":"true"}"#, true);
    let origin = BlockPos { x: 0, y: 380, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_fungus_base(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    assert!(!level.sets.borrow().is_empty());
}

fn block_pile_json(state: &str) -> String {
    format!(
        r#"{{"type":"minecraft:block_pile","config":{{"state_provider":{{"type":"simple_state_provider","state":{{"Name":"{state}"}}}}}}}}"#
    )
}

fn block_pile_retained(state: &str) -> ConfiguredFeature {
    retained_from_json(block_pile_json(state).as_bytes())
}

fn prepare_block_pile_floor(level: &mut RecordingLevel, origin: BlockPos, floor: i32) {
    for dx in -4..=4 {
        for dz in -4..=4 {
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y - 1, origin.z + dz), floor);
        }
    }
}

#[test]
fn place_configured_feature_block_pile_requires_min_y_headroom() {
    let retained = block_pile_retained("pumpkin");
    let origin = BlockPos { x: 0, y: -60, z: 0 };
    let mut level = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_block_pile_places_on_sturdy_floor() {
    let retained = block_pile_retained("pumpkin");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let pumpkin = default_state("minecraft:pumpkin");
    let mut level = RecordingLevel::admitting();
    prepare_block_pile_floor(&mut level, origin, default_state("minecraft:stone"));
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets
        .iter()
        .all(|(_, state, flags)| { *state == pumpkin && *flags == UPDATE_NONE }));
    assert!(sets.iter().any(|(pos, _, _)| pos.y == origin.y));
}

#[test]
fn place_configured_feature_block_pile_skips_cells_without_sturdy_floor() {
    let retained = block_pile_retained("pumpkin");
    let mut level = RecordingLevel::admitting();
    assert!(place_feature(&retained, &mut level, origin()));
    assert!(level.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_block_pile_dirt_path_is_probability_gated() {
    let retained = block_pile_retained("pumpkin");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let pumpkin = default_state("minecraft:pumpkin");
    let mut path = RecordingLevel::admitting();
    prepare_block_pile_floor(&mut path, origin, default_state("minecraft:dirt_path"));
    assert!(place_feature(&retained, &mut path, origin));
    assert!(path
        .sets
        .borrow()
        .iter()
        .all(|(_, state, flags)| { *state == pumpkin && *flags == UPDATE_NONE }));
    let mut stone = RecordingLevel::admitting();
    prepare_block_pile_floor(&mut stone, origin, default_state("minecraft:stone"));
    assert!(place_feature(&retained, &mut stone, origin));
    assert_ne!(*path.sets.borrow(), *stone.sets.borrow());
}

#[test]
fn place_configured_feature_block_pile_uses_owned_dynamic_shape_support_without_placing() {
    let retained = block_pile_retained("pumpkin");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_block_pile_floor(&mut level, origin, default_state("minecraft:powder_snow"));
    assert!(place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_fallen_tree_places_stump_and_log() {
    let retained = fallen_tree_retained("6", "[]", "[]");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_dirt_floor(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(sets.iter().any(|(pos, state, flags)| {
        *pos == origin && block_is(*state, "minecraft:oak_log") && *flags == FEATURE_SET_BLOCK
    }));
    assert!(sets.len() > 1);
    assert!(sets.iter().all(|(_, state, flags)| {
        block_is(*state, "minecraft:oak_log") && *flags == FEATURE_SET_BLOCK
    }));
}

#[test]
fn place_configured_feature_fallen_tree_returns_true_when_log_does_not_fit() {
    let retained = fallen_tree_retained("6", "[]", "[]");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].0, origin);
    assert!(block_is(sets[0].1, "minecraft:oak_log"));
}

#[test]
fn place_configured_feature_fallen_tree_runs_stump_and_log_decorators() {
    let retained = fallen_tree_retained(
        "6",
        r#"[{"type":"minecraft:trunk_vine"}]"#,
        r#"[{"type":"minecraft:attached_to_logs","probability":1.0,"block_provider":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:red_mushroom"}},"directions":["up"]}]"#,
    );
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_dirt_floor(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow();
    assert_eq!(
        sets.iter()
            .filter(|(_, state, flags)| {
                block_is(*state, "minecraft:oak_log") && *flags == FEATURE_SET_BLOCK
            })
            .count(),
        5
    );
    assert_eq!(
        sets.iter()
            .filter(|(_, state, flags)| {
                block_is(*state, "minecraft:red_mushroom") && *flags == tree::BLOCK_UPDATE_FLAGS
            })
            .count(),
        4
    );
    assert!(sets.iter().any(|(_, state, flags)| {
        block_is(*state, "minecraft:vine") && *flags == tree::BLOCK_UPDATE_FLAGS
    }));
}

fn kelp_retained() -> ConfiguredFeature {
    retained_from_json(br#"{"type":"minecraft:kelp","config":{}}"#)
}

fn prepare_kelp_column(level: &mut RecordingLevel, origin: BlockPos) {
    let sand = default_state("minecraft:sand");
    let water = still_water();
    level
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y - 1, origin.z), sand);
    for dy in 0..=16 {
        level
            .cells
            .borrow_mut()
            .insert((origin.x, origin.y + dy, origin.z), water);
    }
}

#[test]
fn place_configured_feature_kelp_places_on_sand() {
    let retained = kelp_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_kelp_column(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(pos, state, flags)| pos.x == origin.x
        && pos.z == origin.z
        && (block_is(*state, "minecraft:kelp") || block_is(*state, "minecraft:kelp_plant"))
        && *flags == UPDATE_CLIENTS));
    assert!(sets
        .iter()
        .any(|(_, state, _)| block_is(*state, "minecraft:kelp")));
    assert!(sets.iter().any(|(_, state, _)| {
        vanilla_block_state_identity(*state).is_some_and(|id| {
            id.name == "minecraft:kelp"
                && id
                    .properties
                    .get("age")
                    .and_then(|age| age.parse::<i32>().ok())
                    .is_some_and(|age| (20..=23).contains(&age))
        })
    }));
}

fn sea_pickle_retained(count: &str) -> ConfiguredFeature {
    retained_from_json(
        format!(r#"{{"type":"minecraft:sea_pickle","config":{{"count":{count}}}}}"#).as_bytes(),
    )
}

#[test]
fn place_configured_feature_sea_pickle_places_on_sand() {
    let retained = sea_pickle_retained("4");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_ocean_floor(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(pos, state, flags)| pos.y == origin.y
        && block_is(*state, "minecraft:sea_pickle")
        && *flags == UPDATE_CLIENTS));
    assert!(sets.iter().all(|(_, state, _)| {
        vanilla_block_state_identity(*state).is_some_and(|id| {
            id.properties.get("waterlogged").map(String::as_str) == Some("true")
                && id
                    .properties
                    .get("pickles")
                    .and_then(|n| n.parse::<i32>().ok())
                    .is_some_and(|n| (1..=4).contains(&n))
        })
    }));
}

fn vines_retained() -> ConfiguredFeature {
    retained_from_json(br#"{"type":"minecraft:vines","config":{}}"#)
}

#[test]
fn place_configured_feature_vines_attaches_to_the_first_acceptable_face() {
    let retained = vines_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    let stone = default_state("minecraft:stone");
    level
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y + 1, origin.z), stone);
    level
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z - 1), stone);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].0, origin);
    assert_eq!(sets[0].2, UPDATE_CLIENTS);
    assert!(block_is(sets[0].1, "minecraft:vine"));
    let identity = vanilla_block_state_identity(sets[0].1).unwrap();
    assert_eq!(
        identity.properties.get("up").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        identity.properties.get("north").map(String::as_str),
        Some("false")
    );
}

#[test]
fn place_configured_feature_vines_skips_down_and_requires_an_empty_origin() {
    let retained = vines_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let stone = default_state("minecraft:stone");
    let mut below_only = RecordingLevel::admitting();
    below_only
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y - 1, origin.z), stone);
    assert!(!place_feature(&retained, &mut below_only, origin));
    assert!(below_only.sets.borrow().is_empty());

    let mut occupied = RecordingLevel::admitting();
    occupied
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z), stone);
    occupied
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z - 1), stone);
    assert!(!place_feature(&retained, &mut occupied, origin));
    assert!(occupied.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_vines_attaches_north_when_up_is_empty() {
    let retained = vines_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    level.cells.borrow_mut().insert(
        (origin.x, origin.y, origin.z - 1),
        default_state("minecraft:stone"),
    );
    assert!(place_feature(&retained, &mut level, origin));
    let identity = vanilla_block_state_identity(level.sets.borrow()[0].1).unwrap();
    assert_eq!(
        identity.properties.get("north").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        identity.properties.get("up").map(String::as_str),
        Some("false")
    );
}

fn chorus_plant_retained() -> ConfiguredFeature {
    retained_from_json(br#"{"type":"minecraft:chorus_plant","config":{}}"#)
}

fn prepare_chorus_floor(level: &mut RecordingLevel, origin: BlockPos) {
    level.cells.borrow_mut().insert(
        (origin.x, origin.y - 1, origin.z),
        default_state("minecraft:end_stone"),
    );
}

#[test]
fn place_configured_feature_chorus_plant_places_on_end_stone() {
    let retained = chorus_plant_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_chorus_floor(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(_, _, flags)| *flags == UPDATE_CLIENTS));
    assert!(sets
        .iter()
        .any(|(_, state, _)| block_is(*state, "minecraft:chorus_plant")));
    assert!(
        block_is(level.get_block(origin), "minecraft:chorus_plant")
            || block_is(level.get_block(origin), "minecraft:chorus_flower")
    );
}

#[test]
fn place_configured_feature_chorus_plant_requires_empty_origin_on_end_stone() {
    let retained = chorus_plant_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut no_floor = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut no_floor, origin));
    assert!(no_floor.sets.borrow().is_empty());

    let mut occupied = RecordingLevel::admitting();
    prepare_chorus_floor(&mut occupied, origin);
    occupied.cells.borrow_mut().insert(
        (origin.x, origin.y, origin.z),
        default_state("minecraft:stone"),
    );
    assert!(!place_feature(&retained, &mut occupied, origin));
    assert!(occupied.sets.borrow().is_empty());
}

/// `ChorusFlowerBlock.generatePlant` gates on the live
/// `minecraft:supports_chorus_plant` (`ChorusPlantFeature.java:21-26`).
#[test]
fn place_configured_feature_chorus_plant_follows_the_live_support_tag() {
    let retained = chorus_plant_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let stone = default_state("minecraft:stone");

    let mut refused = RecordingLevel::admitting();
    refused
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y - 1, origin.z), stone);
    assert!(!place_feature(&retained, &mut refused, origin));
    assert!(refused.sets.borrow().is_empty());

    let mut admitted = RecordingLevel::admitting();
    admitted
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y - 1, origin.z), stone);
    admitted.set_block_tag_membership("minecraft:supports_chorus_plant", "minecraft:stone", true);
    assert!(place_feature(&retained, &mut admitted, origin));
    assert!(admitted
        .sets
        .borrow()
        .iter()
        .any(|(_, state, _)| block_is(*state, "minecraft:chorus_plant")));

    let mut excluded = RecordingLevel::admitting();
    prepare_chorus_floor(&mut excluded, origin);
    excluded.set_block_tag_membership(
        "minecraft:supports_chorus_plant",
        "minecraft:end_stone",
        false,
    );
    assert!(!place_feature(&retained, &mut excluded, origin));
    assert!(excluded.sets.borrow().is_empty());
}

fn basalt_columns_retained_constant(reach: i32, height: i32) -> ConfiguredFeature {
    let json = format!(
        r#"{{"type":"minecraft:basalt_columns","config":{{"reach":{reach},"height":{height}}}}}"#
    );
    retained_from_json(json.as_bytes())
}

fn prepare_basalt_columns_floor(level: &mut RecordingLevel, origin: BlockPos, floor: i32) {
    for dx in -12..=12 {
        for dz in -12..=12 {
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y - 1, origin.z + dz), floor);
        }
    }
}

#[test]
fn place_configured_feature_basalt_columns_requires_air_origin_on_placeable_floor() {
    let retained = basalt_columns_retained_constant(1, 1);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let netherrack = default_state("minecraft:netherrack");
    let basalt = basalt_state();

    let mut open_air = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut open_air, origin));
    assert!(open_air.sets.borrow().is_empty());

    let mut soul_sand = RecordingLevel::admitting();
    prepare_basalt_columns_floor(&mut soul_sand, origin, default_state("minecraft:soul_sand"));
    assert!(!place_feature(&retained, &mut soul_sand, origin));
    assert!(soul_sand.sets.borrow().is_empty());

    let mut occupied = RecordingLevel::filled(netherrack);
    assert!(!place_feature(&retained, &mut occupied, origin));
    assert!(occupied.sets.borrow().is_empty());

    let mut level = RecordingLevel::admitting();
    prepare_basalt_columns_floor(&mut level, origin, netherrack);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets
        .iter()
        .all(|(_, state, flags)| { *state == basalt && *flags == FEATURE_SET_BLOCK }));
    assert!(sets
        .iter()
        .any(|(pos, _, _)| block_is(level.get_block(*pos), "minecraft:basalt")));
}

#[test]
fn place_configured_feature_basalt_columns_treats_lava_at_sea_level_as_ocean() {
    let retained = basalt_columns_retained_constant(1, 1);
    let origin = BlockPos { x: 0, y: 32, z: 0 };
    let netherrack = default_state("minecraft:netherrack");
    let lava = still_lava();
    let basalt = basalt_state();

    let mut above_sea = RecordingLevel::admitting();
    above_sea.sea_level = 31;
    above_sea
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z), lava);
    prepare_basalt_columns_floor(&mut above_sea, origin, netherrack);
    assert!(!place_feature(&retained, &mut above_sea, origin));
    assert!(above_sea.sets.borrow().is_empty());

    let mut ocean = RecordingLevel::admitting();
    ocean.sea_level = 32;
    ocean
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z), lava);
    prepare_basalt_columns_floor(&mut ocean, origin, netherrack);
    assert!(place_feature(&retained, &mut ocean, origin));
    let sets = ocean.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets
        .iter()
        .all(|(_, state, flags)| *state == basalt && *flags == FEATURE_SET_BLOCK));
}

#[test]
fn place_configured_feature_basalt_columns_vanilla_small_columns_are_seed_stable() {
    let retained = retained_from_json(
        br#"{"type":"minecraft:basalt_columns","config":{"height":{"type":"minecraft:uniform","max_inclusive":4,"min_inclusive":1},"reach":1}}"#,
    );
    let origin = BlockPos { x: 4, y: 40, z: -3 };
    let netherrack = default_state("minecraft:netherrack");
    let mut first = RecordingLevel::admitting();
    let mut second = RecordingLevel::admitting();
    prepare_basalt_columns_floor(&mut first, origin, netherrack);
    prepare_basalt_columns_floor(&mut second, origin, netherrack);
    let mut rng_a = WorldgenRandom::with_seed(99);
    let mut rng_b = WorldgenRandom::with_seed(99);
    assert!(place_configured_feature(
        &retained, &mut first, origin, &mut rng_a
    ));
    assert!(place_configured_feature(
        &retained,
        &mut second,
        origin,
        &mut rng_b
    ));
    assert!(!first.sets.borrow().is_empty());
    assert_eq!(*first.sets.borrow(), *second.sets.borrow());
    assert_eq!(*first.cells.borrow(), *second.cells.borrow());
}

fn iceberg_packed_retained() -> ConfiguredFeature {
    retained_from_json(
        br#"{"type":"minecraft:iceberg","config":{"state":{"Name":"minecraft:packed_ice"}}}"#,
    )
}

#[test]
fn place_configured_feature_iceberg_rewrites_origin_y_to_sea_level() {
    let retained = iceberg_packed_retained();
    let origin = BlockPos { x: 0, y: 8, z: 0 };
    let packed_ice = default_state("minecraft:packed_ice");
    let snow_block = default_state("minecraft:snow_block");
    let mut level = RecordingLevel::admitting();
    level.sea_level = 32;
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(_, _, flags)| *flags == FEATURE_SET_BLOCK));
    assert!(sets
        .iter()
        .any(|(pos, state, _)| { pos.y == 32 && (*state == packed_ice || *state == snow_block) }));
    assert!(!sets.iter().any(|(pos, state, _)| {
        pos.y == origin.y && (*state == packed_ice || *state == snow_block)
    }));
}

fn dripstone_cluster_constant_retained() -> ConfiguredFeature {
    retained_from_json(
        br#"{
            "type":"minecraft:dripstone_cluster",
            "config":{
                "floor_to_ceiling_search_range":12,
                "height":4,
                "radius":1,
                "max_stalagmite_stalactite_height_diff":1,
                "height_deviation":3,
                "dripstone_block_layer_thickness":2,
                "density":1.0,
                "wetness":0.0,
                "chance_of_dripstone_column_at_max_distance_from_center":1.0,
                "max_distance_from_edge_affecting_chance_of_dripstone_column":3,
                "max_distance_from_center_affecting_height_bias":8
            }
        }"#,
    )
}

#[test]
fn configured_feature_float_providers_sample_in_vanilla_draw_order() {
    let uniform = FloatProvider::Uniform {
        min_inclusive: 2.0,
        max_exclusive: 6.0,
    };
    let mut uniform_actual = WorldgenRandom::with_seed(17);
    let mut uniform_expected = WorldgenRandom::with_seed(17);
    let expected = 2.0 + uniform_expected.next_float() * 4.0;
    assert_eq!(
        sample_float_provider(&uniform, &mut uniform_actual).to_bits(),
        expected.to_bits()
    );

    let normal = FloatProvider::ClampedNormal {
        mean: 0.5,
        deviation: 0.25,
        min: 0.0,
        max: 1.0,
    };
    let mut normal_actual = WorldgenRandom::with_seed(17);
    let mut normal_expected = WorldgenRandom::with_seed(17);
    let expected = (0.5 + normal_expected.next_gaussian() as f32 * 0.25).clamp(0.0, 1.0);
    assert_eq!(
        sample_float_provider(&normal, &mut normal_actual).to_bits(),
        expected.to_bits()
    );

    let trapezoid = FloatProvider::Trapezoid {
        min: -2.0,
        max: 4.0,
        plateau: 2.0,
    };
    let mut trapezoid_actual = WorldgenRandom::with_seed(17);
    let mut trapezoid_expected = WorldgenRandom::with_seed(17);
    let expected =
        -2.0 + trapezoid_expected.next_float() * 4.0 + trapezoid_expected.next_float() * 2.0;
    assert_eq!(
        sample_float_provider(&trapezoid, &mut trapezoid_actual).to_bits(),
        expected.to_bits()
    );
}

fn prepare_dripstone_cave(level: &mut RecordingLevel, origin: BlockPos, half: i32, radius: i32) {
    let stone = default_state("minecraft:stone");
    let air = default_state("minecraft:air");
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            for dy in -half..=half {
                let state = if dy == -half || dy == half {
                    stone
                } else {
                    air
                };
                level
                    .cells
                    .borrow_mut()
                    .insert((origin.x + dx, origin.y + dy, origin.z + dz), state);
            }
        }
    }
}

#[test]
fn place_configured_feature_dripstone_cluster_requires_empty_or_water_origin() {
    let retained = dripstone_cluster_constant_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut stone = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(!place_feature(&retained, &mut stone, origin));
    assert!(stone.sets.borrow().is_empty());

    let mut cave = RecordingLevel::admitting();
    prepare_dripstone_cave(&mut cave, origin, 6, 2);
    assert!(place_feature(&retained, &mut cave, origin));
    let sets = cave.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(_, _, flags)| *flags == UPDATE_CLIENTS));
    assert!(sets.iter().any(|(_, state, _)| {
        block_is(*state, "minecraft:dripstone_block")
            || block_is(*state, "minecraft:pointed_dripstone")
    }));
}

fn large_dripstone_constant_retained() -> ConfiguredFeature {
    retained_from_json(
        br#"{
            "type":"minecraft:large_dripstone",
            "config":{
                "floor_to_ceiling_search_range":30,
                "column_radius":3,
                "height_scale":1.0,
                "max_column_radius_to_cave_height_ratio":1.0,
                "stalactite_bluntness":0.5,
                "stalagmite_bluntness":0.5,
                "wind_speed":0.0,
                "min_radius_for_wind":4,
                "min_bluntness_for_wind":0.6
            }
        }"#,
    )
}

#[test]
fn place_configured_feature_large_dripstone_requires_a_tall_stone_column() {
    let retained = large_dripstone_constant_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };

    let mut open_air = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut open_air, origin));
    assert!(open_air.sets.borrow().is_empty());

    let mut short_cave = RecordingLevel::admitting();
    prepare_dripstone_cave(&mut short_cave, origin, 2, 8);
    assert!(!place_feature(&retained, &mut short_cave, origin));
    assert!(short_cave.sets.borrow().is_empty());

    let mut cave = RecordingLevel::admitting();
    prepare_dripstone_cave(&mut cave, origin, 8, 8);
    assert!(place_feature(&retained, &mut cave, origin));
    let sets = cave.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(_, state, flags)| {
        block_is(*state, "minecraft:dripstone_block") && *flags == UPDATE_CLIENTS
    }));
}

fn underwater_magma_constant_retained() -> ConfiguredFeature {
    retained_from_json(
        br#"{
            "type":"minecraft:underwater_magma",
            "config":{
                "floor_search_range":5,
                "placement_radius_around_floor":1,
                "placement_probability_per_valid_position":1.0
            }
        }"#,
    )
}

fn prepare_underwater_magma_floor(level: &mut RecordingLevel, origin: BlockPos, floor_y: i32) {
    let stone = default_state("minecraft:stone");
    let water = still_water();
    for dx in -2..=2 {
        for dz in -2..=2 {
            for y in (floor_y - 1)..=floor_y {
                level
                    .cells
                    .borrow_mut()
                    .insert((origin.x + dx, y, origin.z + dz), stone);
            }
            for y in (floor_y + 1)..=origin.y {
                level
                    .cells
                    .borrow_mut()
                    .insert((origin.x + dx, y, origin.z + dz), water);
            }
        }
    }
}

#[test]
fn place_configured_feature_underwater_magma_requires_a_water_column_on_stone() {
    let retained = underwater_magma_constant_retained();
    let origin = BlockPos { x: 0, y: 54, z: 0 };
    let magma = default_state("minecraft:magma_block");

    let mut open_air = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut open_air, origin));
    assert!(open_air.sets.borrow().is_empty());

    let mut water_on_stone = RecordingLevel::admitting();
    prepare_underwater_magma_floor(&mut water_on_stone, origin, 50);
    assert!(place_feature(&retained, &mut water_on_stone, origin));
    let sets = water_on_stone.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets
        .iter()
        .all(|(_, state, flags)| *state == magma && *flags == UPDATE_CLIENTS));
    assert!(sets.iter().any(|(pos, _, _)| pos.y == 50));
}

fn lake_lava_retained() -> ConfiguredFeature {
    retained_from_json(
        br#"{
            "type":"minecraft:lake",
            "config":{
                "barrier":{
                    "type":"minecraft:simple_state_provider",
                    "state":{"Name":"minecraft:stone"}
                },
                "fluid":{
                    "type":"minecraft:simple_state_provider",
                    "state":{"Name":"minecraft:lava","Properties":{"level":"0"}}
                }
            }
        }"#,
    )
}

fn lake_water_retained() -> ConfiguredFeature {
    retained_from_json(
        br#"{
            "type":"minecraft:lake",
            "config":{
                "fluid":{
                    "type":"minecraft:simple_state_provider",
                    "state":{"Name":"minecraft:water"}
                },
                "barrier":{
                    "type":"minecraft:simple_state_provider",
                    "state":{"Name":"minecraft:stone"}
                }
            }
        }"#,
    )
}

#[test]
fn place_configured_feature_lake_returns_false_when_origin_is_too_low() {
    let retained = lake_lava_retained();
    let origin = BlockPos { x: 0, y: -60, z: 0 };
    let mut level = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(!place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_lake_refuses_a_liquid_shell_above_the_surface() {
    let retained = lake_lava_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::filled(still_water());
    assert!(!place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_lake_refuses_a_hollow_non_solid_floor() {
    let retained = lake_lava_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_lake_skips_features_cannot_replace() {
    let retained = lake_lava_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::filled(default_state("minecraft:bedrock"));
    assert!(place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_lake_vanilla_lava_json_carves_fluid_and_cave_air() {
    let retained = lake_lava_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let lava = still_lava();
    let cave_air = default_state("minecraft:cave_air");
    let mut level = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(_, _, flags)| *flags == UPDATE_CLIENTS));
    assert!(sets.iter().any(|(_, state, _)| *state == lava));
    assert!(sets.iter().any(|(_, state, _)| *state == cave_air));
    assert!(level
        .block_ticks
        .borrow()
        .iter()
        .any(|(pos, delay)| { *delay == 0 && level.get_block(*pos) == cave_air }));
}

#[test]
fn place_configured_feature_lake_freezes_water_surface_when_biome_says_so() {
    let retained = lake_water_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let ice = default_state("minecraft:ice");
    let mut frozen = RecordingLevel::filled(default_state("minecraft:stone"));
    frozen.biome_should_freeze = true;
    assert!(place_feature(&retained, &mut frozen, origin));
    assert!(frozen
        .sets
        .borrow()
        .iter()
        .any(|(_, state, flags)| { *state == ice && *flags == UPDATE_CLIENTS }));

    let mut warm = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(place_feature(&retained, &mut warm, origin));
    assert!(warm.sets.borrow().iter().all(|(_, state, _)| *state != ice));
}

fn fallen_tree_json(log_length: &str, stump_decorators: &str, log_decorators: &str) -> String {
    format!(
        r#"{{"type":"minecraft:fallen_tree","config":{{"trunk_provider":{{"type":"simple_state_provider","state":{{"Name":"oak_log"}}}},"log_length":{log_length},"stump_decorators":{stump_decorators},"log_decorators":{log_decorators}}}}}"#
    )
}

fn fallen_tree_retained(
    log_length: &str,
    stump_decorators: &str,
    log_decorators: &str,
) -> ConfiguredFeature {
    retained_from_json(fallen_tree_json(log_length, stump_decorators, log_decorators).as_bytes())
}

fn prepare_dirt_floor(level: &mut RecordingLevel, origin: BlockPos) {
    let dirt = default_state("minecraft:dirt");
    for dx in -8..=8 {
        for dz in -8..=8 {
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y - 1, origin.z + dz), dirt);
        }
    }
}

fn prepare_ocean_floor(level: &mut RecordingLevel, origin: BlockPos) {
    let sand = default_state("minecraft:sand");
    let water = still_water();
    for dx in -8..=8 {
        for dz in -8..=8 {
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y - 1, origin.z + dz), sand);
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y, origin.z + dz), water);
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y + 1, origin.z + dz), water);
        }
    }
}
