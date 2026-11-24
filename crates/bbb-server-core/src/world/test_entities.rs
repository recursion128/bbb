//! The live entity store as this crate's tests reach it: production owners
//! only, moved out of the world facade with no change to what they do.

use super::*;
use crate::world::LevelId;

impl ServerWorld {
    #[cfg(test)]
    pub(crate) fn load_entity_chunk_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        chunk: (i32, i32),
    ) -> bbb_server_persistence::SaveResult<()> {
        let key = LevelId::from(dimension);
        self.level_mut_by_id(&key)
            .chunk_lifecycle
            .add_unknown(chunk, 33);
        self.refresh_loading_chunks(&key)
    }

    /// Writes one biome cell of a column the fixture serves, which is the biome
    /// a spawn's `SpawnContext` (`SpawnContext.java:10-13`) reads through
    /// `BiomeManager.getBiome`.
    #[cfg(test)]
    pub(crate) fn set_noise_biome_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        quart: (i32, i32, i32),
        biome: &str,
    ) -> bool {
        let Some(biome_id) = self
            .registries
            .registry_element_index("minecraft:worldgen/biome", biome)
        else {
            return false;
        };
        let chunks = &mut self.level_mut(dimension).chunks;
        if chunks
            .try_ensure_served_full_column(quart.0 >> 2, quart.2 >> 2)
            .is_err()
        {
            return false;
        }
        chunks
            .column_mut(quart.0 >> 2, quart.2 >> 2)
            .is_some_and(|column| {
                column
                    .set_noise_biome(quart.0 & 3, quart.1, quart.2 & 3, biome_id)
                    .is_ok()
            })
    }

    /// Creates one native non-player entity through the live level store for
    /// a production-path test. Tests use the returned UUID for `/damage`, so
    /// selector resolution and mutation both traverse their real owners.
    #[cfg(test)]
    pub(crate) fn spawn_stored_entity_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_type_id: i32,
        position: Vec3d,
        data: crate::entity::EntityData,
    ) -> (i32, Uuid) {
        let level = self.level_mut(dimension);
        let entity_id = level.entities.spawn(entity_type_id, position, data);
        let uuid = level
            .entities
            .get(entity_id)
            .expect("the store returns the id it just filed")
            .uuid();
        (entity_id, uuid)
    }

    #[cfg(test)]
    pub(crate) fn track_stored_chunk_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        chunk: (i32, i32),
    ) {
        self.level_mut(dimension)
            .entities
            .apply_chunk_visibility(chunk, crate::entity::store::EntityVisibility::Tracked);
    }

    #[cfg(test)]
    pub(crate) fn spawn_stored_entity_for_test_by_key(
        &mut self,
        dimension: &LevelId,
        entity_type_id: i32,
        position: Vec3d,
        data: crate::entity::EntityData,
    ) -> (i32, Uuid) {
        let level = self.level_mut_by_id(dimension);
        let entity_id = level.entities.spawn(entity_type_id, position, data);
        let uuid = level
            .entities
            .get(entity_id)
            .expect("the store returns the id it just filed")
            .uuid();
        (entity_id, uuid)
    }

    #[cfg(test)]
    pub(crate) fn restore_stored_entity_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        uuid: Uuid,
        entity_type_id: i32,
        position: Vec3d,
        data: crate::entity::EntityData,
    ) -> Option<i32> {
        self.level_mut(dimension)
            .entities
            .restore(uuid, entity_type_id, position, data)
    }

    #[cfg(test)]
    pub(crate) fn stored_entity_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
    ) -> Option<&crate::entity::Entity> {
        self.level(dimension).entities.get(entity_id)
    }

    /// `Mob.setTarget` on one stored mob.
    #[cfg(test)]
    pub(crate) fn set_mob_target_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
        target: Option<(i32, Uuid)>,
    ) {
        self.with_level_and_entities(dimension, |level, entities| {
            crate::entity::ai::mob::target::sense::set_mob_target(
                entities,
                level,
                entity_id,
                target.map(
                    |(entity_id, uuid)| crate::entity::ai::mob::target::TargetRef {
                        entity_id,
                        uuid,
                    },
                ),
            );
        });
    }

    /// `Mob.getTarget()` of one stored mob as its last tick left it.
    #[cfg(test)]
    pub(crate) fn mob_target_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
    ) -> Option<Uuid> {
        self.level(dimension).entities.mob_target(entity_id)
    }

    #[cfg(test)]
    pub(crate) fn mob_goal_debug_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
    ) -> Option<bbb_protocol::packets::DebugGoalInfo> {
        self.level(dimension)
            .entities
            .mob_goal_debug_info(entity_id)
    }

    #[cfg(test)]
    pub(crate) fn stored_entity_for_test_by_key(
        &self,
        dimension: &LevelId,
        entity_id: i32,
    ) -> Option<&crate::entity::Entity> {
        self.level_by_id(dimension).entities.get(entity_id)
    }

    #[cfg(test)]
    pub(crate) fn stored_entity_mut_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
    ) -> Option<&mut crate::entity::Entity> {
        self.level_mut(dimension).entities.get_mut(entity_id)
    }

    /// The record one stored entity answers `/data get` with, which is also
    /// what its save writes.
    #[cfg(test)]
    pub(crate) fn stored_entity_command_data_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
    ) -> Option<bbb_protocol::NbtSummaryValue> {
        self.level(dimension).entities.command_data(entity_id).ok()
    }

    /// Loads one saved record into a stored entity, which is the shape a
    /// staged save hands the store.
    #[cfg(test)]
    pub(crate) fn load_stored_entity_record_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
        entries: &[bbb_protocol::NbtSummaryEntry],
    ) {
        // The same read context `/data merge entity` loads through.
        let input = self.entity_nbt_read_snapshot();
        let entities = &mut self.level_mut(dimension).entities;
        entities
            .load_command_data_with_context(entity_id, entries, None, input.context())
            .expect("the staged record loads");
    }

    /// A real patrol leader pillager: `PatrolSpawner`'s own spawn, which is
    /// the one path that sets `PatrolLeader` **and** equips
    /// `Raid.getOminousBannerInstance` (`Raider.isCaptain`,
    /// `Raider.java:148-154`).
    ///
    /// The caller lays the floor the placement predicate needs.
    #[cfg(test)]
    pub(crate) fn spawn_patrol_leader_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        position: BlockPos,
    ) -> i32 {
        let key = LevelId::from(dimension);
        let before = self.level(dimension).entities.ids();
        assert_eq!(
            self.spawn_patrol_member(&key, position, true),
            Ok(true),
            "the patrol spawner publishes a leader at {position:?}"
        );
        let ids = self.level(dimension).entities.ids();
        ids.into_iter()
            .find(|id| !before.contains(id))
            .expect("the patrol spawner published the leader it reported")
    }

    /// `ServerLevel.players()` (`ServerLevel.java:1507-1509`) as the entity
    /// mechanism's own projection, which is what the death loot path resolves
    /// `killerPlayer.getLuck()` from.
    #[cfg(test)]
    pub(crate) fn nearby_players_for_test(
        &self,
        dimension: Java26BuiltinDimension,
    ) -> Vec<crate::entity::NearbyPlayer> {
        self.nearby_players_by_key(&dimension.into())
    }

    /// The luck `LivingEntity.dropFromLootTable` hands `LootParams` for one
    /// killer profile id (`LivingEntity.java:1543-1545`), resolved through the
    /// same projection and the same rule the death path uses.
    #[cfg(test)]
    pub(crate) fn killer_luck_for_test(
        &self,
        _dimension: Java26BuiltinDimension,
        killer: Option<Uuid>,
    ) -> Option<f32> {
        killer.and_then(|uuid| {
            self.levels
                .players()
                .find(|player| player.profile.uuid == uuid)
                .map(crate::world::simulation_level::mob_death_loot::player_luck)
        })
    }

    /// Every stored item entity of one dimension, oldest id first, as
    /// `(entity id, position, item resource id, count, pickup delay, damage)`.
    #[cfg(test)]
    pub(crate) fn stored_items_for_test(
        &self,
        dimension: Java26BuiltinDimension,
    ) -> Vec<(i32, Vec3d, String, i32, i32, Option<i32>)> {
        let level = self.level(dimension);
        level
            .entities
            .ids()
            .into_iter()
            .filter_map(|entity_id| {
                let entity = level.entities.get(entity_id)?;
                let crate::entity::EntityData::Item(item) = entity.data() else {
                    return None;
                };
                let stack = item.item();
                let resource_id = stack.item_resource_id()?;
                let damage = stack.summary().component_patch.damage;
                Some((
                    entity_id,
                    entity.position(),
                    resource_id.to_string(),
                    stack.count(),
                    item.pickup_delay(),
                    damage,
                ))
            })
            .collect()
    }

    /// The stack one stored item entity carries.
    #[cfg(test)]
    pub(crate) fn stored_item_stack_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
    ) -> Option<bbb_server_types::item_stack::RuntimeItemStack> {
        let entity = self.level(dimension).entities.get(entity_id)?;
        let crate::entity::EntityData::Item(item) = entity.data() else {
            return None;
        };
        Some(item.item().clone())
    }

    /// Every stored experience orb of one dimension, oldest id first, as
    /// `(entity id, position, value, count)`.
    #[cfg(test)]
    pub(crate) fn stored_experience_orbs_for_test(
        &self,
        dimension: Java26BuiltinDimension,
    ) -> Vec<(i32, Vec3d, i32, i32)> {
        let level = self.level(dimension);
        level
            .entities
            .ids()
            .into_iter()
            .filter_map(|entity_id| {
                let entity = level.entities.get(entity_id)?;
                let crate::entity::EntityData::ExperienceOrb(orb) = entity.data() else {
                    return None;
                };
                Some((entity_id, entity.position(), orb.value(), orb.count()))
            })
            .collect()
    }
}
