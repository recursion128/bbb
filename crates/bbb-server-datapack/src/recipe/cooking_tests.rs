//! The cooking lookup a furnace, blast furnace, smoker and campfire each run,
//! derived from `RecipeManager`/`RecipeMap` rather than from this crate's shape.

use std::collections::BTreeMap;
use std::sync::Arc;

use bbb_protocol::item_types::{VANILLA_ITEM_IRON_INGOT_ID, VANILLA_ITEM_RAW_IRON_ID};
use bbb_protocol::packets::ItemStackSummary;
use serde_json::json;

use crate::registry_data_loader::DatapackLoadLimits;
use crate::resource_overlay::{MergedServerDataView, PackOverlayInput};
use crate::vanilla_snapshot::VanillaDatapackSnapshot;

use super::ingredient::{item_stack, IngredientTerm};
use super::*;

fn recipe(body: serde_json::Value) -> Recipe {
    super::test_support::parse_recipe_with_builtin_lookup(&body).expect("test recipe parses")
}

fn element(id: &str, body: serde_json::Value) -> RecipeElement {
    RecipeElement {
        id: id.to_string(),
        source_pack: "test".to_string(),
        recipe: recipe(body),
    }
}

fn set(elements: Vec<RecipeElement>) -> RecipeSet {
    RecipeSet {
        recipes: elements,
        warnings: Vec::new(),
    }
}

fn cooking_body(type_id: &str, ingredient: &str, result: &str) -> serde_json::Value {
    json!({
        "type": type_id,
        "ingredient": ingredient,
        "result": { "id": result }
    })
}

#[test]
fn each_cooking_serializer_takes_the_time_and_experience_its_own_codec_declares() {
    let defaults = [
        ("minecraft:smelting", CookingKind::Smelting, 200),
        ("minecraft:blasting", CookingKind::Blasting, 100),
        ("minecraft:smoking", CookingKind::Smoking, 100),
        ("minecraft:campfire_cooking", CookingKind::Campfire, 100),
    ];
    for (type_id, kind, default_time) in defaults {
        let parsed = recipe(cooking_body(
            type_id,
            "minecraft:raw_iron",
            "minecraft:iron_ingot",
        ));
        let body = parsed.cooking().expect("a cooking body");
        assert_eq!(body.kind, kind);
        assert_eq!(kind.recipe_type_id(), type_id);
        assert_eq!(kind.default_cooking_time(), default_time);
        assert_eq!(body.cooking_time, default_time);
        assert_eq!(body.experience, 0.0);
        assert_eq!(body.assemble().item_id, Some(VANILLA_ITEM_IRON_INGOT_ID));
    }

    let declared = recipe(json!({
        "type": "minecraft:smelting",
        "ingredient": "minecraft:raw_iron",
        "result": { "id": "minecraft:iron_ingot" },
        "experience": 0.7,
        "cookingtime": 123
    }));
    let body = declared.cooking().expect("a cooking body");
    assert_eq!(body.cooking_time, 123);
    assert_eq!(body.experience, 0.7);
}

#[test]
fn the_cooking_lookup_scans_only_its_own_recipe_type() {
    let recipes = set(vec![
        element(
            "minecraft:a_blasting",
            cooking_body(
                "minecraft:blasting",
                "minecraft:raw_iron",
                "minecraft:iron_ingot",
            ),
        ),
        element(
            "minecraft:b_smelting",
            cooking_body(
                "minecraft:smelting",
                "minecraft:raw_iron",
                "minecraft:iron_ingot",
            ),
        ),
    ]);
    let tags = ItemTagIndex::new();
    let input = item_stack(VANILLA_ITEM_RAW_IRON_ID, 1);

    for (kind, expected) in [
        (CookingKind::Blasting, Some("minecraft:a_blasting")),
        (CookingKind::Smelting, Some("minecraft:b_smelting")),
        (CookingKind::Smoking, None),
        (CookingKind::Campfire, None),
    ] {
        assert_eq!(
            recipes
                .get_recipe_for_cooking(kind, &input, None, &tags)
                .map(|element| element.id.as_str()),
            expected
        );
    }
}

