use super::native_world::NativeWorld;
use super::*;

const HEART: BlockPos = BlockPos { x: 3, y: 70, z: -4 };

fn heart(axis: &str, state: &str) -> i32 {
    state_of(
        "minecraft:creaking_heart",
        &[
            ("axis", axis),
            ("creaking_heart_state", state),
            ("natural", "false"),
        ],
    )
}

fn pale_oak_log(axis: &str) -> i32 {
    state_of("minecraft:pale_oak_log", &[("axis", axis)])
}

fn axis_directions(axis: &str) -> [Direction; 2] {
    match axis {
        "x" => [Direction::West, Direction::East],
        "y" => [Direction::Down, Direction::Up],
        "z" => [Direction::North, Direction::South],
        _ => unreachable!(),
    }
}

fn world_with_logs(axis: &str, state: &str, day_phase: i64) -> NativeWorld {
    let mut world = NativeWorld::new();
    world.world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(day_phase),
        &mut Vec::new(),
    );
    world.set(HEART, heart(axis, state));
    for direction in axis_directions(axis) {
        world.set(relative(HEART, direction), pale_oak_log(axis));
    }
    world
}

fn dispatch_tick(world: &mut NativeWorld) {
    let state = world.state(HEART);
    world.with_level(|level| tick_block(level, HEART, state));
}

#[test]
fn a_heart_needs_both_logs_along_its_axis() {
    for axis in ["x"] {
        let mut valid = world_with_logs(axis, "uprooted", 12_600);
        dispatch_tick(&mut valid);
        assert_eq!(valid.state(HEART), heart(axis, "awake"), "{axis}");

        for invalid_end in axis_directions(axis) {
            let mut missing = world_with_logs(axis, "uprooted", 12_600);
            missing.set(relative(HEART, invalid_end), stone());
            dispatch_tick(&mut missing);
            assert_eq!(missing.state(HEART), heart(axis, "uprooted"), "{axis}");

            let mut wrong_axis = world_with_logs(axis, "uprooted", 12_600);
            let other_axis = match axis {
                "x" => "y",
                "y" => "z",
                "z" => "x",
                _ => unreachable!(),
            };
            wrong_axis.set(relative(HEART, invalid_end), pale_oak_log(other_axis));
            dispatch_tick(&mut wrong_axis);
            assert_eq!(wrong_axis.state(HEART), heart(axis, "uprooted"), "{axis}");
        }
    }
}

#[test]
fn overworld_day_boundaries_select_dormant_and_awake() {
    for (phase, next) in [
        (12_599, "dormant"),
        (12_600, "awake"),
        (23_400, "awake"),
        (23_401, "dormant"),
    ] {
        let mut world = world_with_logs("y", "uprooted", phase);
        dispatch_tick(&mut world);
        let expected = heart("y", next);
        assert_eq!(world.state(HEART), expected, "phase {phase}");
    }
}

#[test]
fn a_standing_heart_tick_leaves_its_state_alone() {
    for state in ["awake", "dormant"] {
        let mut world = world_with_logs("z", state, 12_600);
        dispatch_tick(&mut world);
        assert_eq!(world.state(HEART), heart("z", state));
    }
}

#[test]
fn the_analog_output_reads_the_object_for_every_state_but_uprooted() {
    let mut uprooted = NativeWorld::new();
    uprooted.set(HEART, heart("y", "uprooted"));
    assert_eq!(
        uprooted.with_level(|level| level.analog_output_signal(HEART, Direction::North)),
        0
    );

    for state in ["awake", "dormant"] {
        let mut world = NativeWorld::new();
        world.set(HEART, heart("y", state));
        let type_id = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID;
        let position = bbb_protocol::packets::Vec3d {
            x: f64::from(HEART.x) + 13.5,
            y: f64::from(HEART.y) + 0.5,
            z: f64::from(HEART.z) + 0.5,
        };
        let (_, uuid) = world.world.spawn_stored_entity_for_test(
            bbb_server_types::save::Java26BuiltinDimension::Overworld,
            type_id,
            position,
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(type_id).unwrap(),
            ),
        );
        let heart = world
            .world
            .level_block_entity_for_test(
                bbb_server_types::save::Java26BuiltinDimension::Overworld,
                HEART,
            )
            .unwrap()
            .creaking_heart_mut()
            .unwrap();
        heart.bind_protector(uuid);
        heart.server_tick_analog(HEART, Some(position));
        assert_eq!(
            world.with_level(|level| level.analog_output_signal(HEART, Direction::North)),
            9,
            "{state}"
        );
    }
}
