use super::*;
use crate::fluid::with_property;
use crate::registry_tags::RegistryTagSnapshot;
use crate::world::ServerWorldConfig;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COW_ID};
use bbb_server_types::interaction::WorldBorder;
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::save::Java26LevelKey;
use std::sync::Arc;

const BED: BlockPos = BlockPos { x: 8, y: 100, z: 8 };

fn dimension() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}

fn world() -> ServerWorld {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    world
        .level_mut_by_id(&world.require_level_id(&dimension()))
        .chunks
        .try_ensure_served_full_column(0, 0)
        .unwrap();
    tags(&mut world, &[]);
    world
}

fn tags(world: &mut ServerWorld, invalid: &[&str]) {
    let registry = "minecraft:block".to_owned();
    let tags = [
        (
            registry.clone(),
            "minecraft:climbable".to_owned(),
            vec![
                "minecraft:ladder".to_owned(),
                "minecraft:vine".to_owned(),
                "minecraft:scaffolding".to_owned(),
            ],
        ),
        (
            registry.clone(),
            "minecraft:fire".to_owned(),
            vec![
                "minecraft:fire".to_owned(),
                "minecraft:soul_fire".to_owned(),
            ],
        ),
        (
            registry.clone(),
            "minecraft:invalid_spawn_inside".to_owned(),
            invalid.iter().map(|name| (*name).to_owned()).collect(),
        ),
    ];
    world
        .level_mut_by_id(&world.require_level_id(&dimension()))
        .chunks
        .install_registry_tags(Arc::new(RegistryTagSnapshot::new(tags)));
}

fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
    let block = vanilla_block_type_for_resource_id(name).unwrap();
    let mut state = vanilla_block_default_state_id(block.id).unwrap();
    for &(property, value) in properties {
        state = with_property(state, property, value)
            .unwrap_or_else(|| panic!("invalid catalog property {name}[{property}={value}]"));
    }
    state
}

fn set(world: &mut ServerWorld, pos: BlockPos, name: &str, properties: &[(&str, &str)]) {
    assert!(world
        .level_mut_by_id(&world.require_level_id(&dimension()))
        .chunks
        .set_block_state(pos.x, pos.y, pos.z, state(name, properties))
        .is_some());
}

fn stone(world: &mut ServerWorld, x: i32, y: i32, z: i32) {
    set(world, BlockPos { x, y, z }, "minecraft:stone", &[]);
}

fn air(world: &mut ServerWorld, x: i32, y: i32, z: i32) {
    set(world, BlockPos { x, y, z }, "minecraft:air", &[]);
}

fn standing(world: &ServerWorld, facing: Option<Direction>, yaw: f32) -> Option<Vec3d> {
    world
        .sleep_stand_up_position(
            &world.require_level_id(&dimension()),
            BED,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            facing,
            yaw,
        )
        .flatten()
}

fn center(x: i32, y: f64, z: i32) -> Option<Vec3d> {
    Some(Vec3d {
        x: f64::from(x) + 0.5,
        y,
        z: f64::from(z) + 0.5,
    })
}

#[test]
fn sleep_stand_up_ordinary_bed_walks_all_surround_candidates_before_bed_tops() {
    let expected = [
        (9, 8),
        (9, 9),
        (9, 10),
        (8, 10),
        (7, 10),
        (7, 9),
        (7, 8),
        (7, 7),
        (8, 7),
        (9, 7),
    ];
    let mut world = world();
    for &(x, z) in &expected {
        stone(&mut world, x, 99, z);
    }
    for (index, &(x, z)) in expected.iter().enumerate() {
        assert_eq!(
            standing(&world, Some(Direction::North), 0.0),
            center(x, 100.0, z),
            "candidate {index}"
        );
        stone(&mut world, x, 100, z);
    }
    assert_eq!(standing(&world, Some(Direction::North), 0.0), None);
    set(
        &mut world,
        BED,
        "minecraft:red_bed",
        &[("facing", "north"), ("part", "head")],
    );
    assert_eq!(
        standing(&world, Some(Direction::North), 0.0),
        center(8, 100.5625, 8)
    );
    stone(&mut world, 8, 100, 8);
    set(
        &mut world,
        BlockPos { x: 8, y: 100, z: 9 },
        "minecraft:red_bed",
        &[("facing", "north"), ("part", "foot")],
    );
    assert_eq!(
        standing(&world, Some(Direction::North), 0.0),
        center(8, 100.5625, 9)
    );
}

