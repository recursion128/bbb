use bbb_protocol::block_types::{
    vanilla_block_type_for_resource_id, VANILLA_BLOCK_REDSTONE_WIRE_ID,
};
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;

use bbb_protocol::packets::Direction;

use crate::block_behavior::redstone_wire::{self};
use crate::redstone::{SignalContext, SIGNAL_MAX};

use super::*;

const WIRE: BlockPos = at(4, 70, -9);

fn wire(power: i32, north: &str, east: &str, south: &str, west: &str) -> i32 {
    state_of(
        "minecraft:redstone_wire",
        &[
            ("east", east),
            ("north", north),
            ("power", &power.to_string()),
            ("south", south),
            ("west", west),
        ],
    )
}

fn cross(power: i32) -> i32 {
    wire(power, "side", "side", "side", "side")
}

fn dot(power: i32) -> i32 {
    wire(power, "none", "none", "none", "none")
}

fn redstone_block() -> i32 {
    state_of("minecraft:redstone_block", &[])
}

fn state_only_signal_source() -> i32 {
    default_state("minecraft:jukebox")
}

fn top_slab() -> i32 {
    state_of(
        "minecraft:oak_slab",
        &[("type", "top"), ("waterlogged", "false")],
    )
}

fn trapdoor() -> i32 {
    state_of(
        "minecraft:oak_trapdoor",
        &[
            ("facing", "north"),
            ("half", "bottom"),
            ("open", "false"),
            ("powered", "false"),
            ("waterlogged", "false"),
        ],
    )
}

fn block_id(name: &str) -> i32 {
    vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("{name} is not a built-in 26.1 block"))
        .id
}

fn place(entries: &[(BlockPos, i32)]) -> Option<i32> {
    let item = vanilla_item_type_for_resource_id("minecraft:redstone")
        .expect("redstone dust is a built-in 26.1 item")
        .id;
    state_for_placement(item, WIRE, vanilla_tags(), &mut |at| {
        entries
            .iter()
            .find(|(pos, _)| *pos == at)
            .map_or_else(air, |(_, state)| *state)
    })
}

fn world_with(entries: &[(BlockPos, i32)]) -> NativeWorld {
    use bbb_server_types::save::Java26BuiltinDimension;
    let mut world = NativeWorld::new();
    world.set(below(WIRE), stone());
    for &(pos, state) in entries {
        world
            .world
            .set_block(Java26BuiltinDimension::Overworld, pos, state, 2 | 16);
    }
    // Loaded wire connections and power predate the update being exercised.
    for &(pos, state) in entries {
        world
            .world
            .set_raw_block_state_for_test(Java26BuiltinDimension::Overworld, pos, state);
    }
    world
}

#[test]
fn a_wire_placed_with_nothing_around_it_lands_as_a_cross() {
    assert_eq!(place(&[(below(WIRE), stone())]), Some(cross(0)));
}

#[test]
fn a_wire_will_not_be_placed_on_air_and_will_be_placed_on_a_hopper() {
    assert_eq!(place(&[]), None, "nothing holds a wire up");
    assert_eq!(
        place(&[(below(WIRE), default_state("minecraft:hopper"))]),
        Some(cross(0)),
        "a hopper supports wire"
    );
    assert_eq!(
        face_sturdy(
            default_state("minecraft:hopper"),
            Direction::Up,
            SupportType::Full
        ),
        Some(false)
    );
}

#[test]
fn a_wire_between_two_wires_is_a_line_along_that_axis() {
    let east = relative(WIRE, Direction::East);
    let west = relative(WIRE, Direction::West);
    assert_eq!(
        place(&[
            (below(WIRE), stone()),
            (east, cross(0)),
            (west, cross(0)),
            (below(east), stone()),
            (below(west), stone()),
        ]),
        Some(wire(0, "none", "side", "none", "side"))
    );
}

#[test]
fn a_wire_climbs_a_full_block_and_only_reaches_a_top_slab() {
    let east = relative(WIRE, Direction::East);
    let above_east = above(east);
    assert_eq!(
        face_sturdy(top_slab(), Direction::Up, SupportType::Full),
        Some(true)
    );
    assert_eq!(
        face_sturdy(top_slab(), Direction::West, SupportType::Full),
        Some(false)
    );
    assert_eq!(
        face_sturdy(stone(), Direction::West, SupportType::Full),
        Some(true)
    );

    assert_eq!(
        place(&[
            (below(WIRE), stone()),
            (east, stone()),
            (above_east, cross(0)),
        ]),
        Some(wire(0, "none", "up", "none", "side")),
        "connection shape"
    );
    assert_eq!(
        place(&[
            (below(WIRE), stone()),
            (east, top_slab()),
            (above_east, cross(0)),
        ]),
        Some(wire(0, "none", "side", "none", "side")),
        "connection shape"
    );
}

