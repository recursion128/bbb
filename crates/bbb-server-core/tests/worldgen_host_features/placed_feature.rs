use bbb_protocol::packets::BlockPos;
use bbb_server_datapack::worldgen::PlacedFeature;
use bbb_server_worldgen::random::WorldgenRandom;

use super::seam::SeamLevel;
use super::support::{default_state, origin, RecordingLevel};
use super::UPDATE_CLIENTS;
use bbb_server_worldgen::placed_feature::place_placed_feature;

/// A placed feature decoded from its JSON.
fn nested_placed_feature(placed_json: &str) -> PlacedFeature {
    PlacedFeature::decode(&bbb_registry_codec::parse_registry_json(placed_json.as_bytes()).unwrap())
        .unwrap()
}

const STONE_SIMPLE_BLOCK: &str = r#"{"type":"minecraft:simple_block","config":{"to_place":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:stone"}}}}"#;

#[test]
fn count_then_random_offset_expands_depth_first_with_per_node_draws() {
    let placed = nested_placed_feature(&format!(
        r#"{{"feature":{STONE_SIMPLE_BLOCK},"placement":[{{"type":"minecraft:count","count":2}},{{"type":"minecraft:random_offset","xz_spread":0,"y_spread":{{"type":"minecraft:uniform","min_inclusive":1,"max_inclusive":4}}}}]}}"#
    ));
    let mut level = RecordingLevel::admitting();
    let mut random = WorldgenRandom::with_seed(1);
    assert!(place_placed_feature(
        &placed,
        &mut level,
        &mut random,
        origin()
    ));

    let mut expected = WorldgenRandom::with_seed(1);
    let stone = default_state("minecraft:stone");
    let mut sets = Vec::new();
    for _ in 0..2 {
        let dy = 1 + expected.next_int(4);
        sets.push((
            BlockPos {
                x: origin().x,
                y: origin().y + dy,
                z: origin().z,
            },
            stone,
            UPDATE_CLIENTS,
        ));
    }
    assert_eq!(*level.sets.borrow(), sets);
}

#[test]
fn environment_scan_walks_to_the_target_and_filters_apply_after() {
    let placed = nested_placed_feature(&format!(
        r#"{{"feature":{STONE_SIMPLE_BLOCK},"placement":[{{"type":"minecraft:environment_scan","direction_of_search":"down","target_condition":{{"type":"minecraft:solid"}},"max_steps":4}},{{"type":"minecraft:block_predicate_filter","predicate":{{"type":"minecraft:matching_blocks","offset":[0,1,0],"blocks":"minecraft:air"}}}}]}}"#
    ));
    let mut level = RecordingLevel::admitting();
    let floor = BlockPos {
        x: origin().x,
        y: origin().y - 3,
        z: origin().z,
    };
    level
        .cells
        .borrow_mut()
        .insert((floor.x, floor.y, floor.z), default_state("minecraft:dirt"));
    let mut random = WorldgenRandom::with_seed(1);
    assert!(place_placed_feature(
        &placed,
        &mut level,
        &mut random,
        origin()
    ));
    assert_eq!(
        *level.sets.borrow(),
        vec![(floor, default_state("minecraft:stone"), UPDATE_CLIENTS)]
    );

    // Scanning past the bottom of the world yields nothing.
    let mut low = RecordingLevel::admitting();
    let bottom = BlockPos {
        x: origin().x,
        y: low.min_y + 1,
        z: origin().z,
    };
    let mut random = WorldgenRandom::with_seed(1);
    assert!(!place_placed_feature(
        &placed,
        &mut low,
        &mut random,
        bottom
    ));
    assert!(low.sets.borrow().is_empty());
}

fn stone_with_placement(placement: &str) -> PlacedFeature {
    nested_placed_feature(&format!(
        r#"{{"feature":{STONE_SIMPLE_BLOCK},"placement":[{placement}]}}"#
    ))
}

fn place_stone<L: bbb_server_worldgen::WorldGenLevel>(
    placement: &str,
    level: &mut L,
    seed: i64,
    at: BlockPos,
) -> bool {
    place_placed_feature(
        &stone_with_placement(placement),
        level,
        &mut WorldgenRandom::with_seed(seed),
        at,
    )
}

