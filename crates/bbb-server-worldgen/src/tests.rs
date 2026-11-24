//! Shared worldgen Feature.place test fixture and the early owner tests.
//!
//! Later Feature.place owners live in [`surface_water_and_caves`].

pub mod surface_water_and_caves;

use std::collections::BTreeMap;

use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;

use super::*;
use crate::random::WorldgenRandom;

pub mod recording_level;
pub use recording_level::{
    block_is_in_builtin_tag, builtin_registry_tag_contains, builtin_registry_tag_members,
    default_state, origin, place_feature, retained_from_json, RecordingLevel,
};

#[test]
fn place_configured_feature_void_start_platform_fills_the_origin_chunk_disk() {
    let retained = retained_from_json(br#"{"type":"minecraft:void_start_platform","config":{}}"#);
    let mut level = RecordingLevel::admitting();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    assert!(place_feature(&retained, &mut level, origin));
    let stone = default_state("minecraft:stone");
    let cobblestone = default_state("minecraft:cobblestone");
    let sets = level.sets.borrow().clone();
    assert_eq!(sets.len(), 256);
    assert!(sets
        .iter()
        .all(|(pos, _, flags)| pos.y == 67 && *flags == UPDATE_CLIENTS));
    let cobble = sets
        .iter()
        .filter(|(_, state, _)| *state == cobblestone)
        .map(|(pos, _, _)| *pos)
        .collect::<Vec<_>>();
    assert_eq!(cobble, [BlockPos { x: 8, y: 67, z: 8 }]);
    assert_eq!(
        sets.iter().filter(|(_, state, _)| *state == stone).count(),
        255
    );
    assert!(sets
        .iter()
        .all(|(pos, _, _)| { pos.x >= 0 && pos.x <= 15 && pos.z >= 0 && pos.z <= 15 }));
}

#[test]
fn place_configured_feature_void_start_platform_skips_chunks_beyond_chebyshev_one() {
    let retained = retained_from_json(br#"{"type":"minecraft:void_start_platform","config":{}}"#);
    let mut far = RecordingLevel::admitting();
    assert!(place_feature(
        &retained,
        &mut far,
        BlockPos { x: 32, y: 64, z: 0 }
    ));
    assert!(far.sets.borrow().is_empty());

    let mut neighbor = RecordingLevel::admitting();
    assert!(place_feature(
        &retained,
        &mut neighbor,
        BlockPos { x: 16, y: 64, z: 0 }
    ));
    let stone = default_state("minecraft:stone");
    let cobblestone = default_state("minecraft:cobblestone");
    let sets = neighbor.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(pos, state, flags)| {
        pos.x >= 16
            && pos.x <= 31
            && pos.z >= 0
            && pos.z <= 15
            && pos.y == 67
            && *flags == UPDATE_CLIENTS
            && *state == stone
    }));
    assert!(!sets.iter().any(|(_, state, _)| *state == cobblestone));
    assert!(sets
        .iter()
        .any(|(pos, _, _)| *pos == BlockPos { x: 16, y: 67, z: 8 }));
    assert!(!sets
        .iter()
        .any(|(pos, _, _)| *pos == BlockPos { x: 31, y: 67, z: 0 }));
}

