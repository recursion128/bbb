use std::collections::BTreeMap;
use std::sync::Arc;

use bbb_protocol::packets::RecipeDisplayId;
use bbb_protocol::recipe_wire_types::{
    VANILLA_RECIPE_BOOK_CATEGORY_BLAST_FURNACE_BLOCKS_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_BLAST_FURNACE_MISC_ID, VANILLA_RECIPE_BOOK_CATEGORY_CAMPFIRE_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_BUILDING_BLOCKS_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_EQUIPMENT_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_REDSTONE_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_BLOCKS_ID, VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_FOOD_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_MISC_ID, VANILLA_RECIPE_BOOK_CATEGORY_SMITHING_ID,
    VANILLA_RECIPE_BOOK_CATEGORY_SMOKER_FOOD_ID, VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID,
};
use serde_json::{json, Value};

use crate::dynamic_registry::DatapackGeneration;
use crate::registry_data_loader::DatapackLoadLimits;
use crate::resource_overlay::{MergedServerDataView, PackOverlayInput};
use crate::vanilla_snapshot::VanillaDatapackSnapshot;

use super::*;

fn library(values: &[(&str, Value)]) -> (RecipeSet, RecipeDisplayCatalog) {
    let mut recipes = values
        .iter()
        .map(|(id, value)| RecipeElement {
            id: (*id).to_string(),
            source_pack: "test".to_string(),
            recipe: super::test_support::parse_recipe_with_builtin_lookup(value).unwrap(),
        })
        .collect::<Vec<_>>();
    recipes.sort_by(|left, right| left.id.cmp(&right.id));
    let mut set = RecipeSet {
        recipes,
        warnings: Vec::new(),
    };
    let catalog = set.finalize_client_catalog(
        &ItemTagIndex::default(),
        &DatapackGeneration {
            registries: Vec::new(),
            worldgen: Default::default(),
        },
    );
    (set, catalog)
}

fn shaped(category: &str, show_notification: Option<bool>) -> Value {
    let mut body = json!({
        "type": "minecraft:crafting_shaped",
        "category": category,
        "key": {"#": "minecraft:oak_planks"},
        "pattern": ["##"],
        "result": {"id": "minecraft:stick", "count": 4}
    });
    if let Some(show_notification) = show_notification {
        body["show_notification"] = json!(show_notification);
    }
    body
}

fn body(type_id: &str) -> Value {
    let mut body = match type_id {
        "minecraft:crafting_shaped" => json!({
            "key": {"#": "minecraft:oak_planks"},
            "pattern": ["##"],
            "result": {"id": "minecraft:stick"}
        }),
        "minecraft:crafting_shapeless" => json!({
            "ingredients": ["minecraft:oak_planks"],
            "result": {"id": "minecraft:stick"}
        }),
        "minecraft:crafting_transmute" => json!({
            "input": "minecraft:shulker_box",
            "material": "minecraft:red_dye",
            "result": {"id": "minecraft:red_shulker_box"}
        }),
        "minecraft:crafting_dye" => json!({
            "target": "minecraft:leather_helmet",
            "dye": "minecraft:red_dye",
            "result": {"id": "minecraft:leather_helmet"}
        }),
        "minecraft:crafting_imbue" => json!({
            "source": "minecraft:potion",
            "material": "minecraft:oak_planks",
            "result": {"id": "minecraft:stick"}
        }),
        "minecraft:smelting"
        | "minecraft:blasting"
        | "minecraft:smoking"
        | "minecraft:campfire_cooking"
        | "minecraft:stonecutting" => json!({
            "ingredient": "minecraft:stone",
            "result": {"id": "minecraft:stick"}
        }),
        "minecraft:smithing_transform" => json!({
            "base": "minecraft:stone",
            "result": {"id": "minecraft:stick"}
        }),
        "minecraft:smithing_trim" => json!({
            "pattern": "minecraft:coast",
            "template": "minecraft:stone",
            "base": "minecraft:stone",
            "addition": "minecraft:stone"
        }),
        "minecraft:crafting_special_repairitem" => json!({}),
        other => panic!("no fixture body for {other}"),
    };
    body["type"] = json!(type_id);
    body
}

fn declared_category(type_id: &str, category: Option<&str>) -> i32 {
    let mut value = body(type_id);
    if let Some(category) = category {
        value["category"] = json!(category);
    }
    super::test_support::parse_recipe_with_builtin_lookup(&value)
        .expect("the fixture body decodes")
        .category_id()
}

