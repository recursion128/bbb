use std::collections::BTreeMap;

use bbb_protocol::packets::{
    IngredientSummary as ProtocolIngredientSummary,
    StonecutterSelectableRecipeSummary as ProtocolStonecutterSelectableRecipeSummary,
    UpdateRecipes as ProtocolUpdateRecipes,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

mod slot_display;

use slot_display::slot_display_holders_are_resolved;
pub use slot_display::SlotDisplayResolveError;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientRecipesState {
    pub property_sets: BTreeMap<String, Vec<i32>>,
    pub stonecutter_recipes: Vec<ProtocolStonecutterSelectableRecipeSummary>,
}

impl WorldStore {
    pub fn apply_update_recipes(&mut self, packet: ProtocolUpdateRecipes) {
        self.counters.update_recipes_packets += 1;
        if !packet.stonecutter_recipes.iter().all(|recipe| {
            ingredient_holder_set_is_resolved(self.frozen_registry_access(), &recipe.input)
                && slot_display_holders_are_resolved(self, &recipe.option_display)
        }) {
            return;
        }

        self.recipes.property_sets.clear();
        for property_set in packet.property_sets {
            self.recipes
                .property_sets
                .insert(property_set.key, property_set.item_ids);
        }
        self.recipes.stonecutter_recipes = packet.stonecutter_recipes;

        self.update_recipe_counters();
    }

    pub fn recipes(&self) -> &ClientRecipesState {
        &self.recipes
    }

    pub(crate) fn reset_recipes_for_new_listener(&mut self) {
        self.recipes = ClientRecipesState::default();
        self.update_recipe_counters();
    }

    fn update_recipe_counters(&mut self) {
        self.counters.recipe_property_sets_tracked = self.recipes.property_sets.len();
        self.counters.recipe_property_set_items_tracked =
            self.recipes.property_sets.values().map(Vec::len).sum();
        self.counters.stonecutter_recipes_tracked = self.recipes.stonecutter_recipes.len();
    }
}

pub(crate) fn ingredient_holder_set_is_resolved(
    access: crate::FrozenRegistryAccess<'_>,
    ingredient: &ProtocolIngredientSummary,
) -> bool {
    match (ingredient.tag.as_deref(), ingredient.item_ids.as_slice()) {
        (Some(tag), []) => access.holder_set_by_tag("minecraft:item", tag).is_ok(),
        (None, holder_ids) if !holder_ids.is_empty() => access
            .holder_set_by_holder_ids("minecraft:item", holder_ids)
            .is_ok_and(|holder_set| {
                holder_set
                    .entries
                    .iter()
                    .all(|entry| entry.resource_id != "minecraft:air")
            }),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        IngredientSummary, RecipePropertySetSummary, SlotDisplaySummary,
        StonecutterSelectableRecipeSummary,
    };
    use bbb_protocol::recipe_wire_types::VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID;

    #[test]
    fn update_recipes_replaces_client_recipe_access_state() {
        let mut store = WorldStore::new();

        store.apply_update_recipes(update_recipes(
            vec![
                ("minecraft:furnace_input", vec![42, 43]),
                ("minecraft:smithing_base", vec![99]),
            ],
            vec![stonecutter_entry(vec![11, 12], vec![4, 77])],
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

    #[test]
    fn update_recipes_resolves_named_item_holder_sets_and_rejects_invalid_packets_atomically() {
        let mut store = WorldStore::new();
        store.apply_update_tags(bbb_protocol::packets::UpdateTags {
            registries: vec![bbb_protocol::packets::RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![bbb_protocol::packets::TagNetworkPayload {
                    tag: "example:cuttable".to_string(),
                    entries: vec![42],
                }],
            }],
        });

        let mut tagged = stonecutter_entry(Vec::new(), vec![4, 77]);
        tagged.input.tag = Some("example:cuttable".to_string());
        store.apply_update_recipes(update_recipes(
            vec![("minecraft:furnace_input", vec![42])],
            vec![tagged],
        ));
        let accepted = store.recipes().clone();

        let mut missing_tag = stonecutter_entry(Vec::new(), vec![4, 78]);
        missing_tag.input.tag = Some("example:missing".to_string());
        store.apply_update_recipes(update_recipes(
            vec![("minecraft:smoker_input", vec![43])],
            vec![missing_tag],
        ));
        assert_eq!(store.recipes(), &accepted);

        store.apply_update_recipes(update_recipes(
            Vec::new(),
            vec![stonecutter_entry(vec![i32::MAX], vec![4, 79])],
        ));
        assert_eq!(store.recipes(), &accepted);
        assert_eq!(store.counters().update_recipes_packets, 3);
    }

    #[test]
    fn configuration_reentry_rebuilds_recipe_container_and_preserves_packet_counters() {
        let mut store = WorldStore::new();
        store.apply_update_recipes(update_recipes(
            vec![("minecraft:furnace_input", vec![42, 43])],
            vec![stonecutter_entry(vec![11, 12], vec![4, 77])],
        ));

        store.clear_client_level();

        assert_eq!(store.recipes(), &ClientRecipesState::default());
        assert_eq!(store.counters().update_recipes_packets, 1);
        assert_eq!(store.counters().recipe_property_sets_tracked, 0);
        assert_eq!(store.counters().recipe_property_set_items_tracked, 0);
        assert_eq!(store.counters().stonecutter_recipes_tracked, 0);
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
        raw_payload: Vec<u8>,
    ) -> StonecutterSelectableRecipeSummary {
        StonecutterSelectableRecipeSummary {
            input: IngredientSummary {
                tag: None,
                item_ids,
            },
            option_display: SlotDisplaySummary {
                display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
                raw_payload,
                item_stack: None,
                tag: None,
            },
        }
    }
}