#[test]
fn rarity_filter_draws_next_float_against_one_over_chance() {
    let mut always = RecordingLevel::admitting();
    assert!(place_stone(
        r#"{"type":"minecraft:rarity_filter","chance":1}"#,
        &mut always,
        1,
        origin(),
    ));
    assert_eq!(
        *always.sets.borrow(),
        vec![(origin(), default_state("minecraft:stone"), UPDATE_CLIENTS)]
    );

    let mut expected = WorldgenRandom::with_seed(1);
    let admit = expected.next_float() < 0.5;
    let mut maybe = RecordingLevel::admitting();
    assert_eq!(
        place_stone(
            r#"{"type":"minecraft:rarity_filter","chance":2}"#,
            &mut maybe,
            1,
            origin(),
        ),
        admit
    );
    assert_eq!(maybe.sets.borrow().len(), usize::from(admit));
}

#[test]
fn in_square_scatters_x_then_z_with_next_int_16() {
    let mut expected = WorldgenRandom::with_seed(1);
    let pos = BlockPos {
        x: expected.next_int(16) + origin().x,
        y: origin().y,
        z: expected.next_int(16) + origin().z,
    };
    let mut level = RecordingLevel::admitting();
    assert!(place_stone(
        r#"{"type":"minecraft:in_square"}"#,
        &mut level,
        1,
        origin()
    ));
    assert_eq!(
        *level.sets.borrow(),
        vec![(pos, default_state("minecraft:stone"), UPDATE_CLIENTS)]
    );
}

#[test]
fn heightmap_placement_snaps_to_the_named_height_or_filters_at_min_y() {
    let mut seamed = SeamLevel::admitting();
    seamed
        .motion_blocking_no_leaves_heights
        .insert((origin().x, origin().z), 80);
    assert!(place_stone(
        r#"{"type":"minecraft:heightmap","heightmap":"MOTION_BLOCKING_NO_LEAVES"}"#,
        &mut seamed,
        1,
        origin(),
    ));
    assert_eq!(
        *seamed.inner.sets.borrow(),
        vec![(
            BlockPos {
                x: origin().x,
                y: 80,
                z: origin().z,
            },
            default_state("minecraft:stone"),
            UPDATE_CLIENTS
        )]
    );

    let mut floor = SeamLevel::admitting();
    floor
        .motion_blocking_no_leaves_heights
        .insert((origin().x, origin().z), floor.inner.min_y);
    assert!(!place_stone(
        r#"{"type":"minecraft:heightmap","heightmap":"MOTION_BLOCKING_NO_LEAVES"}"#,
        &mut floor,
        1,
        origin(),
    ));
    assert!(floor.inner.sets.borrow().is_empty());
}

#[test]
fn height_range_samples_absolute_and_uniform_providers() {
    let mut constant = RecordingLevel::admitting();
    assert!(place_stone(
        r#"{"type":"minecraft:height_range","height":{"absolute":70}}"#,
        &mut constant,
        1,
        origin(),
    ));
    assert_eq!(
        *constant.sets.borrow(),
        vec![(
            BlockPos {
                x: origin().x,
                y: 70,
                z: origin().z,
            },
            default_state("minecraft:stone"),
            UPDATE_CLIENTS
        )]
    );

    let mut expected = WorldgenRandom::with_seed(1);
    let y = 1 + expected.next_int(4);
    let mut uniform = RecordingLevel::admitting();
    assert!(place_stone(
        r#"{"type":"minecraft:height_range","height":{"type":"minecraft:uniform","min_inclusive":{"absolute":1},"max_inclusive":{"absolute":4}}}"#,
        &mut uniform,
        1,
        origin(),
    ));
    assert_eq!(
        *uniform.sets.borrow(),
        vec![(
            BlockPos {
                x: origin().x,
                y,
                z: origin().z,
            },
            default_state("minecraft:stone"),
            UPDATE_CLIENTS
        )]
    );
}

