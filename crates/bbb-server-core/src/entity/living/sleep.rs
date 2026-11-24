use crate::entity::Entity;
use crate::world::simulation_level::LevelCtx;
use uuid::Uuid;

impl Entity {
    pub fn sleeping_pos(&self) -> Option<bbb_protocol::packets::BlockPos> {
        self.living().and_then(|living| living.sleeping_pos)
    }

    pub fn set_sleeping_pos(&mut self, pos: Option<bbb_protocol::packets::BlockPos>) {
        let Some(living) = self.living_mut() else {
            return;
        };
        if living.sleeping_pos != pos {
            living.sleeping_pos = pos;
            self.mark_synched_dirty(
                bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID,
            );
        }
    }

    pub fn is_preventing_player_rest(
        &self,
        player: Uuid,
        player_is_enemy: bool,
        game_time: i64,
        universal_anger: bool,
    ) -> bool {
        if !crate::entity::ai::mob::spec::is_instance_of(self.entity_type_id(), "Monster") {
            return false;
        }
        if self.entity_type_id()
            != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
        {
            return true;
        }
        player_is_enemy
            && crate::entity::ai::mob::target::anger::read(self).is_some_and(|anger| {
                anger.target == Some(player)
                    || (universal_anger && anger.target.is_none() && anger.is_angry(game_time))
            })
    }
}

pub(crate) fn snap_first_sleep(entity: &mut Entity) {
    if entity.is_first_tick() {
        if let Some(pos) = entity.sleeping_pos() {
            entity.set_pos(bbb_protocol::packets::Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.6875,
                z: f64::from(pos.z) + 0.5,
            });
        }
    }
}

/// `LivingEntity.startSleeping` (`LivingEntity.java:3706-3721`) for a body
/// out of the store.
pub(crate) fn start_sleeping(
    entity: &mut Entity,
    store: &mut crate::entity::EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pos: bbb_protocol::packets::BlockPos,
) -> Option<()> {
    if let Some(occupied) = level
        .block_state_checked(pos)
        .filter(|state| crate::block_behavior::bed::is_bed(*state))
        .and_then(|state| crate::fluid::with_property(state, "occupied", "true"))
    {
        level.set_entity_block(
            crate::entity::level::EntityBlockContext::new(Some(&mut *store), Some(entity)),
            pos,
            occupied,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        );
    }
    entity.set_pose_id_with_level(
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID,
        level,
        Some(&mut *store),
    )?;
    entity.set_pos(bbb_protocol::packets::Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.6875,
        z: f64::from(pos.z) + 0.5,
    });
    store.refile_detached(entity);
    entity.set_sleeping_pos(Some(pos));
    entity.set_delta_movement(bbb_protocol::packets::Vec3d::default());
    entity.set_needs_sync(true);
    Some(())
}

pub(crate) fn tick_sleep(
    entity: &mut Entity,
    store: Option<&mut crate::entity::EntityStore>,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let Some(pos) = entity.sleeping_pos() else {
        return true;
    };
    let should_stop = if !entity.is_alive_with_health() {
        true
    } else {
        let Some(state) = level.block_state_checked(pos) else {
            return false;
        };
        !crate::block_behavior::bed::is_bed(state)
    };
    if should_stop
        && level
            .stop_sleeping(
                crate::entity::level::EntityBlockContext::new(store, None),
                entity,
            )
            .is_none()
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests;
