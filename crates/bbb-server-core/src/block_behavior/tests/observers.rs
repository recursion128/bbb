use super::*;

const OBSERVER: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn observer(facing: &str, powered: bool) -> i32 {
    state_of(
        "minecraft:observer",
        &[
            ("facing", facing),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
}

#[test]
fn observation_face_changes_produce_and_finish_a_pulse_in_every_direction() {
    for facing in DIRECTION_VALUES {
        let off = observer(direction_serialized_name(facing), false);
        let on = observer(direction_serialized_name(facing), true);
        let mut world = NativeWorld::ticking(OBSERVER);
        world.set(OBSERVER, off);
        world.set(relative(OBSERVER, facing), stone());
        world.shape_update(OBSERVER, facing);
        for _ in 0..2 {
            world.run_tick();
        }
        assert_eq!(world.state(OBSERVER), on, "{facing:?}");
        for _ in 0..2 {
            world.run_tick();
        }
        assert_eq!(world.state(OBSERVER), off, "{facing:?}");
    }
}

#[test]
fn an_update_on_the_other_face_leaves_the_observer_unpowered() {
    let off = observer("east", false);
    let mut world = NativeWorld::ticking(OBSERVER);
    world.set(OBSERVER, off);
    world.set(relative(OBSERVER, Direction::West), stone());
    world.shape_update(OBSERVER, Direction::West);
    for _ in 0..4 {
        world.run_tick();
    }
    assert_eq!(world.state(OBSERVER), off);
}

#[test]
fn a_placed_powered_observer_recovers_unless_its_pulse_is_pending() {
    let powered = observer("east", true);
    let unpowered = observer("east", false);
    let dimension = bbb_server_types::save::Java26BuiltinDimension::Overworld;
    let mut recovered = NativeWorld::new();
    recovered
        .world
        .set_block(dimension, OBSERVER, powered, 2 | 16);
    assert_eq!(recovered.state(OBSERVER), unpowered);

    let mut pending = NativeWorld::ticking(OBSERVER);
    pending.with_level(|level| {
        level.schedule_block_tick(
            OBSERVER,
            BlockTickType::of_block_state(powered).unwrap(),
            2,
            TickPriority::Normal,
        )
    });
    pending
        .world
        .set_block(dimension, OBSERVER, powered, 2 | 16);
    assert_eq!(pending.state(OBSERVER), powered);
    for _ in 0..2 {
        pending.run_tick();
    }
    assert_eq!(pending.state(OBSERVER), unpowered);
}