#[test]
fn a_trapdoor_is_climbed_although_no_wire_could_stand_on_it() {
    let east = relative(WIRE, Direction::East);
    assert_eq!(
        face_sturdy(trapdoor(), Direction::Up, SupportType::Full),
        Some(false),
        "a trapdoor cannot support wire"
    );
    assert_eq!(
        place(&[
            (below(WIRE), stone()),
            (east, trapdoor()),
            (above(east), cross(0)),
        ]),
        Some(wire(0, "none", "side", "none", "side"))
    );
}

#[test]
fn a_conductor_overhead_stops_the_climb() {
    let east = relative(WIRE, Direction::East);
    assert_eq!(
        place(&[
            (below(WIRE), stone()),
            (above(WIRE), stone()),
            (east, stone()),
            (above(east), cross(0)),
        ]),
        Some(cross(0))
    );
}

#[test]
fn a_wire_reaches_down_across_an_empty_neighbour() {
    let east = relative(WIRE, Direction::East);
    assert_eq!(
        place(&[(below(WIRE), stone()), (below(east), cross(0))]),
        Some(wire(0, "none", "side", "none", "side"))
    );
    assert_eq!(
        place(&[
            (below(WIRE), stone()),
            (east, stone()),
            (below(east), cross(0)),
        ]),
        Some(cross(0))
    );
}

#[test]
fn only_a_dot_stays_a_dot() {
    let mut world = world_with(&[(WIRE, dot(0))]);
    world.shape_update(WIRE, Direction::Up);
    assert_eq!(world.state(WIRE), dot(0));

    let mut world = world_with(&[(WIRE, cross(0))]);
    world.shape_update(WIRE, Direction::Up);
    assert_eq!(world.state(WIRE), cross(0));
}

#[test]
fn a_right_click_toggles_cross_and_dot_but_not_a_line() {
    let mut world = world_with(&[(WIRE, cross(11))]);
    let consumed = {
        let state = world.state(WIRE);
        world.with_level(|level| {
            use_without_item(
                level,
                WIRE,
                state,
                interaction_player(true, Direction::North),
            )
        })
    };
    assert!(consumed);
    assert_eq!(world.state(WIRE), dot(11));

    let consumed = {
        let state = world.state(WIRE);
        world.with_level(|level| {
            use_without_item(
                level,
                WIRE,
                state,
                interaction_player(true, Direction::North),
            )
        })
    };
    assert!(consumed);
    assert_eq!(world.state(WIRE), cross(11));

    let line = wire(11, "none", "side", "none", "side");
    let mut world = world_with(&[(WIRE, line)]);
    let consumed = world.with_level(|level| {
        use_without_item(
            level,
            WIRE,
            line,
            interaction_player(true, Direction::North),
        )
    });
    assert!(!consumed);
    assert_eq!(world.state(WIRE), line);
}

#[test]
fn a_player_who_may_not_build_cannot_toggle_wire() {
    let mut world = world_with(&[(WIRE, cross(4))]);
    let consumed = world.with_level(|level| {
        use_without_item(
            level,
            WIRE,
            cross(4),
            interaction_player(false, Direction::North),
        )
    });
    assert!(!consumed);
    assert_eq!(world.state(WIRE), cross(4));
}

#[test]
fn an_update_from_below_removes_a_wire_whose_support_is_gone() {
    let mut world = world_with(&[(WIRE, cross(0))]);
    world.set(below(WIRE), air());
    world.shape_update(WIRE, Direction::Down);
    assert_eq!(world.state(WIRE), air());

    let mut world = world_with(&[(WIRE, cross(0))]);
    world.shape_update(WIRE, Direction::Down);
    assert_eq!(world.state(WIRE), cross(0));
}

#[test]
fn a_side_that_stays_connected_changes_only_its_own_property() {
    let east = relative(WIRE, Direction::East);
    let mut world = world_with(&[
        (WIRE, wire(0, "none", "side", "none", "side")),
        (east, stone()),
        (above(east), cross(0)),
        (relative(WIRE, Direction::West), cross(0)),
    ]);
    world.set(below(relative(WIRE, Direction::West)), stone());
    world.shape_update(WIRE, Direction::East);
    assert_eq!(world.state(WIRE), wire(0, "none", "up", "none", "side"));
}

