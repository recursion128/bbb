use crate::connection::ConnectionId;
use crate::entity::block_drops::loot_tool_for_runtime_stack;
use crate::loot_runtime::{matches_conditions, matches_item_predicate};
use bbb_registry_codec::RegistryElementValueRef as V;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::{
    parse_item_predicate_source as parse_item_predicate, ItemPredicate, LootCondition,
};
use bbb_server_datapack::recipe::ItemTagIndex;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::{construct_beacon::player_conditions, ServerWorld, WorldEvent};

pub(super) const RECIPE_CRAFTED: &str = "minecraft:recipe_crafted";
pub(super) const CRAFTER_RECIPE_CRAFTED: &str = "minecraft:crafter_recipe_crafted";

pub(super) struct RecipeCraftedCondition {
    recipe_id: String,
    ingredients: Vec<ItemPredicate>,
    player: Vec<LootCondition>,
}

impl RecipeCraftedCondition {
    // RecipeCraftedTrigger.java:26-35.
    pub(super) fn parse(
        value: V<'_>,
        trigger: &str,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Self, String> {
        let recipe_id = value
            .field("recipe_id")
            .ok_or_else(|| format!("{trigger} recipe_id is missing"))?;
        let recipe_id =
            bbb_server_datapack::loot::codec::id(recipe_id).map_err(|error| error.to_string())?;
        let ingredients = value
            .field("ingredients")
            .map(|values| bbb_server_datapack::loot::codec::list(values, parse_item_predicate))
            .transpose()
            .map_err(|error| format!("{trigger} ingredient: {error}"))?
            .unwrap_or_default();
        Ok(Self {
            recipe_id,
            ingredients,
            player: player_conditions(value.field("player"), trigger, lookup)?,
        })
    }

    // RecipeCraftedTrigger.java:54-78.
    fn matches_inputs(
        &self,
        recipe_id: &str,
        used_ingredients: &[RuntimeItemStack],
        tags: Option<&ItemTagIndex>,
        enchantment_registry: &[String],
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> bool {
        if recipe_id != self.recipe_id {
            return false;
        }
        if self.ingredients.is_empty() {
            return true;
        }
        let Some(tags) = tags else {
            return false;
        };
        let mut remaining = used_ingredients.iter().collect::<Vec<_>>();
        for predicate in &self.ingredients {
            let mut found = None;
            for (index, stack) in remaining.iter().enumerate() {
                let tool = loot_tool_for_runtime_stack(stack, enchantment_registry);
                match matches_item_predicate(predicate, &tool, tags, resolver) {
                    Ok(true) => {
                        found = Some(index);
                        break;
                    }
                    Ok(false) => {}
                    Err(_) => return false,
                }
            }
            let Some(index) = found else {
                return false;
            };
            remaining.remove(index);
        }
        true
    }
}

impl ServerWorld {
    // RecipeCraftedTrigger.java:22-23; SimpleCriterionTrigger.java:43-70.
    pub(in crate::world) fn trigger_recipe_crafted(
        &mut self,
        connection_id: ConnectionId,
        trigger: &str,
        recipe_id: &str,
        used_ingredients: &[RuntimeItemStack],
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player
            .cold
            .advancements
            .trigger_listeners(trigger)
            .next()
            .is_none()
        {
            return;
        }
        let Some(startup) = self.cold.config.datapacks.as_ref() else {
            return;
        };
        let Some(matching) = self.with_advancement_player_params(player, |level, params| {
            let advancement_triggers = &self.cold.advancement_triggers;
            let mut matching = Vec::new();
            for (index, criterion) in player.cold.advancements.trigger_listeners(trigger) {
                let Some(condition) = advancement_triggers
                    .recipe_crafted
                    .get(trigger)
                    .and_then(|compiled| compiled.get(&index))
                    .and_then(|criteria| criteria.get(criterion))
                else {
                    continue;
                };
                if !condition.matches_inputs(
                    recipe_id,
                    used_ingredients,
                    advancement_triggers.item_tags.as_ref(),
                    &self.datapack_tables.enchantment_registry,
                    level.entities.item_resolver(),
                ) {
                    continue;
                }
                if matches_conditions(
                    &startup.loot,
                    trigger,
                    &condition.player,
                    &params,
                    &level.random,
                ) == Ok(true)
                {
                    matching.push((index, criterion.clone()));
                }
            }
            matching
        }) else {
            return;
        };
        self.apply_advancement_matches(connection_id, matching, events);
    }
}

#[cfg(test)]
mod tests;
