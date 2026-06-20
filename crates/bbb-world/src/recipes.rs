use std::collections::BTreeMap;

use bbb_protocol::packets::{
    StonecutterSelectableRecipeSummary as ProtocolStonecutterSelectableRecipeSummary,
    UpdateRecipes as ProtocolUpdateRecipes,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientRecipesState {
    pub property_sets: BTreeMap<String, Vec<i32>>,
    pub stonecutter_recipes: Vec<ProtocolStonecutterSelectableRecipeSummary>,
}

impl WorldStore {
    pub fn apply_update_recipes(&mut self, packet: ProtocolUpdateRecipes) {
        self.counters.update_recipes_packets += 1;

        self.recipes.property_sets.clear();
        for property_set in packet.property_sets {
            self.recipes
                .property_sets
                .insert(property_set.key, property_set.item_ids);
        }
        self.recipes.stonecutter_recipes = packet.stonecutter_recipes;

        self.counters.recipe_property_sets_tracked = self.recipes.property_sets.len();
        self.counters.recipe_property_set_items_tracked =
            self.recipes.property_sets.values().map(Vec::len).sum();
        self.counters.stonecutter_recipes_tracked = self.recipes.stonecutter_recipes.len();
    }

    pub fn recipes(&self) -> &ClientRecipesState {
        &self.recipes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        IngredientSummary, RecipePropertySetSummary, SlotDisplaySummary,
        StonecutterSelectableRecipeSummary,
    };

    #[test]
    fn update_recipes_replaces_client_recipe_access_state() {
        let mut store = WorldStore::new();

        store.apply_update_recipes(update_recipes(
            vec![
                ("minecraft:furnace_input", vec![42, 43]),
                ("minecraft:smithing_base", vec![99]),
            ],
            vec![stonecutter_entry(vec![11, 12], 4, vec![4, 77])],
        ));

        assert_eq!(
            store.recipes().property_sets.get("minecraft:furnace_input"),
            Some(&vec![42, 43])
        );
        assert_eq!(store.recipes().stonecutter_recipes.len(), 1);

        store.apply_update_recipes(update_recipes(
            vec![("minecraft:smoker_input", vec![7])],
            Vec::new(),
        ));

        assert!(!store
            .recipes()
            .property_sets
            .contains_key("minecraft:furnace_input"));
        assert_eq!(
            store.recipes().property_sets.get("minecraft:smoker_input"),
            Some(&vec![7])
        );
        assert!(store.recipes().stonecutter_recipes.is_empty());

        let counters = store.counters();
        assert_eq!(counters.update_recipes_packets, 2);
        assert_eq!(counters.recipe_property_sets_tracked, 1);
        assert_eq!(counters.recipe_property_set_items_tracked, 1);
        assert_eq!(counters.stonecutter_recipes_tracked, 0);
    }

    fn update_recipes(
        property_sets: Vec<(&str, Vec<i32>)>,
        stonecutter_recipes: Vec<StonecutterSelectableRecipeSummary>,
    ) -> ProtocolUpdateRecipes {
        ProtocolUpdateRecipes {
            property_sets: property_sets
                .into_iter()
                .map(|(key, item_ids)| RecipePropertySetSummary {
                    key: key.to_string(),
                    item_ids,
                })
                .collect(),
            stonecutter_recipes,
        }
    }

    fn stonecutter_entry(
        item_ids: Vec<i32>,
        display_type_id: i32,
        raw_payload: Vec<u8>,
    ) -> StonecutterSelectableRecipeSummary {
        StonecutterSelectableRecipeSummary {
            input: IngredientSummary {
                tag: None,
                item_ids,
            },
            option_display: SlotDisplaySummary {
                display_type_id,
                raw_payload,
                item_stack: None,
            },
        }
    }
}