/// `Recipe.recipeBookCategory` (`Recipe.java:46`) as each concrete kind answers
/// it: the crafting fold (`CraftingRecipe.java:39-46`) over
/// `CustomRecipe.category` (`CustomRecipe.java:20-22`) for a special recipe,
/// the three per-furnace folds (`SmeltingRecipe.java:42-48`,
/// `BlastingRecipe.java:42-47`, `SmokingRecipe.java:42-44`,
/// `CampfireCookingRecipe.java:42-44`) and the two constants
/// (`StonecutterRecipe.java:47-49`, `SmithingRecipe.java:28-30`).
#[test]
fn every_serializer_falls_into_the_category_its_own_vanilla_kind_answers() {
    for type_id in [
        "minecraft:crafting_shaped",
        "minecraft:crafting_shapeless",
        "minecraft:crafting_transmute",
        "minecraft:crafting_dye",
        "minecraft:crafting_imbue",
    ] {
        assert_eq!(
            [
                declared_category(type_id, Some("building")),
                declared_category(type_id, Some("equipment")),
                declared_category(type_id, Some("redstone")),
                declared_category(type_id, Some("misc")),
                declared_category(type_id, None),
            ],
            [
                VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_BUILDING_BLOCKS_ID,
                VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_EQUIPMENT_ID,
                VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_REDSTONE_ID,
                VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID,
                VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID,
            ],
            "{type_id}"
        );
    }

    // The declared category folds away wherever the Vanilla switch drops it.
    assert_eq!(
        [
            declared_category("minecraft:smelting", Some("blocks")),
            declared_category("minecraft:smelting", Some("food")),
            declared_category("minecraft:smelting", Some("misc")),
            declared_category("minecraft:smelting", None),
        ],
        [
            VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_BLOCKS_ID,
            VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_FOOD_ID,
            VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_MISC_ID,
            VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_MISC_ID,
        ]
    );
    assert_eq!(
        [
            declared_category("minecraft:blasting", Some("blocks")),
            declared_category("minecraft:blasting", Some("food")),
            declared_category("minecraft:blasting", Some("misc")),
            declared_category("minecraft:blasting", None),
        ],
        [
            VANILLA_RECIPE_BOOK_CATEGORY_BLAST_FURNACE_BLOCKS_ID,
            VANILLA_RECIPE_BOOK_CATEGORY_BLAST_FURNACE_MISC_ID,
            VANILLA_RECIPE_BOOK_CATEGORY_BLAST_FURNACE_MISC_ID,
            VANILLA_RECIPE_BOOK_CATEGORY_BLAST_FURNACE_MISC_ID,
        ]
    );
    for category in [Some("blocks"), Some("food"), Some("misc"), None] {
        assert_eq!(
            declared_category("minecraft:smoking", category),
            VANILLA_RECIPE_BOOK_CATEGORY_SMOKER_FOOD_ID
        );
        assert_eq!(
            declared_category("minecraft:campfire_cooking", category),
            VANILLA_RECIPE_BOOK_CATEGORY_CAMPFIRE_ID
        );
    }

    assert_eq!(
        declared_category("minecraft:stonecutting", None),
        VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID
    );
    assert_eq!(
        declared_category("minecraft:smithing_transform", None),
        VANILLA_RECIPE_BOOK_CATEGORY_SMITHING_ID
    );
    assert_eq!(
        declared_category("minecraft:smithing_trim", None),
        VANILLA_RECIPE_BOOK_CATEGORY_SMITHING_ID
    );
    assert_eq!(
        declared_category("minecraft:crafting_special_repairitem", Some("redstone")),
        VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID
    );
}

#[test]
fn an_unlock_carries_the_recipe_notification_and_always_highlights() {
    let (set, catalog) = library(&[
        ("test:loud", shaped("redstone", None)),
        ("test:quiet", shaped("misc", Some(false))),
    ]);
    let library = RecipeBookLibrary::new(&set, &catalog);

    let add = library.unlocked(["test:loud", "test:quiet"]);

    assert!(add.refusals.is_empty(), "{:?}", add.refusals);
    let flags = add
        .entries
        .iter()
        .map(|entry| (entry.notification, entry.highlight, entry.flags))
        .collect::<Vec<_>>();
    assert_eq!(flags, vec![(true, true, 3), (false, true, 2)]);
    assert_eq!(
        add.entries[0].contents.category_id,
        VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_REDSTONE_ID
    );
}

