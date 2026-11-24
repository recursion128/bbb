//! `PlacementInfo` and the `PlaceRecipeHelper` grid walk, derived from the
//! Vanilla owner of each rather than from this crate's own shape.

use std::collections::BTreeMap;
use std::sync::Arc;

use bbb_protocol::item_types::{
    VANILLA_ITEM_OAK_PLANKS_ID, VANILLA_ITEM_RED_DYE_ID, VANILLA_ITEM_STICK_ID,
};
use bbb_protocol::packets::ItemStackSummary;
use serde_json::json;

use crate::pack::ServerPackResources;
use crate::registry_data_loader::DatapackLoadLimits;
use crate::resource_overlay::{MergedServerDataView, PackOverlayInput};
use crate::vanilla_snapshot::VanillaDatapackSnapshot;

use super::ingredient::item_stack;
use super::placement::{place_recipe, EMPTY_SLOT};
use super::test_support::{builtin_lookup, load_recipes_with_builtin_lookup};
use super::*;

fn recipe(body: serde_json::Value) -> Recipe {
    super::test_support::parse_recipe_with_builtin_lookup(&body).expect("test recipe parses")
}

fn tags() -> ItemTagIndex {
    let mut tags = ItemTagIndex::new();
    tags.insert("minecraft:planks", [VANILLA_ITEM_OAK_PLANKS_ID]);
    tags.insert("minecraft:dyes", [VANILLA_ITEM_RED_DYE_ID]);
    tags.insert("minecraft:empty_tag", []);
    tags
}

