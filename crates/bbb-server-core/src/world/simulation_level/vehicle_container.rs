use super::*;

impl LevelCtx<'_, '_> {
    pub(super) fn unpack_actual_vehicle_container(
        &mut self,
        entity: &mut crate::entity::Entity,
        context: crate::entity::level::EntityBlockContext<'_>,
    ) -> Option<()> {
        crate::entity::item_slots::unpack_vehicle_cargo(entity, |position, request, available| {
            let store = context
                .store
                .as_deref()
                .ok_or("vehicle context has no entity store")?;
            block_entity_loot::fill_unpacked_container(
                self.chunks,
                &mut self.drop_tables,
                &self.loot_level_facts,
                Some(crate::entity::store::EntityBodyReadView::new(
                    store,
                    Some(entity),
                    context.ancestors,
                )),
                self.weather,
                self.random,
                crate::loot_runtime::LootOrigin {
                    x: position.x,
                    y: position.y,
                    z: position.z,
                },
                request,
                None,
                available,
            )
        })
        .ok()?;
        Some(())
    }
}