#[test]
fn place_configured_feature_end_platform_writes_obsidian_floor_on_air() {
    let retained = retained_from_json(br#"{"type":"minecraft:end_platform","config":{}}"#);
    let mut level = RecordingLevel::admitting();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    assert!(place_feature(&retained, &mut level, origin));
    let obsidian = default_state("minecraft:obsidian");
    let air = default_state("minecraft:air");
    let sets = level.sets.borrow().clone();
    assert_eq!(sets.len(), 25);
    assert!(sets.iter().all(|(pos, state, flags)| {
        *state == obsidian
            && *flags == UPDATE_NEIGHBORS | UPDATE_CLIENTS
            && pos.y == 63
            && pos.x >= -2
            && pos.x <= 2
            && pos.z >= -2
            && pos.z <= 2
    }));
    assert_eq!(
        sets[0].0,
        BlockPos {
            x: -2,
            y: 63,
            z: -2
        }
    );
    assert_eq!(level.get_block(BlockPos { x: 0, y: 64, z: 0 }), air);
    assert_eq!(level.get_block(BlockPos { x: 0, y: 63, z: 0 }), obsidian);
}

#[test]
fn place_configured_feature_end_platform_replaces_nonmatching_and_skips_correct_cells() {
    let retained = retained_from_json(br#"{"type":"minecraft:end_platform","config":{}}"#);
    let stone = default_state("minecraft:stone");
    let obsidian = default_state("minecraft:obsidian");
    let air = default_state("minecraft:air");
    let mut level = RecordingLevel::filled(stone);
    let origin = BlockPos { x: 5, y: 10, z: 5 };
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert_eq!(sets.len(), 100);
    assert_eq!(
        sets.iter()
            .filter(|(pos, state, flags)| {
                pos.y == 9 && *state == obsidian && *flags == UPDATE_NEIGHBORS | UPDATE_CLIENTS
            })
            .count(),
        25
    );
    assert_eq!(
        sets.iter()
            .filter(|(pos, state, _)| pos.y != 9 && *state == air)
            .count(),
        75
    );

    let mut already = RecordingLevel::filled(stone);
    assert!(place_feature(&retained, &mut already, origin));
    already.sets.borrow_mut().clear();
    assert!(place_feature(&retained, &mut already, origin));
    assert!(already.sets.borrow().is_empty());
}

fn desert_well_retained() -> ConfiguredFeature {
    retained_from_json(br#"{"type":"minecraft:desert_well","config":{}}"#)
}

#[test]
fn place_configured_feature_desert_well_returns_false_when_the_floor_is_hollow() {
    let retained = desert_well_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let sand = default_state("minecraft:sand");
    let mut level = RecordingLevel::admitting();
    for dx in -2..=2 {
        for dz in -2..=2 {
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y, origin.z + dz), sand);
        }
    }
    assert!(!place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_desert_well_places_sandstone_water_and_suspicious_sand() {
    let retained = desert_well_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let sand = catalog_default_state("minecraft:sand");
    let sandstone = catalog_default_state("minecraft:sandstone");
    let water = catalog_default_state("minecraft:water");
    let slab = catalog_default_state("minecraft:sandstone_slab");
    let suspicious = catalog_default_state("minecraft:suspicious_sand");
    let mut level = RecordingLevel::filled(sand);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets
        .iter()
        .any(|(_, state, flags)| *state == sandstone && *flags == UPDATE_CLIENTS));
    assert!(sets
        .iter()
        .any(|(_, state, flags)| *state == water && *flags == UPDATE_CLIENTS));
    assert!(sets
        .iter()
        .any(|(_, state, flags)| *state == slab && *flags == UPDATE_CLIENTS));
    assert!(sets
        .iter()
        .any(|(_, state, flags)| *state == sand && *flags == UPDATE_CLIENTS));
    let sus = sets
        .iter()
        .filter(|(_, state, flags)| *state == suspicious && *flags == FEATURE_SET_BLOCK)
        .count();
    assert_eq!(sus, 2);
    let loot = level.brushable_loot.borrow().clone();
    assert_eq!(loot.len(), 2);
    assert!(loot.iter().all(|(pos, table, seed)| {
        *table == DESERT_WELL_ARCHAEOLOGY_LOOT && *seed == block_pos_as_long(*pos)
    }));
    assert_eq!(
        level.get_block(BlockPos {
            x: origin.x,
            y: origin.y + 1,
            z: origin.z
        }),
        water
    );
}

#[test]
fn place_configured_feature_desert_well_walks_down_through_air_to_sand() {
    let retained = desert_well_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let sand = catalog_default_state("minecraft:sand");
    let water = catalog_default_state("minecraft:water");
    let mut level = RecordingLevel::admitting();
    for y in 38..=40 {
        for dx in -2..=2 {
            for dz in -2..=2 {
                level
                    .cells
                    .borrow_mut()
                    .insert((origin.x + dx, y, origin.z + dz), sand);
            }
        }
    }
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(level.get_block(BlockPos { x: 0, y: 40, z: 0 }), water);
}

#[test]
fn place_configured_feature_fill_layer_fills_air_cells_at_min_y_plus_height() {
    let retained = retained_from_json(
        br#"{"type":"minecraft:fill_layer","config":{"height":4,"state":{"Name":"stone"}}}"#,
    );
    let mut level = RecordingLevel::admitting();
    level.min_y = 0;
    let origin = BlockPos { x: 8, y: 64, z: 8 };
    assert!(place_feature(&retained, &mut level, origin));
    let stone = default_state("minecraft:stone");
    let sets = level.sets.borrow().clone();
    assert_eq!(sets.len(), 256);
    assert!(sets.iter().all(|(pos, state, flags)| {
        *state == stone
            && *flags == UPDATE_CLIENTS
            && pos.y == 4
            && pos.x >= 8
            && pos.x <= 23
            && pos.z >= 8
            && pos.z <= 23
    }));
    assert_eq!(sets[0].0, BlockPos { x: 8, y: 4, z: 8 });
    assert_eq!(sets[1].0, BlockPos { x: 8, y: 4, z: 9 });
}

#[test]
fn place_configured_feature_fill_layer_skips_non_air_cells() {
    let retained = retained_from_json(
        br#"{"type":"minecraft:fill_layer","config":{"height":4,"state":{"Name":"stone"}}}"#,
    );
    let dirt = default_state("minecraft:dirt");
    let mut level = RecordingLevel::filled(dirt);
    level.min_y = 0;
    assert!(place_feature(
        &retained,
        &mut level,
        BlockPos { x: 0, y: 0, z: 0 }
    ));
    assert!(level.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_glowstone_blob_requires_air_origin_and_nether_ceiling() {
    let retained = retained_from_json(br#"{"type":"minecraft:glowstone_blob","config":{}}"#);
    let glowstone = default_state("minecraft:glowstone");
    let netherrack = default_state("minecraft:netherrack");
    let origin = BlockPos { x: 0, y: 64, z: 0 };

    let mut blocked = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(!place_feature(&retained, &mut blocked, origin));
    assert!(blocked.sets.borrow().is_empty());

    let mut open_air = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut open_air, origin));
    assert!(open_air.sets.borrow().is_empty());

    let mut level = RecordingLevel::admitting();
    level
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y + 1, origin.z), netherrack);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert_eq!(sets[0], (origin, glowstone, UPDATE_CLIENTS));
    assert!(sets
        .iter()
        .all(|(_, state, flags)| { *state == glowstone && *flags == UPDATE_CLIENTS }));
}

#[test]
fn place_configured_feature_end_island_writes_end_stone_with_feature_set_block_flags() {
    let retained = retained_from_json(br#"{"type":"minecraft:end_island","config":{}}"#);
    let mut level = RecordingLevel::admitting();
    let origin = BlockPos {
        x: 10,
        y: 80,
        z: -4,
    };
    assert!(place_feature(&retained, &mut level, origin));
    let end_stone = default_state("minecraft:end_stone");
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(pos, state, flags)| {
        *state == end_stone && *flags == UPDATE_NEIGHBORS | UPDATE_CLIENTS && pos.y <= origin.y
    }));
    assert!(sets.iter().any(|(pos, _, _)| *pos == origin));
}

#[test]
fn place_configured_feature_end_gateway_places_the_delayed_structure() {
    let retained =
        retained_from_json(br#"{"type":"minecraft:end_gateway","config":{"exact":false}}"#);
    let mut level = RecordingLevel::admitting();
    let origin = BlockPos { x: 0, y: 0, z: 0 };
    assert!(place_feature(&retained, &mut level, origin));
    let gateway = default_state("minecraft:end_gateway");
    let air = default_state("minecraft:air");
    let bedrock = default_state("minecraft:bedrock");
    let sets = level.sets.borrow().clone();
    assert_eq!(sets.len(), 45);
    assert!(sets
        .iter()
        .all(|(_, _, flags)| *flags == UPDATE_NEIGHBORS | UPDATE_CLIENTS));
    assert_eq!(
        sets[0],
        (
            BlockPos {
                x: -1,
                y: -2,
                z: -1
            },
            air,
            UPDATE_NEIGHBORS | UPDATE_CLIENTS
        )
    );
    assert_eq!(level.get_block(origin), gateway);
    assert_eq!(level.get_block(BlockPos { x: 0, y: 2, z: 0 }), bedrock);
    assert_eq!(level.get_block(BlockPos { x: 0, y: -2, z: 0 }), bedrock);
    assert_eq!(level.get_block(BlockPos { x: 1, y: 0, z: 0 }), air);
    assert_eq!(level.get_block(BlockPos { x: 0, y: 1, z: 0 }), bedrock);
    assert!(level.end_gateway_exits.borrow().is_empty());
}

#[test]
fn place_configured_feature_end_gateway_places_structure_when_exit_is_present() {
    let retained = retained_from_json(
        br#"{"type":"minecraft:end_gateway","config":{"exact":true,"exit":[100,50,0]}}"#,
    );
    let mut level = RecordingLevel::admitting();
    let origin = BlockPos { x: 5, y: 10, z: 5 };
    assert!(place_feature(&retained, &mut level, origin));
    let gateway = default_state("minecraft:end_gateway");
    assert_eq!(level.sets.borrow().len(), 45);
    assert_eq!(level.get_block(origin), gateway);
    assert_eq!(
        *level.end_gateway_exits.borrow(),
        [(
            origin,
            BlockPos {
                x: 100,
                y: 50,
                z: 0
            },
            true
        )]
    );
}

fn aged_weeping_vines(age: i32) -> i32 {
    let mut properties = BTreeMap::new();
    properties.insert("age".to_string(), age.to_string());
    vanilla_block_state_id_by_name_and_properties("minecraft:weeping_vines", &properties).unwrap()
}

fn prepare_weeping_vines_ceiling(level: &mut RecordingLevel, origin: BlockPos, ceiling: i32) {
    level
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y + 1, origin.z), ceiling);
}

