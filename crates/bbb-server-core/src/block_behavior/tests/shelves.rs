use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::packets::{BlockPos, Direction};

use super::{state_of, stone, NativeWorld};

const SHELF: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const FLOOR: BlockPos = BlockPos { x: 0, y: 63, z: 0 };
const SIDE: BlockPos = BlockPos { x: 1, y: 64, z: 0 };

fn powered(state: i32) -> Option<String> {
    vanilla_block_state_identity(state)?
        .properties
        .get("powered")
        .cloned()
}

fn redstone_block() -> i32 {
    super::default_state("minecraft:redstone_block")
}

#[test]
fn a_north_face_cursor_hits_shelf_slot_zero() {
    use crate::block_behavior::shelf::hit_slot;
    assert_eq!(
        hit_slot(Direction::North, Direction::North, [0.9, 0.5, 0.0]),
        Some(0)
    );
    assert_eq!(
        hit_slot(Direction::North, Direction::Up, [0.9, 0.5, 0.0]),
        None
    );
}

#[test]
fn a_shelf_powers_when_it_has_a_neighbour_signal() {
    let mut world = NativeWorld::new();
    world.set(FLOOR, stone());
    let unpowered = chained(false, "unconnected");
    let powered = chained(true, "unconnected");
    world.set(SHELF, unpowered);
    world.set(SIDE, redstone_block());
    world.neighbor_changed(
        SHELF,
        bbb_server_types::block_queries::block_type_of(redstone_block()).unwrap(),
    );
    assert_eq!(world.state(SHELF), powered);
    assert_eq!(
        world
            .sounds()
            .into_iter()
            .map(|sound| (sound.pos, sound.resource_id))
            .collect::<Vec<_>>(),
        [(SHELF, "minecraft:block.shelf.activate".to_owned())]
    );
    assert_eq!(
        world
            .events()
            .into_iter()
            .filter(|event| event.pos == SHELF)
            .map(|event| (event.game_event, event.pos, event.affected_state))
            .collect::<Vec<_>>(),
        [(VanillaGameEvent::BLOCK_ACTIVATE, SHELF, Some(powered))]
    );

    world.sounds();

    world.set(SIDE, super::air());
    world.neighbor_changed(
        SHELF,
        bbb_server_types::block_queries::block_type_of(redstone_block()).unwrap(),
    );
    assert_eq!(world.state(SHELF), unpowered);
    assert_eq!(
        world
            .sounds()
            .into_iter()
            .map(|sound| (sound.pos, sound.resource_id))
            .collect::<Vec<_>>(),
        [(SHELF, "minecraft:block.shelf.deactivate".to_owned())]
    );
    assert_eq!(
        world
            .events()
            .into_iter()
            .filter(|event| event.pos == SHELF
                && event.game_event == VanillaGameEvent::BLOCK_DEACTIVATE)
            .map(|event| (event.game_event, event.pos, event.affected_state))
            .collect::<Vec<_>>(),
        [(VanillaGameEvent::BLOCK_DEACTIVATE, SHELF, Some(unpowered))]
    );
}

#[test]
fn unchanged_power_has_no_write_sound_or_event() {
    let initial = chained(false, "unconnected");
    let mut world = NativeWorld::new();
    world.set(SHELF, initial);

    world.neighbor_changed(
        SHELF,
        bbb_server_types::block_queries::block_type_of(stone()).unwrap(),
    );

    assert_eq!(world.state(SHELF), initial);
    assert!(world
        .sounds()
        .into_iter()
        .map(|sound| (sound.pos, sound.resource_id))
        .collect::<Vec<_>>()
        .is_empty());
    assert!(world
        .events()
        .into_iter()
        .filter(|event| event.pos == SHELF)
        .map(|event| (event.game_event, event.pos, event.affected_state))
        .collect::<Vec<_>>()
        .is_empty());
}

const LEFT: BlockPos = BlockPos { x: 1, y: 64, z: 0 };
const RIGHT: BlockPos = BlockPos { x: -1, y: 64, z: 0 };

fn chained(powered: bool, part: &str) -> i32 {
    state_of(
        "minecraft:oak_shelf",
        &[
            ("facing", "north"),
            ("powered", if powered { "true" } else { "false" }),
            ("side_chain", part),
            ("waterlogged", "false"),
        ],
    )
}

fn part(world: &mut NativeWorld, pos: BlockPos) -> Option<String> {
    vanilla_block_state_identity(world.state(pos))?
        .properties
        .get("side_chain")
        .cloned()
}

fn place(world: &mut NativeWorld, pos: BlockPos) {
    let state = world.state(pos);
    world.with_level(|level| super::on_place(level, pos, state, super::air(), false));
}