#[test]
fn a_cross_recomputes_its_whole_shape_when_a_side_changes() {
    let north = relative(WIRE, Direction::North);
    let south = relative(WIRE, Direction::South);
    let east = relative(WIRE, Direction::East);
    let mut world = world_with(&[
        (WIRE, cross(0)),
        (north, cross(0)),
        (south, cross(0)),
        (east, stone()),
    ]);
    world.set(below(north), stone());
    world.set(below(south), stone());
    world.shape_update(WIRE, Direction::East);
    assert_eq!(world.state(WIRE), wire(0, "side", "none", "side", "none"));
}

#[test]
fn a_wire_feeds_up_and_the_sides_it_points_at_and_nothing_else() {
    let east = relative(WIRE, Direction::East);
    let west = relative(WIRE, Direction::West);
    let blocks = [
        (WIRE, wire(9, "none", "side", "none", "side")),
        (east, cross(0)),
        (west, cross(0)),
        (below(WIRE), stone()),
        (below(east), stone()),
        (below(west), stone()),
    ];
    let mut read = |at: BlockPos| {
        blocks
            .iter()
            .find(|(pos, _)| *pos == at)
            .map_or_else(air, |(_, state)| *state)
    };
    let state = wire(9, "none", "side", "none", "side");
    let mut context = SignalContext::emitting(&mut read);
    assert_eq!(
        crate::redstone::signal(state, WIRE, Direction::Down, &mut context),
        0
    );
    assert_eq!(
        crate::redstone::signal(state, WIRE, Direction::Up, &mut context),
        9
    );
    assert_eq!(
        crate::redstone::signal(state, WIRE, Direction::East, &mut context),
        9
    );
    assert_eq!(
        crate::redstone::signal(state, WIRE, Direction::North, &mut context),
        0
    );
    assert_eq!(
        crate::redstone::signal(
            wire(0, "none", "side", "none", "side"),
            WIRE,
            Direction::Up,
            &mut context
        ),
        0
    );
}

#[test]
fn a_wire_beside_a_source_takes_its_full_strength() {
    let east = relative(WIRE, Direction::East);
    let mut world = world_with(&[(WIRE, cross(0)), (east, redstone_block())]);
    world.neighbor_changed(WIRE, block_id("minecraft:redstone_block"));
    assert_eq!(power_of_state(world.state(WIRE)), SIGNAL_MAX);
}

#[test]
fn a_wire_beside_a_full_strength_wire_settles_one_lower() {
    let east = relative(WIRE, Direction::East);
    let mut world = world_with(&[(WIRE, cross(0)), (east, cross(SIGNAL_MAX))]);
    world.set(below(east), stone());
    world.set(relative(east, Direction::East), redstone_block());
    world.neighbor_changed(WIRE, VANILLA_BLOCK_REDSTONE_WIRE_ID);
    assert_eq!(power_of_state(world.state(WIRE)), SIGNAL_MAX - 1);
}

#[test]
fn the_incoming_signal_climbs_over_a_conductor_unless_one_is_overhead() {
    let east = relative(WIRE, Direction::East);
    let up_east = above(east);
    let mut world = world_with(&[
        (WIRE, cross(0)),
        (east, stone()),
        (up_east, cross(SIGNAL_MAX)),
    ]);
    world.set(relative(up_east, Direction::East), redstone_block());
    world.neighbor_changed(WIRE, VANILLA_BLOCK_REDSTONE_WIRE_ID);
    assert_eq!(power_of_state(world.state(WIRE)), SIGNAL_MAX - 1);

    let mut world = world_with(&[
        (WIRE, cross(0)),
        (above(WIRE), stone()),
        (east, stone()),
        (up_east, cross(SIGNAL_MAX)),
    ]);
    world.set(relative(up_east, Direction::East), redstone_block());
    world.neighbor_changed(WIRE, VANILLA_BLOCK_REDSTONE_WIRE_ID);
    assert_eq!(power_of_state(world.state(WIRE)), 0);
}

#[test]
fn the_incoming_signal_climbs_down_past_an_empty_neighbour() {
    let east = relative(WIRE, Direction::East);
    let mut world = world_with(&[
        (WIRE, cross(0)),
        (above(WIRE), stone()),
        (below(east), cross(SIGNAL_MAX)),
    ]);
    world.set(below(below(east)), stone());
    world.set(relative(below(east), Direction::East), redstone_block());
    world.neighbor_changed(WIRE, VANILLA_BLOCK_REDSTONE_WIRE_ID);
    assert_eq!(power_of_state(world.state(WIRE)), SIGNAL_MAX - 1);
}

