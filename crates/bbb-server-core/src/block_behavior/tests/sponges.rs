use super::native_world::NativeWorld;

use super::*;

use crate::block_behavior::sponge::MAX_COUNT;

const SPONGE: BlockPos = BlockPos { x: 1, y: 70, z: -2 };

fn sponge() -> i32 {
    default_state("minecraft:sponge")
}

fn wet_sponge() -> i32 {
    default_state("minecraft:wet_sponge")
}

fn place(world: &mut NativeWorld, pos: BlockPos, state: i32, old_state: i32) {
    world.world.set_block(
        bbb_server_types::save::Java26BuiltinDimension::Overworld,
        pos,
        old_state,
        18,
    );
    world.world.set_block(
        bbb_server_types::save::Java26BuiltinDimension::Overworld,
        pos,
        state,
        18,
    );
}

fn cube(radius: i32) -> Vec<BlockPos> {
    let mut cells = Vec::new();
    for x in -radius..=radius {
        for y in -radius..=radius {
            for z in -radius..=radius {
                let pos = offset(SPONGE, x, y, z);
                if pos != SPONGE {
                    cells.push(pos);
                }
            }
        }
    }
    cells
}

fn water_cells(world: &mut NativeWorld, cells: &[BlockPos]) -> usize {
    cells
        .iter()
        .filter(|pos| world.state(**pos) == water(0))
        .count()
}

#[test]
fn a_dry_sponge_with_no_water_beside_it_stays_dry() {
    let mut world = NativeWorld::new();
    place(&mut world, SPONGE, sponge(), air());

    assert_eq!(world.state(SPONGE), sponge());
    assert!(world.sounds().is_empty());
}

#[test]
fn a_sponge_takes_the_water_around_it_and_turns_wet() {
    let mut world = NativeWorld::new();
    let cells = cube(1);
    for pos in &cells {
        world.set(*pos, water(0));
    }

    place(&mut world, SPONGE, sponge(), air());

    assert_eq!(world.state(SPONGE), wet_sponge());
    assert_eq!(water_cells(&mut world, &cells), 0);
    assert!(world
        .sounds()
        .iter()
        .any(|sound| sound.pos == SPONGE && sound.resource_id == "minecraft:block.sponge.absorb"));
}

#[test]
fn a_large_water_cube_is_partly_absorbed_and_the_sponge_turns_wet() {
    let mut world = NativeWorld::new();
    let cells = cube(2);
    for pos in &cells {
        world.set(*pos, water(0));
    }

    place(&mut world, SPONGE, sponge(), air());

    assert_eq!(world.state(SPONGE), wet_sponge());
    let remaining = water_cells(&mut world, &cells);
    assert!(remaining > 0);
    assert!(cells.len() - remaining >= usize::try_from(MAX_COUNT).unwrap());
}

#[test]
fn a_waterlogged_neighbour_is_drained_rather_than_removed() {
    let waterlogged = state_of(
        "minecraft:oak_slab",
        &[("type", "bottom"), ("waterlogged", "true")],
    );
    let drained = state_of(
        "minecraft:oak_slab",
        &[("type", "bottom"), ("waterlogged", "false")],
    );
    let mut world = NativeWorld::new();
    world.set(relative(SPONGE, Direction::East), waterlogged);

    place(&mut world, SPONGE, sponge(), air());

    assert_eq!(world.state(relative(SPONGE, Direction::East)), drained);
    assert_eq!(world.state(SPONGE), wet_sponge());
}

#[test]
fn kelp_and_seagrass_drop_before_the_water_goes() {
    let kelp = default_state("minecraft:kelp_plant");
    let mut world = NativeWorld::new();
    world.set(relative(SPONGE, Direction::Down), kelp);
    world.set(relative(SPONGE, Direction::East), water(0));

    place(&mut world, SPONGE, sponge(), air());

    assert_eq!(world.state(relative(SPONGE, Direction::Down)), air());
    assert!(world
        .items()
        .iter()
        .any(|(id, count)| id == "minecraft:kelp" && *count > 0));
}

#[test]
fn a_sponge_replacing_a_sponge_does_not_sweep_again() {
    let mut world = NativeWorld::new();
    world.set(SPONGE, sponge());
    world.set(relative(SPONGE, Direction::East), water(0));

    place(&mut world, SPONGE, sponge(), sponge());

    assert_eq!(world.state(SPONGE), sponge());
    assert_eq!(world.state(relative(SPONGE, Direction::East)), water(0));
}

#[test]
fn a_wet_sponge_stays_wet_in_the_live_level() {
    let mut world = NativeWorld::new();
    place(&mut world, SPONGE, wet_sponge(), air());
    assert_eq!(world.state(SPONGE), wet_sponge());
    assert!(world.level_events().is_empty());
}
