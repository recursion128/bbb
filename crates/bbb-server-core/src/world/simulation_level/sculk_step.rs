use super::*;
use crate::entity::level::SculkSensorStepSource;
use bbb_protocol::block_entity_types::{
    VANILLA_BLOCK_ENTITY_TYPE_CALIBRATED_SCULK_SENSOR_ID, VANILLA_BLOCK_ENTITY_TYPE_SCULK_SENSOR_ID,
};
use bbb_server_types::directions::relative;

impl LevelCtx<'_, '_> {
    pub(in crate::world) fn sculk_sensor_step(
        &mut self,
        state: i32,
        pos: BlockPos,
        source: SculkSensorStepSource,
    ) -> Option<()> {
        if !sculk_sensor_block::can_activate(state) || source.is_warden {
            return Some(());
        }
        if self
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::sculk_sensor)
            .is_none()
        {
            if matches!(
                self.chunks.block_entities().loaned_type(pos),
                Some(VANILLA_BLOCK_ENTITY_TYPE_SCULK_SENSOR_ID)
                    | Some(VANILLA_BLOCK_ENTITY_TYPE_CALIBRATED_SCULK_SENSOR_ID)
            ) {
                return None;
            }
            return Some(());
        }
        if let Some(live_state) = self.chunks.get_block_state(pos.x, pos.y, pos.z) {
            let back_signal =
                crate::block_entity::sculk_sensor::calibrated_back_direction(live_state)
                    .map(|back| LevelCtx::get_signal(self, relative(pos, back), back));
            if crate::block_entity::sculk_sensor::accepts_vibration(
                live_state,
                pos,
                pos,
                VanillaGameEvent::STEP,
                back_signal,
                Some(live_state),
            ) {
                if let Some(sensor) = self
                    .chunks
                    .block_entities_mut()
                    .get_mut(pos)
                    .and_then(BlockEntity::sculk_sensor_mut)
                {
                    crate::block_entity::vibration::schedule_vibration(
                        sensor.vibration_data_mut(),
                        self.game_time,
                        VanillaGameEvent::STEP.resource_id(),
                        Some(crate::block_entity::vibration::VibrationEntity {
                            uuid: source.uuid,
                            is_player: false,
                            is_projectile: source.is_projectile,
                            controlling_passenger_player: None,
                            projectile_owner: source.projectile_owner,
                            item_owner: None,
                        }),
                        source.position,
                        crate::block_entity::vibration::block_position_source(pos),
                    );
                }
            }
        }
        Some(())
    }
}
