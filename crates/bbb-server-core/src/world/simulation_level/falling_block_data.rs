use super::*;

impl LevelCtx<'_, '_> {
    pub(super) fn merge_falling_block_data(
        &mut self,
        pos: BlockPos,
        entries: &[bbb_protocol::NbtSummaryEntry],
    ) -> Option<()> {
        if self.chunks.block_entities().get(pos).is_none() {
            return Some(());
        }
        let facts = &self.loot_level_facts;
        let resolver = facts.item_stack_resolver.as_ref()?;
        let block_entity = self.chunks.block_entities_mut().get_mut(pos)?;
        let block_state = block_entity.block_state();
        if let Err(error) =
            block_entity.merge_falling_block_data_with_resolver(entries, resolver.as_dyn())
        {
            tracing::warn!("falling block entity data: {error}");
        }
        // FallingBlockEntity.java:213-218: setChanged is unconditional after loading.
        self.block_entity_set_changed(pos, block_state);
        Some(())
    }
}
