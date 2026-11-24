use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID;
use bbb_protocol::packets::BlockPos;
use uuid::Uuid;

use crate::entity::level::SculkSensorStepSource;
use crate::entity::{Entity, EntityStore};

pub(crate) fn step_on_sensor(
    entity: &Entity,
    store: Option<&mut EntityStore>,
    level: &mut LevelCtx<'_, '_>,
    state: i32,
    pos: BlockPos,
) -> Option<()> {
    let is_projectile = entity.is_projectile_class();
    let projectile_owner = if is_projectile {
        entity
            .data()
            .projectile_owner()
            .filter(|uuid| owner_is_present(entity, store.as_deref(), level, *uuid))
    } else {
        None
    };
    level.step_on_sculk_sensor(
        crate::entity::level::EntityBlockContext::new(store, Some(entity)),
        state,
        pos,
        SculkSensorStepSource {
            entity_id: entity.id(),
            uuid: entity.uuid(),
            position: entity.position(),
            is_warden: entity.entity_type_id() == VANILLA_ENTITY_TYPE_WARDEN_ID,
            is_projectile,
            projectile_owner,
        },
    )
}

fn owner_is_present(
    entity: &Entity,
    store: Option<&EntityStore>,
    level: &LevelCtx<'_, '_>,
    uuid: Uuid,
) -> bool {
    if entity.uuid() == uuid {
        return entity.removal_reason().is_none();
    }
    if let Some(id) = store.and_then(|store| store.entity_id_for_uuid(uuid)) {
        // A detached body keeps its registered UUID while it ticks.
        if store
            .and_then(|store| store.get(id))
            .is_none_or(|owner| owner.removal_reason().is_none())
        {
            return true;
        }
    }
    level.player(uuid).is_some() || level.entity_exists_in_other_dimension(uuid) == Some(true)
}

#[cfg(test)]
mod tests;