#[test]
fn place_configured_feature_weeping_vines_requires_air_origin_and_nether_ceiling() {
    let retained = retained_from_json(br#"{"type":"minecraft:weeping_vines","config":{}}"#);
    let netherrack = default_state("minecraft:netherrack");
    let nether_wart = default_state("minecraft:nether_wart_block");
    let plant = default_state("minecraft:weeping_vines_plant");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let allowed_heads: Vec<i32> = (17..=25).map(aged_weeping_vines).collect();

    let mut blocked = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(!place_feature(&retained, &mut blocked, origin));
    assert!(blocked.sets.borrow().is_empty());

    let mut open_air = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut open_air, origin));
    assert!(open_air.sets.borrow().is_empty());

    let mut level = RecordingLevel::admitting();
    prepare_weeping_vines_ceiling(&mut level, origin, netherrack);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert_eq!(sets[0], (origin, nether_wart, UPDATE_CLIENTS));
    assert!(sets.iter().all(|(_, _, flags)| *flags == UPDATE_CLIENTS));
    assert!(sets.iter().all(|(_, state, _)| {
        *state == nether_wart || *state == plant || allowed_heads.contains(state)
    }));
    assert!(sets
        .iter()
        .any(|(_, state, _)| allowed_heads.contains(state) || *state == plant));
    assert_eq!(level.get_block(origin), nether_wart);
}

#[test]
fn place_configured_feature_weeping_vines_accepts_nether_wart_ceiling() {
    let retained = retained_from_json(br#"{"type":"minecraft:weeping_vines","config":{}}"#);
    let nether_wart = default_state("minecraft:nether_wart_block");
    let origin = BlockPos { x: 2, y: 40, z: -4 };
    let mut level = RecordingLevel::admitting();
    prepare_weeping_vines_ceiling(&mut level, origin, nether_wart);
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(level.get_block(origin), nether_wart);
    assert!(!level.sets.borrow().is_empty());
}

fn twisting_retained() -> ConfiguredFeature {
    retained_from_json(
        br#"{"type":"minecraft:twisting_vines","config":{"spread_width":4,"spread_height":2,"max_height":8}}"#,
    )
}

fn aged_twisting_vines(age: i32) -> i32 {
    let mut properties = BTreeMap::new();
    properties.insert("age".to_string(), age.to_string());
    vanilla_block_state_id_by_name_and_properties("minecraft:twisting_vines", &properties).unwrap()
}

fn twisting_level_with_floor(floor: i32, origin: BlockPos) -> RecordingLevel {
    let air = default_state("minecraft:air");
    let level = RecordingLevel::filled(floor);
    for y in origin.y..=origin.y + 32 {
        for dx in -8..=8 {
            for dz in -8..=8 {
                level
                    .cells
                    .borrow_mut()
                    .insert((origin.x + dx, y, origin.z + dz), air);
            }
        }
    }
    level
}

#[test]
fn place_configured_feature_twisting_vines_requires_air_origin_and_nether_floor() {
    let retained = twisting_retained();
    let netherrack = default_state("minecraft:netherrack");
    let plant = default_state("minecraft:twisting_vines_plant");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let allowed_heads: Vec<i32> = (17..=25).map(aged_twisting_vines).collect();

    let mut blocked = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(!place_feature(&retained, &mut blocked, origin));
    assert!(blocked.sets.borrow().is_empty());

    let mut open_air = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut open_air, origin));
    assert!(open_air.sets.borrow().is_empty());

    let mut level = twisting_level_with_floor(netherrack, origin);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(_, _, flags)| *flags == UPDATE_CLIENTS));
    assert!(sets.iter().all(|(pos, state, _)| {
        pos.y >= origin.y && (*state == plant || allowed_heads.contains(state))
    }));
}

#[test]
fn place_configured_feature_twisting_vines_accepts_warped_floors() {
    let retained = twisting_retained();
    let origin = BlockPos { x: 2, y: 40, z: -4 };
    for floor in [
        default_state("minecraft:warped_nylium"),
        default_state("minecraft:warped_wart_block"),
    ] {
        let mut level = twisting_level_with_floor(floor, origin);
        assert!(place_feature(&retained, &mut level, origin));
        assert!(!level.sets.borrow().is_empty());
    }
}

#[test]
fn place_configured_feature_twisting_vines_skips_attempts_outside_build_height() {
    let retained = twisting_retained();
    let netherrack = default_state("minecraft:netherrack");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    level.min_y = origin.y;
    level
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y - 1, origin.z), netherrack);
    assert!(place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}

pub fn basalt_state() -> i32 {
    let mut properties = BTreeMap::new();
    properties.insert("axis".to_string(), "y".to_string());
    vanilla_block_state_id_by_name_and_properties("minecraft:basalt", &properties).unwrap()
}

fn prepare_basalt_pillar_shaft(level: &mut RecordingLevel, origin: BlockPos, floor_y: i32) {
    let stone = default_state("minecraft:stone");
    level
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y + 1, origin.z), stone);
    level
        .cells
        .borrow_mut()
        .insert((origin.x, floor_y, origin.z), stone);
}

#[test]
fn place_configured_feature_basalt_pillar_requires_air_origin_under_ceiling() {
    let retained = retained_from_json(br#"{"type":"minecraft:basalt_pillar","config":{}}"#);
    let basalt = basalt_state();
    let origin = BlockPos { x: 0, y: 64, z: 0 };

    let mut blocked = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(!place_feature(&retained, &mut blocked, origin));
    assert!(blocked.sets.borrow().is_empty());

    let mut open_air = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut open_air, origin));
    assert!(open_air.sets.borrow().is_empty());

    let mut level = RecordingLevel::admitting();
    prepare_basalt_pillar_shaft(&mut level, origin, 60);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets
        .iter()
        .all(|(_, state, flags)| { *state == basalt && *flags == UPDATE_CLIENTS }));
    assert_eq!(level.get_block(origin), basalt);
    assert_eq!(
        level.get_block(BlockPos {
            x: origin.x,
            y: 61,
            z: origin.z
        }),
        basalt
    );
}

#[test]
fn place_configured_feature_basalt_pillar_stops_at_outside_build_height() {
    let retained = retained_from_json(br#"{"type":"minecraft:basalt_pillar","config":{}}"#);
    let origin = BlockPos { x: 0, y: 0, z: 0 };
    let mut level = RecordingLevel::admitting();
    level.min_y = 0;
    level.max_y = 16;
    level.cells.borrow_mut().insert(
        (origin.x, origin.y + 1, origin.z),
        default_state("minecraft:stone"),
    );
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(level.get_block(origin), basalt_state());
    assert!(level
        .sets
        .borrow()
        .iter()
        .all(|(pos, _, _)| pos.y >= level.min_y));
}

pub fn still_water() -> i32 {
    let mut properties = BTreeMap::new();
    properties.insert("level".to_string(), "0".to_string());
    vanilla_block_state_id_by_name_and_properties("minecraft:water", &properties).unwrap()
}

fn prepare_blue_ice_pocket(level: &mut RecordingLevel, origin: BlockPos) {
    let water = still_water();
    let packed_ice = default_state("minecraft:packed_ice");
    level
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z), water);
    level
        .cells
        .borrow_mut()
        .insert((origin.x + 1, origin.y, origin.z), packed_ice);
}

