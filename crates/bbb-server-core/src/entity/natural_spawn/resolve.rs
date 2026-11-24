use crate::random::GameRandom;
use bbb_protocol::entity_types::vanilla_entity_type;
use bbb_protocol::packets::Vec3d;

use super::mob_eligibility::{natural_spawn_cluster_size, natural_spawn_group_size_reached};
use crate::entity::save::create_fresh_entity;
use crate::entity::{
    finalize_spawn, tropical_fish_is_school, Entity, EntityStore, SpawnGroupData, SpawnLevel,
    SpawnReason,
};

#[derive(Clone, Debug, PartialEq)]
pub struct NaturalSpawnPublishResult {
    pub root_entity_id: i32,
    pub group_data: Option<SpawnGroupData>,
    pub max_spawn_cluster_size: i32,
    pub max_group_size_reached: bool,
}

pub fn create_natural_spawn_root(
    store: &mut EntityStore,
    entity_type_id: i32,
    position: Vec3d,
) -> Option<Box<Entity>> {
    if let Some(facts) = vanilla_entity_type(entity_type_id) {
        return create_fresh_entity(store, facts.resource_id, position);
    }
    let resource_id = store
        .plugins()
        .content_registry()
        .entity_by_id(entity_type_id)?
        .resource_id
        .clone();
    create_fresh_entity(store, &resource_id, position)
}

pub(crate) fn finalize_and_publish_natural_spawn(
    level: &mut dyn SpawnLevel<'_>,
    mut root: Box<Entity>,
    reason: SpawnReason,
    group: Option<SpawnGroupData>,
    random: &GameRandom,
) -> Option<NaturalSpawnPublishResult> {
    let entity_type_id = root.entity_type_id();
    let is_custom_mob = matches!(
        root.data(),
        crate::entity::EntityData::GeneralLiving(data)
            if data.plugin_mob().is_some() && data.is_mob()
    );
    let published = finalize_spawn(level, &mut root, reason, group, random).and_then(|group| {
        let cluster_size = match natural_spawn_cluster_size(entity_type_id) {
            Ok(size) => size,
            Err(_) if is_custom_mob => 4,
            Err(_) => return None,
        };
        let group_size_reached = match natural_spawn_group_size_reached(
            entity_type_id,
            tropical_fish_is_school(&root),
        ) {
            Ok(reached) => reached,
            Err(_) if is_custom_mob => false,
            Err(_) => return None,
        };
        Some((group, cluster_size, group_size_reached))
    });
    let store = level.entities();
    let Some((group_data, max_spawn_cluster_size, max_group_size_reached)) = published else {
        store.discard_fresh(root);
        return None;
    };
    Some(NaturalSpawnPublishResult {
        root_entity_id: store.add_fresh(root),
        group_data,
        max_spawn_cluster_size,
        max_group_size_reached,
    })
}
