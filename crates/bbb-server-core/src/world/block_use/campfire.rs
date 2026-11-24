//! `CampfireBlock.getTicker`'s two server bodies (`CampfireBlock.java:308-321`):
//! a lit campfire cooks, an unlit one cools back down.

use crate::world::LevelId;
use bbb_protocol::packets::{BlockPos, ItemStackSummary};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::block_behavior::campfire as campfire_block;
use crate::block_entity::BlockEntity;
use crate::level_event::VanillaGameEvent;
use bbb_server_datapack::recipe::{CookingKind, CookingRecipe};

use crate::world::ServerWorld;

impl ServerWorld {
    /// One `BlockEntityTicker.tick` for a campfire (`CampfireBlock.java:308-321`).
    ///
    /// The payload advances its own slots; the finished ones come back here
    /// because the recipe result, the drop and the block update need the level.
    pub(in crate::world) fn tick_campfire(&mut self, dimension: &LevelId, pos: BlockPos) {
        let Some(block_state_id) = self
            .level_by_id(dimension)
            .chunks
            .get_block_state(pos.x, pos.y, pos.z)
        else {
            return;
        };
        if !campfire_block::is_lit(block_state_id) {
            let Some(changed) = self
                .level_mut_by_id(dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(BlockEntity::campfire_mut)
                .map(crate::block_entity::campfire::CampfireBlockEntity::cooldown_tick)
            else {
                return;
            };
            if changed {
                self.level_mut_by_id(dimension)
                    .chunks
                    .block_entities_mut()
                    .set_changed(pos);
            }
            return;
        }
        let Some(campfire) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::campfire)
        else {
            return;
        };
        let items = campfire.original_items();
        let progress = campfire.cooking_progress();
        let total = campfire.cooking_time();
        let mut any_item = false;
        let mut any_finishing = false;
        for slot in 0..items.len() {
            if items[slot].is_empty() {
                continue;
            }
            any_item = true;
            if progress[slot] + 1 >= total[slot] {
                any_finishing = true;
            }
        }
        if !any_item {
            return;
        }
        if !any_finishing {
            let changed = self
                .level_mut_by_id(dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(BlockEntity::campfire_mut)
                .expect("lit campfire remains present")
                .cook_tick()
                .changed;
            if changed {
                // `:82-84`.
                self.level_mut_by_id(dimension)
                    .chunks
                    .block_entities_mut()
                    .set_changed(pos);
            }
            return;
        }
        let mut outputs = Vec::new();
        for slot in 0..items.len() {
            if items[slot].is_empty() || progress[slot] + 1 < total[slot] {
                continue;
            }
            let input = &items[slot];
            let result = match self.campfire_recipe(&input.item_identity_summary()) {
                Some(recipe) => match self.assemble_cooking_recipe(recipe) {
                    Ok(result) => result,
                    Err(error) => {
                        tracing::warn!("campfire cooking output: {error}");
                        return;
                    }
                },
                None => input.clone(),
            };
            outputs.push((slot, result));
        }
        let changed = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::campfire_mut)
            .expect("cooking campfire remains present")
            .cook_tick()
            .changed;
        for (slot, result) in outputs {
            self.drop_campfire_result(dimension, pos, &result);
            self.level_mut_by_id(dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(BlockEntity::campfire_mut)
                .expect("cooking campfire remains present")
                .take_finished_slot(slot);
            self.level_mut_by_id(dimension)
                .changed_block_broadcasts
                .insert((pos.x, pos.y, pos.z));
            self.level_mut_by_id(dimension).event_sink.game_event(
                VanillaGameEvent::BLOCK_CHANGE,
                pos,
                Some(block_state_id),
            );
        }
        if changed {
            // `:82-84`.
            self.level_mut_by_id(dimension)
                .chunks
                .block_entities_mut()
                .set_changed(pos);
        }
    }

    /// `RecipeManager.getRecipeFor` (`RecipeManager.java:131-146`) over
    /// `RecipeType.CAMPFIRE_COOKING`, which both the placement path and the
    /// cooking tick read.
    pub(super) fn campfire_recipe(&self, input: &ItemStackSummary) -> Option<&CookingRecipe> {
        if input.item_id.is_none() || input.count <= 0 {
            return None;
        }
        let tags = self.item_tags()?;
        let datapacks = self.datapacks()?;
        datapacks
            .recipe_index
            .get_recipe_for_cooking(&datapacks.recipes, CookingKind::Campfire, input, None, tags)
            .and_then(|element| element.recipe.cooking_of_kind(CookingKind::Campfire))
    }

    /// `Containers.dropItemStack(level, pos.getX(), pos.getY(), pos.getZ(), result)`
    /// (`CampfireBlockEntity.java:71`), the same seam the container drops use.
    fn drop_campfire_result(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        result: &RuntimeItemStack,
    ) {
        if result.is_empty() {
            return;
        }
        let position = bbb_protocol::packets::Vec3d {
            x: f64::from(pos.x),
            y: f64::from(pos.y),
            z: f64::from(pos.z),
        };
        self.drop_runtime_container_contents(dimension, position, std::slice::from_ref(result));
    }
}
