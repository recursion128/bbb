use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_WARDEN_ID};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::Aabb;
use uuid::Uuid;

use crate::entity::level::{SculkPlayerIdentity, SculkShriekerStepSource};
use crate::entity::{Entity, EntityData, EntityStore};

pub(crate) fn step_on_shrieker(
    entity: &Entity,
    store: Option<&mut EntityStore>,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
) -> Option<()> {
    let Some(player) = try_get_player(entity, store.as_deref(), level)? else {
        return Some(());
    };
    let warden_nearby = nearby_warden(entity, store.as_deref(), pos);
    level.step_on_sculk_shrieker(
        crate::entity::level::EntityBlockContext::new(store, Some(entity)),
        pos,
        SculkShriekerStepSource {
            player,
            warden_nearby,
        },
    )
}

fn try_get_player(
    entity: &Entity,
    store: Option<&EntityStore>,
    level: &LevelCtx<'_, '_>,
) -> Option<Option<SculkPlayerIdentity>> {
    if let Some(player) = entity_player(entity) {
        return Some(Some(player));
    }
    if let Some(player) = crate::entity::direct_controlling_player_with_registry_tags(
        entity,
        store,
        &level.capture_registry_tags(),
        |id| level.player_by_entity_id(id),
    )? {
        return Some(Some(player));
    }
    let owner = if entity.is_projectile_class() {
        entity.data().projectile_owner()
    } else if let EntityData::Item(data) = entity.data() {
        data.thrower()
    } else {
        None
    };
    owner.map_or(Some(None), |uuid| resolve_owner(entity, store, level, uuid))
}

fn entity_player(entity: &Entity) -> Option<SculkPlayerIdentity> {
    (entity.entity_type_id() == VANILLA_ENTITY_TYPE_PLAYER_ID).then_some(SculkPlayerIdentity {
        uuid: entity.uuid(),
        entity_id: entity.id(),
    })
}

fn resolve_owner(
    entity: &Entity,
    store: Option<&EntityStore>,
    level: &LevelCtx<'_, '_>,
    uuid: Uuid,
) -> Option<Option<SculkPlayerIdentity>> {
    if entity.uuid() == uuid && entity.is_alive() {
        return Some(entity_player(entity));
    }
    let store = store?;
    if let Some(id) = store.entity_id_for_uuid(uuid) {
        let owner = store.get(id)?;
        if owner.is_alive() {
            return Some(entity_player(owner));
        }
    }
    if let Some(player) = level.player(uuid) {
        return Some(Some(SculkPlayerIdentity {
            uuid,
            entity_id: player.entity_id,
        }));
    }
    level
        .sculk_player_in_other_dimension(uuid)
        .map(|id| id.map(|entity_id| SculkPlayerIdentity { uuid, entity_id }))
}

fn nearby_warden(entity: &Entity, store: Option<&EntityStore>, pos: BlockPos) -> Option<bool> {
    let area = Aabb::of_size(
        Vec3d {
            x: pos.x as f64 + 0.5,
            y: pos.y as f64 + 0.5,
            z: pos.z as f64 + 0.5,
        },
        48.0,
        48.0,
        48.0,
    );
    let intersects = |entity: &Entity| {
        entity.entity_type_id() == VANILLA_ENTITY_TYPE_WARDEN_ID
            && entity.is_alive()
            && area.intersects(entity.bounding_box())
    };
    if intersects(entity) {
        return Some(true);
    }
    let store = store?;
    if store
        .ids_in_box_sections(area)
        .into_iter()
        .any(|id| store.get(id).is_some_and(intersects))
    {
        return Some(true);
    }
    if store
        .unavailable_registered_uuids()
        .any(|uuid| uuid != entity.uuid())
    {
        return None;
    }
    Some(false)
}

#[cfg(test)]
pub(crate) mod tests;
