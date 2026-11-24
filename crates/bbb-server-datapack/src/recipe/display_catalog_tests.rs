use std::sync::Arc;

use bbb_protocol::item_types::{VANILLA_ITEM_COBBLESTONE_ID, VANILLA_ITEM_STONE_ID};
use bbb_protocol::packets::{RecipeDisplayId, SlotDisplayValue};
use bbb_protocol::recipe_wire_types::{
    RecipeDisplayType, VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_EQUIPMENT_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_BLOCKS_ID,
};
use serde_json::{json, Value};

use crate::dynamic_registry::{DatapackGeneration, LoadedDynamicRegistry, RawRegistryElement};
use crate::registry_data_loader::DatapackLoadLimits;
use crate::resource_overlay::{MergedServerDataView, PackOverlayInput};
use crate::vanilla_snapshot::VanillaDatapackSnapshot;

use super::*;

fn recipes(values: &[(&str, Value)]) -> RecipeSet {
    let mut recipes = values
        .iter()
        .map(|(id, value)| RecipeElement {
            id: (*id).to_string(),
            source_pack: "test".to_string(),
            recipe: super::test_support::parse_recipe_with_builtin_lookup(value).unwrap(),
        })
        .collect::<Vec<_>>();
    recipes.sort_by(|left, right| left.id.cmp(&right.id));
    RecipeSet {
        recipes,
        warnings: Vec::new(),
    }
}

fn generation_with_trim(pattern: &str) -> DatapackGeneration {
    DatapackGeneration {
        registries: vec![LoadedDynamicRegistry {
            registry_id: "minecraft:trim_pattern".to_string(),
            elements: vec![RawRegistryElement {
                id: pattern.to_string(),
                source_pack: "test".to_string(),
                source_json: Arc::from(b"{}".as_slice()),
                source: Some(
                    bbb_registry_codec::decode_direct_registry_json(
                        &serde_json::to_vec(&(json!({}))).unwrap(),
                    )
                    .unwrap(),
                ),
                codec_value: None,
            }],
        }],
        worldgen: Default::default(),
    }
}

#[test]
fn compiles_all_wire_recipe_display_forms_and_metadata() {
    let mut set = recipes(&[
        (
            "test:a_shaped",
            json!({
                "type": "minecraft:crafting_shaped",
                "category": "equipment",
                "group": "shared",
                "key": {"#": "minecraft:oak_planks"},
                "pattern": ["# ", "##"],
                "result": {"id": "minecraft:stick", "count": 4}
            }),
        ),
        (
            "test:b_shapeless",
            json!({
                "type": "minecraft:crafting_shapeless",
                "group": "shared",
                "ingredients": ["minecraft:coal", "minecraft:stick"],
                "result": {"id": "minecraft:torch", "count": 4}
            }),
        ),
        (
            "test:c_furnace",
            json!({
                "type": "minecraft:smelting",
                "category": "blocks",
                "ingredient": "minecraft:cobblestone",
                "result": {"id": "minecraft:stone"},
                "experience": 0.1,
                "cookingtime": 20
            }),
        ),
        (
            "test:d_stonecutter",
            json!({
                "type": "minecraft:stonecutting",
                "ingredient": "minecraft:stone",
                "result": {"id": "minecraft:stone_slab", "count": 2}
            }),
        ),
        (
            "test:e_smithing",
            json!({
                "type": "minecraft:smithing_transform",
                "base": "minecraft:diamond_sword",
                "result": {"id": "minecraft:netherite_sword"}
            }),
        ),
    ]);
    let catalog = set.finalize_client_catalog(
        &ItemTagIndex::new(),
        &DatapackGeneration {
            registries: Vec::new(),
            worldgen: Default::default(),
        },
    );

    assert_eq!(catalog.all_displays().len(), 5);
    assert_eq!(
        catalog
            .all_displays()
            .iter()
            .map(|entry| entry.display.display_type)
            .collect::<Vec<_>>(),
        vec![
            RecipeDisplayType::CraftingShaped,
            RecipeDisplayType::CraftingShapeless,
            RecipeDisplayType::Furnace,
            RecipeDisplayType::Stonecutter,
            RecipeDisplayType::Smithing,
        ]
    );
    assert_eq!(catalog.all_displays()[0].group, Some(0));
    assert_eq!(catalog.all_displays()[1].group, Some(0));
    assert_eq!(
        catalog.all_displays()[0].category_id,
        VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_EQUIPMENT_ID
    );
    assert_eq!(
        catalog.all_displays()[2].category_id,
        VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_BLOCKS_ID
    );
    assert_eq!(
        catalog.all_displays()[0]
            .crafting_requirements
            .as_ref()
            .unwrap()
            .len(),
        3
    );
    let smithing = catalog.all_displays()[4].display.smithing.as_ref().unwrap();
    assert!(matches!(
        smithing.template.decode_value().unwrap(),
        SlotDisplayValue::Empty
    ));
    assert!(matches!(
        smithing.addition.decode_value().unwrap(),
        SlotDisplayValue::Empty
    ));
    assert!(set
        .recipes
        .iter()
        .all(|element| element.recipe.show_notification()));
    let shaped = catalog.all_displays()[0].display.crafting.as_ref().unwrap();
    let bbb_protocol::packets::CraftingRecipeDisplaySummary::Shaped { ingredients, .. } =
        shaped.as_ref()
    else {
        panic!("expected shaped display")
    };
    assert!(matches!(
        ingredients[0].decode_value().unwrap(),
        SlotDisplayValue::Composite { .. }
    ));
}

