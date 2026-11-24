//! `CaveVines.use` (`CaveVines.java:23-46`) for both halves:
//! `CaveVinesBlock.useWithoutItem` (`CaveVinesBlock.java:65-68`) and
//! `CaveVinesPlantBlock.useWithoutItem` (`CaveVinesPlantBlock.java:49-52`).

use super::BlockUseContext;
use crate::entity::store::EntityBodyReadView;
use crate::loot_runtime::{LootEntity, StoredLootEntity};
use crate::world::{ServerWorld, WorldEvent};
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_CAVE_VINES_ID, VANILLA_BLOCK_CAVE_VINES_PLANT_ID};

/// The `matches:` column for `CaveVinesBlock`.
pub(super) fn is_cave_vines(block_state_id: i32) -> bool {
    vanilla_block_state_block_type_id(block_state_id) == Some(VANILLA_BLOCK_CAVE_VINES_ID)
}

/// The `matches:` column for `CaveVinesPlantBlock`.
pub(super) fn is_cave_vines_plant(block_state_id: i32) -> bool {
    vanilla_block_state_block_type_id(block_state_id) == Some(VANILLA_BLOCK_CAVE_VINES_PLANT_ID)
}

/// The `without_item:` column: [`ServerWorld::use_cave_vines_without_item`].
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_cave_vines_without_item(context, events)
}

impl ServerWorld {
    /// `CaveVines.use` (`CaveVines.java:23-46`).
    pub(super) fn use_cave_vines_without_item(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = context.dimension;
        let pos = context.update.hit.pos;
        let source_id = self.levels[context.connection_id].entity_id();
        let harvested = self.with_level_and_entities(dimension, |level, store| {
            let Some(player) = store.player(source_id) else {
                return false;
            };
            let entity = player.body();
            level.harvest_cave_vines(
                pos,
                source_id,
                LootEntity::Stored(StoredLootEntity {
                    entity,
                    bodies: Some(EntityBodyReadView::new(store, None, None)),
                    raid_at_position: level.raid_at(entity.block_position()),
                }),
            )
        });
        self.post_pending_game_events(&dimension);
        let _ = events;
        harvested
    }
}
