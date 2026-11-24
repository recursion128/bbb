use super::*;
use crate::block_entity::vibration::VibrationData;
use crate::world::simulation_level::LevelCtx;

const SENSOR_POS: BlockPos = BlockPos { x: 8, y: 64, z: 8 };

fn sensor_state(calibrated: bool) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(if calibrated {
        bbb_protocol::block_types::VANILLA_BLOCK_CALIBRATED_SCULK_SENSOR_ID
    } else {
        bbb_protocol::block_types::VANILLA_BLOCK_SCULK_SENSOR_ID
    })
    .unwrap()
}

fn data(level: &LevelCtx<'_, '_>) -> VibrationData {
    level
        .chunks
        .block_entities()
        .get(SENSOR_POS)
        .unwrap()
        .sculk_sensor()
        .unwrap()
        .vibration_data()
        .clone()
}

fn standing_body(level: &mut LevelCtx<'_, '_>, calibrated: bool) -> (EntityStore, i32, Vec3d) {
    let state = sensor_state(calibrated);
    assert!(LevelCtx::set_block(
        level,
        SENSOR_POS,
        state,
        2,
        crate::neighbor_updater::UPDATE_LIMIT
    ));
    let top = level
        .block_collision_boxes(SENSOR_POS)
        .iter()
        .map(|shape| shape.max_y)
        .reduce(f64::max)
        .unwrap();
    let at = Vec3d {
        x: 8.5,
        y: f64::from(SENSOR_POS.y) + top,
        z: 8.5,
    };
    let mut store = batch_store();
    let id = crate::entity::spawn_armor_stand(&mut store, at).unwrap();
    let entity = store.get_mut(id).unwrap();
    crate::entity::set_spawn_on_ground(entity, &|pos| level.block_collision_boxes(pos));
    entity.set_shared_flag(
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
        true,
    );
    entity.set_delta_movement(Vec3d {
        x: 0.0,
        y: -0.01,
        z: 0.0,
    });
    (store, id, at)
}

#[test]
fn stored_sensors_actual_stationary_careful_tick_schedules_detached_source() {
    for calibrated in [false, true] {
        with_config(crate::tests::datapack_config(), |level| {
            let (mut store, id, at) = standing_body(level, calibrated);
            let uuid = store.get(id).unwrap().uuid();
            let before_ticks = level.chunks.scheduled_tick_keys::<BlockTickType>().count();
            assert!(matches!(
                crate::entity::tick_one_entity(&mut store, level, id),
                crate::entity::EntityTickOutcome::Complete
            ));
            assert_eq!(store.get(id).unwrap().position(), at);
            let data = data(level);
            let (candidate, tick) = data.selection_strategy().current().unwrap();
            assert_eq!(candidate.game_event, "minecraft:step");
            assert_eq!(candidate.source, Some(uuid));
            assert_eq!(candidate.projectile_owner, None);
            assert_eq!(candidate.pos, at);
            assert_eq!(candidate.distance, 0.0);
            assert_eq!(tick, level.game_time);
            assert!(data.current_vibration().is_none());
            assert_eq!(
                level.chunks.scheduled_tick_keys::<BlockTickType>().count(),
                before_ticks
            );
        });
    }
}