#[test]
fn the_cooking_lookup_takes_the_first_match_of_its_type_in_identifier_order() {
    let body = cooking_body("minecraft:smelting", "minecraft:raw_iron", "minecraft:coal");
    let recipes = set(vec![
        element("minecraft:a_first", body.clone()),
        element("minecraft:b_second", body),
    ]);
    let chosen = recipes
        .get_recipe_for_cooking(
            CookingKind::Smelting,
            &item_stack(VANILLA_ITEM_RAW_IRON_ID, 1),
            None,
            &ItemTagIndex::new(),
        )
        .expect("a recipe matches");
    assert_eq!(chosen.id, "minecraft:a_first");
}

#[test]
fn an_empty_input_reaches_no_cooking_recipe() {
    let recipes = set(vec![element(
        "minecraft:smelt",
        cooking_body(
            "minecraft:smelting",
            "minecraft:raw_iron",
            "minecraft:iron_ingot",
        ),
    )]);
    let tags = ItemTagIndex::new();

    for input in [
        ItemStackSummary::empty(),
        item_stack(VANILLA_ITEM_RAW_IRON_ID, 0),
    ] {
        assert!(recipes
            .get_recipe_for_cooking(CookingKind::Smelting, &input, None, &tags)
            .is_none());
        assert!(recipes
            .get_recipe_for_cooking(
                CookingKind::Smelting,
                &input,
                Some("minecraft:smelt"),
                &tags
            )
            .is_none());
    }
}

#[test]
fn the_builtin_pack_answers_every_cooking_recipe_over_its_own_type() {
    let snapshot = Arc::new(VanillaDatapackSnapshot::embedded().unwrap());
    let pack = snapshot.vanilla_pack();
    let inputs = [PackOverlayInput {
        pack_id: "vanilla",
        resources: &pack,
        filter: None,
    }];
    let view = MergedServerDataView::merge(&inputs);
    let tags = ItemTagIndex::from_loaded(
        super::test_support::builtin_tags()
            .registry("minecraft:item")
            .unwrap(),
    );
    let recipes = super::test_support::load_recipes_with_builtin_lookup(
        &view,
        &inputs,
        DatapackLoadLimits::default(),
    );

    let mut by_kind = BTreeMap::<&str, usize>::new();
    for element in &recipes.recipes {
        let Some(body) = element.recipe.cooking() else {
            continue;
        };
        *by_kind.entry(body.kind.recipe_type_id()).or_default() += 1;
        let first_item = body
            .ingredient
            .terms
            .iter()
            .find_map(|term| match term {
                IngredientTerm::Item(item_id) => Some(*item_id),
                IngredientTerm::Tag(tag) => tags.members(tag).next(),
            })
            .expect("every builtin cooking ingredient resolves to an item");
        let found = recipes
            .get_recipe_for_cooking(body.kind, &item_stack(first_item, 1), None, &tags)
            .expect("the input its own recipe declares reaches a recipe of that type");
        assert_eq!(
            found
                .recipe
                .cooking()
                .expect("cooking body")
                .kind
                .recipe_type_id(),
            body.kind.recipe_type_id()
        );
    }

    assert_eq!(
        by_kind,
        BTreeMap::from([
            ("minecraft:blasting", 25),
            ("minecraft:campfire_cooking", 9),
            ("minecraft:smelting", 73),
            ("minecraft:smoking", 9),
        ])
    );

    let iron = recipes
        .get_recipe_for_cooking(
            CookingKind::Smelting,
            &item_stack(VANILLA_ITEM_RAW_IRON_ID, 1),
            None,
            &tags,
        )
        .expect("raw iron smelts")
        .recipe
        .cooking()
        .expect("cooking body")
        .clone();
    assert_eq!(iron.cooking_time, 200);
    assert_eq!(iron.experience, 0.7);
    assert_eq!(iron.assemble().item_id, Some(VANILLA_ITEM_IRON_INGOT_ID));

    let blasted = recipes
        .get_recipe_for_cooking(
            CookingKind::Blasting,
            &item_stack(VANILLA_ITEM_RAW_IRON_ID, 1),
            None,
            &tags,
        )
        .expect("raw iron blasts")
        .recipe
        .cooking()
        .expect("cooking body")
        .clone();
    assert_eq!(blasted.cooking_time, 100);
}