#[test]
fn sleep_stand_up_yaw_selects_the_opposite_side_with_native_cardinal_rounding() {
    let mut world = world();
    for (x, z) in [(9, 8), (7, 8), (8, 9), (8, 7)] {
        stone(&mut world, x, 99, z);
    }
    for (facing, yaw, x, z) in [
        (Direction::North, 0.0, 9, 8),
        (Direction::North, -90.0, 7, 8),
        (Direction::South, 90.0, 9, 8),
        (Direction::East, 0.0, 8, 7),
        (Direction::West, 180.0, 8, 9),
    ] {
        assert_eq!(standing(&world, Some(facing), yaw), center(x, 100.0, z));
    }
}

#[test]
fn sleep_stand_up_bunk_search_visits_lower_surround_before_upper_bed_top() {
    let mut world = world();
    for y in [99, 100] {
        set(
            &mut world,
            BlockPos { x: 8, y, z: 8 },
            "minecraft:red_bed",
            &[("facing", "north")],
        );
    }
    stone(&mut world, 9, 98, 8);
    assert_eq!(
        standing(&world, Some(Direction::North), 0.0),
        center(9, 99.0, 8)
    );
    stone(&mut world, 9, 99, 8);
    assert_eq!(
        standing(&world, Some(Direction::North), 0.0),
        center(9, 100.0, 8)
    );
    stone(&mut world, 9, 100, 8);
    assert_eq!(
        standing(&world, Some(Direction::North), 0.0),
        center(8, 100.5625, 8)
    );
}

#[test]
fn sleep_stand_up_anchor_prioritizes_horizontal_then_lower_then_upper_then_center() {
    let mut world = world();
    stone(&mut world, 8, 99, 7);
    stone(&mut world, 7, 98, 8);
    stone(&mut world, 8, 100, 9);
    stone(&mut world, 8, 100, 8);
    assert_eq!(standing(&world, None, 0.0), center(8, 100.0, 7));
    air(&mut world, 8, 99, 7);
    assert_eq!(standing(&world, None, 0.0), center(7, 99.0, 8));
    air(&mut world, 7, 98, 8);
    assert_eq!(standing(&world, None, 0.0), center(8, 101.0, 9));
    air(&mut world, 8, 100, 9);
    assert_eq!(standing(&world, None, 0.0), center(8, 101.0, 8));
}

#[test]
fn sleep_stand_up_safe_pass_exhausts_later_candidates_before_unsafe_floor_fallback() {
    let mut world = world();
    set(
        &mut world,
        BlockPos { x: 9, y: 99, z: 8 },
        "minecraft:magma_block",
        &[],
    );
    stone(&mut world, 9, 99, 9);
    assert_eq!(
        standing(&world, Some(Direction::North), 0.0),
        center(9, 100.0, 9)
    );
    air(&mut world, 9, 99, 9);
    assert_eq!(
        standing(&world, Some(Direction::North), 0.0),
        center(9, 100.0, 8)
    );
    set(
        &mut world,
        BlockPos { x: 9, y: 100, z: 8 },
        "minecraft:fire",
        &[],
    );
    assert_eq!(
        standing(&world, Some(Direction::North), 0.0),
        center(9, 100.0, 8)
    );
}

#[test]
fn sleep_stand_up_floor_ignores_climbable_and_open_trapdoors_but_collision_keeps_their_shapes() {
    let mut world = world();
    stone(&mut world, 9, 99, 8);
    let candidate = BlockPos { x: 9, y: 100, z: 8 };
    set(
        &mut world,
        candidate,
        "minecraft:ladder",
        &[("facing", "north")],
    );
    assert_eq!(
        standing(&world, Some(Direction::North), 0.0),
        center(9, 100.0, 8)
    );
    set(
        &mut world,
        candidate,
        "minecraft:oak_trapdoor",
        &[("open", "true"), ("facing", "north")],
    );
    assert_eq!(
        standing(&world, Some(Direction::North), 0.0),
        center(9, 100.0, 8)
    );
    set(
        &mut world,
        candidate,
        "minecraft:oak_trapdoor",
        &[("open", "false"), ("half", "bottom")],
    );
    assert_eq!(
        standing(&world, Some(Direction::North), 0.0),
        center(9, 100.1875, 8)
    );
    air(&mut world, 9, 100, 8);
    set(
        &mut world,
        BlockPos { x: 9, y: 99, z: 8 },
        "minecraft:stone_slab",
        &[("type", "bottom")],
    );
    assert_eq!(standing(&world, Some(Direction::North), 0.0), None);
    set(
        &mut world,
        candidate,
        "minecraft:stone_slab",
        &[("type", "bottom")],
    );
    assert_eq!(
        standing(&world, Some(Direction::North), 0.0),
        center(9, 100.5, 8)
    );
}

