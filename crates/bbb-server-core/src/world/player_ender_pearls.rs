//! Player-owned pearl trees use playerdata as their sole save source.

use bbb_server_persistence::player_schema::ender_pearls::Java26PlayerEnderPearl;
use bbb_server_persistence::Java26EntityRecordRef;
use bbb_server_types::save::{Java26LevelKey, SaveError, SaveResult};
use rustc_hash::FxHashSet;
use uuid::Uuid;

use crate::entity::save::{read_player_entity_tree, write_player_entity_tree};
use crate::entity::store::EntityBodyReadView;
use crate::entity::{Entity, EntityData};

use super::{LevelId, ServerWorld, WorldEvent};

fn owned_pearl(entity: &Entity, owner: Uuid) -> bool {
    entity.is_alive()
        && matches!(entity.data(), EntityData::EnderPearl(data) if data.projectile().owner() == Some(owner))
}

fn has_owned_pearl_ancestor(bodies: EntityBodyReadView<'_>, entity: &Entity, owner: Uuid) -> bool {
    let mut id = entity.id();
    while let Some(vehicle) = bodies.valid_vehicle(id) {
        let Some(parent) = bodies.get(vehicle) else {
            break;
        };
        if owned_pearl(parent, owner) {
            return true;
        }
        id = vehicle;
    }
    false
}

pub(in crate::world) fn saved_player_ender_pearls<'a>(
    owner: Uuid,
    levels: impl IntoIterator<Item = (&'a Java26LevelKey, EntityBodyReadView<'a>)>,
) -> SaveResult<Vec<Java26PlayerEnderPearl>> {
    let mut records = Vec::new();
    for (dimension, bodies) in levels {
        for pearl in bodies.bodies().filter(|entity| {
            owned_pearl(entity, owner) && !has_owned_pearl_ancestor(bodies, entity, owner)
        }) {
            records.push(Java26PlayerEnderPearl {
                dimension: dimension.clone(),
                entity: write_player_entity_tree(bodies, pearl).map_err(|error| {
                    SaveError::InvalidNbt {
                        path: None,
                        detail: format!("player {owner} ender pearl: {error}"),
                    }
                })?,
            });
        }
    }
    Ok(records)
}

impl ServerWorld {
    pub(super) fn player_ender_pearls(
        &self,
        owner: Uuid,
    ) -> SaveResult<Vec<Java26PlayerEnderPearl>> {
        saved_player_ender_pearls(
            owner,
            self.levels.values().map(|level| {
                (
                    level.dimension.as_ref(),
                    EntityBodyReadView::new(&level.entities, None, None),
                )
            }),
        )
    }

    pub(super) fn player_ender_pearl_save_exclusions(&self) -> FxHashSet<Uuid> {
        let mut excluded = FxHashSet::default();
        for level in self.levels.values() {
            for pearl in level.entities.bodies() {
                if pearl.is_alive()
                    && matches!(pearl.data(), EntityData::EnderPearl(data)
                        if data.projectile().owner().is_some_and(|owner| self.connection_of(owner).is_some()))
                {
                    excluded.insert(pearl.uuid());
                }
            }
        }
        excluded
    }

    pub(super) fn unload_player_ender_pearls(&mut self, owner: Uuid, events: &mut Vec<WorldEvent>) {
        for dimension in self.level_ids() {
            let ids = self
                .level_by_id(&dimension)
                .entities
                .bodies()
                .filter(|entity| owned_pearl(entity, owner))
                .map(Entity::id)
                .collect::<Vec<_>>();
            if ids.is_empty() {
                continue;
            }
            for id in ids {
                self.level_mut_by_id(&dimension)
                    .entities
                    .unload_player_entity_tree(id);
            }
            self.apply_level_entity_lifecycle_by_key(&dimension, events);
        }
    }

    pub(super) fn unload_restored_ender_pearls(&mut self, uuids: impl IntoIterator<Item = Uuid>) {
        let mut events = Vec::new();
        for uuid in uuids {
            for dimension in self.level_ids() {
                let id = self
                    .level_by_id(&dimension)
                    .entities
                    .entity_id_for_uuid(uuid);
                if let Some(id) = id {
                    self.level_mut_by_id(&dimension)
                        .entities
                        .unload_player_entity_tree(id);
                    self.apply_level_entity_lifecycle_by_key(&dimension, &mut events);
                }
            }
        }
    }

    fn pearl_tree_is_registered(
        &self,
        dimension: &LevelId,
        record: Java26EntityRecordRef<'_>,
    ) -> bool {
        self.entity_reference(dimension, record.uuid()).is_some()
            || self.connection_of(record.uuid()).is_some()
            || record
                .passengers()
                .any(|passenger| self.pearl_tree_is_registered(dimension, passenger))
    }

    pub(super) fn restore_player_ender_pearls(
        &mut self,
        owner: Uuid,
        records: &[Java26PlayerEnderPearl],
        events: &mut Vec<WorldEvent>,
    ) {
        let input = self.entity_nbt_read_snapshot();
        for record in records {
            let Some(dimension) = self.level_id(&record.dimension) else {
                continue;
            };
            if self.pearl_tree_is_registered(&dimension, record.entity.as_ref()) {
                continue;
            }
            let _ = read_player_entity_tree(
                &mut self.level_mut_by_id(&dimension).entities,
                record.entity.as_ref(),
                input.context(),
            );
        }
        for dimension in self.level_ids() {
            let chunks = self
                .level_by_id(&dimension)
                .entities
                .bodies()
                .filter(|entity| owned_pearl(entity, owner))
                .map(|entity| super::player_packets::player_chunk(entity.position()))
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            if chunks.is_empty() {
                continue;
            }
            self.refresh_ender_pearl_tickets(&dimension);
            self.refresh_simulation_chunk_levels_by_key(&dimension);
            if let Err(error) = self
                .install_full_chunks_by_key(&dimension, &chunks)
                .and_then(|()| self.refresh_loading_chunks(&dimension))
            {
                tracing::warn!(%error, "player ender pearl chunk preparation");
            }
            self.apply_level_entity_lifecycle_by_key(&dimension, events);
        }
    }
}