#[test]
fn place_configured_feature_blue_ice_requires_low_water_and_packed_ice() {
    let retained = retained_from_json(br#"{"type":"minecraft:blue_ice","config":{}}"#);
    let blue_ice = default_state("minecraft:blue_ice");
    let origin = BlockPos { x: 0, y: 60, z: 0 };

    let mut high = RecordingLevel::admitting();
    prepare_blue_ice_pocket(&mut high, origin);
    assert!(!place_feature(
        &retained,
        &mut high,
        BlockPos { x: 0, y: 63, z: 0 }
    ));
    assert!(high.sets.borrow().is_empty());

    let mut dry = RecordingLevel::admitting();
    dry.cells.borrow_mut().insert(
        (origin.x + 1, origin.y, origin.z),
        default_state("minecraft:packed_ice"),
    );
    assert!(!place_feature(&retained, &mut dry, origin));
    assert!(dry.sets.borrow().is_empty());

    let mut no_ice = RecordingLevel::admitting();
    no_ice
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z), still_water());
    assert!(!place_feature(&retained, &mut no_ice, origin));
    assert!(no_ice.sets.borrow().is_empty());

    let mut level = RecordingLevel::admitting();
    prepare_blue_ice_pocket(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert_eq!(sets[0], (origin, blue_ice, UPDATE_CLIENTS));
    assert!(sets
        .iter()
        .all(|(_, state, flags)| *state == blue_ice && *flags == UPDATE_CLIENTS));
}

#[test]
fn place_configured_feature_blue_ice_accepts_water_below_origin() {
    let retained = retained_from_json(br#"{"type":"minecraft:blue_ice","config":{}}"#);
    let origin = BlockPos { x: 2, y: 50, z: -4 };
    let mut level = RecordingLevel::admitting();
    level
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y - 1, origin.z), still_water());
    level.cells.borrow_mut().insert(
        (origin.x, origin.y + 1, origin.z),
        default_state("minecraft:packed_ice"),
    );
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(level.get_block(origin), default_state("minecraft:blue_ice"));
}

fn replace_blobs_constant(radius: i32) -> ConfiguredFeature {
    let json = format!(
        r#"{{"type":"minecraft:netherrack_replace_blobs","config":{{"target":{{"Name":"netherrack"}},"state":{{"Name":"basalt","Properties":{{"axis":"y"}}}},"radius":{radius}}}}}"#
    );
    retained_from_json(json.as_bytes())
}

#[test]
fn place_configured_feature_netherrack_replace_blobs_replaces_target_block() {
    let retained = replace_blobs_constant(1);
    let netherrack = default_state("minecraft:netherrack");
    let basalt = basalt_state();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut missing = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(!place_feature(&retained, &mut missing, origin));
    assert!(missing.sets.borrow().is_empty());

    let mut level = RecordingLevel::filled(netherrack);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(_, state, flags)| {
        *state == basalt && *flags == UPDATE_NEIGHBORS | UPDATE_CLIENTS
    }));
    assert_eq!(level.get_block(origin), basalt);
    assert_eq!(
        level.get_block(BlockPos {
            x: origin.x + 1,
            y: origin.y,
            z: origin.z
        }),
        basalt
    );
}

pub fn still_lava() -> i32 {
    let mut properties = BTreeMap::new();
    properties.insert("level".to_string(), "0".to_string());
    vanilla_block_state_id_by_name_and_properties("minecraft:lava", &properties).unwrap()
}

fn delta_retained_constant(size: i32, rim_size: i32) -> ConfiguredFeature {
    let json = format!(
        r#"{{"type":"minecraft:delta_feature","config":{{"contents":{{"Name":"lava","Properties":{{"level":"0"}}}},"rim":{{"Name":"magma_block"}},"size":{size},"rim_size":{rim_size}}}}}"#
    );
    retained_from_json(json.as_bytes())
}

fn prepare_delta_bowl(level: &mut RecordingLevel, origin: BlockPos) {
    let air = default_state("minecraft:air");
    for dx in -16..=16 {
        for dz in -16..=16 {
            level
                .cells
                .borrow_mut()
                .insert((origin.x + dx, origin.y + 1, origin.z + dz), air);
        }
    }
}

#[test]
fn place_configured_feature_delta_feature_places_contents_in_a_bowl() {
    let retained = delta_retained_constant(2, 0);
    let lava = still_lava();
    let origin = BlockPos { x: 0, y: 64, z: 0 };

    let mut open_air = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut open_air, origin));
    assert!(open_air.sets.borrow().is_empty());

    let mut forbidden = RecordingLevel::filled(default_state("minecraft:bedrock"));
    prepare_delta_bowl(&mut forbidden, origin);
    assert!(!place_feature(&retained, &mut forbidden, origin));
    assert!(forbidden.sets.borrow().is_empty());

    let mut level = RecordingLevel::filled(default_state("minecraft:netherrack"));
    prepare_delta_bowl(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(_, state, flags)| {
        *state == lava && *flags == UPDATE_NEIGHBORS | UPDATE_CLIENTS
    }));
    assert_eq!(level.get_block(origin), lava);
}

fn replace_single_block_json(target_json: &str, state_name: &str) -> ConfiguredFeature {
    let json = format!(
        r#"{{"type":"minecraft:replace_single_block","config":{{"targets":[{{"target":{target_json},"state":{{"Name":"{state_name}"}}}}]}}}}"#
    );
    retained_from_json(json.as_bytes())
}

