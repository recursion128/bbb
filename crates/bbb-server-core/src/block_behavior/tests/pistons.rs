use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::relative;

use super::{state_of, NativeWorld};

const BASE: BlockPos = BlockPos { x: 0, y: 65, z: 0 };

fn piston(facing: &str, extended: bool, sticky: bool) -> i32 {
    state_of(
        if sticky {
            "minecraft:sticky_piston"
        } else {
            "minecraft:piston"
        },
        &[
            ("extended", if extended { "true" } else { "false" }),
            ("facing", facing),
        ],
    )
}

fn block(name: &str) -> i32 {
    state_of(name, &[])
}

fn block_type(name: &str) -> i32 {
    vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("{name} is a generated block"))
        .id
}

#[test]
fn powered_piston_pushes_the_block_and_extends() {
    let mut world = NativeWorld::ticking(BASE);
    let base = piston("east", false, false);
    let arm = relative(BASE, Direction::East);
    world.set(BASE, base);
    world.set(arm, block("minecraft:stone"));
    world.set(
        relative(BASE, Direction::West),
        block("minecraft:redstone_block"),
    );

    world.neighbor_changed(BASE, block_type("minecraft:redstone_block"));
    for _ in 0..5 {
        world.run_tick();
    }
    assert_eq!(
        world.state(arm),
        crate::block_behavior::piston::piston_head_state(Direction::East, false)
    );
    assert_eq!(
        world.state(relative(arm, Direction::East)),
        block("minecraft:stone")
    );
    assert_eq!(world.state(BASE), piston("east", true, false));
}

#[test]
fn a_piston_cannot_push_thirteen_blocks() {
    let mut world = NativeWorld::ticking(BASE);
    let base = piston("east", false, false);
    world.set(BASE, base);
    world.set(
        relative(BASE, Direction::West),
        block("minecraft:redstone_block"),
    );
    let mut pos = BASE;
    for _ in 0..13 {
        pos = relative(pos, Direction::East);
        world.set(pos, block("minecraft:stone"));
    }

    world.neighbor_changed(BASE, block_type("minecraft:redstone_block"));
    for _ in 0..5 {
        world.run_tick();
    }
    assert_eq!(world.state(BASE), base);
    assert_eq!(
        world.state(relative(BASE, Direction::East)),
        block("minecraft:stone")
    );
}

#[test]
fn slime_and_honey_do_not_form_a_branch() {
    let mut world = NativeWorld::ticking(BASE);
    let base = piston("east", false, false);
    let arm = relative(BASE, Direction::East);
    world.set(BASE, base);
    world.set(arm, block("minecraft:slime_block"));
    world.set(
        relative(arm, Direction::North),
        block("minecraft:honey_block"),
    );
    world.set(
        relative(BASE, Direction::West),
        block("minecraft:redstone_block"),
    );
    world.neighbor_changed(BASE, block_type("minecraft:redstone_block"));
    for _ in 0..5 {
        world.run_tick();
    }
    assert_eq!(
        world.state(relative(arm, Direction::East)),
        block("minecraft:slime_block")
    );
    assert_eq!(
        world.state(relative(arm, Direction::North)),
        block("minecraft:honey_block")
    );
}

#[test]
fn sticky_retraction_pulls_the_block_in_front_of_its_head() {
    let mut world = NativeWorld::ticking(BASE);
    let base = piston("east", true, true);
    let arm = relative(BASE, Direction::East);
    let front = relative(arm, Direction::East);
    world.set(BASE, base);
    world.set(
        arm,
        crate::block_behavior::piston::piston_head_state(Direction::East, true),
    );
    world.set(front, block("minecraft:stone"));

    world.neighbor_changed(BASE, block_type("minecraft:stone"));
    for _ in 0..5 {
        world.run_tick();
    }
    assert_eq!(world.state(BASE), piston("east", false, true));
    assert_eq!(world.state(arm), block("minecraft:stone"));
    assert_eq!(world.state(front), block("minecraft:air"));
}

/// `PistonBaseBlock.triggerEvent` (`PistonBaseBlock.java:184-191`) shape-updates
/// around the base as soon as it starts to retract, so an observer watching
/// the base pulses two ticks after the base turns into a moving piston.
#[test]
fn an_observer_sees_a_piston_base_start_to_retract() {
    let mut world = NativeWorld::ticking(BASE);
    let observer = relative(BASE, Direction::North);
    let watching = |powered: &str| {
        state_of(
            "minecraft:observer",
            &[("facing", "south"), ("powered", powered)],
        )
    };
    world.set(BASE, piston("east", true, true));
    world.set(
        relative(BASE, Direction::East),
        crate::block_behavior::piston::piston_head_state(Direction::East, true),
    );
    world.set(observer, watching("false"));

    world.neighbor_changed(BASE, block_type("minecraft:stone"));
    let mut moving_at = None;
    let mut powered_at = None;
    for tick in 1..=8 {
        world.run_tick();
        if moving_at.is_none()
            && bbb_server_types::block_queries::block_type_of(world.state(BASE))
                == Some(block_type("minecraft:moving_piston"))
        {
            moving_at = Some(tick);
        }
        if powered_at.is_none() && world.state(observer) == watching("true") {
            powered_at = Some(tick);
        }
    }
    let moving_at = moving_at.expect("the base starts to retract");
    assert_eq!(powered_at, Some(moving_at + 2));
}