#[test]
fn compiles_dye_and_imbue_wrapper_displays() {
    let mut tags = ItemTagIndex::new();
    tags.insert(
        "minecraft:dyes",
        [bbb_protocol::item_types::VANILLA_ITEM_RED_DYE_ID],
    );
    let mut set = recipes(&[
        (
            "test:dye",
            json!({
                "type": "minecraft:crafting_dye",
                "target": "minecraft:wolf_armor",
                "dye": "#minecraft:dyes",
                "result": {"id": "minecraft:wolf_armor"}
            }),
        ),
        (
            "test:imbue",
            json!({
                "type": "minecraft:crafting_imbue",
                "source": "minecraft:lingering_potion",
                "material": "minecraft:arrow",
                "result": {"id": "minecraft:tipped_arrow", "count": 8}
            }),
        ),
    ]);
    let catalog = set.finalize_client_catalog(
        &tags,
        &DatapackGeneration {
            registries: Vec::new(),
            worldgen: Default::default(),
        },
    );

    assert!(matches!(
        catalog.displays_for_recipe("test:dye")[0]
            .display
            .result()
            .unwrap()
            .decode_value()
            .unwrap(),
        SlotDisplayValue::Dyed { .. }
    ));
    assert!(matches!(
        catalog.displays_for_recipe("test:imbue")[0]
            .display
            .result()
            .unwrap()
            .decode_value()
            .unwrap(),
        SlotDisplayValue::WithAnyPotion { .. }
    ));
}