#[test]
fn place_configured_feature_replace_single_block_always_true_replaces_origin() {
    let retained =
        replace_single_block_json(r#"{"predicate_type":"always_true"}"#, "diamond_block");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let diamond = default_state("minecraft:diamond_block");
    let mut level = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(*level.sets.borrow(), [(origin, diamond, UPDATE_CLIENTS)]);
    assert_eq!(level.get_block(origin), diamond);
}

#[test]
fn place_configured_feature_replace_single_block_block_match_is_selective() {
    let retained = replace_single_block_json(
        r#"{"predicate_type":"block_match","block":"stone"}"#,
        "gold_block",
    );
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let gold = default_state("minecraft:gold_block");

    let mut dirt = RecordingLevel::filled(default_state("minecraft:dirt"));
    assert!(place_feature(&retained, &mut dirt, origin));
    assert!(dirt.sets.borrow().is_empty());

    let mut stone = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(place_feature(&retained, &mut stone, origin));
    assert_eq!(stone.get_block(origin), gold);
}

#[test]
fn place_configured_feature_replace_single_block_random_match_is_probability_gated() {
    let never = replace_single_block_json(
        r#"{"predicate_type":"random_block_match","block":"stone","probability":0.0}"#,
        "gold_block",
    );
    let always = replace_single_block_json(
        r#"{"predicate_type":"random_block_match","block":"stone","probability":1.0}"#,
        "gold_block",
    );
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let gold = default_state("minecraft:gold_block");

    let mut closed = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(place_feature(&never, &mut closed, origin));
    assert!(closed.sets.borrow().is_empty());

    let mut open = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(place_feature(&always, &mut open, origin));
    assert_eq!(open.get_block(origin), gold);
}

fn pointed_dripstone_retained() -> ConfiguredFeature {
    retained_from_json(br#"{"type":"minecraft:pointed_dripstone","config":{}}"#)
}

fn pointed_dripstone_state_for_test(direction: &str, thickness: &str, waterlogged: bool) -> i32 {
    let mut properties = BTreeMap::new();
    properties.insert("vertical_direction".to_string(), direction.to_string());
    properties.insert("thickness".to_string(), thickness.to_string());
    properties.insert(
        "waterlogged".to_string(),
        if waterlogged { "true" } else { "false" }.to_string(),
    );
    vanilla_block_state_id_by_name_and_properties("minecraft:pointed_dripstone", &properties)
        .unwrap()
}

#[test]
fn place_configured_feature_pointed_dripstone_requires_a_dripstone_base() {
    let retained = pointed_dripstone_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut air = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut air, origin));
    assert!(air.sets.borrow().is_empty());
}

/// `DripstoneUtils.isDripstoneBase` (`DripstoneUtils.java:113-115`) reads the
/// live `minecraft:dripstone_replaceable_blocks`, so a snapshot moves the floor.
#[test]
fn place_configured_feature_pointed_dripstone_follows_the_live_replaceable_tag() {
    let retained = pointed_dripstone_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let dirt = default_state("minecraft:dirt");
    let stone = default_state("minecraft:stone");

    let mut refused = RecordingLevel::admitting();
    refused
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y - 1, origin.z), dirt);
    assert!(!place_feature(&retained, &mut refused, origin));
    assert!(refused.sets.borrow().is_empty());

    let mut admitted = RecordingLevel::admitting();
    admitted
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y - 1, origin.z), dirt);
    admitted.set_block_tag_membership(DRIPSTONE_REPLACEABLE_TAG, "minecraft:dirt", true);
    assert!(place_feature(&retained, &mut admitted, origin));
    assert!(admitted
        .sets
        .borrow()
        .iter()
        .any(|(_, state, _)| block_is(*state, "minecraft:pointed_dripstone")));

    let mut excluded = RecordingLevel::admitting();
    excluded
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y - 1, origin.z), stone);
    excluded.set_block_tag_membership(DRIPSTONE_REPLACEABLE_TAG, "minecraft:stone", false);
    assert!(!place_feature(&retained, &mut excluded, origin));
    assert!(excluded.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_pointed_dripstone_grows_up_from_replaceable_floor() {
    let retained = pointed_dripstone_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let stone = default_state("minecraft:stone");
    let dripstone_block = default_state("minecraft:dripstone_block");
    let mut level = RecordingLevel::admitting();
    level
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y - 1, origin.z), stone);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(_, _, flags)| *flags == UPDATE_CLIENTS));
    assert_eq!(
        level.get_block(BlockPos {
            x: origin.x,
            y: origin.y - 1,
            z: origin.z
        }),
        dripstone_block
    );
    assert!(block_is(
        level.get_block(origin),
        "minecraft:pointed_dripstone"
    ));
}

#[test]
fn place_configured_feature_pointed_dripstone_waterlogs_in_water() {
    let retained = pointed_dripstone_retained();
    let origin = BlockPos { x: 0, y: 40, z: 0 };
    let mut level = RecordingLevel::admitting();
    level
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z), still_water());
    level.cells.borrow_mut().insert(
        (origin.x, origin.y - 1, origin.z),
        default_state("minecraft:stone"),
    );
    assert!(place_feature(&retained, &mut level, origin));
    let origin_state = level.get_block(origin);
    assert!(
        origin_state == pointed_dripstone_state_for_test("up", "tip", true)
            || origin_state == pointed_dripstone_state_for_test("up", "frustum", true)
    );
}

fn scattered_ore_json(target_json: &str, state_name: &str, size: i32, discard: f32) -> String {
    format!(
        r#"{{"type":"minecraft:scattered_ore","config":{{"targets":[{{"target":{target_json},"state":{{"Name":"{state_name}"}}}}],"size":{size},"discard_chance_on_air_exposure":{discard}}}}}"#
    )
}

fn scattered_ore_retained(
    target_json: &str,
    state_name: &str,
    size: i32,
    discard: f32,
) -> ConfiguredFeature {
    retained_from_json(scattered_ore_json(target_json, state_name, size, discard).as_bytes())
}

#[test]
fn place_configured_feature_scattered_ore_always_true_replaces_origin() {
    let retained =
        scattered_ore_retained(r#"{"predicate_type":"always_true"}"#, "diamond_ore", 1, 0.0);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let diamond = default_state("minecraft:diamond_ore");
    let mut level = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(*level.sets.borrow(), [(origin, diamond, UPDATE_CLIENTS)]);
    assert_eq!(level.get_block(origin), diamond);
}

#[test]
fn place_configured_feature_scattered_ore_block_match_is_selective() {
    let retained = scattered_ore_retained(
        r#"{"predicate_type":"block_match","block":"stone"}"#,
        "gold_block",
        1,
        0.0,
    );
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let gold = default_state("minecraft:gold_block");

    let mut dirt = RecordingLevel::filled(default_state("minecraft:dirt"));
    assert!(place_feature(&retained, &mut dirt, origin));
    assert!(dirt.sets.borrow().is_empty());

    let mut stone = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(place_feature(&retained, &mut stone, origin));
    assert_eq!(stone.get_block(origin), gold);
}

#[test]
fn place_configured_feature_scattered_ore_discards_air_adjacent_when_chance_is_one() {
    let always =
        scattered_ore_retained(r#"{"predicate_type":"always_true"}"#, "diamond_ore", 1, 0.0);
    let never =
        scattered_ore_retained(r#"{"predicate_type":"always_true"}"#, "diamond_ore", 1, 1.0);
    let origin = BlockPos { x: 0, y: 40, z: 0 };
    let stone = default_state("minecraft:stone");
    let diamond = default_state("minecraft:diamond_ore");

    let mut exposed_never = RecordingLevel::admitting();
    exposed_never
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z), stone);
    assert!(place_feature(&never, &mut exposed_never, origin));
    assert!(exposed_never.sets.borrow().is_empty());

    let mut exposed_always = RecordingLevel::admitting();
    exposed_always
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z), stone);
    assert!(place_feature(&always, &mut exposed_always, origin));
    assert_eq!(exposed_always.get_block(origin), diamond);
}

fn ore_json(target_json: &str, state_name: &str, size: i32, discard: f32) -> String {
    format!(
        r#"{{"type":"minecraft:ore","config":{{"targets":[{{"target":{target_json},"state":{{"Name":"{state_name}"}}}}],"size":{size},"discard_chance_on_air_exposure":{discard}}}}}"#
    )
}

