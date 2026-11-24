//! Hoglin hunt initiation and the visible adult cohort's shared cooldown.

use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID};

use crate::entity::ai::mob::target::TargetRef;
use crate::entity::ai::MemoryModuleType;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;

use super::{can_hunt, dont_hunt, memory, set_anger_target, visit_visible_adults};

// PiglinAi.java:154-167; StartHuntingHoglin.java:11-30.
pub(super) fn try_start(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    current: Option<TargetRef>,
    nearest: Option<TargetRef>,
    recent_hunt: bool,
) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_PIGLIN_ID
        || !matches!(entity.data(), EntityData::GeneralLiving(data) if !data.is_baby())
        || !can_hunt(entity)
        || current.is_some()
        || super::celebration(entity).is_some()
        || store
            .mob_ai(entity.id())
            .and_then(|runtime| runtime.piglin())
            .and_then(super::PiglinRuntime::ride_target)
            .is_some()
        || [
            MemoryModuleType::AngryAt,
            MemoryModuleType::HuntedRecently,
            MemoryModuleType::AdmiringItem,
        ]
        .into_iter()
        .any(|kind| memory(entity, kind))
    {
        return;
    }

    let Some(target) = nearest.filter(|_| !recent_hunt) else {
        return;
    };
    if !set_anger_target(entity, store, level, target) {
        return;
    }

    let mut peers = Vec::new();
    visit_visible_adults(entity, store, level, |peer, _| {
        if peer.entity_type_id() != VANILLA_ENTITY_TYPE_HOGLIN_ID {
            peers.push(TargetRef {
                entity_id: peer.id(),
                uuid: peer.uuid(),
            });
        }
    });
    super::combat::broadcast_anger_target(entity, store, level, target);
    for peer in peers {
        store.with_entity_mut(peer.entity_id, |body, store| {
            if body.uuid() == peer.uuid && !memory(body, MemoryModuleType::HuntedRecently) {
                dont_hunt(body, store.random());
            }
        });
    }
}