#[test]
fn a_wire_with_nothing_under_it_is_dropped_by_a_neighbour_update() {
    let mut world = world_with(&[(WIRE, cross(SIGNAL_MAX))]);
    world.set(below(WIRE), air());
    world.neighbor_changed(WIRE, VANILLA_BLOCK_REDSTONE_WIRE_ID);
    assert_eq!(world.state(WIRE), air());
    assert!(world
        .items()
        .iter()
        .any(|(name, count)| name == "minecraft:redstone" && *count == 1));
}

#[test]
fn a_wire_connects_to_a_state_only_signal_source() {
    let east = relative(WIRE, Direction::East);
    assert_eq!(
        place(&[(below(WIRE), stone()), (east, state_only_signal_source())]),
        Some(wire(0, "none", "side", "none", "side"))
    );
    assert_eq!(
        place(&[
            (below(WIRE), stone()),
            (below(east), state_only_signal_source()),
        ]),
        Some(cross(0))
    );
}

fn power_of_state(block_state_id: i32) -> i32 {
    assert!(
        redstone_wire::is_redstone_wire(block_state_id),
        "not a wire state"
    );
    int_property(block_state_id, "power").expect("every wire state carries POWER")
}

#[test]
fn experimental_redstone_orientation_table_covers_all_48_states() {
    use crate::block_behavior::redstone_wire::experimental::{RedstoneOrientation, SideBias};
    use std::collections::BTreeSet;

    let all_dirs = [
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ];
    let mut seen = BTreeSet::new();
    for up in all_dirs {
        for front in all_dirs {
            if up == front || up == bbb_server_types::directions::opposite(front) {
                continue;
            }
            for bias in [SideBias::Left, SideBias::Right] {
                let orientation = RedstoneOrientation::of(up, front, bias);
                assert!(seen.insert(orientation.index()));
                assert_eq!(
                    RedstoneOrientation::from_index(orientation.index()),
                    orientation
                );
                assert_eq!(orientation.up(), up);
                assert_eq!(orientation.front(), front);
                assert_eq!(orientation.side_bias(), bias);
                assert_eq!(
                    orientation.directions(),
                    [
                        bbb_server_types::directions::opposite(front),
                        front,
                        orientation.side(),
                        bbb_server_types::directions::opposite(orientation.side()),
                        bbb_server_types::directions::opposite(up),
                        up,
                    ]
                );
                assert_eq!(orientation.with_mirror().side_bias(), bias.opposite());
                assert_eq!(orientation.with_front_preserve_up(up), orientation);
                assert_eq!(
                    orientation.with_front_preserve_up(bbb_server_types::directions::opposite(up)),
                    orientation
                );
            }
        }
    }
    assert_eq!(seen.len(), 48);

    // Facing East with Up=+Y and Left bias puts Left = North, Right = South.
    let east_left = RedstoneOrientation::of(Direction::Up, Direction::East, SideBias::Left);
    assert_eq!(
        east_left.directions(),
        [
            Direction::West,
            Direction::East,
            Direction::North,
            Direction::South,
            Direction::Down,
            Direction::Up,
        ]
    );
    // Facing West with Up=+Y and Left bias flips Left = South, Right = North.
    let west_left = RedstoneOrientation::of(Direction::Up, Direction::West, SideBias::Left);
    assert_eq!(
        west_left.directions(),
        [
            Direction::East,
            Direction::West,
            Direction::South,
            Direction::North,
            Direction::Down,
            Direction::Up,
        ]
    );
}