#[test]
fn surface_relative_threshold_and_water_depth_read_named_heightmaps() {
    let mut relative = SeamLevel::admitting();
    relative
        .world_surface_heights
        .insert((origin().x, origin().z), 64);
    assert!(place_stone(
        r#"{"type":"minecraft:surface_relative_threshold_filter","heightmap":"WORLD_SURFACE","min_inclusive":-1,"max_inclusive":0}"#,
        &mut relative,
        1,
        origin(),
    ));
    assert_eq!(relative.inner.sets.borrow().len(), 1);

    let mut below = SeamLevel::admitting();
    below
        .world_surface_heights
        .insert((origin().x, origin().z), 64);
    assert!(!place_stone(
        r#"{"type":"minecraft:surface_relative_threshold_filter","heightmap":"WORLD_SURFACE","min_inclusive":-1,"max_inclusive":0}"#,
        &mut below,
        1,
        BlockPos {
            x: origin().x,
            y: 62,
            z: origin().z,
        },
    ));
    assert!(below.inner.sets.borrow().is_empty());

    let mut water = SeamLevel::admitting();
    water
        .ocean_floor_heights
        .insert((origin().x, origin().z), 60);
    water
        .world_surface_heights
        .insert((origin().x, origin().z), 63);
    assert!(place_stone(
        r#"{"type":"minecraft:surface_water_depth_filter","max_water_depth":3}"#,
        &mut water,
        1,
        origin(),
    ));
    assert_eq!(water.inner.sets.borrow().len(), 1);

    let mut too_deep = SeamLevel::admitting();
    too_deep
        .ocean_floor_heights
        .insert((origin().x, origin().z), 60);
    too_deep
        .world_surface_heights
        .insert((origin().x, origin().z), 63);
    assert!(!place_stone(
        r#"{"type":"minecraft:surface_water_depth_filter","max_water_depth":2}"#,
        &mut too_deep,
        1,
        origin(),
    ));
    assert!(too_deep.inner.sets.borrow().is_empty());
}

#[test]
fn noise_based_and_threshold_count_use_biome_info_noise() {
    let origin = origin();
    let mut based = RecordingLevel::admitting();
    let based_result = place_stone(
        r#"{"type":"minecraft:noise_based_count","noise_to_count_ratio":4,"noise_factor":80.0,"noise_offset":0.3}"#,
        &mut based,
        1,
        origin,
    );
    let mut threshold = RecordingLevel::admitting();
    let threshold_result = place_stone(
        r#"{"type":"minecraft:noise_threshold_count","noise_level":-0.8,"below_noise":3,"above_noise":1}"#,
        &mut threshold,
        1,
        origin,
    );
    assert_eq!(
        (
            based_result,
            based.sets.borrow().clone(),
            threshold_result,
            threshold.sets.borrow().clone()
        ),
        (
            true,
            vec![
                (
                    BlockPos { x: 4, y: 64, z: -8 },
                    default_state("minecraft:stone"),
                    UPDATE_CLIENTS
                );
                4
            ],
            true,
            vec![(
                BlockPos { x: 4, y: 64, z: -8 },
                default_state("minecraft:stone"),
                UPDATE_CLIENTS
            )]
        )
    );
}

#[test]
fn count_on_every_layer_walks_motion_blocking_air_over_ground() {
    let mut expected = WorldgenRandom::with_seed(1);
    let x = expected.next_int(16) + origin().x;
    let z = expected.next_int(16) + origin().z;
    let mut level = SeamLevel::admitting();
    level.motion_blocking_heights.insert((x, z), 71);
    level.set_cell(BlockPos { x, y: 70, z }, default_state("minecraft:dirt"));
    assert!(place_stone(
        r#"{"type":"minecraft:count_on_every_layer","count":1}"#,
        &mut level,
        1,
        origin(),
    ));
    assert_eq!(
        *level.inner.sets.borrow(),
        vec![(
            BlockPos { x, y: 71, z },
            default_state("minecraft:stone"),
            UPDATE_CLIENTS
        )]
    );
}

#[test]
fn fixed_placement_keeps_only_positions_in_the_origin_chunk() {
    let mut same = RecordingLevel::admitting();
    assert!(place_stone(
        r#"{"type":"minecraft:fixed_placement","positions":[[4,80,-8],[20,80,-8]]}"#,
        &mut same,
        1,
        origin(),
    ));
    assert_eq!(
        *same.sets.borrow(),
        vec![(
            BlockPos { x: 4, y: 80, z: -8 },
            default_state("minecraft:stone"),
            UPDATE_CLIENTS
        )]
    );

    let mut other_chunk = RecordingLevel::admitting();
    assert!(!place_stone(
        r#"{"type":"minecraft:fixed_placement","positions":[[20,80,-8]]}"#,
        &mut other_chunk,
        1,
        origin(),
    ));
    assert!(other_chunk.sets.borrow().is_empty());
}
