//! `SweetBerryBushBlock.useItemOn` (`SweetBerryBushBlock.java:102-112`) and
//! `useWithoutItem` (`:116-141`): bone meal on a bush that is not age 3 PASSes;
//! age `> 1` picks berries.

use bbb_protocol::item_types::VANILLA_ITEM_BONE_MEAL_ID;
use bbb_protocol::packets::{SoundEvent, SoundEventHolder, SoundSource, Vec3d};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;

use super::BlockUseContext;
use crate::block_behavior::sweet_berry_bush as sweet_berry_bush_block;
use crate::block_behavior::{BlockUseResult, SET_BLOCK_UPDATE_CLIENTS};
use crate::level_event::VanillaGameEvent;
use crate::loot_runtime::{LootEntity, LootTool};
use crate::world::simulation_level::block_entity_loot::{DropBlockEntity, DropTable};
use crate::world::{ServerWorld, WorldEvent};

/// `BuiltInLootTables.HARVEST_SWEET_BERRY_BUSH` (`BuiltInLootTables.java:106`).
const HARVEST_LOOT_TABLE: &str = "minecraft:harvest/sweet_berry_bush";

/// `SoundEvents.SWEET_BERRY_BUSH_PICK_BERRIES` (`SweetBerryBushBlock.java:130-132`).
const PICK_SOUND: &str = "minecraft:block.sweet_berry_bush.pick_berries";

/// The `item_on:` column: [`ServerWorld::use_sweet_berry_bush_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_sweet_berry_bush_item_on(context)
}

/// The `without_item:` column: [`ServerWorld::use_sweet_berry_bush_without_item`].
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_sweet_berry_bush_without_item(context, events)
}

impl ServerWorld {
    /// `SweetBerryBushBlock.useItemOn` (`SweetBerryBushBlock.java:102-112`).
    pub(super) fn use_sweet_berry_bush_item_on(
        &mut self,
        context: &BlockUseContext<'_>,
    ) -> BlockUseResult {
        let age = sweet_berry_bush_block::age(context.block_state_id);
        if age != Some(sweet_berry_bush_block::MAX_AGE)
            && context.selected_item.item_id() == Some(VANILLA_ITEM_BONE_MEAL_ID)
        {
            return BlockUseResult::Pass;
        }
        BlockUseResult::TryWithEmptyHand
    }

    /// `SweetBerryBushBlock.useWithoutItem` (`SweetBerryBushBlock.java:116-141`).
    pub(super) fn use_sweet_berry_bush_without_item(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Some(age) = sweet_berry_bush_block::age(context.block_state_id) else {
            return false;
        };
        // `:119`: age `<= 1` is `super.useWithoutItem`, which PASSes.
        if age <= 1 {
            return false;
        }
        let Some(new_state) = sweet_berry_bush_block::with_age(context.block_state_id, 1) else {
            return false;
        };
        let dimension = context.dimension;
        let pos = context.update.hit.pos;
        let source_entity_id = self.levels[context.connection_id].entity_id();
        // `:121-129`: `dropFromBlockInteractLootTable` then `Block.popResource`.
        let interacting = self.loot_view_for_player(self.levels.player_at(context.connection_id));
        self.with_level(dimension, |level| {
            level.drop_block_loot_at(
                pos,
                context.block_state_id,
                DropTable::Named(HARVEST_LOOT_TABLE),
                DropBlockEntity::Live,
                &LootTool::empty(),
                LootEntity::Facts(&interacting),
            );
        });
        if let Some(registry_id) = vanilla_sound_event_registry_id(PICK_SOUND) {
            // `:131`: pitch is `0.8F + level.getRandom().nextFloat() * 0.4F`.
            let pitch = 0.8 + self.random.next_float() * 0.4;
            let sound = SoundEvent {
                sound: SoundEventHolder::Reference { registry_id },
                source: SoundSource::Blocks,
                position: Vec3d {
                    x: f64::from(pos.x) + 0.5,
                    y: f64::from(pos.y) + 0.5,
                    z: f64::from(pos.z) + 0.5,
                },
                volume: 1.0,
                pitch,
                seed: self.random.next_long(),
            };
            self.broadcast_level_sound_by_key(&dimension, &sound, None, events);
        }
        self.set_block(dimension, pos, new_state, SET_BLOCK_UPDATE_CLIENTS);
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(
                VanillaGameEvent::BLOCK_CHANGE,
                pos,
                Some(source_entity_id),
                Some(new_state),
            );
        true
    }
}