#[test]
fn a_restore_never_notifies_and_highlights_only_where_the_book_says() {
    let (set, catalog) = library(&[
        ("test:seen", shaped("misc", None)),
        ("test:unseen", shaped("misc", None)),
    ]);
    let library = RecipeBookLibrary::new(&set, &catalog);

    let add = library.restored([("test:seen", false), ("test:unseen", true)]);

    assert!(add.refusals.is_empty(), "{:?}", add.refusals);
    let flags = add
        .entries
        .iter()
        .map(|entry| (entry.notification, entry.highlight, entry.flags))
        .collect::<Vec<_>>();
    assert_eq!(flags, vec![(false, false, 0), (false, true, 2)]);
}

#[test]
fn every_display_of_a_multi_display_recipe_becomes_its_own_entry() {
    let (set, catalog) = library(&[(
        "test:transmute",
        json!({
            "type": "minecraft:crafting_transmute",
            "category": "misc",
            "input": "minecraft:shulker_box",
            "material": "minecraft:red_dye",
            "material_count": {"min": 1, "max": 3},
            "result": {"id": "minecraft:red_shulker_box"}
        }),
    )]);
    let library = RecipeBookLibrary::new(&set, &catalog);

    let add = library.unlocked(["test:transmute"]);

    assert_eq!(add.entries.len(), 3);
    for (index, entry) in add.entries.iter().enumerate() {
        assert_eq!(entry.contents.id.index, index as i32);
        assert_eq!(
            library.book_category_id("test:transmute"),
            Some(entry.contents.category_id)
        );
        assert_eq!(
            catalog.recipe_from_display(entry.contents.id),
            Some(("test:transmute", &entry.contents))
        );
    }
}

#[test]
fn the_builtin_pack_holds_every_recipe_but_its_specials() {
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
    let library = RecipeBookLibrary::new(&set, &catalog);

    let holdable = library.holdable_recipe_ids().collect::<Vec<_>>();
    assert_eq!(set.recipes.len(), 1515);
    assert_eq!(holdable.len(), 1491);

    let ids = set
        .recipes
        .iter()
        .map(|element| element.id.as_str())
        .collect::<Vec<_>>();
    let add = library.unlocked(ids.iter().copied());
    assert_eq!(add.entries.len(), 1498);
    assert_eq!(add.refusals.len(), 24);
    assert!(add
        .refusals
        .iter()
        .all(|refusal| refusal.reason == BookRefusalReason::Special));

    let mut by_category = BTreeMap::<i32, usize>::new();
    for entry in &add.entries {
        *by_category.entry(entry.contents.category_id).or_default() += 1;
    }
    assert_eq!(
        by_category,
        BTreeMap::from([
            (
                VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_BUILDING_BLOCKS_ID,
                372
            ),
            (VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_REDSTONE_ID, 115),
            (VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_EQUIPMENT_ID, 124),
            (VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_MISC_ID, 466),
            (VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_FOOD_ID, 9),
            (VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_BLOCKS_ID, 34),
            (VANILLA_RECIPE_BOOK_CATEGORY_FURNACE_MISC_ID, 30),
            (VANILLA_RECIPE_BOOK_CATEGORY_BLAST_FURNACE_BLOCKS_ID, 2),
            (VANILLA_RECIPE_BOOK_CATEGORY_BLAST_FURNACE_MISC_ID, 23),
            (VANILLA_RECIPE_BOOK_CATEGORY_SMOKER_FOOD_ID, 9),
            (VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID, 275),
            (VANILLA_RECIPE_BOOK_CATEGORY_SMITHING_ID, 30),
            (VANILLA_RECIPE_BOOK_CATEGORY_CAMPFIRE_ID, 9),
        ])
    );

    for entry in &add.entries {
        let (parent, display) = catalog
            .recipe_from_display(entry.contents.id)
            .expect("every published display names its recipe");
        assert_eq!(display, &entry.contents);
        assert_eq!(library.book_category_id(parent), Some(display.category_id));
    }
    assert_eq!(
        catalog.recipe_from_display(RecipeDisplayId {
            index: catalog.all_displays().len() as i32
        }),
        None
    );
}
