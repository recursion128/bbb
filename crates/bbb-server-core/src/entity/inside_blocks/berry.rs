use crate::world::simulation_level::LevelCtx;
use std::collections::BTreeSet;

use crate::block_behavior::sweet_berry_bush;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
use bbb_protocol::packets::Vec3d;

use crate::entity::passenger::{
    direct_controlling_living_with_registry_tags, DirectLivingController,
};
use crate::entity::{Entity, EntityStore};

pub(crate) fn movement(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<Vec3d> {
    let tags = level.capture_registry_tags();
    let controller = |body: &Entity| {
        direct_controlling_living_with_registry_tags(body, Some(store), &tags, |id| {
            level.player_by_entity_id(id)
        })
    };
    let direct = controller(entity)?;
    let mut current = entity;
    let mut seen = BTreeSet::new();
    let authoritative = loop {
        if !seen.insert(current.id()) {
            return None;
        }
        if current.entity_type_id() == VANILLA_ENTITY_TYPE_PLAYER_ID {
            break true;
        }
        match controller(current)? {
            Some(DirectLivingController::Player(_)) => break true,
            Some(DirectLivingController::Stored(id)) => {
                current = if id == entity.id() {
                    entity
                } else {
                    store.get(id)?
                };
            }
            None => break false,
        }
    };
    if !authoritative {
        let old = entity.old_position();
        let position = entity.position();
        return Some(Vec3d {
            x: old.x - position.x,
            y: old.y - position.y,
            z: old.z - position.z,
        });
    }
    if let Some(DirectLivingController::Player(identity)) = direct {
        if entity.is_alive() && entity.living().is_none_or(|living| living.health() > 0.0) {
            let player = level.player_by_entity_id(identity.entity_id)?;
            return (player.uuid == identity.uuid && player.entity_id == identity.entity_id)
                .then_some(player.known_movement);
        }
    }
    if entity.entity_type_id() == VANILLA_ENTITY_TYPE_PLAYER_ID {
        // Stored Player rows have no independent client-input writer.
        return None;
    }
    Some(entity.delta_movement())
}

/// `SweetBerryBushBlock.entityInside` for a stored body.
pub(crate) fn entity_inside<F>(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    state: i32,
    hurt: &mut F,
) -> Option<()>
where
    F: FnMut(
        &mut Entity,
        &mut EntityStore,
        &mut LevelCtx<'_, '_>,
        &'static str,
        f32,
    ) -> Option<bool>,
{
    if sweet_berry_bush::inside_applies(entity.living().is_some(), entity.entity_type_id()) {
        entity.make_stuck_in_block(sweet_berry_bush::stuck_multiplier());
        if sweet_berry_bush::age(state)? != 0
            && sweet_berry_bush::movement_hurts(movement(entity, store, level)?)
        {
            hurt(entity, store, level, "minecraft:sweet_berry_bush", 1.0)?;
        }
    }
    Some(())
}