#[test]
fn expands_transmute_ids_and_keeps_special_zero_display_recipe() {
    let mut set = recipes(&[
        (
            "test:a_transmute",
            json!({
                "type": "minecraft:crafting_transmute",
                "group": "first",
                "input": "minecraft:diamond",
                "material": "minecraft:stick",
                "material_count": {"min": 1, "max": 3},
                "result": {"id": "minecraft:iron_ingot"},
                "add_material_count_to_result": true
            }),
        ),
        (
            "test:z_special",
            json!({"type": "minecraft:crafting_special_repairitem"}),
        ),
    ]);
    let catalog = set.finalize_client_catalog(
        &ItemTagIndex::new(),
        &DatapackGeneration {
            registries: Vec::new(),
            worldgen: Default::default(),
        },
    );

    assert_eq!(set.recipes.len(), 2);
    assert!(!set
        .recipe("test:z_special")
        .unwrap()
        .recipe
        .show_notification());
    assert!(catalog.displays_for_recipe("test:z_special").is_empty());
    let displays = catalog.displays_for_recipe("test:a_transmute");
    assert_eq!(displays.len(), 3);
    assert_eq!(
        displays
            .iter()
            .map(|entry| entry.id.index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        displays
            .iter()
            .map(|entry| {
                entry
                    .display
                    .result()
                    .unwrap()
                    .item_stack
                    .as_ref()
                    .unwrap()
                    .count
            })
            .collect::<Vec<_>>(),
        vec![2, 3, 4]
    );
    assert_eq!(
        catalog.parent_recipe_id(RecipeDisplayId { index: 2 }),
        Some("test:a_transmute")
    );
    assert_eq!(
        catalog.parent_recipe_id(RecipeDisplayId { index: -1 }),
        None
    );
}

#[test]
fn builds_property_sets_stonecutter_rows_and_trim_holder() {
    let mut tags = ItemTagIndex::new();
    tags.insert(
        "test:furnace_inputs",
        [VANILLA_ITEM_COBBLESTONE_ID, VANILLA_ITEM_STONE_ID],
    );
    let mut set = recipes(&[
        (
            "test:furnace",
            json!({
                "type": "minecraft:smelting",
                "ingredient": "#test:furnace_inputs",
                "result": {"id": "minecraft:stone"}
            }),
        ),
        (
            "test:smithing",
            json!({
                "type": "minecraft:smithing_trim",
                "template": "minecraft:sentry_armor_trim_smithing_template",
                "base": "minecraft:diamond_chestplate",
                "addition": "minecraft:iron_ingot",
                "pattern": "test:sentry"
            }),
        ),
        (
            "test:stonecutter",
            json!({
                "type": "minecraft:stonecutting",
                "ingredient": ["minecraft:cobblestone", "minecraft:stone"],
                "result": {"id": "minecraft:stone_slab", "count": 2}
            }),
        ),
    ]);
    let catalog = set.finalize_client_catalog(&tags, &generation_with_trim("test:sentry"));

    assert_eq!(catalog.update_recipes().property_sets.len(), 7);
    let furnace = catalog
        .update_recipes()
        .property_sets
        .iter()
        .find(|property| property.key == "minecraft:furnace_input")
        .unwrap();
    assert_eq!(
        furnace.item_ids,
        vec![VANILLA_ITEM_STONE_ID, VANILLA_ITEM_COBBLESTONE_ID]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    );
    assert_eq!(catalog.update_recipes().stonecutter_recipes.len(), 1);
    assert_eq!(
        catalog.update_recipes().stonecutter_recipes[0]
            .input
            .item_ids,
        vec![VANILLA_ITEM_COBBLESTONE_ID, VANILLA_ITEM_STONE_ID]
    );
    let trim = catalog.displays_for_recipe("test:smithing")[0]
        .display
        .result()
        .unwrap()
        .decode_value()
        .unwrap();
    let SlotDisplayValue::SmithingTrim { pattern, .. } = trim else {
        panic!("expected trim display")
    };
    assert_eq!(pattern.registry_id, Some(0));
}

#[test]
fn builtin_pack_finalizes_with_expected_catalog_shape() {
    let snapshot = Arc::new(VanillaDatapackSnapshot::embedded().unwrap());
    let pack = snapshot.vanilla_pack();
    let inputs = [PackOverlayInput {
        pack_id: "vanilla",
        resources: &pack,
        filter: None,
    }];
    let view = MergedServerDataView::merge(&inputs);
    let generation = super::test_support::builtin_generation();
    let tags = super::test_support::builtin_tags();
    let item_tags = ItemTagIndex::from_loaded(tags.registry("minecraft:item").unwrap());
    let mut set = super::test_support::load_recipes_with_builtin_lookup(
        &view,
        &inputs,
        DatapackLoadLimits::default(),
    );
    let catalog = set.finalize_client_catalog(&item_tags, generation);

    assert_eq!(set.recipes.len(), 1515);
    assert!(set.warnings.is_empty(), "{:?}", set.warnings);
    assert_eq!(catalog.update_recipes().property_sets.len(), 7);
    assert_eq!(catalog.update_recipes().stonecutter_recipes.len(), 275);
    assert_eq!(catalog.displays_for_recipe("minecraft:oak_planks").len(), 1);
    assert_eq!(catalog.all_displays().len(), 1_498);
}
