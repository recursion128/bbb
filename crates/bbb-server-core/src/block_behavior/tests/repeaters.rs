//! `DiodeBlock` and `RepeaterBlock`.
//!
//! Every world here is built the same way: dirt under everything so the
//! `SupportType.RIGID` check passes, a repeater at [`REPEATER`] whose `FACING`
//! is north — so its **input** is one cell north and its **output** one cell
//! south — and whatever the case under test needs beside it.

use bbb_protocol::block_types::vanilla_block_type_for_resource_id;

use bbb_protocol::packets::Direction;

use crate::block_behavior::repeater::{self, TICKS_PER_DELAY_STEP};
use crate::redstone::SIGNAL_MAX;

use super::*;

/// A position no diode member reads; the three redstone members of a diode
/// answer from the block state alone.
const ORIGIN: BlockPos = BlockPos { x: 0, y: 0, z: 0 };

const REPEATER: BlockPos = at(0, 70, 0);

const INPUT: BlockPos = at(0, 70, -1);

const RIGHT_SIDE: BlockPos = at(1, 70, 0);

const LEFT_SIDE: BlockPos = at(-1, 70, 0);

fn repeater_state(facing: &str, delay: i32, locked: bool, powered: bool) -> i32 {
    state_of(
        "minecraft:repeater",
        &[
            ("delay", &delay.to_string()),
            ("facing", facing),
            ("locked", if locked { "true" } else { "false" }),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
}

fn comparator_state(facing: &str, powered: bool) -> i32 {
    state_of(
        "minecraft:comparator",
        &[
            ("facing", facing),
            ("mode", "compare"),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
}

fn redstone_block() -> i32 {
    state_of("minecraft:redstone_block", &[])
}

fn redstone_torch(lit: bool) -> i32 {
    state_of(
        "minecraft:redstone_torch",
        &[("lit", if lit { "true" } else { "false" })],
    )
}

fn block_id(name: &str) -> i32 {
    vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("{name} is not a built-in 26.1 block"))
        .id
}

/// A world with a floor under every cell the tests touch and a north-facing
/// repeater standing on it.
fn world_with_repeater(delay: i32, locked: bool, powered: bool) -> NativeWorld {
    let mut world = NativeWorld::ticking(REPEATER);
    for x in -2..=2 {
        for z in -2..=2 {
            world.set(at(x, 69, z), dirt());
        }
    }
    world.set(REPEATER, repeater_state("north", delay, locked, powered));
    world
}

// ---------------------------------------------------------------------------
// The two per-state redstone members
// ---------------------------------------------------------------------------

#[test]
fn a_powered_repeater_feeds_only_the_side_its_facing_names() {
    for (facing, fed) in [
        ("north", Direction::North),
        ("south", Direction::South),
        ("west", Direction::West),
        ("east", Direction::East),
    ] {
        let powered = repeater_state(facing, 1, false, true);
        let unpowered = repeater_state(facing, 1, false, false);
        for direction in DIRECTION_VALUES {
            let expected = if direction == fed { SIGNAL_MAX } else { 0 };
            // A diode reads neither the position nor the wire gate, so any
            // reader answers for it.
            let mut read = |_: BlockPos| air();
            let mut context = crate::redstone::SignalContext::emitting(&mut read);
            assert_eq!(
                crate::redstone::signal(powered, ORIGIN, direction, &mut context),
                expected,
                "{facing} repeater towards {direction:?}"
            );
            assert_eq!(
                crate::redstone::direct_signal(powered, ORIGIN, direction, &mut context),
                expected,
                "{facing} repeater directly towards {direction:?}"
            );
            assert_eq!(
                crate::redstone::signal(unpowered, ORIGIN, direction, &mut context),
                0,
                "unpowered {facing} repeater towards {direction:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The delay
// ---------------------------------------------------------------------------

#[test]
fn each_delay_setting_costs_two_game_ticks_before_the_repeater_switches_on() {
    for delay in 1..=4 {
        let mut world = world_with_repeater(delay, false, false);
        world.set(INPUT, redstone_block());
        world.neighbor_changed(REPEATER, block_id("minecraft:redstone_block"));

        let ticks = delay * TICKS_PER_DELAY_STEP;
        for elapsed in 1..ticks {
            world.run_tick();
            assert_eq!(
                bool_property(world.state(REPEATER), "powered"),
                Some(false),
                "delay {delay} switched on after {elapsed} ticks"
            );
        }
        world.run_tick();
        assert_eq!(
            bool_property(world.state(REPEATER), "powered"),
            Some(true),
            "delay {delay} never switched on"
        );
    }
}

#[test]
fn a_pulse_shorter_than_the_delay_still_switches_the_repeater_on_and_off_again() {
    let mut world = world_with_repeater(2, false, false);
    world.set(INPUT, redstone_block());
    world.neighbor_changed(REPEATER, block_id("minecraft:redstone_block"));

    world.set(INPUT, air());
    let ticks = 2 * TICKS_PER_DELAY_STEP;
    for _ in 0..ticks {
        world.run_tick();
    }
    assert_eq!(bool_property(world.state(REPEATER), "powered"), Some(true));

    for elapsed in 1..ticks {
        world.run_tick();
        assert_eq!(
            bool_property(world.state(REPEATER), "powered"),
            Some(true),
            "switched off early, after {elapsed} of the second delay"
        );
    }
    world.run_tick();
    assert_eq!(bool_property(world.state(REPEATER), "powered"), Some(false));
}

// ---------------------------------------------------------------------------
// The lock
// ---------------------------------------------------------------------------

#[test]
fn a_side_diode_prevents_repeater_output_and_suppresses_a_pending_pulse() {
    for side in [RIGHT_SIDE, LEFT_SIDE] {
        // The side repeater has to be *feeding* this one, so its own `FACING`
        // is the direction it reads its input from, i.e. away from here.
        let facing = if side == RIGHT_SIDE { "east" } else { "west" };
        let mut world = world_with_repeater(1, false, false);
        world.set(INPUT, redstone_block());
        world.set(
            at(if side == RIGHT_SIDE { 2 } else { -2 }, 70, 0),
            redstone_block(),
        );
        world.set(side, repeater_state(facing, 1, false, true));
        world.neighbor_changed(REPEATER, block_id("minecraft:redstone_block"));
        for _ in 0..TICKS_PER_DELAY_STEP {
            world.run_tick();
        }
        assert_eq!(bool_property(world.state(REPEATER), "powered"), Some(false));

        let mut world = world_with_repeater(1, false, false);
        world.set(INPUT, redstone_block());
        world.set(side, repeater_state(facing, 1, false, false));
        world.neighbor_changed(REPEATER, block_id("minecraft:redstone_block"));
        for _ in 0..TICKS_PER_DELAY_STEP {
            world.run_tick();
        }
        assert_eq!(bool_property(world.state(REPEATER), "powered"), Some(true));
    }

    let mut world = world_with_repeater(1, false, false);
    world.set(INPUT, redstone_block());
    world.neighbor_changed(REPEATER, block_id("minecraft:redstone_block"));
    world.set(at(2, 70, 0), redstone_block());
    world.set(RIGHT_SIDE, repeater_state("east", 1, false, true));
    for _ in 0..TICKS_PER_DELAY_STEP {
        world.run_tick();
    }
    assert_eq!(bool_property(world.state(REPEATER), "powered"), Some(false));
}

#[test]
fn only_a_diode_can_lock_a_repeater_and_a_torch_beside_one_does_nothing() {
    let mut world = world_with_repeater(1, false, false);
    world.set(INPUT, redstone_block());
    world.set(RIGHT_SIDE, redstone_torch(true));
    // The torch really is emitting towards the repeater; it is the
    // `onlyDiodes` gate and not a missing signal that ignores it.
    assert_eq!(
        crate::redstone::signal(
            redstone_torch(true),
            ORIGIN,
            Direction::East,
            &mut crate::redstone::SignalContext::emitting(&mut |_: BlockPos| air())
        ),
        SIGNAL_MAX
    );
    world.neighbor_changed(REPEATER, block_id("minecraft:redstone_torch"));
    for _ in 0..TICKS_PER_DELAY_STEP {
        world.run_tick();
    }
    assert_eq!(bool_property(world.state(REPEATER), "powered"), Some(true));

    // A powered comparator on the same side is a diode, so its output
    // signal is what locks the repeater. The integer lives on the block
    // entity, not the POWERED property.
    let mut world = world_with_repeater(1, false, false);
    world.set(INPUT, redstone_block());
    world.set(at(2, 70, 0), redstone_block());
    world.set(RIGHT_SIDE, comparator_state("east", true));
    world.with_level(|level| {
        assert!(level.set_comparator_block_entity_output_signal(RIGHT_SIDE, 15))
    });
    world.neighbor_changed(REPEATER, block_id("minecraft:comparator"));
    for _ in 0..TICKS_PER_DELAY_STEP {
        world.run_tick();
    }
    assert_eq!(bool_property(world.state(REPEATER), "powered"), Some(false));
}

// ---------------------------------------------------------------------------
// `updateShape`
// ---------------------------------------------------------------------------

#[test]
fn locked_is_recomputed_off_the_axis_and_left_alone_along_it() {
    // A repeater whose stored `LOCKED` is stale in both directions, so neither
    // assertion can pass by the property already holding the right value.
    let mut world = world_with_repeater(1, false, false);
    world.set(RIGHT_SIDE, repeater_state("east", 1, false, true));
    world.shape_update(REPEATER, Direction::East);
    assert_eq!(bool_property(world.state(REPEATER), "locked"), Some(true));

    let mut world = world_with_repeater(1, false, false);
    world.set(RIGHT_SIDE, repeater_state("east", 1, false, true));
    world.shape_update(REPEATER, Direction::North);
    assert_eq!(bool_property(world.state(REPEATER), "locked"), Some(false));
    world.shape_update(REPEATER, Direction::South);
    assert_eq!(bool_property(world.state(REPEATER), "locked"), Some(false));

    // Vertical is off the axis too, because `FACING` is horizontal.
    world.shape_update(REPEATER, Direction::Up);
    assert_eq!(bool_property(world.state(REPEATER), "locked"), Some(true));

    // And the unlocking direction is the same one: the side went away.
    let mut world = world_with_repeater(1, true, false);
    world.shape_update(REPEATER, Direction::West);
    assert_eq!(bool_property(world.state(REPEATER), "locked"), Some(false));
}

#[test]
fn losing_the_block_below_pops_the_repeater_and_only_from_below() {
    let mut world = world_with_repeater(1, false, false);
    world.set(at(0, 69, 0), air());
    world.shape_update(REPEATER, Direction::Down);
    assert_eq!(world.state(REPEATER), air());

    let mut world = world_with_repeater(1, false, false);
    world.set(at(0, 69, 0), air());
    world.shape_update(REPEATER, Direction::East);
    assert!(repeater::is_repeater(world.state(REPEATER)));

    // A fence is the `RIGID` counter-example, and it is a real one: the same
    // block supports a torch.
    let mut world = world_with_repeater(1, false, false);
    world.set(at(0, 69, 0), default_state("minecraft:oak_fence"));
    world.shape_update(REPEATER, Direction::Down);
    assert_eq!(world.state(REPEATER), air());
}

#[test]
fn a_repeater_with_nothing_under_it_drops_itself_on_a_neighbour_update() {
    let mut world = world_with_repeater(1, false, false);
    world.set(at(0, 69, 0), air());
    world.neighbor_changed(REPEATER, block_id("minecraft:air"));

    assert_eq!(world.state(REPEATER), air());
    assert_eq!(world.items(), [("minecraft:repeater".to_owned(), 1)]);
}

// ---------------------------------------------------------------------------
// The right click
// ---------------------------------------------------------------------------

#[test]
fn a_right_click_cycles_the_delay_and_a_player_who_may_not_build_is_refused() {
    let mut world = world_with_repeater(1, false, false);
    for expected in [2, 3, 4, 1] {
        let state = world.state(REPEATER);
        let consumed = world.with_level(|level| {
            use_without_item(
                level,
                REPEATER,
                state,
                interaction_player(true, Direction::North),
            )
        });
        assert!(consumed);
        assert_eq!(int_property(world.state(REPEATER), "delay"), Some(expected));
    }

    let mut world = world_with_repeater(1, false, false);
    let state = world.state(REPEATER);
    let consumed = world.with_level(|level| {
        use_without_item(
            level,
            REPEATER,
            state,
            interaction_player(false, Direction::North),
        )
    });
    assert!(!consumed);
    assert_eq!(int_property(world.state(REPEATER), "delay"), Some(1));
}
