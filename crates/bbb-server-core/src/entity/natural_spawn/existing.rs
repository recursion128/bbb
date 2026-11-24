use bbb_protocol::entity_types::{vanilla_entity_type, VanillaEntityMobCategory};
use bbb_protocol::packets::BlockPos;

use crate::entity::{Entity, EntityData, EntityStore};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NaturalSpawnExistingFacts {
    pub entity_type_id: i32,
    pub category: VanillaEntityMobCategory,
    pub position: BlockPos,
    pub is_mob: bool,
    pub persistence_required: bool,
    pub requires_custom_persistence: bool,
}

pub fn natural_spawn_existing_facts(
    store: &EntityStore,
    entity: &Entity,
) -> Option<NaturalSpawnExistingFacts> {
    let entity_type_id = entity.entity_type_id();
    let category = vanilla_entity_type(entity_type_id)
        .map(|facts| facts.mob_category)
        .or_else(|| {
            store
                .plugins()
                .content_registry()
                .entity_by_id(entity_type_id)
                .map(|spec| spec.category)
        })?;
    let mob = match entity.data() {
        EntityData::GeneralLiving(data) => data.mob_retained().map(|mob| (data, mob)),
        _ => None,
    };
    let (is_mob, persistence_required, requires_custom_persistence) = match mob {
        Some((data, mob)) => (
            true,
            mob.persistence_required(),
            data.requires_custom_persistence(store, entity)
                .unwrap_or(false),
        ),
        None => (false, false, false),
    };
    Some(NaturalSpawnExistingFacts {
        entity_type_id,
        category,
        position: entity.block_position(),
        is_mob,
        persistence_required,
        requires_custom_persistence,
    })
}

#[cfg(test)]
mod tests;
