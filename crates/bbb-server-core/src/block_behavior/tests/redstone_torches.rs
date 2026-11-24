use bbb_protocol::block_types::vanilla_block_type_for_resource_id;

use crate::block_behavior::redstone_torch::{MAX_RECENT_TOGGLES, RESTART_DELAY, TOGGLE_DELAY};
use crate::level_event::LEVEL_EVENT_REDSTONE_TORCH_BURNOUT;
use crate::redstone::{SignalContext, SIGNAL_MAX};

use super::*;

const TORCH: BlockPos = at(0, 64, 0);

macro_rules! air_context {
    () => {
        &mut SignalContext::emitting(&mut |_: BlockPos| air())
    };
}

fn torch(lit: bool) -> i32 {
    state_of(
        "minecraft:redstone_torch",
        &[("lit", if lit { "true" } else { "false" })],
    )
}

fn wall_torch(facing: &str, lit: bool) -> i32 {
    state_of(
        "minecraft:redstone_wall_torch",
        &[
            ("facing", facing),
            ("lit", if lit { "true" } else { "false" }),
        ],
    )
}

fn block_id(name: &str) -> i32 {
    vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("{name} is not a built-in 26.1 block"))
        .id
}

fn redstone_block() -> i32 {
    state_of("minecraft:redstone_block", &[])
}

#[test]
fn a_standing_torch_feeds_every_side_but_the_one_it_points_at() {
    for direction in DIRECTION_VALUES {
        let expected = if direction == Direction::Up {
            0
        } else {
            SIGNAL_MAX
        };
        assert_eq!(
            crate::redstone::signal(torch(true), TORCH, direction, air_context!()),
            expected,
            "lit torch towards {direction:?}"
        );
        assert_eq!(
            crate::redstone::signal(torch(false), TORCH, direction, air_context!()),
            0,
            "unlit torch towards {direction:?}"
        );
    }
}

#[test]
fn a_wall_torch_feeds_every_side_but_its_own_facing() {
    for (facing, dark) in [
        ("north", Direction::North),
        ("south", Direction::South),
        ("west", Direction::West),
        ("east", Direction::East),
    ] {
        for direction in DIRECTION_VALUES {
            let expected = if direction == dark { 0 } else { SIGNAL_MAX };
            assert_eq!(
                crate::redstone::signal(wall_torch(facing, true), TORCH, direction, air_context!()),
                expected,
                "{facing} wall torch towards {direction:?}"
            );
        }
        assert_eq!(
            crate::redstone::signal(
                wall_torch(facing, true),
                TORCH,
                Direction::Up,
                air_context!()
            ),
            SIGNAL_MAX
        );
    }
}

#[test]
fn a_torch_directly_powers_only_the_cell_above_it() {
    for state in [torch(true), wall_torch("north", true)] {
        for direction in DIRECTION_VALUES {
            let expected = if direction == Direction::Down {
                SIGNAL_MAX
            } else {
                0
            };
            assert_eq!(
                crate::redstone::direct_signal(state, TORCH, direction, air_context!()),
                expected,
                "{state} towards {direction:?}"
            );
        }
    }
    assert_eq!(
        crate::redstone::direct_signal(torch(false), TORCH, Direction::Down, air_context!()),
        0
    );
}

#[test]
fn a_standing_torch_listens_to_the_cell_below_it_only() {
    let mut world = NativeWorld::ticking(TORCH);
    world.set(TORCH, torch(true));
    world.set(below(TORCH), stone());
    world.set(relative(TORCH, Direction::East), redstone_block());
    world.neighbor_changed(TORCH, block_id("minecraft:redstone_block"));
    for _ in 0..TOGGLE_DELAY {
        world.run_tick();
    }
    assert_eq!(world.state(TORCH), torch(true));

    world.set(below(TORCH), redstone_block());
    world.neighbor_changed(TORCH, block_id("minecraft:redstone_block"));
    for _ in 0..TOGGLE_DELAY {
        world.run_tick();
    }
    assert_eq!(world.state(TORCH), torch(false));
}

#[test]
fn a_wall_torch_listens_to_the_wall_behind_it() {
    for (facing, wall) in [
        ("north", Direction::South),
        ("south", Direction::North),
        ("west", Direction::East),
        ("east", Direction::West),
    ] {
        let mut world = NativeWorld::ticking(TORCH);
        world.set(TORCH, wall_torch(facing, true));
        for direction in DIRECTION_VALUES {
            if direction != wall {
                world.set(relative(TORCH, direction), redstone_block());
            }
        }
        world.neighbor_changed(TORCH, block_id("minecraft:redstone_block"));

        for _ in 0..TOGGLE_DELAY {
            world.run_tick();
        }
        assert_eq!(world.state(TORCH), wall_torch(facing, true));
        world.set(relative(TORCH, wall), redstone_block());
        world.neighbor_changed(TORCH, block_id("minecraft:redstone_block"));
        for _ in 0..TOGGLE_DELAY {
            world.run_tick();
        }
        assert_eq!(world.state(TORCH), wall_torch(facing, false));
    }
}

#[test]
fn a_torch_inverts_two_ticks_after_the_update_that_armed_it() {
    let mut world = NativeWorld::ticking(TORCH);
    world.set(TORCH, torch(true));
    world.set(below(TORCH), redstone_block());
    world.neighbor_changed(TORCH, block_id("minecraft:redstone_block"));

    for elapsed in 1..TOGGLE_DELAY {
        world.run_tick();
        assert_eq!(world.state(TORCH), torch(true), "still lit after {elapsed}");
    }
    world.run_tick();
    assert_eq!(world.state(TORCH), torch(false));

    world.set(below(TORCH), stone());
    world.neighbor_changed(TORCH, block_id("minecraft:stone"));
    for _ in 0..TOGGLE_DELAY {
        world.run_tick();
    }
    assert_eq!(world.state(TORCH), torch(true));
}

