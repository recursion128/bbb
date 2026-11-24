use super::*;
use crate::block_entity::vibration::{VibrationData, VibrationInfo};
use crate::entity::level::SculkSensorStepSource;
use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;

const SENSOR: BlockPos = BlockPos { x: 8, y: 64, z: 8 };
const BACK: BlockPos = BlockPos { z: 9, ..SENSOR };

fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
    vanilla_block_state_id_by_name_and_properties(
        name,
        &properties
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect(),
    )
    .unwrap()
}

fn sensor_state(calibrated: bool, phase: &str) -> i32 {
    let mut properties = vec![
        ("sculk_sensor_phase", phase),
        ("power", "0"),
        ("waterlogged", "false"),
    ];
    if calibrated {
        properties.push(("facing", "north"));
    }
    state(
        if calibrated {
            "minecraft:calibrated_sculk_sensor"
        } else {
            "minecraft:sculk_sensor"
        },
        &properties,
    )
}

fn install(level: &mut LevelCtx<'_, '_>, calibrated: bool, phase: &str) -> i32 {
    let state = sensor_state(calibrated, phase);
    assert!(LevelCtx::set_block(
        level,
        SENSOR,
        state,
        2,
        crate::neighbor_updater::UPDATE_LIMIT
    ));
    state
}

fn source() -> SculkSensorStepSource {
    SculkSensorStepSource {
        entity_id: 41,
        uuid: Uuid::from_u128(123),
        position: Vec3d {
            x: 8.75,
            y: 64.75,
            z: 8.5,
        },
        is_warden: false,
        is_projectile: false,
        projectile_owner: None,
    }
}

fn data(level: &LevelCtx<'_, '_>) -> VibrationData {
    level
        .chunks
        .block_entities()
        .get(SENSOR)
        .unwrap()
        .sculk_sensor()
        .unwrap()
        .vibration_data()
        .clone()
}

fn seeded_data(level: &mut LevelCtx<'_, '_>) -> VibrationData {
    let data = level
        .chunks
        .block_entities_mut()
        .get_mut(SENSOR)
        .unwrap()
        .sculk_sensor_mut()
        .unwrap()
        .vibration_data_mut_for_test();
    let existing = VibrationInfo {
        game_event: VanillaGameEvent::BLOCK_PLACE.resource_id(),
        distance: 50.0,
        pos: Vec3d {
            x: 40.0,
            y: 64.0,
            z: 8.0,
        },
        source: Some(Uuid::from_u128(77)),
        projectile_owner: Some(Uuid::from_u128(78)),
    };
    data.set_current_vibration(Some(existing));
    data.set_travel_time_in_ticks(7);
    data.set_reload_vibration_particle(true);
    data.selection_strategy_mut().add_candidate(existing, 10);
    data.clone()
}

#[test]
fn ground_sensors_direct_step_saves_source_and_position() {
    for projectile in [false, true] {
        with_config(crate::tests::datapack_config(), |level| {
            let state = install(level, false, "inactive");
            let mut source = source();
            source.is_projectile = projectile;
            source.projectile_owner = projectile.then_some(Uuid::from_u128(124));
            assert_eq!(
                LevelCtx::step_on_sculk_sensor(
                    level,
                    crate::entity::level::EntityBlockContext::new(None, None),
                    state,
                    SENSOR,
                    source
                ),
                Some(())
            );
            let vibration = data(level);
            let (candidate, _) = vibration.selection_strategy().current().unwrap();
            assert_eq!(candidate.game_event, VanillaGameEvent::STEP.resource_id());
            assert_eq!(candidate.source, Some(source.uuid));
            assert_eq!(
                candidate.projectile_owner,
                projectile.then_some(Uuid::from_u128(124))
            );
            assert_eq!(candidate.pos, source.position);
            assert_eq!(
                candidate.distance,
                (0.25_f64 * 0.25 + 0.25 * 0.25).sqrt() as f32
            );
            assert_eq!(level.block_state(SENSOR), state);
        });
    }
}

#[test]
fn ground_sensors_contact_and_live_phase_gates_are_known_before_owner_reads() {
    for (contact, live, warden) in [
        ("active", "inactive", false),
        ("cooldown", "inactive", false),
        ("inactive", "active", false),
        ("inactive", "cooldown", false),
        ("inactive", "inactive", true),
    ] {
        with_config(crate::tests::datapack_config(), |level| {
            install(level, false, live);
            let before = seeded_data(level);
            let mut source = source();
            source.is_warden = warden;
            assert_eq!(
                LevelCtx::step_on_sculk_sensor(
                    level,
                    crate::entity::level::EntityBlockContext::new(None, None),
                    sensor_state(false, contact),
                    SENSOR,
                    source
                ),
                Some(())
            );
            assert_eq!(data(level), before);
            assert_eq!(level.block_state(SENSOR), sensor_state(false, live));
        });
    }
}

fn comparator_input(level: &mut LevelCtx<'_, '_>, output: Option<i32>) {
    let comparator = state(
        "minecraft:comparator",
        &[
            ("facing", "south"),
            ("mode", "compare"),
            ("powered", "true"),
        ],
    );
    assert!(LevelCtx::set_block(
        level,
        BACK,
        comparator,
        2,
        crate::neighbor_updater::UPDATE_LIMIT
    ));
    if let Some(output) = output {
        level
            .chunks
            .block_entities_mut()
            .get_mut(BACK)
            .unwrap()
            .comparator_mut()
            .unwrap()
            .set_output_signal(output);
    } else {
        level.chunks.block_entities_mut().remove(BACK);
    }
}

#[test]
fn ground_sensors_calibrated_input_zero_step_one_and_mismatch_two_are_resolved() {
    for output in [Some(0), Some(1), Some(2), None] {
        with_config(crate::tests::datapack_config(), |level| {
            let state = install(level, true, "inactive");
            comparator_input(level, output);
            let source = source();
            assert_eq!(
                LevelCtx::step_on_sculk_sensor(
                    level,
                    crate::entity::level::EntityBlockContext::new(None, None),
                    state,
                    SENSOR,
                    source
                ),
                Some(())
            );
            assert_eq!(
                data(level).selection_strategy().current().is_some(),
                output != Some(2)
            );
        });
    }
}

#[test]
fn ground_sensors_direct_contact_bypasses_running_vibration_and_listener_radius() {
    with_config(crate::tests::datapack_config(), |level| {
        let state = install(level, false, "inactive");
        let before = seeded_data(level);
        let mut source = source();
        source.position.x = 30.5;
        assert_eq!(
            LevelCtx::step_on_sculk_sensor(
                level,
                crate::entity::level::EntityBlockContext::new(None, None),
                state,
                SENSOR,
                source
            ),
            Some(())
        );
        let after = data(level);
        assert_eq!(after.current_vibration(), before.current_vibration());
        assert_eq!(after.travel_time_in_ticks(), 7);
        assert!(after.should_reload_vibration_particle());
        let candidate = after.selection_strategy().current().unwrap().0;
        assert_eq!(candidate.source, Some(source.uuid));
        assert!(candidate.distance > 8.0);
    });
}