#[test]
fn experimental_redstone_wire_propagates_power_turn_off_and_repowering() {
    use bbb_server_types::save::Java26BuiltinDimension;

    let mut world = NativeWorld::new();
    world.world.set_redstone_experiments_for_test(true);

    // Build a 6-block east-west wire line over stone.
    for dx in -1..=6 {
        world.set(at(WIRE.x + dx, WIRE.y - 1, WIRE.z), stone());
    }
    for dx in 0..=4 {
        let pos = at(WIRE.x + dx, WIRE.y, WIRE.z);
        world
            .world
            .set_block(Java26BuiltinDimension::Overworld, pos, cross(0), 3);
    }

    // Power from the west end: experimental evaluator lights the whole line in one pass.
    let west_source = at(WIRE.x - 1, WIRE.y, WIRE.z);
    world.world.set_block(
        Java26BuiltinDimension::Overworld,
        west_source,
        redstone_block(),
        3,
    );
    for dx in 0..=4 {
        assert_eq!(
            power_of_state(world.state(at(WIRE.x + dx, WIRE.y, WIRE.z))),
            SIGNAL_MAX - dx
        );
    }

    // Add a weaker indirect source on the east end (a lit wire fed by a far redstone block),
    // then remove the strong west source: turn-off clears stale power and turn-on repowers
    // cleanly from the remaining east feed.
    let east_source = at(WIRE.x + 5, WIRE.y, WIRE.z);
    world.world.set_block(
        Java26BuiltinDimension::Overworld,
        east_source,
        redstone_block(),
        3,
    );
    world
        .world
        .set_block(Java26BuiltinDimension::Overworld, west_source, air(), 3);
    for dx in 0..=4 {
        assert_eq!(
            power_of_state(world.state(at(WIRE.x + dx, WIRE.y, WIRE.z))),
            SIGNAL_MAX - (4 - dx)
        );
    }

    // Remove the remaining source: all wires turn off to 0.
    world
        .world
        .set_block(Java26BuiltinDimension::Overworld, east_source, air(), 3);
    for dx in 0..=4 {
        assert_eq!(
            power_of_state(world.state(at(WIRE.x + dx, WIRE.y, WIRE.z))),
            0
        );
    }
}

#[test]
fn experimental_redstone_wire_uses_orientation_aware_neighbor_update_order() {
    use bbb_server_types::save::Java26BuiltinDimension;

    let piston = |facing: &str| {
        state_of(
            "minecraft:piston",
            &[("extended", "false"), ("facing", facing)],
        )
    };

    // Two opposing pistons below North and South wire branches compete for the
    // same shared air cell `(4, 68, 0)`. Under `minecraft:redstone_experiments`,
    // `side = Left` rotates with the incoming wire direction, so feeding from the
    // West updates North first while feeding from the East updates South first.
    let run_case = |experimental: bool, from_west: bool| -> (bool, bool) {
        let center = at(4, 70, 0);
        let mut world = NativeWorld::ticking(center);
        world.world.set_redstone_experiments_for_test(experimental);

        let feed_wire = if from_west {
            at(3, 70, 0)
        } else {
            at(5, 70, 0)
        };
        let source = if from_west {
            at(2, 70, 0)
        } else {
            at(6, 70, 0)
        };
        let north_wire = at(4, 70, -1);
        let south_wire = at(4, 70, 1);
        let north_piston = at(4, 68, -1);
        let south_piston = at(4, 68, 1);

        for x in 1..=7 {
            for z in -2..=2 {
                world.set(at(x, 69, z), stone());
                world.set(at(x, 68, z), air());
            }
        }
        world
            .world
            .set_block(Java26BuiltinDimension::Overworld, center, cross(0), 2 | 16);
        world.world.set_block(
            Java26BuiltinDimension::Overworld,
            feed_wire,
            cross(0),
            2 | 16,
        );
        world.world.set_block(
            Java26BuiltinDimension::Overworld,
            north_wire,
            cross(0),
            2 | 16,
        );
        world.world.set_block(
            Java26BuiltinDimension::Overworld,
            south_wire,
            cross(0),
            2 | 16,
        );
        world.world.set_block(
            Java26BuiltinDimension::Overworld,
            north_piston,
            piston("south"),
            2 | 16,
        );
        world.world.set_block(
            Java26BuiltinDimension::Overworld,
            south_piston,
            piston("north"),
            2 | 16,
        );

        // Power the feed wire, which propagates into `center` with front = East (from West)
        // or front = West (from East).
        world.world.set_block(
            Java26BuiltinDimension::Overworld,
            source,
            redstone_block(),
            3,
        );
        for _ in 0..5 {
            world.run_tick();
        }
        let north_extended = bool_property(world.state(north_piston), "extended").unwrap_or(false);
        let south_extended = bool_property(world.state(south_piston), "extended").unwrap_or(false);
        (north_extended, south_extended)
    };

    // Experimental evaluator: Left-biased update order follows the signal direction.
    assert_eq!(run_case(true, true), (true, false));
    assert_eq!(run_case(true, false), (false, true));

    // Default evaluator: fixed UPDATE_ORDER always favors North regardless of direction.
    assert_eq!(run_case(false, true), (true, false));
    assert_eq!(run_case(false, false), (true, false));
}
