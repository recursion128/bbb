use crate::fluid::FlowingFluid;

use super::*;

const WALL: BlockPos = BlockPos { x: 4, y: 70, z: -8 };

fn wall(up: bool, north: &str, east: &str, south: &str, west: &str, waterlogged: bool) -> i32 {
    state_of(
        "minecraft:cobblestone_wall",
        &[
            ("east", east),
            ("north", north),
            ("south", south),
            ("up", if up { "true" } else { "false" }),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
            ("west", west),
        ],
    )
}

#[test]
fn a_waterlogged_wall_saves_a_water_tick_and_preserves_its_state() {
    let state = wall(true, "none", "none", "none", "none", true);
    let mut world = NativeWorld::new();
    world.set(WALL, state);
    let answer = { update(&mut world, state, WALL, Direction::Down, stone()) };
    assert_eq!(answer, state);
    assert!(world
        .saved_fluid_ticks()
        .iter()
        .any(|tick| tick.id == "minecraft:water"
            && (tick.x, tick.y, tick.z) == (WALL.x, WALL.y, WALL.z)
            && tick.delay == FlowingFluid::WATER_TICK_DELAY));
}

#[test]
fn a_side_update_uses_the_four_connection_families() {
    let initial = wall(true, "none", "none", "none", "none", false);
    for neighbour in [
        default_state("minecraft:brick_wall"),
        default_state("minecraft:iron_bars"),
        stone(),
        state_of(
            "minecraft:oak_fence_gate",
            &[
                ("facing", "north"),
                ("in_wall", "false"),
                ("open", "false"),
                ("powered", "false"),
            ],
        ),
    ] {
        let mut world = NativeWorld::new();
        world.set(above(WALL), air());
        let answer = update(&mut world, initial, WALL, Direction::East, neighbour);
        assert_eq!(string_property(answer, "east").as_deref(), Some("low"));
        assert_eq!(bool_property(answer, "up"), Some(true));
    }

    let gate_along_connection = state_of(
        "minecraft:oak_fence_gate",
        &[
            ("facing", "east"),
            ("in_wall", "false"),
            ("open", "false"),
            ("powered", "false"),
        ],
    );
    let mut world = NativeWorld::new();
    let answer = update(
        &mut world,
        initial,
        WALL,
        Direction::East,
        gate_along_connection,
    );
    assert_eq!(string_property(answer, "east").as_deref(), Some("none"));
}

#[test]
fn a_full_block_above_makes_opposite_walls_tall_and_suppresses_the_post() {
    let initial = wall(true, "low", "none", "low", "none", false);
    let mut world = NativeWorld::new();
    let answer = update(&mut world, initial, WALL, Direction::Up, stone());
    assert_eq!(answer, wall(false, "tall", "none", "tall", "none", false));
}

#[test]
fn only_the_down_face_of_a_slab_makes_a_wall_side_tall() {
    let initial = wall(true, "low", "none", "none", "none", false);
    let bottom_slab = state_of(
        "minecraft:oak_slab",
        &[("type", "bottom"), ("waterlogged", "false")],
    );
    let top_slab = state_of(
        "minecraft:oak_slab",
        &[("type", "top"), ("waterlogged", "false")],
    );
    let mut world = NativeWorld::new();
    let bottom_answer = update(&mut world, initial, WALL, Direction::Up, bottom_slab);
    assert_eq!(
        string_property(bottom_answer, "north").as_deref(),
        Some("tall")
    );

    let top_answer = update(&mut world, initial, WALL, Direction::Up, top_slab);
    assert_eq!(string_property(top_answer, "north").as_deref(), Some("low"));
}

#[test]
fn a_wall_post_override_raises_an_otherwise_straight_low_wall() {
    let initial = wall(false, "low", "none", "low", "none", false);
    let mut world = NativeWorld::new();
    let air_answer = update(&mut world, initial, WALL, Direction::Up, air());
    assert_eq!(bool_property(air_answer, "up"), Some(false));

    let torch_answer = update(
        &mut world,
        initial,
        WALL,
        Direction::Up,
        default_state("minecraft:torch"),
    );
    assert_eq!(bool_property(torch_answer, "up"), Some(true));
}

fn update(
    world: &mut NativeWorld,
    state: i32,
    pos: BlockPos,
    direction: Direction,
    neighbor_state: i32,
) -> i32 {
    world.set(pos, state);
    world.set(relative(pos, direction), neighbor_state);
    world.shape_update(pos, direction);
    world.state(pos)
}