fn grid(width: i32, height: i32, cells: &[i32]) -> CraftingInput {
    let items = cells
        .iter()
        .map(|item_id| {
            if *item_id < 0 {
                ItemStackSummary::empty()
            } else {
                item_stack(*item_id, 1)
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(items.len(), (width * height) as usize);
    CraftingInput::of(width, height, items)
}

/// The `L` of a shaped 2x2 that is neither symmetrical nor square-filled, so a
/// mirrored input reaches a different pattern cell than the declared one.
fn asymmetric_2x2() -> Recipe {
    recipe(json!({
        "type": "minecraft:crafting_shaped",
        "key": { "#": "minecraft:oak_planks", "S": "minecraft:stick" },
        "pattern": ["#S", "##"],
        "result": { "id": "minecraft:crafting_table" }
    }))
}

#[test]
fn shaped_placement_info_keeps_the_holes_its_pattern_declares() {
    let recipe = recipe(json!({
        "type": "minecraft:crafting_shaped",
        "key": { "#": "minecraft:oak_planks" },
        "pattern": ["# #", "###"],
        "result": { "id": "minecraft:crafting_table" }
    }));
    let info = recipe.placement_info(&tags());
    assert_eq!(info.ingredients().len(), 5);
    assert_eq!(
        info.slots_to_ingredient_index(),
        [0, EMPTY_SLOT, 1, 2, 3, 4]
    );
    assert!(!info.is_impossible_to_place());
}

#[test]
fn every_placed_kind_builds_the_ingredient_list_its_vanilla_body_does() {
    let tags = tags();

    let shapeless = recipe(json!({
        "type": "minecraft:crafting_shapeless",
        "ingredients": ["minecraft:coal", "minecraft:stick"],
        "result": { "id": "minecraft:torch", "count": 4 }
    }));
    assert_eq!(
        shapeless.placement_info(&tags).slots_to_ingredient_index(),
        [0, 1]
    );

    // `TransmuteRecipe.createPlacementInfo` repeats the material to its declared
    // maximum, which defaults to one (`TransmuteRecipe.java:25-27`).
    let transmute = recipe(json!({
        "type": "minecraft:crafting_transmute",
        "input": "minecraft:stone",
        "material": "minecraft:coal",
        "material_count": { "min": 1, "max": 4 },
        "result": { "id": "minecraft:diamond" }
    }));
    let info = transmute.placement_info(&tags);
    assert_eq!(info.slots_to_ingredient_index(), [0, 1, 2, 3, 4]);
    assert!(info.ingredients()[1..]
        .iter()
        .all(|ingredient| ingredient.terms == info.ingredients()[1].terms));
    assert_ne!(info.ingredients()[0].terms, info.ingredients()[1].terms);

    let dye = recipe(json!({
        "type": "minecraft:crafting_dye",
        "target": "minecraft:leather_helmet",
        "dye": "#minecraft:dyes",
        "result": { "id": "minecraft:leather_helmet" }
    }));
    assert_eq!(dye.placement_info(&tags).ingredients().len(), 2);

    // `ImbueRecipe.createPlacementInfo` puts the source in the middle of eight
    // material slots (`ImbueRecipe.java:94-98`).
    let imbue = recipe(json!({
        "type": "minecraft:crafting_imbue",
        "source": "minecraft:stone",
        "material": "minecraft:coal",
        "result": { "id": "minecraft:diamond" }
    }));
    let info = imbue.placement_info(&tags);
    assert_eq!(
        info.slots_to_ingredient_index(),
        [0, 1, 2, 3, 4, 5, 6, 7, 8]
    );
    assert_ne!(info.ingredients()[4].terms, info.ingredients()[0].terms);
    assert_eq!(info.ingredients()[8].terms, info.ingredients()[0].terms);

    let smelting = recipe(json!({
        "type": "minecraft:smelting",
        "ingredient": "minecraft:raw_iron",
        "result": { "id": "minecraft:iron_ingot" }
    }));
    assert_eq!(
        smelting.placement_info(&tags).slots_to_ingredient_index(),
        [0]
    );

    // `SmithingTransformRecipe.createPlacementInfo` runs through
    // `createFromOptionals`, so an absent template leaves an EMPTY_SLOT.
    let smithing = recipe(json!({
        "type": "minecraft:smithing_transform",
        "base": "minecraft:diamond_sword",
        "addition": "minecraft:netherite_ingot",
        "result": { "id": "minecraft:netherite_sword" }
    }));
    assert_eq!(
        smithing.placement_info(&tags).slots_to_ingredient_index(),
        [EMPTY_SLOT, 0, 1]
    );

    let trim = recipe(json!({
        "type": "minecraft:smithing_trim",
        "template": "minecraft:coast_armor_trim_smithing_template",
        "base": "minecraft:iron_chestplate",
        "addition": "minecraft:copper_ingot",
        "pattern": "minecraft:coast"
    }));
    assert_eq!(
        trim.placement_info(&tags).slots_to_ingredient_index(),
        [0, 1, 2]
    );
}

#[test]
fn a_special_recipe_has_no_placement_info_at_all() {
    let special = recipe(json!({ "type": "minecraft:crafting_special_repairitem" }));
    let info = special.placement_info(&tags());
    assert!(info.ingredients().is_empty());
    assert!(info.is_impossible_to_place());
}

#[test]
fn an_ingredient_whose_tag_resolved_to_nothing_is_not_placeable() {
    let recipe = recipe(json!({
        "type": "minecraft:crafting_shapeless",
        "ingredients": ["#minecraft:empty_tag", "minecraft:stick"],
        "result": { "id": "minecraft:torch" }
    }));
    let info = recipe.placement_info(&tags());
    assert!(info.is_impossible_to_place());
    assert!(info.ingredients().is_empty());
}

#[test]
fn place_recipe_centers_a_narrow_recipe_the_way_vanilla_does() {
    let visited = |grid_width, grid_height, recipe_width, recipe_height, entries: &[i32]| {
        let mut visited = Vec::new();
        place_recipe(
            grid_width,
            grid_height,
            recipe_width,
            recipe_height,
            entries,
            |entry, grid_index, grid_x, grid_y| visited.push((*entry, grid_index, grid_x, grid_y)),
        );
        visited
    };

    assert_eq!(visited(3, 3, 1, 1, &[10]), vec![(10, 4, 1, 1)]);
    assert_eq!(visited(2, 2, 1, 1, &[20]), vec![(20, 0, 0, 0)]);
    assert_eq!(
        visited(3, 3, 1, 2, &[30, 31]),
        vec![(30, 1, 1, 0), (31, 4, 1, 1)]
    );
    assert_eq!(
        visited(3, 3, 2, 2, &[40, 41, 42, 43]),
        vec![(40, 0, 0, 0), (41, 1, 1, 0), (42, 3, 0, 1), (43, 4, 1, 1)]
    );
    assert_eq!(
        visited(3, 3, 3, 3, &[0, 1, 2, 3, 4, 5, 6, 7, 8]),
        (0..9)
            .map(|index| (index, index, index % 3, index / 3))
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_shaped_recipe_places_its_own_extent_into_a_larger_grid() {
    let tags = tags();
    let recipe = asymmetric_2x2();

    let placement = recipe.grid_placement(3, 3, &tags).expect("placeable");
    assert_eq!(
        placement.slots(),
        [0, 1, EMPTY_SLOT, 2, 3, EMPTY_SLOT, EMPTY_SLOT, EMPTY_SLOT, EMPTY_SLOT]
    );
    assert_eq!(
        placement.ingredient_at(1).map(|i| i.terms.clone()),
        Some(vec![super::ingredient::IngredientTerm::Item(
            VANILLA_ITEM_STICK_ID
        )])
    );
    assert!(placement.ingredient_at(2).is_none());

    let placement = recipe.grid_placement(2, 2, &tags).expect("placeable");
    assert_eq!(placement.slots(), [0, 1, 2, 3]);
}

#[test]
fn a_shaped_recipe_matched_mirrored_still_places_in_its_declared_orientation() {
    let tags = tags();
    let recipe = asymmetric_2x2();

    let declared = grid(
        2,
        2,
        &[
            VANILLA_ITEM_OAK_PLANKS_ID,
            VANILLA_ITEM_STICK_ID,
            VANILLA_ITEM_OAK_PLANKS_ID,
            VANILLA_ITEM_OAK_PLANKS_ID,
        ],
    );
    let mirrored = grid(
        2,
        2,
        &[
            VANILLA_ITEM_STICK_ID,
            VANILLA_ITEM_OAK_PLANKS_ID,
            VANILLA_ITEM_OAK_PLANKS_ID,
            VANILLA_ITEM_OAK_PLANKS_ID,
        ],
    );
    assert!(recipe.matches_crafting(&builtin_lookup(), &declared, &tags, None));
    assert!(recipe.matches_crafting(&builtin_lookup(), &mirrored, &tags, None));

    // `ServerPlaceRecipe` reads `placementInfo`, which the mirror never touches.
    let stick = vec![super::ingredient::IngredientTerm::Item(
        VANILLA_ITEM_STICK_ID,
    )];
    let placement = recipe.grid_placement(2, 2, &tags).expect("placeable");
    assert_eq!(
        placement.ingredient_at(1).map(|i| i.terms.clone()),
        Some(stick)
    );
}

#[test]
fn a_recipe_with_no_shape_fills_the_grid_it_is_placed_into() {
    let tags = tags();
    let shapeless = recipe(json!({
        "type": "minecraft:crafting_shapeless",
        "ingredients": ["minecraft:coal", "minecraft:stick", "minecraft:stone"],
        "result": { "id": "minecraft:torch", "count": 4 }
    }));

    let placement = shapeless.grid_placement(3, 3, &tags).expect("placeable");
    assert_eq!(
        placement.slots(),
        [0, 1, 2, EMPTY_SLOT, EMPTY_SLOT, EMPTY_SLOT, EMPTY_SLOT, EMPTY_SLOT, EMPTY_SLOT]
    );

    // `AbstractFurnaceMenu` places into one slot (`AbstractFurnaceMenu.java:193`).
    let smelting = recipe(json!({
        "type": "minecraft:smelting",
        "ingredient": "minecraft:raw_iron",
        "result": { "id": "minecraft:iron_ingot" }
    }));
    let placement = smelting.grid_placement(1, 1, &tags).expect("placeable");
    assert_eq!(placement.slots(), [0]);
}

#[test]
fn placement_refuses_what_no_menu_places_instead_of_laying_it_out() {
    let tags = tags();

    let special = recipe(json!({
        "type": "minecraft:crafting_special_bookcloning",
        "source": "minecraft:written_book",
        "material": "minecraft:paper",
        "result": { "id": "minecraft:written_book" }
    }));
    assert_eq!(
        special.grid_placement(3, 3, &tags),
        Err(PlacementRefusal::Special)
    );

    let stonecutting = recipe(json!({
        "type": "minecraft:stonecutting",
        "ingredient": "minecraft:stone",
        "result": { "id": "minecraft:stone_bricks" }
    }));
    assert_eq!(
        stonecutting.grid_placement(1, 1, &tags),
        Err(PlacementRefusal::NoPlacingMenu)
    );

    let trim = recipe(json!({
        "type": "minecraft:smithing_trim",
        "template": "minecraft:coast_armor_trim_smithing_template",
        "base": "minecraft:iron_chestplate",
        "addition": "minecraft:copper_ingot",
        "pattern": "minecraft:coast"
    }));
    assert_eq!(
        trim.grid_placement(3, 3, &tags),
        Err(PlacementRefusal::NoPlacingMenu)
    );

    let impossible = recipe(json!({
        "type": "minecraft:crafting_shapeless",
        "ingredients": ["#minecraft:empty_tag"],
        "result": { "id": "minecraft:torch" }
    }));
    assert_eq!(
        impossible.grid_placement(3, 3, &tags),
        Err(PlacementRefusal::ImpossibleToPlace)
    );
}

/// One in-memory pack, enough to run `load_recipes` over two namespaces.
struct MockPack {
    files: BTreeMap<(String, String), Vec<u8>>,
}

impl ServerPackResources for MockPack {
    fn provided_namespaces(&self) -> Vec<String> {
        let mut namespaces = self
            .files
            .keys()
            .map(|(namespace, _)| namespace.clone())
            .collect::<Vec<_>>();
        namespaces.dedup();
        namespaces
    }

    fn list_paths(&self, namespace: &str, dir: &str) -> Vec<String> {
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{dir}/")
        };
        self.files
            .keys()
            .filter(|(pack_namespace, path)| {
                pack_namespace == namespace && path.starts_with(&prefix)
            })
            .map(|(_, path)| path.clone())
            .collect()
    }

    fn read(&self, namespace: &str, path: &str) -> Option<Vec<u8>> {
        self.files
            .get(&(namespace.to_string(), path.to_string()))
            .cloned()
    }

    fn read_root(&self, _path: &str) -> Option<Vec<u8>> {
        None
    }
}

#[test]
fn the_lookup_scans_in_identifier_order_which_reads_the_path_first() {
    let body = json!({
        "type": "minecraft:crafting_shapeless",
        "ingredients": ["minecraft:stick"],
        "result": { "id": "minecraft:torch" }
    })
    .to_string()
    .into_bytes();
    let pack = MockPack {
        files: [
            (
                ("minecraft".to_string(), "recipe/b.json".to_string()),
                body.clone(),
            ),
            (
                ("zebra".to_string(), "recipe/a.json".to_string()),
                body.clone(),
            ),
            (("alpaca".to_string(), "recipe/a.json".to_string()), body),
        ]
        .into_iter()
        .collect(),
    };
    let inputs = [PackOverlayInput {
        pack_id: "test",
        resources: &pack,
        filter: None,
    }];
    let view = MergedServerDataView::merge(&inputs);
    let recipes = load_recipes_with_builtin_lookup(&view, &inputs, DatapackLoadLimits::default());

    assert_eq!(
        recipes
            .recipes
            .iter()
            .map(|element| element.id.as_str())
            .collect::<Vec<_>>(),
        ["alpaca:a", "zebra:a", "minecraft:b"]
    );
}

#[test]
fn the_builtin_pack_counts_its_placement_refusals_by_reason() {
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
    assert_eq!(recipes.recipes.len(), 1515);

    let mut crafting_grid = 0;
    let mut furnace_grid = 0;
    let mut refused = BTreeMap::<PlacementRefusal, usize>::new();
    for element in &recipes.recipes {
        match element.recipe.grid_placement(3, 3, &tags) {
            Ok(_) if matches!(element.recipe.kind, RecipeKind::Cooking(_)) => furnace_grid += 1,
            Ok(_) => crafting_grid += 1,
            Err(refusal) => *refused.entry(refusal).or_default() += 1,
        }
    }

    assert_eq!(crafting_grid, 1070);
    assert_eq!(furnace_grid, 116);
    assert_eq!(
        refused,
        BTreeMap::from([
            (PlacementRefusal::Special, 24),
            (PlacementRefusal::NoPlacingMenu, 305),
        ])
    );

    // A furnace places into its one input slot (`AbstractFurnaceMenu.java:193`).
    let single = recipes
        .recipes
        .iter()
        .filter(|element| matches!(element.recipe.kind, RecipeKind::Cooking(_)))
        .filter(|element| {
            element
                .recipe
                .grid_placement(1, 1, &tags)
                .is_ok_and(|placement| placement.slots() == [0])
        })
        .count();
    assert_eq!(single, 116);
}

#[test]
fn the_first_matching_recipe_in_identifier_order_wins() {
    let planks = json!({
        "type": "minecraft:crafting_shapeless",
        "ingredients": ["minecraft:oak_planks"],
        "result": { "id": "minecraft:stick", "count": 4 }
    });
    let recipes = RecipeSet {
        recipes: vec![
            RecipeElement {
                id: "minecraft:a_first".to_string(),
                source_pack: "test".to_string(),
                recipe: recipe(planks.clone()),
            },
            RecipeElement {
                id: "minecraft:b_second".to_string(),
                source_pack: "test".to_string(),
                recipe: recipe(planks),
            },
        ],
        warnings: Vec::new(),
    };
    let tags = tags();
    let input = grid(1, 1, &[VANILLA_ITEM_OAK_PLANKS_ID]);

    let chosen = recipes
        .get_recipe_for_crafting(&builtin_lookup(), &input, None, &tags, None)
        .expect("a recipe matches");
    assert_eq!(chosen.id, "minecraft:a_first");
    assert_eq!(
        chosen.recipe.assemble_crafting(&input, &tags).item_id,
        Some(VANILLA_ITEM_STICK_ID)
    );

    let hinted = recipes
        .get_recipe_for_crafting(
            &builtin_lookup(),
            &input,
            Some("minecraft:b_second"),
            &tags,
            None,
        )
        .expect("the hint matches");
    assert_eq!(hinted.id, "minecraft:b_second");
}