#[test]
fn a_powered_shelf_with_nobody_beside_it_stays_unconnected() {
    let mut world = NativeWorld::new();
    load_saved_shelves(
        &mut world,
        &[
            (SHELF, chained(true, "unconnected")),
            (LEFT, chained(false, "unconnected")),
        ],
    );
    place(&mut world, SHELF);
    assert_eq!(part(&mut world, SHELF).as_deref(), Some("unconnected"));
    assert_eq!(part(&mut world, LEFT).as_deref(), Some("unconnected"));
}

#[test]
fn a_shelf_powering_up_beside_another_joins_from_both_ends() {
    let mut world = NativeWorld::new();
    load_saved_shelves(
        &mut world,
        &[
            (SHELF, chained(true, "unconnected")),
            (LEFT, chained(true, "unconnected")),
        ],
    );
    place(&mut world, SHELF);
    assert_eq!(part(&mut world, SHELF).as_deref(), Some("right"));
    assert_eq!(part(&mut world, LEFT).as_deref(), Some("left"));

    let mut world = NativeWorld::new();
    load_saved_shelves(
        &mut world,
        &[
            (SHELF, chained(true, "unconnected")),
            (LEFT, chained(true, "unconnected")),
            (RIGHT, chained(true, "unconnected")),
        ],
    );
    place(&mut world, SHELF);
    assert_eq!(part(&mut world, SHELF).as_deref(), Some("center"));
    assert_eq!(part(&mut world, LEFT).as_deref(), Some("left"));
    assert_eq!(part(&mut world, RIGHT).as_deref(), Some("right"));
}

#[test]
fn the_chain_stops_at_three() {
    let mut world = NativeWorld::new();
    load_saved_shelves(
        &mut world,
        &[
            (SHELF, chained(true, "unconnected")),
            (LEFT, chained(true, "right")),
            (BlockPos { x: 2, y: 64, z: 0 }, chained(true, "center")),
            (BlockPos { x: 3, y: 64, z: 0 }, chained(true, "left")),
        ],
    );
    place(&mut world, SHELF);
    assert_eq!(part(&mut world, SHELF).as_deref(), Some("unconnected"));
    assert_eq!(part(&mut world, LEFT).as_deref(), Some("right"));
}

#[test]
fn a_shelf_losing_its_signal_lets_both_neighbours_go_and_says_so() {
    let mut world = NativeWorld::new();
    load_saved_shelves(
        &mut world,
        &[
            (SHELF, chained(true, "center")),
            (LEFT, chained(true, "left")),
            (RIGHT, chained(true, "right")),
        ],
    );
    world.set(crate::fluid::below(SHELF), stone());
    world.neighbor_changed(
        SHELF,
        bbb_server_types::block_queries::block_type_of(redstone_block()).unwrap(),
    );

    assert_eq!(part(&mut world, SHELF).as_deref(), Some("unconnected"));
    assert_eq!(powered(world.state(SHELF)).as_deref(), Some("false"));
    assert_eq!(
        world
            .sounds()
            .into_iter()
            .filter(|sound| sound.pos == SHELF)
            .map(|sound| (sound.pos, sound.resource_id))
            .collect::<Vec<_>>(),
        vec![(SHELF, "minecraft:block.shelf.deactivate".to_owned())]
    );
    assert_eq!(
        world
            .events()
            .into_iter()
            .filter(|event| event.pos == SHELF)
            .map(|event| (event.game_event, event.pos, event.affected_state))
            .collect::<Vec<_>>(),
        vec![(
            VanillaGameEvent::BLOCK_DEACTIVATE,
            SHELF,
            Some(chained(false, "unconnected"))
        )]
    );

    assert_eq!(part(&mut world, LEFT).as_deref(), Some("unconnected"));
    assert_eq!(part(&mut world, RIGHT).as_deref(), Some("unconnected"));
}

#[test]
fn a_shelf_that_goes_lets_both_neighbours_go_with_it() {
    let mut world = NativeWorld::new();
    load_saved_shelves(
        &mut world,
        &[
            (LEFT, chained(true, "left")),
            (RIGHT, chained(true, "right")),
        ],
    );
    let removed = chained(true, "center");
    {
        world.with_level(|level| {
            super::affect_neighbors_after_removal(level, SHELF, removed, false)
        });
    }
    assert_eq!(part(&mut world, LEFT).as_deref(), Some("unconnected"));
    assert_eq!(part(&mut world, RIGHT).as_deref(), Some("unconnected"));
}

fn load_saved_shelves(world: &mut NativeWorld, states: &[(BlockPos, i32)]) {
    use bbb_server_types::save::Java26BuiltinDimension;
    for &(pos, state) in states {
        if powered(state).as_deref() == Some("true") {
            world.set(crate::fluid::below(pos), redstone_block());
        }
        world.world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            chained(false, "unconnected"),
            2 | 16,
        );
    }
    // Loaded chain states predate the placement or power change under test.
    for &(pos, state) in states {
        world
            .world
            .set_raw_block_state_for_test(Java26BuiltinDimension::Overworld, pos, state);
    }
}
