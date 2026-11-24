use bbb_protocol::block_types::VANILLA_BLOCK_REDSTONE_BLOCK_ID;

use super::*;

const LAMP: BlockPos = at(4, 70, -9);

fn redstone_lamp(lit: bool) -> i32 {
    state_of(
        "minecraft:redstone_lamp",
        &[("lit", if lit { "true" } else { "false" })],
    )
}

fn redstone_block() -> i32 {
    state_of("minecraft:redstone_block", &[])
}

fn power_cell() -> BlockPos {
    relative(LAMP, Direction::East)
}

#[test]
fn a_lamp_placed_into_a_powered_cell_lands_already_lit() {
    let powered = redstone_lamp::state_for_placement(
        LAMP,
        &mut |pos| {
            if pos == power_cell() {
                redstone_block()
            } else {
                air()
            }
        },
        Default::default(),
    );
    assert_eq!(powered, redstone_lamp(true));

    let unpowered = redstone_lamp::state_for_placement(LAMP, &mut |_| air(), Default::default());
    assert_eq!(unpowered, redstone_lamp(false));
}

#[test]
fn a_lamp_lights_up_the_moment_a_neighbour_powers_it() {
    let mut world = NativeWorld::ticking(LAMP);
    world.set(LAMP, redstone_lamp(false));
    world.set(power_cell(), redstone_block());

    world.neighbor_changed(LAMP, VANILLA_BLOCK_REDSTONE_BLOCK_ID);

    assert_eq!(world.state(LAMP), redstone_lamp(true));
}

#[test]
fn a_lit_lamp_that_loses_its_power_schedules_a_tick_instead_of_going_dark() {
    let mut world = NativeWorld::ticking(LAMP);
    world.set(LAMP, redstone_lamp(true));

    world.neighbor_changed(LAMP, VANILLA_BLOCK_REDSTONE_BLOCK_ID);

    assert_eq!(
        world.state(LAMP),
        redstone_lamp(true),
        "the lamp is still lit when the update lands"
    );

    for elapsed in 1..redstone_lamp::TICK_DELAY {
        world.run_tick();
        assert_eq!(
            world.state(LAMP),
            redstone_lamp(true),
            "still lit after {elapsed} tick(s)"
        );
    }
    world.run_tick();
    assert_eq!(world.state(LAMP), redstone_lamp(false));
}

#[test]
fn the_pending_tick_leaves_the_lamp_lit_when_the_power_came_back() {
    let mut world = NativeWorld::ticking(LAMP);
    world.set(LAMP, redstone_lamp(true));
    world.neighbor_changed(LAMP, VANILLA_BLOCK_REDSTONE_BLOCK_ID);

    world.run_tick();
    world.set(power_cell(), redstone_block());
    world.neighbor_changed(LAMP, VANILLA_BLOCK_REDSTONE_BLOCK_ID);

    for _ in 0..redstone_lamp::TICK_DELAY {
        world.run_tick();
    }
    assert_eq!(
        world.state(LAMP),
        redstone_lamp(true),
        "the pending tick found the signal back and left the lamp alone"
    );
}
