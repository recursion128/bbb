use bbb_protocol::packets::RecipeBookRemove;

use crate::commands::{CommandPlayerTarget, RecipeCommandAction};
use crate::connection::ClientboundPlayPacket;

use super::super::player_packets::send;
use super::super::{ServerWorld, WorldEvent};

impl ServerWorld {
    /// `RecipeCommand.giveRecipes` / `takeRecipes` (`RecipeCommand.java:72-113`)
    /// through `ServerRecipeBook.addRecipes` / `removeRecipes`
    /// (`ServerRecipeBook.java:61-95`): the recipe-display count over every
    /// target, or `None` when no datapack registry is loaded.
    pub(crate) fn recipe_command(
        &mut self,
        action: RecipeCommandAction,
        targets: &[CommandPlayerTarget],
        recipe_ids: &[String],
        events: &mut Vec<WorldEvent>,
    ) -> Option<i32> {
        let startup = self.cold.config.datapacks.clone()?;
        let mut display_count = 0_i32;
        for target in targets {
            let Some(connection_id) = self.command_player_connection(target) else {
                continue;
            };
            match action {
                RecipeCommandAction::Give => {
                    let sent = self.award_recipes(connection_id, recipe_ids, events);
                    display_count =
                        display_count.saturating_add(i32::try_from(sent).unwrap_or(i32::MAX));
                }
                RecipeCommandAction::Take => {
                    let player = self
                        .levels
                        .player_mut(connection_id)
                        .expect("the resolved player is connected");
                    let book = player.inventory.recipe_book_mut();
                    let mut removed = Vec::new();
                    for recipe_id in recipe_ids {
                        if !book.remove(recipe_id) {
                            continue;
                        }
                        removed.extend(
                            startup
                                .recipe_displays
                                .displays_for_recipe(recipe_id)
                                .iter()
                                .map(|display| display.id),
                        );
                    }
                    display_count = display_count
                        .saturating_add(i32::try_from(removed.len()).unwrap_or(i32::MAX));
                    if !removed.is_empty() {
                        events.push(send(
                            connection_id,
                            ClientboundPlayPacket::RecipeBookRemove(RecipeBookRemove {
                                recipe_ids: removed,
                            }),
                        ));
                    }
                }
            }
        }
        Some(display_count)
    }
}
