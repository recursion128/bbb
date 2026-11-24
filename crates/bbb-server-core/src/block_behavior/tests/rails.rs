use super::*;

use crate::block_behavior::rail::{
    can_survive, detector_direct_signal, detector_signal, shape_of, RailShape,
};

const FLOOR_Y: i32 = 63;
const RAIL_Y: i32 = 64;

fn rail(shape: &str) -> i32 {
    state_of(
        "minecraft:rail",
        &[("shape", shape), ("waterlogged", "false")],
    )
}

fn powered_rail(shape: &str, powered: bool) -> i32 {
    state_of(
        "minecraft:powered_rail",
        &[
            ("powered", if powered { "true" } else { "false" }),
            ("shape", shape),
            ("waterlogged", "false"),
        ],
    )
}

fn detector_rail(shape: &str, powered: bool) -> i32 {
    state_of(
        "minecraft:detector_rail",
        &[
            ("powered", if powered { "true" } else { "false" }),
            ("shape", shape),
            ("waterlogged", "false"),
        ],
    )
}

fn rail_block_type() -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_block_type_id(rail("north_south"))
        .expect("rail has a registry id")
}

fn plate() -> NativeWorld {
    let mut world = NativeWorld::new();
    for x in -3..=12 {
        for z in -3..=3 {
            world.set(at(x, FLOOR_Y, z), stone());
        }
    }
    world
}

fn place(world: &mut NativeWorld, pos: BlockPos, state: i32) {
    load_saved_rail(world, pos, state);
    world.with_level(|level| on_place(level, pos, state, air(), false));
}

#[test]
fn a_rail_needs_a_rigid_face_below_it() {
    let pos = at(0, RAIL_Y, 0);
    let read = |support: i32| {
        can_survive(pos, &mut |at_pos| {
            if at_pos == below(pos) {
                support
            } else {
                air()
            }
        })
    };
    assert_eq!(read(stone()), Some(true));
    assert_eq!(read(air()), Some(false));
}

#[test]
fn a_rail_laid_beside_another_turns_both_of_them_to_meet() {
    let west = at(0, RAIL_Y, 0);
    let east = at(1, RAIL_Y, 0);
    let mut world = plate();
    place(&mut world, west, rail("north_south"));
    assert_eq!(world.state(west), rail("north_south"));

    place(&mut world, east, rail("north_south"));

    assert_eq!(world.state(east), rail("east_west"));
    assert_eq!(world.state(west), rail("east_west"));
}

#[test]
fn a_plain_rail_corners_where_a_powered_rail_stays_straight() {
    let corner = at(0, RAIL_Y, 0);
    let east = at(1, RAIL_Y, 0);
    let south = at(0, RAIL_Y, 1);

    let mut world = plate();
    load_saved_rail(&mut world, east, rail("east_west"));
    load_saved_rail(&mut world, south, rail("north_south"));
    place(&mut world, corner, rail("north_south"));
    assert_eq!(world.state(corner), rail("south_east"));

    let mut world = plate();
    load_saved_rail(&mut world, east, powered_rail("east_west", false));
    load_saved_rail(&mut world, south, powered_rail("north_south", false));
    place(&mut world, corner, powered_rail("north_south", false));
    assert_eq!(world.state(corner), powered_rail("north_south", false));
}

#[test]
fn a_rail_climbs_to_meet_one_a_cell_higher() {
    let low = at(0, RAIL_Y, 0);
    let high = at(0, RAIL_Y + 1, 1);
    let mut world = plate();
    world.set(at(0, RAIL_Y, 1), stone());
    load_saved_rail(&mut world, high, rail("north_south"));

    place(&mut world, low, rail("north_south"));

    assert_eq!(world.state(low), rail("ascending_south"));
    assert_eq!(shape_of(world.state(low)), Some(RailShape::AscendingSouth));
    assert!(RailShape::AscendingSouth.is_slope());
}