#[test]
fn a_torch_ignores_an_input_pulse_shorter_than_its_delay() {
    let mut world = NativeWorld::ticking(TORCH);
    world.set(TORCH, torch(true));
    world.set(below(TORCH), redstone_block());
    world.neighbor_changed(TORCH, block_id("minecraft:redstone_block"));
    world.set(below(TORCH), stone());
    world.neighbor_changed(TORCH, block_id("minecraft:stone"));

    for _ in 0..TOGGLE_DELAY * 2 {
        world.run_tick();
    }
    assert_eq!(world.state(TORCH), torch(true));
}

fn force_one_toggle(world: &mut NativeWorld) {
    world.set(below(TORCH), redstone_block());
    world.neighbor_changed(TORCH, block_id("minecraft:redstone_block"));
    for _ in 0..TOGGLE_DELAY {
        world.run_tick();
    }
    world.set(below(TORCH), stone());
    world.neighbor_changed(TORCH, block_id("minecraft:stone"));
    for _ in 0..TOGGLE_DELAY {
        world.run_tick();
    }
}

#[test]
fn the_eighth_forced_toggle_inside_the_window_burns_the_torch_out() {
    let mut world = NativeWorld::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world.world, 1, "observer", 1);
    world.set(TORCH, torch(true));
    world.set(below(TORCH), stone());

    for cycle in 1..MAX_RECENT_TOGGLES {
        force_one_toggle(&mut world);
        assert_eq!(
            world.state(TORCH),
            torch(true),
            "cycle {cycle} still relights"
        );
    }

    world.set(below(TORCH), redstone_block());
    world.neighbor_changed(TORCH, block_id("minecraft:redstone_block"));
    let mut output = Vec::new();
    for _ in 0..TOGGLE_DELAY {
        output.extend(world.world.tick(Vec::new()).events);
    }
    assert_eq!(world.state(TORCH), torch(false));
    assert!(output.iter().any(|event| matches!(event,
        crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::LevelEvent(event), .. }
        if event.event_type == LEVEL_EVENT_REDSTONE_TORCH_BURNOUT && event.pos == TORCH && event.data == 0
    )));

    world.set(below(TORCH), stone());
    world.neighbor_changed(TORCH, block_id("minecraft:stone"));
    for _ in 0..TOGGLE_DELAY * 2 {
        world.run_tick();
    }
    assert_eq!(
        world.state(TORCH),
        torch(false),
        "burnt out and staying dark"
    );
}

#[test]
fn a_burnt_out_torch_files_its_restart_a_hundred_and_sixty_ticks_out() {
    let mut world = NativeWorld::ticking(TORCH);
    world.set(TORCH, torch(true));
    world.set(below(TORCH), stone());
    for _ in 1..MAX_RECENT_TOGGLES {
        force_one_toggle(&mut world);
    }
    world.set(below(TORCH), redstone_block());
    world.neighbor_changed(TORCH, block_id("minecraft:redstone_block"));
    for _ in 0..TOGGLE_DELAY {
        world.run_tick();
    }
    let burnt_out_at = world.world.game_time();
    assert_eq!(world.state(TORCH), torch(false));

    world.set(below(TORCH), stone());
    world.neighbor_changed(TORCH, block_id("minecraft:stone"));
    while world.world.game_time() < burnt_out_at + i64::from(RESTART_DELAY) - 1 {
        world.run_tick();
    }
    assert_eq!(world.state(TORCH), torch(false));
    world.run_tick();
    assert_eq!(
        world.world.game_time(),
        burnt_out_at + i64::from(RESTART_DELAY)
    );
    assert_eq!(world.state(TORCH), torch(true), "the torch comes back");
}

#[test]
fn a_standing_torch_pops_when_the_block_under_it_goes() {
    let mut world = NativeWorld::ticking(TORCH);
    world.set(TORCH, torch(true));
    world.set(below(TORCH), stone());
    assert_eq!(
        can_survive(torch(true), TORCH, vanilla_tags(), &mut |pos| world
            .state(pos)),
        Some(true)
    );

    world.set(below(TORCH), air());
    assert_eq!(
        can_survive(torch(true), TORCH, vanilla_tags(), &mut |pos| world
            .state(pos)),
        Some(false)
    );
    world.shape_update(TORCH, Direction::North);
    assert_eq!(world.state(TORCH), torch(true));
    world.shape_update(TORCH, Direction::Down);
    assert_eq!(world.state(TORCH), air());
}

#[test]
fn a_wall_torch_pops_when_its_wall_goes() {
    let mut world = NativeWorld::ticking(TORCH);
    let state = wall_torch("east", true);
    let wall = relative(TORCH, Direction::West);
    world.set(TORCH, state);
    world.set(wall, stone());
    assert_eq!(
        can_survive(state, TORCH, vanilla_tags(), &mut |pos| world.state(pos)),
        Some(true)
    );

    world.set(wall, air());
    assert_eq!(
        can_survive(state, TORCH, vanilla_tags(), &mut |pos| world.state(pos)),
        Some(false)
    );
    world.shape_update(TORCH, Direction::Down);
    assert_eq!(world.state(TORCH), state, "the floor is not its support");
    world.shape_update(TORCH, Direction::West);
    assert_eq!(world.state(TORCH), air());
}
