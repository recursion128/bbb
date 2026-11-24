use bbb_protocol::block_types::{
    vanilla_concrete_block_type_id_for_powder, VanillaGetStateForPlacementOverride,
    VANILLA_BLOCK_TYPES,
};

use super::*;

const POWDER_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };
const LIQUID_DIRECTIONS: [Direction; 5] = [
    Direction::Up,
    Direction::North,
    Direction::South,
    Direction::West,
    Direction::East,
];

fn powder_entries() -> Vec<&'static bbb_protocol::block_types::BlockTypeCatalogEntry> {
    VANILLA_BLOCK_TYPES
        .iter()
        .filter(|entry| {
            entry.get_state_for_placement_override
                == Some(VanillaGetStateForPlacementOverride::ConcretePowderBlock)
        })
        .collect()
}

fn powder_state(entry: &bbb_protocol::block_types::BlockTypeCatalogEntry) -> i32 {
    default_state(entry.resource_id)
}

fn concrete_state(entry: &bbb_protocol::block_types::BlockTypeCatalogEntry) -> i32 {
    let target = vanilla_concrete_block_type_id_for_powder(entry.id)
        .expect("every generated concrete-powder entry has its constructor target");
    default_block_state_id(target)
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction, neighbour: i32) -> i32 {
    world.set(POWDER_POS, state);
    world.set(relative(POWDER_POS, direction), neighbour);
    world.shape_update(POWDER_POS, direction);
    world.state(POWDER_POS)
}

#[test]
fn source_water_on_each_exposed_side_solidifies_the_powder() {
    for entry in powder_entries().into_iter().take(1) {
        let state = powder_state(entry);
        for direction in LIQUID_DIRECTIONS {
            let mut world = NativeWorld::new();
            assert_eq!(
                update(&mut world, state, direction, water(0)),
                concrete_state(entry),
                "{} {direction:?}",
                entry.resource_id
            );
        }
    }
}

#[test]
fn flowing_water_solidifies_but_water_only_below_does_not() {
    let entry = powder_entries()[0];
    let state = powder_state(entry);
    let mut world = NativeWorld::new();
    assert_eq!(
        update(&mut world, state, Direction::North, water(5)),
        concrete_state(entry)
    );

    let mut world = NativeWorld::new();
    assert_eq!(update(&mut world, state, Direction::Down, water(0)), state);
}

#[test]
fn a_sturdy_waterlogged_face_does_not_solidify() {
    let state = powder_state(powder_entries()[0]);

    let barrier = state_of("minecraft:barrier", &[("waterlogged", "true")]);
    let mut world = NativeWorld::new();
    assert_eq!(update(&mut world, state, Direction::Up, barrier), state);
}

mod landing;
