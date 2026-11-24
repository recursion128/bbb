//! Test-only accessors that belong to neither the entity store nor a tick
//! module.

use super::*;

impl ServerWorld {
    #[cfg(test)]
    pub(crate) fn with_level_for_test<R>(
        &mut self,
        dimension: Java26BuiltinDimension,
        body: impl FnOnce(&mut LevelCtx<'_, '_>) -> R,
    ) -> R {
        self.with_level(dimension, body)
    }

    pub(crate) fn seeded_container_fill_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        pos: BlockPos,
        table: &str,
        seed: i64,
    ) -> Vec<bbb_server_types::item_stack::RuntimeItemStack> {
        use crate::loot_runtime::{fill_container, param_sets, LootOrigin, LootParams};
        let (level, tables, facts, _) = self.level_and_drop_tables(&dimension.into());
        let chunks = std::cell::RefCell::new(&mut level.chunks);
        let environment = super::simulation_level::mob_death_loot::LevelLootEnvironment::for_level(
            tables.recipes,
            crate::world::level_blocks::LootChunks::Chunks(&chunks),
            &facts,
            Some(crate::entity::store::EntityBodyReadView::new(
                &level.entities,
                None,
                None,
            )),
        )
        .expect("fixture loot environment");
        let mut slots = vec![bbb_server_types::item_stack::RuntimeItemStack::empty(); 27];
        let written = fill_container(
            tables.tables.expect("fixture loot tables"),
            table,
            &LootParams {
                origin: Some(LootOrigin::at_center_of(pos)),
                item_environment: Some(&environment),
                ..LootParams::new(param_sets::CHEST)
            },
            &crate::random::GameRandom::with_seed(seed),
            (0..slots.len()).collect(),
        )
        .expect("fixture seeded fill");
        for (slot, stack) in written {
            slots[slot] = stack;
        }
        slots
    }

    #[cfg(test)]
    pub(crate) fn unpack_container_loot_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        pos: BlockPos,
    ) {
        self.with_level(dimension, |level| level.unpack_container_loot(pos, None))
            .expect("the unpack runs");
    }

    /// The record a chunk save writes for the block entity at `pos`, through
    /// the world's own item publication.
    #[cfg(test)]
    pub(crate) fn block_entity_save_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        pos: BlockPos,
    ) -> Option<Vec<bbb_protocol::NbtSummaryEntry>> {
        let resolver = self.item_stack_resolver();
        self.level(dimension)
            .chunks
            .block_entities()
            .get(pos)?
            .save_with_full_metadata_with_resolver(&resolver)
            .ok()
    }

    // Inject an unavailable provider without publishing an invalid wire block state.
    #[cfg(test)]
    pub(crate) fn set_raw_block_state_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        pos: BlockPos,
        state: i32,
    ) {
        self.level_mut(dimension)
            .chunks
            .set_block_state(pos.x, pos.y, pos.z, state)
            .unwrap();
    }

    #[cfg(test)]
    pub(crate) fn add_inhabited_time_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        chunk_x: i32,
        chunk_z: i32,
        ticks: i64,
    ) {
        assert!(self
            .level_mut(dimension)
            .chunks
            .increment_inhabited_time(chunk_x, chunk_z, ticks));
    }

    #[cfg(test)]
    pub(crate) fn set_game_time_for_test(&mut self, game_time: i64) {
        self.game_time = game_time;
    }

    #[cfg(test)]
    pub(crate) fn level_registry_tags_for_test(
        &self,
        dimension: Java26BuiltinDimension,
    ) -> Arc<RegistryTagSnapshot> {
        self.level(dimension)
            .chunks
            .registry_tags_shared()
            .expect("served levels bind registry tags at construction")
    }

    #[cfg(test)]
    pub(crate) fn runtime_registry_tag_members_for_test(
        &self,
        registry_id: &str,
        tag_id: &str,
    ) -> PersistentRegistryTagMembers {
        RuntimePersistentRegistryResolver {
            datapacks: self.cold.config.datapacks.as_deref(),
            content_registry: self.registries.content_registry(),
        }
        .tag_members(registry_id, tag_id)
    }

    /// The mutable half of [`Self::get_block_entity`], which no packet handler
    /// needs — every writer here reaches the object through the level slice —
    /// and which the tests use to stand a block entity up in a known state.
    #[cfg(test)]
    pub(crate) fn level_block_entity_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        pos: BlockPos,
    ) -> Option<&mut BlockEntity> {
        self.level_mut(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
    }
}
