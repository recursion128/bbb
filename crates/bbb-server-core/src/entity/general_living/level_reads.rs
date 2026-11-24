//! The body's own level reads: eyes in fluid and entity-type tags.

use super::*;
use crate::world::simulation_level::LevelCtx;

pub(crate) fn can_breathe_underwater_membership(level: &LevelCtx<'_, '_>, entity: &Entity) -> bool {
    if entity.entity_type_id() == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
        && matches!(entity.data(), EntityData::GeneralLiving(data)
            if data.ageable_retained().is_some_and(AgeableMobRetainedState::is_baby))
    {
        true
    } else {
        entity_type_tag_membership(level, entity, "minecraft:can_breathe_under_water")
    }
}

pub(crate) fn eye_water_contact_from_tracker(
    entity: &Entity,
    level: &LevelCtx<'_, '_>,
) -> Option<bool> {
    if !entity.is_eye_in_fluid(crate::entity::environment::FluidTag::Water) {
        return Some(false);
    }
    let eye = BlockPos {
        x: entity.position().x.floor() as i32,
        y: (entity.position().y + f64::from(entity.eye_height())).floor() as i32,
        z: entity.position().z.floor() as i32,
    };
    let eye_state = level.block_state(eye);
    let kind = match vanilla_block_state_block_type_id(eye_state) {
        Some(kind) => Some(kind),
        None => {
            level.content_registry().block_by_state_id(eye_state)?;
            None
        }
    };
    Some(kind != Some(VANILLA_BLOCK_BUBBLE_COLUMN_ID))
}

pub(super) fn entity_type_tag_membership(
    level: &LevelCtx<'_, '_>,
    entity: &Entity,
    tag_id: &str,
) -> bool {
    level.registry_tag_membership(
        "minecraft:entity_type",
        tag_id,
        entity.data().entity_resource_id(),
    )
}
