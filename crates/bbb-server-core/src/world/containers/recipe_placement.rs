//! The recipe book's menu requests: placing a recipe and learning the ones
//! crafting takes used.

use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{PlaceGhostRecipe, PlaceRecipeCommand, RecipeDisplayId};

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::inventory::{MenuBroadcastMode, PostPlaceAction};

use super::super::{send, ServerWorld, WorldEvent};

impl ServerWorld {
    /// `ServerGamePacketListenerImpl.handlePlaceRecipe` (:1954-1981).
    pub(in crate::world) fn apply_place_recipe(
        &mut self,
        connection_id: ConnectionId,
        request: PlaceRecipeCommand,
        events: &mut Vec<WorldEvent>,
    ) {
        let resolver = self.item_stack_resolver();
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player.game_type == GameType::Spectator
            || player.inventory.container_id() != request.container_id
        {
            return;
        }
        let creative = player.game_type == GameType::Creative;
        if !matches!(self.container_still_valid(connection_id), Ok(true)) {
            return;
        }
        let Some(startup) = self.cold.config.datapacks.clone() else {
            return;
        };
        let Some((recipe_id, display)) =
            startup
                .recipe_displays
                .recipe_from_display(RecipeDisplayId {
                    index: request.recipe_index,
                })
        else {
            return;
        };
        if !self.levels[connection_id]
            .inventory
            .recipe_book()
            .contains(recipe_id)
        {
            return;
        }
        self.refresh_open_container(connection_id);
        let inventory = &mut self.levels[connection_id].inventory;
        let (action, dropped) = match inventory.place_recipe(
            &resolver,
            Some(&mut self.maps),
            recipe_id,
            request.use_max_items,
            creative,
        ) {
            Ok(placed) => placed,
            Err(error) => {
                tracing::warn!("place recipe: {error}");
                return;
            }
        };
        let sync = inventory.take_crafting_sync();
        Self::send_menu_sync(connection_id, sync, events);
        if action == PostPlaceAction::PlaceGhostRecipe {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::PlaceGhostRecipe(PlaceGhostRecipe {
                    container_id: request.container_id,
                    recipe_display: display.display.clone(),
                }),
            ));
        }
        let mut mode = MenuBroadcastMode::Changes;
        if !self.drop_clicked_items(connection_id, dropped) {
            mode = MenuBroadcastMode::FullState;
        }
        if let Err(error) = self.flush_open_container(connection_id) {
            tracing::warn!("place recipe: {error}");
            self.refresh_open_container(connection_id);
            mode = MenuBroadcastMode::FullState;
        }
        self.broadcast_player_menu(connection_id, mode, events);
    }

    /// `ResultSlot.checkTakeAchievements` (`ResultSlot.java:50-59`) through
    /// `RecipeCraftingHolder.awardUsedRecipes` for the crafting grids.
    pub(in crate::world) fn award_crafted_recipes(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let crafts = self.levels[connection_id].inventory.take_crafted_recipes();
        let mut recipes = Vec::new();
        for craft in crafts {
            self.trigger_recipe_crafted(
                connection_id,
                "minecraft:recipe_crafted",
                &craft.recipe_id,
                &craft.ingredients,
                events,
            );
            if craft.unlock {
                recipes.push(craft.recipe_id);
            }
        }
        if !recipes.is_empty() {
            self.award_recipes(connection_id, &recipes, events);
        }
    }
}