#[test]
fn sleep_stand_up_player_invalid_inside_checks_both_blocks_and_does_not_apply_to_other_types() {
    let mut world = world();
    stone(&mut world, 9, 99, 8);
    set(
        &mut world,
        BlockPos { x: 9, y: 100, z: 8 },
        "minecraft:fire",
        &[],
    );
    tags(&mut world, &["minecraft:fire"]);
    assert_eq!(standing(&world, Some(Direction::North), 0.0), None);
    assert_eq!(
        world
            .sleep_stand_up_position(
                &world.require_level_id(&dimension()),
                BED,
                VANILLA_ENTITY_TYPE_COW_ID,
                Some(Direction::North),
                0.0
            )
            .flatten(),
        center(9, 100.0, 8)
    );
    air(&mut world, 9, 100, 8);
    set(
        &mut world,
        BlockPos { x: 9, y: 101, z: 8 },
        "minecraft:fire",
        &[],
    );
    assert_eq!(standing(&world, Some(Direction::North), 0.0), None);
    tags(&mut world, &[]);
    assert_eq!(
        standing(&world, Some(Direction::North), 0.0),
        center(9, 100.0, 8)
    );
}

#[test]
fn sleep_stand_up_uses_type_dimensions_and_rejects_blocked_border_or_unloaded_chunks() {
    let mut world = world();
    stone(&mut world, 9, 99, 8);
    stone(&mut world, 9, 101, 8);
    assert_eq!(standing(&world, Some(Direction::North), 0.0), None);
    assert_eq!(
        world
            .sleep_stand_up_position(
                &world.require_level_id(&dimension()),
                BED,
                VANILLA_ENTITY_TYPE_CHICKEN_ID,
                Some(Direction::North),
                0.0
            )
            .flatten(),
        center(9, 100.0, 8)
    );
    air(&mut world, 9, 101, 8);
    let original_border = world
        .level_by_id(&world.require_level_id(&dimension()))
        .world_border;
    world
        .level_mut_by_id(&world.require_level_id(&dimension()))
        .world_border = WorldBorder::from_settings(8.0, 8.0, 0.2, 5.0, 5, 300, 3.0, 0, 3.0, 0);
    assert_eq!(standing(&world, Some(Direction::North), 0.0), None);
    world
        .level_mut_by_id(&world.require_level_id(&dimension()))
        .world_border = original_border;
    tags(&mut world, &[]);
    assert_eq!(
        world.sleep_stand_up_position(
            &world.require_level_id(&dimension()),
            BlockPos {
                x: 24,
                y: 100,
                z: 8
            },
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            None,
            0.0
        ),
        None
    );
    assert!(!world
        .level_by_id(&world.require_level_id(&dimension()))
        .chunks
        .is_loaded(1, 0));
}

#[test]
fn sleep_stand_up_anchor_preserves_all_eight_horizontal_offsets() {
    let mut world = world();
    let expected = [
        (8, 7),
        (7, 8),
        (8, 9),
        (9, 8),
        (7, 7),
        (9, 7),
        (7, 9),
        (9, 9),
    ];
    for &(x, z) in &expected {
        stone(&mut world, x, 99, z);
    }
    for &(x, z) in &expected {
        assert_eq!(standing(&world, None, 0.0), center(x, 100.0, z));
        stone(&mut world, x, 100, z);
    }
    assert_eq!(standing(&world, None, 0.0), center(8, 101.0, 7));
}

#[test]
fn sleep_stand_up_null_collision_uses_powder_snow_empty_shape_and_missing_neighbors_fail_closed() {
    let mut world = world();
    stone(&mut world, 9, 99, 8);
    set(
        &mut world,
        BlockPos { x: 9, y: 100, z: 8 },
        "minecraft:powder_snow",
        &[],
    );
    assert_eq!(
        standing(&world, Some(Direction::North), 0.0),
        center(9, 100.0, 8)
    );
    let mut world = self::world();
    stone(&mut world, 15, 99, 8);
    let pos = BlockPos {
        x: 14,
        y: 100,
        z: 8,
    };
    let giant = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GIANT_ID;
    assert_eq!(
        world.sleep_stand_up_position(
            &world.require_level_id(&dimension()),
            pos,
            giant,
            Some(Direction::North),
            0.0
        ),
        None
    );
    assert!(!world
        .level_by_id(&world.require_level_id(&dimension()))
        .chunks
        .is_loaded(1, 0));
    world
        .level_mut_by_id(&world.require_level_id(&dimension()))
        .chunks
        .try_ensure_served_full_column(1, 0)
        .unwrap();
    assert_eq!(
        world
            .sleep_stand_up_position(
                &world.require_level_id(&dimension()),
                pos,
                giant,
                Some(Direction::North),
                0.0
            )
            .flatten(),
        center(15, 100.0, 8)
    );
}