fn ore_retained(target_json: &str, state_name: &str, size: i32, discard: f32) -> ConfiguredFeature {
    retained_from_json(ore_json(target_json, state_name, size, discard).as_bytes())
}

#[test]
fn place_configured_feature_ore_always_true_writes_update_none_blob() {
    let retained = ore_retained(r#"{"predicate_type":"always_true"}"#, "diamond_ore", 9, 0.0);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let diamond = default_state("minecraft:diamond_ore");
    let mut level = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets
        .iter()
        .all(|(_, state, flags)| *state == diamond && *flags == UPDATE_NONE));
}

#[test]
fn place_configured_feature_ore_tag_match_replaces_stone_ore_replaceables() {
    let retained = retained_from_json(
        br#"{
            "type":"minecraft:ore",
            "config":{
                "discard_chance_on_air_exposure":0.0,
                "size":9,
                "targets":[
                    {
                        "state":{"Name":"minecraft:iron_ore"},
                        "target":{
                            "predicate_type":"minecraft:tag_match",
                            "tag":"minecraft:stone_ore_replaceables"
                        }
                    },
                    {
                        "state":{"Name":"minecraft:deepslate_iron_ore"},
                        "target":{
                            "predicate_type":"minecraft:tag_match",
                            "tag":"minecraft:deepslate_ore_replaceables"
                        }
                    }
                ]
            }
        }"#,
    );
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let iron = default_state("minecraft:iron_ore");

    let mut dirt = RecordingLevel::filled(default_state("minecraft:dirt"));
    assert!(!place_feature(&retained, &mut dirt, origin));
    assert!(dirt.sets.borrow().is_empty());

    let mut stone = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(place_feature(&retained, &mut stone, origin));
    let sets = stone.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets
        .iter()
        .all(|(_, state, flags)| *state == iron && *flags == UPDATE_NONE));
}

#[test]
fn place_configured_feature_ore_discards_air_adjacent_when_chance_is_one() {
    let always = ore_retained(r#"{"predicate_type":"always_true"}"#, "diamond_ore", 9, 0.0);
    let never = ore_retained(r#"{"predicate_type":"always_true"}"#, "diamond_ore", 9, 1.0);
    let origin = BlockPos { x: 0, y: 40, z: 0 };
    let stone = default_state("minecraft:stone");
    let diamond = default_state("minecraft:diamond_ore");

    let mut exposed_never = RecordingLevel::admitting();
    exposed_never
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z), stone);
    assert!(!place_feature(&never, &mut exposed_never, origin));
    assert!(exposed_never.sets.borrow().is_empty());

    let mut exposed_always = RecordingLevel::admitting();
    exposed_always
        .cells
        .borrow_mut()
        .insert((origin.x, origin.y, origin.z), stone);
    assert!(place_feature(&always, &mut exposed_always, origin));
    assert!(exposed_always
        .sets
        .borrow()
        .iter()
        .any(|(_, state, _)| *state == diamond));
}