#[test]
fn a_rail_that_loses_its_floor_drops_and_goes() {
    let pos = at(0, RAIL_Y, 0);
    let mut world = plate();
    place(&mut world, pos, rail("north_south"));
    world.set(below(pos), air());

    world.neighbor_changed(pos, rail_block_type());

    assert_eq!(world.state(pos), air());
    assert!(world
        .items()
        .iter()
        .any(|(name, count)| name == "minecraft:rail" && *count == 1));
}

#[test]
fn an_ascending_rail_also_needs_the_cell_it_leans_on() {
    let pos = at(0, RAIL_Y, 0);
    let mut world = plate();
    load_saved_rail(&mut world, pos, rail("ascending_south"));
    world.set(at(0, RAIL_Y, 1), stone());

    world.neighbor_changed(pos, rail_block_type());
    assert_eq!(world.state(pos), rail("ascending_south"));

    world.set(at(0, RAIL_Y, 1), air());
    world.neighbor_changed(pos, rail_block_type());
    assert_eq!(world.state(pos), air());
}

#[test]
fn a_powered_rail_signal_reaches_eight_rails_and_no_further() {
    let run = |length: i32| {
        let mut world = plate();
        world.set(at(0, RAIL_Y, -1), default_state("minecraft:redstone_block"));
        for x in 0..length {
            load_saved_rail(
                &mut world,
                at(x, RAIL_Y, 0),
                powered_rail("east_west", true),
            );
        }
        let far = at(length, RAIL_Y, 0);
        load_saved_rail(&mut world, far, powered_rail("east_west", false));
        world.neighbor_changed(far, rail_block_type());
        world.state(far)
    };
    assert_eq!(run(8), powered_rail("east_west", true));
    assert_eq!(run(9), powered_rail("east_west", false));
}

#[test]
fn a_powered_rail_beside_a_source_turns_on() {
    let pos = at(0, RAIL_Y, 0);
    let mut world = plate();
    world.set(at(0, RAIL_Y, -1), default_state("minecraft:redstone_block"));
    load_saved_rail(&mut world, pos, powered_rail("east_west", false));

    world.neighbor_changed(pos, rail_block_type());

    assert_eq!(world.state(pos), powered_rail("east_west", true));
}

#[test]
fn a_waterlogged_rail_re_arms_its_water_on_every_shape_update() {
    let pos = at(0, RAIL_Y, 0);
    let waterlogged = state_of(
        "minecraft:rail",
        &[("shape", "north_south"), ("waterlogged", "true")],
    );
    let mut world = plate();
    load_saved_rail(&mut world, pos, waterlogged);

    world.shape_update(pos, Direction::North);

    assert_eq!(world.state(pos), waterlogged);
    assert!(world.saved_fluid_ticks().iter().any(|tick| tick.x == pos.x
        && tick.y == pos.y
        && tick.z == pos.z
        && tick.id == "minecraft:water"));
}

#[test]
fn a_pressed_detector_rail_sources_fifteen_and_powers_only_the_cell_above() {
    assert_eq!(detector_signal(detector_rail("north_south", true)), 15);
    assert_eq!(detector_signal(detector_rail("north_south", false)), 0);
    for direction in DIRECTION_VALUES {
        let expected = if direction == Direction::Up { 15 } else { 0 };
        assert_eq!(
            detector_direct_signal(detector_rail("north_south", true), direction),
            expected,
            "{direction:?}"
        );
        assert_eq!(
            detector_direct_signal(detector_rail("north_south", false), direction),
            0,
            "{direction:?}"
        );
    }
}

fn load_saved_rail(world: &mut NativeWorld, pos: BlockPos, state: i32) {
    use bbb_server_types::save::Java26BuiltinDimension;
    world
        .world
        .set_block(Java26BuiltinDimension::Overworld, pos, state, 2 | 16);
    // A loaded rail predates the shape or power update under test.
    world
        .world
        .set_raw_block_state_for_test(Java26BuiltinDimension::Overworld, pos, state);
}
