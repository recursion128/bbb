use crate::level_event::{LevelEventSink, VanillaGameEvent};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::Vec3d;

use crate::entity::store::EntityStore;
use crate::entity::RemovalReason;

pub(crate) fn kill(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, entity_id: i32) -> bool {
    let Some(entity) = store.remove(entity_id, RemovalReason::Killed) else {
        return false;
    };
    raise_entity_die(&entity, level);
    true
}

pub(crate) fn raise_entity_die(entity: &crate::entity::Entity, level: &mut LevelCtx<'_, '_>) {
    level.game_event_at(
        VanillaGameEvent::ENTITY_DIE,
        entity.position(),
        Some(entity.id()),
    );
}

/// Raises the stored victim's death event after its body is returned.
pub fn raise_entity_die_in_sink(sink: &mut LevelEventSink, position: Vec3d, entity_id: i32) {
    sink.game_event_with_source_at(
        VanillaGameEvent::ENTITY_DIE,
        position,
        Some(entity_id),
        None,
    );
}