#[test]
fn place_configured_feature_ore_skips_when_ocean_floor_wg_is_below_blob() {
    let retained = ore_retained(r#"{"predicate_type":"always_true"}"#, "diamond_ore", 9, 0.0);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::filled(default_state("minecraft:stone"));
    level.ocean_floor_wg_height = Some(i32::MIN);
    assert!(!place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}

fn coral_retained(type_id: &str) -> ConfiguredFeature {
    retained_from_json(format!(r#"{{"type":"{type_id}","config":{{}}}}"#).as_bytes())
}

#[test]
fn place_configured_feature_coral_tree_returns_true_on_air_without_mutating() {
    let retained = coral_retained("minecraft:coral_tree");
    let mut air = RecordingLevel::admitting();
    assert!(place_feature(&retained, &mut air, origin()));
    assert!(air.sets.borrow().is_empty());
}

#[test]
fn place_configured_feature_coral_tree_places_coral_block_in_water() {
    let retained = coral_retained("minecraft:coral_tree");
    let origin = BlockPos { x: 0, y: 40, z: 0 };
    let mut level = RecordingLevel::filled(still_water());
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert_eq!(sets[0].0, origin);
    assert_eq!(sets[0].2, FEATURE_SET_BLOCK);
    assert!(block_is_in_builtin_tag(sets[0].1, CORAL_BLOCKS_TAG));
}

/// `CoralFeature.place` (`CoralFeature.java:30`) indexes the live
/// `HolderSet.Named` contents, so the generation's tag order picks the block.
#[test]
fn place_configured_feature_coral_tree_draws_by_index_into_the_live_tag_order() {
    let retained = coral_retained("minecraft:coral_tree");
    let origin = BlockPos { x: 0, y: 40, z: 0 };
    let builtin = builtin_registry_tag_members("minecraft:block", CORAL_BLOCKS_TAG);
    // Rotating by one leaves no index answering the built-in catalog order.
    let mut rotated: Vec<String> = builtin.iter().map(|id| id.to_string()).collect();
    rotated.rotate_left(1);
    for (index, member) in rotated.iter().enumerate() {
        assert_ne!(member, &builtin[index]);
    }
    for seed in 0..8 {
        let mut oracle = WorldgenRandom::with_seed(seed);
        let index = oracle.next_int(rotated.len() as i32) as usize;
        let mut level = RecordingLevel::filled(still_water());
        level.set_block_tag_members(CORAL_BLOCKS_TAG, rotated.clone());
        let mut random = WorldgenRandom::with_seed(seed);
        assert!(place_configured_feature(
            &retained,
            &mut level,
            origin,
            &mut random
        ));
        assert_eq!(level.sets.borrow()[0].1, default_state(&rotated[index]));
    }
}

fn spring_retained() -> ConfiguredFeature {
    retained_from_json(
        br#"{"type":"minecraft:spring_feature","config":{"state":{"Name":"minecraft:water"},"valid_blocks":["minecraft:stone"]}}"#,
    )
}

fn prepare_spring_bowl(level: &mut RecordingLevel, origin: BlockPos) {
    let stone = default_state("minecraft:stone");
    let mut cells = level.cells.borrow_mut();
    cells.insert((origin.x, origin.y + 1, origin.z), stone);
    cells.insert((origin.x, origin.y - 1, origin.z), stone);
    cells.insert((origin.x - 1, origin.y, origin.z), stone);
    cells.insert((origin.x + 1, origin.y, origin.z), stone);
    cells.insert((origin.x, origin.y, origin.z - 1), stone);
}

#[test]
fn place_configured_feature_spring_feature_places_source_water_and_schedules_a_tick() {
    let retained = spring_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_spring_bowl(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(
        *level.sets.borrow(),
        [(origin, still_water(), UPDATE_CLIENTS)]
    );
    assert_eq!(*level.fluid_ticks.borrow(), [(origin, 0)]);
}

#[test]
fn place_configured_feature_spring_feature_rejects_wrong_rock_count() {
    let retained = spring_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_spring_bowl(&mut level, origin);
    level.cells.borrow_mut().insert(
        (origin.x, origin.y, origin.z + 1),
        default_state("minecraft:stone"),
    );
    assert!(!place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
    assert!(level.fluid_ticks.borrow().is_empty());
}

#[test]
fn place_configured_feature_spring_feature_requires_ceiling_valid_block() {
    let retained = spring_retained();
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_spring_bowl(&mut level, origin);
    level
        .cells
        .borrow_mut()
        .remove(&(origin.x, origin.y + 1, origin.z));
    assert!(!place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}

fn block_blob_json(predicate_json: &str) -> String {
    format!(
        r#"{{"type":"minecraft:block_blob","config":{{"state":{{"Name":"mossy_cobblestone"}},"can_place_on":{predicate_json}}}}}"#
    )
}

fn block_blob_retained(predicate_json: &str) -> ConfiguredFeature {
    retained_from_json(block_blob_json(predicate_json).as_bytes())
}

#[test]
fn place_configured_feature_block_blob_true_predicate_places_mossy_cobble() {
    let retained = block_blob_retained(r#"{"type":"true"}"#);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mossy = default_state("minecraft:mossy_cobblestone");
    let mut level = RecordingLevel::admitting();
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets
        .iter()
        .all(|(_, state, flags)| { *state == mossy && *flags == FEATURE_SET_BLOCK }));
    assert!(block_is(
        level.get_block(origin),
        "minecraft:mossy_cobblestone"
    ));
}

#[test]
fn place_configured_feature_block_blob_matching_blocks_walks_down_to_stone() {
    let retained =
        block_blob_retained(r#"{"type":"matching_blocks","blocks":["minecraft:stone"]}"#);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let stone = default_state("minecraft:stone");
    let mossy = default_state("minecraft:mossy_cobblestone");
    let mut level = RecordingLevel::admitting();
    level
        .cells
        .borrow_mut()
        .insert((origin.x, 60, origin.z), stone);
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(
        level.get_block(BlockPos { x: 0, y: 64, z: 0 }),
        default_state("minecraft:air")
    );
    assert_eq!(level.get_block(BlockPos { x: 0, y: 61, z: 0 }), mossy);
}

fn spike_retained(place_json: &str, replace_json: &str) -> ConfiguredFeature {
    let json = format!(
        r#"{{"type":"minecraft:spike","config":{{"state":{{"Name":"packed_ice"}},"can_place_on":{place_json},"can_replace":{replace_json}}}}}"#
    );
    retained_from_json(json.as_bytes())
}

#[test]
fn place_configured_feature_spike_true_predicate_places_packed_ice() {
    let retained = spike_retained(r#"{"type":"true"}"#, r#"{"type":"true"}"#);
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let ice = default_state("minecraft:packed_ice");
    let mut level = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets
        .iter()
        .all(|(_, state, flags)| { *state == ice && *flags == FEATURE_SET_BLOCK }));
    assert!(sets
        .iter()
        .any(|(pos, _, _)| pos.x == origin.x && pos.z == origin.z));
}

#[test]
fn place_configured_feature_spike_matching_blocks_requires_a_landing_block() {
    let retained = spike_retained(
        r#"{"type":"matching_blocks","blocks":["minecraft:stone"]}"#,
        r#"{"type":"true"}"#,
    );
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut air = RecordingLevel::admitting();
    assert!(!place_feature(&retained, &mut air, origin));
    assert!(air.sets.borrow().is_empty());

    let mut stone = RecordingLevel::filled(default_state("minecraft:stone"));
    assert!(place_feature(&retained, &mut stone, origin));
    assert!(
        block_is(stone.get_block(origin), "minecraft:packed_ice")
            || stone.sets.borrow().iter().any(|(pos, state, _)| {
                pos.x == origin.x && pos.z == origin.z && block_is(*state, "minecraft:packed_ice")
            })
    );
}

fn huge_mushroom_json(type_id: &str) -> String {
    format!(
        r#"{{"type":"{type_id}","config":{{"cap_provider":{{"type":"simple_state_provider","state":{{"Name":"brown_mushroom_block"}}}},"stem_provider":{{"type":"simple_state_provider","state":{{"Name":"mushroom_stem"}}}},"can_place_on":{{"type":"true"}}}}}}"#
    )
}

fn huge_mushroom_retained(type_id: &str) -> ConfiguredFeature {
    retained_from_json(huge_mushroom_json(type_id).as_bytes())
}

fn prepare_mushroom_floor(level: &mut RecordingLevel, origin: BlockPos) {
    level.cells.borrow_mut().insert(
        (origin.x, origin.y - 1, origin.z),
        default_state("minecraft:stone"),
    );
}

#[test]
fn place_configured_feature_huge_brown_mushroom_places_stem_and_cap() {
    let retained = huge_mushroom_retained("minecraft:huge_brown_mushroom");
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_mushroom_floor(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    let sets = level.sets.borrow().clone();
    assert!(!sets.is_empty());
    assert!(sets.iter().all(|(_, _, flags)| *flags == FEATURE_SET_BLOCK));
    assert!(block_is(level.get_block(origin), "minecraft:mushroom_stem"));
    assert!(sets
        .iter()
        .any(|(_, state, _)| block_is(*state, "minecraft:brown_mushroom_block")));
}

#[test]
fn place_configured_feature_huge_red_mushroom_places_stem_and_cap() {
    let retained = huge_mushroom_json("minecraft:huge_red_mushroom");
    let retained = retained_from_json(retained.as_bytes());
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_mushroom_floor(&mut level, origin);
    assert!(place_feature(&retained, &mut level, origin));
    assert!(block_is(level.get_block(origin), "minecraft:mushroom_stem"));
    assert!(level.sets.borrow().iter().any(|(_, state, flags)| {
        *flags == FEATURE_SET_BLOCK && block_is(*state, "minecraft:brown_mushroom_block")
    }));
}

#[test]
fn place_configured_feature_huge_mushroom_requires_can_place_on() {
    let json = r#"{"type":"minecraft:huge_brown_mushroom","config":{"cap_provider":{"type":"simple_state_provider","state":{"Name":"brown_mushroom_block"}},"stem_provider":{"type":"simple_state_provider","state":{"Name":"mushroom_stem"}},"can_place_on":{"type":"matching_blocks","blocks":["minecraft:dirt"]}}}"#;
    let retained = retained_from_json(json.as_bytes());
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let mut level = RecordingLevel::admitting();
    prepare_mushroom_floor(&mut level, origin);
    assert!(!place_feature(&retained, &mut level, origin));
    assert!(level.sets.borrow().is_empty());
}

fn block_column_json(
    layers_json: &str,
    direction: &str,
    placement_json: &str,
    prioritize_tip: bool,
) -> String {
    format!(
        r#"{{"type":"minecraft:block_column","config":{{"layers":{layers_json},"direction":"{direction}","allowed_placement":{placement_json},"prioritize_tip":{prioritize_tip}}}}}"#
    )
}

fn block_column_retained(
    layers_json: &str,
    direction: &str,
    placement_json: &str,
    prioritize_tip: bool,
) -> ConfiguredFeature {
    retained_from_json(
        block_column_json(layers_json, direction, placement_json, prioritize_tip).as_bytes(),
    )
}

#[test]
fn place_configured_feature_block_column_places_upward_layers() {
    let retained = block_column_retained(
        r#"[{"height":3,"provider":{"type":"simple_state_provider","state":{"Name":"stone"}}}]"#,
        "up",
        r#"{"type":"true"}"#,
        false,
    );
    let origin = BlockPos { x: 0, y: 64, z: 0 };
    let stone = default_state("minecraft:stone");
    let mut level = RecordingLevel::admitting();
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(
        *level.sets.borrow(),
        [
            (origin, stone, UPDATE_CLIENTS),
            (
                BlockPos {
                    x: origin.x,
                    y: origin.y + 1,
                    z: origin.z
                },
                stone,
                UPDATE_CLIENTS
            ),
            (
                BlockPos {
                    x: origin.x,
                    y: origin.y + 2,
                    z: origin.z
                },
                stone,
                UPDATE_CLIENTS
            ),
        ]
    );
}

#[test]
fn place_configured_feature_block_column_prioritize_tip_keeps_the_upper_layer() {
    let layers = r#"[{"height":2,"provider":{"type":"simple_state_provider","state":{"Name":"stone"}}},{"height":2,"provider":{"type":"simple_state_provider","state":{"Name":"dirt"}}}]"#;
    let origin = BlockPos { x: 0, y: 40, z: 0 };
    let dirt = default_state("minecraft:dirt");
    let stone = default_state("minecraft:stone");
    let blocker = BlockPos {
        x: origin.x,
        y: origin.y + 3,
        z: origin.z,
    };

    let mut tip = RecordingLevel::admitting();
    tip.cells
        .borrow_mut()
        .insert((blocker.x, blocker.y, blocker.z), stone);
    let retained_tip = block_column_retained(
        layers,
        "up",
        r#"{"type":"matching_block_tag","tag":"minecraft:air"}"#,
        true,
    );
    assert!(place_feature(&retained_tip, &mut tip, origin));
    assert_eq!(
        *tip.sets.borrow(),
        [
            (origin, dirt, UPDATE_CLIENTS),
            (
                BlockPos {
                    x: origin.x,
                    y: origin.y + 1,
                    z: origin.z
                },
                dirt,
                UPDATE_CLIENTS
            ),
        ]
    );

    let mut base = RecordingLevel::admitting();
    base.cells
        .borrow_mut()
        .insert((blocker.x, blocker.y, blocker.z), stone);
    let retained_base = block_column_retained(
        layers,
        "up",
        r#"{"type":"matching_block_tag","tag":"minecraft:air"}"#,
        false,
    );
    assert!(place_feature(&retained_base, &mut base, origin));
    assert_eq!(
        *base.sets.borrow(),
        [
            (origin, stone, UPDATE_CLIENTS),
            (
                BlockPos {
                    x: origin.x,
                    y: origin.y + 1,
                    z: origin.z
                },
                stone,
                UPDATE_CLIENTS
            ),
        ]
    );
}

#[test]
fn place_configured_feature_block_column_evaluates_all_block_predicate_variants() {
    let retained = block_column_retained(
        r#"[{"height":3,"provider":{"type":"simple_state_provider","state":{"Name":"minecraft:big_dripleaf_stem","Properties":{"facing":"north","waterlogged":"false"}}}}]"#,
        "up",
        r#"{"type":"minecraft:any_of","predicates":[{"type":"minecraft:matching_blocks","blocks":"minecraft:air"},{"type":"minecraft:matching_fluids","fluids":"minecraft:water"}]}"#,
        false,
    );
    let origin = BlockPos { x: 0, y: 20, z: 0 };
    let mut level = RecordingLevel::admitting();
    // `BlockColumnFeature` checks `allowed_placement` starting at `origin + direction`.
    level.cells.borrow_mut().insert((0, 21, 0), still_water());
    level
        .cells
        .borrow_mut()
        .insert((0, 23, 0), default_state("minecraft:stone"));
    assert!(place_feature(&retained, &mut level, origin));
    assert_eq!(level.sets.borrow().len(), 2);
    assert_eq!(level.sets.borrow()[0].0, BlockPos { x: 0, y: 20, z: 0 });
    assert_eq!(level.sets.borrow()[1].0, BlockPos { x: 0, y: 21, z: 0 });
}

#[test]
fn block_state_in_tag_hook_uses_override_and_defaults_to_registry_tag_contains() {
    struct FastTagLevel {
        stone: i32,
        direct_calls: std::cell::Cell<usize>,
    }

    impl WorldGenLevel for FastTagLevel {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
        }
        fn get_block(&self, _pos: BlockPos) -> i32 {
            self.stone
        }
        fn set_block(&mut self, _pos: BlockPos, _state: i32, _update_flags: i32) -> bool {
            true
        }
        fn get_min_y(&self) -> i32 {
            -64
        }
        fn get_max_y(&self) -> i32 {
            319
        }
        fn get_sea_level(&self) -> i32 {
            63
        }
        fn schedule_fluid_tick(&mut self, _pos: BlockPos, _delay: i32) {}
        fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}
        fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}
        fn registry_tag_contains(&self, _r: &str, _t: &str, _e: &str) -> bool {
            panic!("overridden block_state_in_tag must bypass registry_tag_contains");
        }
        fn block_state_in_tag(&self, state_id: i32, tag_id: &str) -> bool {
            self.direct_calls.set(self.direct_calls.get() + 1);
            state_id == self.stone && tag_id == "minecraft:base_stone_overworld"
        }
    }

    let stone = default_state("minecraft:stone");
    let predicate = bbb_server_datapack::worldgen::BlockPredicate::MatchingBlockTag {
        offset: (0, 0, 0),
        tag: "minecraft:base_stone_overworld".to_string(),
    };
    let fast = FastTagLevel {
        stone,
        direct_calls: std::cell::Cell::new(0),
    };
    assert_eq!(
        block_predicates::block_predicate_test(&predicate, &fast, BlockPos { x: 0, y: 0, z: 0 }),
        Some(true)
    );
    assert_eq!(fast.direct_calls.get(), 1);

    let mut fallback = RecordingLevel::admitting();
    fallback.set_block_tag_membership("minecraft:custom_tag", "minecraft:stone", true);
    assert!(fallback.block_state_in_tag(stone, "minecraft:custom_tag"));
    assert!(!fallback.block_state_in_tag(default_state("minecraft:dirt"), "minecraft:custom_tag"));
}

impl RecordingLevel {
    pub fn set_block_tag_membership(&mut self, tag_id: &str, block_id: &str, membership: bool) {
        self.registry_tag_overrides
            .insert((tag_id.to_string(), block_id.to_string()), membership);
    }
    pub fn set_block_tag_members(&mut self, tag_id: &str, members: Vec<String>) {
        self.registry_tag_member_overrides
            .insert(("minecraft:block".to_string(), tag_id.to_string()), members);
    }
}
