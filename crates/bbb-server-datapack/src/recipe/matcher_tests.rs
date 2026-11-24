//! `matches` for the custom crafting matchers, derived from the Vanilla class
//! of each rather than from this crate's own shape.

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_ARROW_ID, VANILLA_ITEM_BLUE_DYE_ID, VANILLA_ITEM_DIAMOND_ID,
    VANILLA_ITEM_FEATHER_ID, VANILLA_ITEM_FIREWORK_STAR_ID, VANILLA_ITEM_GLOWSTONE_DUST_ID,
    VANILLA_ITEM_GOLD_NUGGET_ID, VANILLA_ITEM_GUNPOWDER_ID, VANILLA_ITEM_IRON_SWORD_ID,
    VANILLA_ITEM_LEATHER_HORSE_ARMOR_ID, VANILLA_ITEM_LINGERING_POTION_ID, VANILLA_ITEM_PAPER_ID,
    VANILLA_ITEM_RED_DYE_ID, VANILLA_ITEM_RED_SHULKER_BOX_ID, VANILLA_ITEM_SHIELD_ID,
    VANILLA_ITEM_SHULKER_BOX_ID, VANILLA_ITEM_STONE_ID, VANILLA_ITEM_WHITE_BANNER_ID,
    VANILLA_ITEM_WOODEN_SWORD_ID, VANILLA_ITEM_WRITTEN_BOOK_ID,
};
use bbb_protocol::packets::{
    BannerPatternLayerSummary, ItemStackSummary, WrittenBookContentSummary,
};
use serde_json::json;

use super::ingredient::{item_stack, ItemTagIndex};
use super::input::CraftingInput;
use super::test_support::builtin_lookup;
use super::*;

const EMPTY: i32 = -1;

/// Every `#minecraft:` tag the bodies under test name.
fn tags() -> ItemTagIndex {
    let mut tags = ItemTagIndex::new();
    tags.insert(
        "minecraft:dyes",
        [VANILLA_ITEM_RED_DYE_ID, VANILLA_ITEM_BLUE_DYE_ID],
    );
    tags.insert("minecraft:banners", [VANILLA_ITEM_WHITE_BANNER_ID]);
    tags.insert(
        "minecraft:shulker_boxes",
        [VANILLA_ITEM_SHULKER_BOX_ID, VANILLA_ITEM_RED_SHULKER_BOX_ID],
    );
    tags.insert("minecraft:skulls", [VANILLA_ITEM_STONE_ID]);
    tags
}

fn recipe(body: serde_json::Value) -> Recipe {
    super::test_support::parse_recipe_with_builtin_lookup(&body).expect("test recipe parses")
}

/// One grid of bare stacks; `EMPTY` leaves a cell alone.
fn cells(width: i32, height: i32, ids: &[i32]) -> CraftingInput {
    let items = ids
        .iter()
        .map(|id| {
            if *id == EMPTY {
                ItemStackSummary::empty()
            } else {
                item_stack(*id, 1)
            }
        })
        .collect();
    CraftingInput::of(width, height, items)
}

fn stacks(width: i32, height: i32, items: Vec<ItemStackSummary>) -> CraftingInput {
    CraftingInput::of(width, height, items)
}

fn banner_with_pattern(count: i32) -> ItemStackSummary {
    let mut stack = item_stack(VANILLA_ITEM_WHITE_BANNER_ID, count);
    stack.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID];
    stack.component_patch.banner_pattern_layers = vec![BannerPatternLayerSummary {
        registry_id: None,
        asset_id: Some("minecraft:stripe_bottom".to_string()),
        translation_key: None,
        color_id: 15,
    }];
    stack
}

fn book_of_generation(generation: i32) -> ItemStackSummary {
    let mut stack = item_stack(VANILLA_ITEM_WRITTEN_BOOK_ID, 1);
    stack.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID];
    stack.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "Field Notes".into(),
        title_filter: None,
        author: "alice".into(),
        generation,
        pages: vec!["page one".to_string()],
        page_filters: Vec::new(),
        resolved: false,
    });
    stack
}

// -- minecraft:crafting_special_mapextending ------------------------------

// -- minecraft:crafting_special_firework_star -----------------------------

/// A stack whose patch takes one component off the item's registration.
fn without_component(mut stack: ItemStackSummary, type_id: i32) -> ItemStackSummary {
    stack.component_patch.removed_type_ids.push(type_id);
    stack
}

fn firework_star() -> Recipe {
    recipe(json!({
        "type": "minecraft:crafting_special_firework_star",
        "dye": "#minecraft:dyes",
        "fuel": "minecraft:gunpowder",
        "result": { "id": "minecraft:firework_star" },
        "shapes": {
            "burst": "minecraft:feather",
            "creeper": "#minecraft:skulls",
            "large_ball": "minecraft:fire_charge",
            "star": "minecraft:gold_nugget"
        },
        "trail": "minecraft:diamond",
        "twinkle": "minecraft:glowstone_dust"
    }))
}

/// `FireworkStarRecipe.matches` (`FireworkStarRecipe.java:86-136`) is
/// shapeless with no grid guard: fuel plus at least one dye is enough on a
/// 2x2, and the optional shape, trail and twinkle fill a 3x3.
#[test]
fn firework_star_matches_fuel_and_dye_on_both_grids() {
    let recipe = firework_star();
    let tags = tags();
    assert!(recipe.matches_crafting(
        &builtin_lookup(),
        &cells(
            2,
            2,
            &[
                VANILLA_ITEM_GUNPOWDER_ID,
                VANILLA_ITEM_RED_DYE_ID,
                EMPTY,
                EMPTY
            ]
        ),
        &tags,
        None
    ));
    assert!(recipe.matches_crafting(
        &builtin_lookup(),
        &cells(
            3,
            3,
            &[
                VANILLA_ITEM_GUNPOWDER_ID,
                VANILLA_ITEM_RED_DYE_ID,
                VANILLA_ITEM_BLUE_DYE_ID,
                VANILLA_ITEM_GOLD_NUGGET_ID,
                VANILLA_ITEM_DIAMOND_ID,
                VANILLA_ITEM_GLOWSTONE_DUST_ID,
                EMPTY,
                EMPTY,
                EMPTY,
            ],
        ),
        &tags,
        None
    ));
}

/// `:135` needs both flags, `:101-117` allow one each of twinkle, trail and
/// fuel, `:126-130` one shape, and `:122-124` refuse an item that is none of
/// them.
#[test]
fn firework_star_refuses_a_missing_flag_a_double_or_a_stranger() {
    let recipe = firework_star();
    let tags = tags();
    let fuel = VANILLA_ITEM_GUNPOWDER_ID;
    let dye = VANILLA_ITEM_RED_DYE_ID;
    // No dye, then no fuel.
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[fuel, VANILLA_ITEM_DIAMOND_ID, EMPTY, EMPTY]),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[dye, VANILLA_ITEM_DIAMOND_ID, EMPTY, EMPTY]),
        &tags,
        None
    ));
    // A lone fuel: `:87-89` and `:135` both refuse it, and only the pair
    // is load-bearing -- neither guard can be the sole decider, because
    // `hasFuel && hasDye` already needs two distinct cells.
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[fuel, EMPTY, EMPTY, EMPTY]),
        &tags,
        None
    ));
    // A second fuel, a second trail and a second shape each refuse.
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[fuel, dye, fuel, EMPTY]),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(
            2,
            2,
            &[fuel, dye, VANILLA_ITEM_DIAMOND_ID, VANILLA_ITEM_DIAMOND_ID]
        ),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(
            2,
            2,
            &[
                fuel,
                dye,
                VANILLA_ITEM_FEATHER_ID,
                VANILLA_ITEM_GOLD_NUGGET_ID
            ]
        ),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(
            2,
            2,
            &[
                fuel,
                dye,
                VANILLA_ITEM_GLOWSTONE_DUST_ID,
                VANILLA_ITEM_GLOWSTONE_DUST_ID
            ]
        ),
        &tags,
        None
    ));
    // An item that is neither flag, dye nor shape.
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[fuel, dye, VANILLA_ITEM_ARROW_ID, EMPTY]),
        &tags,
        None
    ));
}

// -- minecraft:crafting_special_firework_star_fade ------------------------

fn firework_star_fade() -> Recipe {
    recipe(json!({
        "type": "minecraft:crafting_special_firework_star_fade",
        "dye": "#minecraft:dyes",
        "result": { "id": "minecraft:firework_star" },
        "target": "minecraft:firework_star"
    }))
}

/// `FireworkStarFadeRecipe.matches` (`FireworkStarFadeRecipe.java:45-73`):
/// one target and any number of dyes, on either grid.
#[test]
fn firework_star_fade_matches_one_star_and_any_dyes() {
    let recipe = firework_star_fade();
    let tags = tags();
    let star = VANILLA_ITEM_FIREWORK_STAR_ID;
    assert!(recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[star, VANILLA_ITEM_RED_DYE_ID, EMPTY, EMPTY]),
        &tags,
        None
    ));
    assert!(recipe.matches_crafting(
        &builtin_lookup(),
        &cells(
            3,
            3,
            &[
                star,
                VANILLA_ITEM_RED_DYE_ID,
                VANILLA_ITEM_BLUE_DYE_ID,
                VANILLA_ITEM_RED_DYE_ID,
                VANILLA_ITEM_BLUE_DYE_ID,
                VANILLA_ITEM_RED_DYE_ID,
                EMPTY,
                EMPTY,
                EMPTY,
            ],
        ),
        &tags,
        None
    ));
}

/// `:63-65` allow only one target, `:59-61` refuse a stranger and `:72` needs
/// a dye to be present at all.
#[test]
fn firework_star_fade_refuses_two_targets_a_stranger_or_no_dye() {
    let recipe = firework_star_fade();
    let tags = tags();
    let star = VANILLA_ITEM_FIREWORK_STAR_ID;
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[star, star, VANILLA_ITEM_RED_DYE_ID, EMPTY]),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(
            2,
            2,
            &[star, VANILLA_ITEM_RED_DYE_ID, VANILLA_ITEM_ARROW_ID, EMPTY]
        ),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[star, star, EMPTY, EMPTY]),
        &tags,
        None
    ));
    // `:56` asks the stack for `DataComponents.DYE`, like the star recipe.
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                item_stack(star, 1),
                without_component(
                    item_stack(VANILLA_ITEM_RED_DYE_ID, 1),
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
                ),
                ItemStackSummary::empty(),
                ItemStackSummary::empty(),
            ],
        ),
        &tags,
        None
    ));
}

// -- minecraft:crafting_special_shielddecoration --------------------------

fn shield_decoration() -> Recipe {
    recipe(json!({
        "type": "minecraft:crafting_special_shielddecoration",
        "banner": "#minecraft:banners",
        "result": { "id": "minecraft:shield" },
        "target": "minecraft:shield"
    }))
}

/// `ShieldDecorationRecipe.matches` (`ShieldDecorationRecipe.java:44-81`) has
/// no grid guard, so the same two ingredients match wherever they sit.
#[test]
fn shield_decoration_matches_a_banner_and_a_clear_shield_on_both_grids() {
    let recipe = shield_decoration();
    let tags = tags();
    let pair = [
        banner_with_pattern(1),
        item_stack(VANILLA_ITEM_SHIELD_ID, 1),
    ];
    assert!(recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                pair[0].clone(),
                pair[1].clone(),
                ItemStackSummary::empty(),
                ItemStackSummary::empty(),
            ],
        ),
        &tags,
        None
    ));
    let mut wide = vec![ItemStackSummary::empty(); 9];
    wide[0] = pair[0].clone();
    wide[8] = pair[1].clone();
    assert!(recipe.matches_crafting(&builtin_lookup(), &stacks(3, 3, wide), &tags, None));
    // `:55-60` never asks the banner for patterns, unlike the duplicate recipe.
    assert!(recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                item_stack(VANILLA_ITEM_WHITE_BANNER_ID, 1),
                item_stack(VANILLA_ITEM_SHIELD_ID, 1),
                ItemStackSummary::empty(),
                ItemStackSummary::empty(),
            ],
        ),
        &tags,
        None
    ));
}

/// `:70-73` refuse a target that already carries patterns. The other two
/// refusals here are decided by `:45-47` and `:56-58` together: with exactly
/// two ingredients a second banner already leaves `:80` without a target, so
/// neither guard is observable on its own.
#[test]
fn shield_decoration_refuses_a_decorated_shield_two_banners_or_a_third_item() {
    let recipe = shield_decoration();
    let tags = tags();
    let mut decorated_shield = item_stack(VANILLA_ITEM_SHIELD_ID, 1);
    decorated_shield.component_patch = banner_with_pattern(1).component_patch;
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                banner_with_pattern(1),
                decorated_shield,
                ItemStackSummary::empty(),
                ItemStackSummary::empty(),
            ],
        ),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                banner_with_pattern(1),
                banner_with_pattern(1),
                item_stack(VANILLA_ITEM_SHIELD_ID, 1),
                ItemStackSummary::empty(),
            ],
        ),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                banner_with_pattern(1),
                item_stack(VANILLA_ITEM_SHIELD_ID, 1),
                item_stack(VANILLA_ITEM_ARROW_ID, 1),
                ItemStackSummary::empty(),
            ],
        ),
        &tags,
        None
    ));
}

// -- minecraft:crafting_special_repairitem --------------------------------

fn repair_item() -> Recipe {
    recipe(json!({ "type": "minecraft:crafting_special_repairitem" }))
}

/// `RepairItemRecipe.getItemsToCombine` (`RepairItemRecipe.java:24-43`) plus
/// `canCombine` (`:45-53`): exactly two ingredients, the same item, one each,
/// both carrying max-damage and damage. No grid guard, so either grid works.
#[test]
fn repair_item_matches_two_single_damageable_twins_on_both_grids() {
    let recipe = repair_item();
    let tags = tags();
    let sword = VANILLA_ITEM_WOODEN_SWORD_ID;
    assert!(recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[sword, sword, EMPTY, EMPTY]),
        &tags,
        None
    ));
    let mut wide = [EMPTY; 9];
    wide[0] = sword;
    wide[8] = sword;
    assert!(recipe.matches_crafting(&builtin_lookup(), &cells(3, 3, &wide), &tags, None));
}

/// `:25-27` refuse anything but two ingredients, `:46` refuses two different
/// items, `:47-48` refuse a stack of more than one, and `:49-52` refuse an
/// item with no durability at all.
#[test]
fn repair_item_refuses_mismatched_stacked_or_undamageable_inputs() {
    let recipe = repair_item();
    let tags = tags();
    let sword = VANILLA_ITEM_WOODEN_SWORD_ID;
    // `:46` compares the items, so two different tools refuse even though
    // both are repairable on their own.
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[sword, VANILLA_ITEM_IRON_SWORD_ID, EMPTY, EMPTY]),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[sword, VANILLA_ITEM_STONE_ID, EMPTY, EMPTY]),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[sword, sword, sword, EMPTY]),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[sword, EMPTY, EMPTY, EMPTY]),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(
            2,
            2,
            &[VANILLA_ITEM_STONE_ID, VANILLA_ITEM_STONE_ID, EMPTY, EMPTY]
        ),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                item_stack(sword, 2),
                item_stack(sword, 1),
                ItemStackSummary::empty(),
                ItemStackSummary::empty(),
            ],
        ),
        &tags,
        None
    ));
}

// -- minecraft:crafting_special_bookcloning -------------------------------

fn book_cloning() -> Recipe {
    recipe(json!({
        "type": "minecraft:crafting_special_bookcloning",
        "material": "minecraft:paper",
        "result": { "id": "minecraft:written_book" },
        "source": "minecraft:written_book"
    }))
}

/// `BookCloningRecipe.matches` (`BookCloningRecipe.java:58-91`): one written
/// source and at least one material, on either grid.
#[test]
fn book_cloning_matches_one_written_source_and_material_on_both_grids() {
    let recipe = book_cloning();
    let tags = tags();
    assert!(recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                book_of_generation(0),
                item_stack(VANILLA_ITEM_PAPER_ID, 1),
                ItemStackSummary::empty(),
                ItemStackSummary::empty(),
            ],
        ),
        &tags,
        None
    ));
    let mut wide = vec![item_stack(VANILLA_ITEM_PAPER_ID, 1); 9];
    wide[4] = book_of_generation(1);
    assert!(recipe.matches_crafting(&builtin_lookup(), &stacks(3, 3, wide), &tags, None));
}

/// `:70-73`: `canCraftCopy` (`:54-56`) holds the generation to
/// `allowed_generations`, which defaults to 0..1
/// (`DEFAULT_BOOK_GENERATION_RANGES`, `:18`), so a copy of a copy cannot be
/// cloned again and a book with no written content is not a source at all.
#[test]
fn book_cloning_refuses_a_second_generation_copy_and_a_blank_book() {
    let recipe = book_cloning();
    let tags = tags();
    let paper = item_stack(VANILLA_ITEM_PAPER_ID, 1);
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                book_of_generation(2),
                paper.clone(),
                ItemStackSummary::empty(),
                ItemStackSummary::empty(),
            ],
        ),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                item_stack(VANILLA_ITEM_WRITTEN_BOOK_ID, 1),
                paper.clone(),
                ItemStackSummary::empty(),
                ItemStackSummary::empty(),
            ],
        ),
        &tags,
        None
    ));
    // `:75-77` one source only, `:81-83` no stranger, and a lone book that
    // `:90` refuses for want of a material.
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                book_of_generation(0),
                book_of_generation(0),
                paper.clone(),
                ItemStackSummary::empty(),
            ],
        ),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                book_of_generation(0),
                paper,
                item_stack(VANILLA_ITEM_ARROW_ID, 1),
                ItemStackSummary::empty(),
            ],
        ),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                book_of_generation(0),
                ItemStackSummary::empty(),
                ItemStackSummary::empty(),
                ItemStackSummary::empty(),
            ],
        ),
        &tags,
        None
    ));
}

/// A `allowed_generations` the body widens is honoured rather than ignored.
#[test]
fn book_cloning_honours_a_widened_allowed_generations() {
    let recipe = recipe(json!({
        "type": "minecraft:crafting_special_bookcloning",
        "allowed_generations": { "min": 0, "max": 2 },
        "material": "minecraft:paper",
        "result": { "id": "minecraft:written_book" },
        "source": "minecraft:written_book"
    }));
    assert!(recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                book_of_generation(2),
                item_stack(VANILLA_ITEM_PAPER_ID, 1),
                ItemStackSummary::empty(),
                ItemStackSummary::empty(),
            ],
        ),
        &tags(),
        None
    ));
}

// -- minecraft:crafting_transmute -----------------------------------------

fn shulker_transmute() -> Recipe {
    recipe(json!({
        "type": "minecraft:crafting_transmute",
        "group": "shulker_box_dye",
        "input": "#minecraft:shulker_boxes",
        "material": "minecraft:red_dye",
        "result": { "id": "minecraft:red_shulker_box" }
    }))
}

/// `TransmuteRecipe.matches` (`TransmuteRecipe.java:97-139`): one input and a
/// material count inside the declared bounds, which default to exactly one
/// (`DEFAULT_MATERIAL_COUNT`, `:24`).
#[test]
fn transmute_matches_one_input_and_the_declared_material_count() {
    let recipe = shulker_transmute();
    let tags = tags();
    assert!(recipe.matches_crafting(
        &builtin_lookup(),
        &cells(
            2,
            2,
            &[
                VANILLA_ITEM_SHULKER_BOX_ID,
                VANILLA_ITEM_RED_DYE_ID,
                EMPTY,
                EMPTY
            ]
        ),
        &tags,
        None
    ));
    // `:100`: a second material puts the ingredient count past max + 1.
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(
            2,
            2,
            &[
                VANILLA_ITEM_SHULKER_BOX_ID,
                VANILLA_ITEM_RED_DYE_ID,
                VANILLA_ITEM_RED_DYE_ID,
                EMPTY,
            ],
        ),
        &tags,
        None
    ));
}

/// `:126-132`: a result of exactly one that `createWithOriginalComponents`
/// (`:85-87`) would rebuild from the input's own patch *is* the input, and
/// vanilla refuses to offer it -- dyeing a red shulker box red is not a
/// recipe.
#[test]
fn transmute_refuses_a_single_result_identical_to_its_input() {
    let recipe = shulker_transmute();
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(
            2,
            2,
            &[
                VANILLA_ITEM_RED_SHULKER_BOX_ID,
                VANILLA_ITEM_RED_DYE_ID,
                EMPTY,
                EMPTY
            ],
        ),
        &tags(),
        None
    ));
}

/// A wider `material_count` is what makes the rest of `:97-139` observable:
/// inside a 5..8 window a second input, a stranger, or a material count under
/// the minimum each refuse on their own rather than on the entry bound at
/// `:100`.
fn wide_transmute() -> Recipe {
    recipe(json!({
        "type": "minecraft:crafting_transmute",
        "input": "#minecraft:shulker_boxes",
        "material": "minecraft:red_dye",
        "material_count": { "min": 5, "max": 8 },
        "result": { "id": "minecraft:red_shulker_box" }
    }))
}

/// `:100` and `:125`: the material count has to land inside the window, which
/// a 2x2 cannot reach for a five-material recipe.
#[test]
fn transmute_needs_the_material_count_inside_its_window() {
    let tags = tags();
    let wide = wide_transmute();
    let dye = VANILLA_ITEM_RED_DYE_ID;
    let mut eight = [dye; 9];
    eight[4] = VANILLA_ITEM_SHULKER_BOX_ID;
    assert!(wide.matches_crafting(&builtin_lookup(), &cells(3, 3, &eight), &tags, None));
    // Four materials is one short of the minimum.
    let mut four = [EMPTY; 9];
    four[0] = VANILLA_ITEM_SHULKER_BOX_ID;
    four[1..5].fill(dye);
    assert!(!wide.matches_crafting(&builtin_lookup(), &cells(3, 3, &four), &tags, None));
    assert!(!wide.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[VANILLA_ITEM_SHULKER_BOX_ID, dye, dye, dye]),
        &tags,
        None
    ));
    // One past the maximum refuses on the same `:100` bound from the other
    // side; the in-loop `:118` guard can never be the one that fires, because
    // the materials are always one fewer than the ingredient count.
    let narrow = recipe(json!({
        "type": "minecraft:crafting_transmute",
        "input": "#minecraft:shulker_boxes",
        "material": "minecraft:red_dye",
        "material_count": { "min": 1, "max": 3 },
        "result": { "id": "minecraft:red_shulker_box" }
    }));
    let mut five = [EMPTY; 9];
    five[0] = VANILLA_ITEM_SHULKER_BOX_ID;
    five[1..5].fill(dye);
    assert!(!narrow.matches_crafting(&builtin_lookup(), &cells(3, 3, &five), &tags, None));
}

/// `:108-110` allow one input only and `:114-116` refuse a stranger, both
/// tested inside a window wide enough that the count bound is not what
/// refuses.
#[test]
fn transmute_refuses_a_second_input_or_a_stranger_inside_its_window() {
    let tags = tags();
    let wide = wide_transmute();
    let dye = VANILLA_ITEM_RED_DYE_ID;
    // The second input comes last so that dropping the guard would leave a
    // `found_input` the identity refusal at `:126-132` still lets through.
    let mut two_inputs = [dye; 8].to_vec();
    two_inputs[0] = VANILLA_ITEM_RED_SHULKER_BOX_ID;
    two_inputs[1] = VANILLA_ITEM_SHULKER_BOX_ID;
    two_inputs.push(EMPTY);
    assert!(!wide.matches_crafting(&builtin_lookup(), &cells(3, 3, &two_inputs), &tags, None));

    let mut stranger = [dye; 7].to_vec();
    stranger[0] = VANILLA_ITEM_SHULKER_BOX_ID;
    stranger[1] = VANILLA_ITEM_ARROW_ID;
    stranger.push(EMPTY);
    stranger.push(EMPTY);
    assert!(!wide.matches_crafting(&builtin_lookup(), &cells(3, 3, &stranger), &tags, None));
}

// -- minecraft:crafting_dye -----------------------------------------------

fn armor_dye() -> Recipe {
    recipe(json!({
        "type": "minecraft:crafting_dye",
        "dye": "#minecraft:dyes",
        "group": "dyed_armor",
        "result": { "id": "minecraft:leather_horse_armor" },
        "target": "minecraft:leather_horse_armor"
    }))
}

/// `DyeRecipe.matches` (`DyeRecipe.java:62-90`): one target and any number of
/// dyes, on either grid.
#[test]
fn dye_matches_one_target_and_any_number_of_dyes() {
    let recipe = armor_dye();
    let tags = tags();
    let armor = VANILLA_ITEM_LEATHER_HORSE_ARMOR_ID;
    assert!(recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[armor, VANILLA_ITEM_RED_DYE_ID, EMPTY, EMPTY]),
        &tags,
        None
    ));
    let mut wide = [VANILLA_ITEM_BLUE_DYE_ID; 9];
    wide[4] = armor;
    assert!(recipe.matches_crafting(&builtin_lookup(), &cells(3, 3, &wide), &tags, None));
}

/// `:74-76` allow one target only, `:80-82` refuse an item that is not a dye,
/// and `:63-65` need two ingredients.
#[test]
fn dye_refuses_two_targets_a_stranger_or_a_lone_target() {
    let recipe = armor_dye();
    let tags = tags();
    let armor = VANILLA_ITEM_LEATHER_HORSE_ARMOR_ID;
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[armor, armor, VANILLA_ITEM_RED_DYE_ID, EMPTY]),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(
            2,
            2,
            &[armor, VANILLA_ITEM_RED_DYE_ID, VANILLA_ITEM_ARROW_ID, EMPTY]
        ),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[armor, VANILLA_ITEM_ARROW_ID, EMPTY, EMPTY]),
        &tags,
        None
    ));
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(2, 2, &[armor, EMPTY, EMPTY, EMPTY]),
        &tags,
        None
    ));
    // `:80` reads the stack's own `DataComponents.DYE`.
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &stacks(
            2,
            2,
            vec![
                item_stack(armor, 1),
                without_component(
                    item_stack(VANILLA_ITEM_RED_DYE_ID, 1),
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
                ),
                ItemStackSummary::empty(),
                ItemStackSummary::empty(),
            ],
        ),
        &tags,
        None
    ));
}

// -- minecraft:crafting_imbue ---------------------------------------------

fn tipped_arrow() -> Recipe {
    recipe(json!({
        "type": "minecraft:crafting_imbue",
        "material": "minecraft:arrow",
        "result": { "count": 8, "id": "minecraft:tipped_arrow" },
        "source": "minecraft:lingering_potion"
    }))
}

/// `ImbueRecipe.matches` (`ImbueRecipe.java:59-78`) is the one custom matcher
/// with an explicit grid guard: a full 3x3 with the source dead centre.
#[test]
fn imbue_matches_only_a_full_3x3_with_the_source_in_the_middle() {
    let recipe = tipped_arrow();
    let tags = tags();
    let mut ring = [VANILLA_ITEM_ARROW_ID; 9];
    ring[4] = VANILLA_ITEM_LINGERING_POTION_ID;
    assert!(recipe.matches_crafting(&builtin_lookup(), &cells(3, 3, &ring), &tags, None));
    // `:60`: a 2x2 is refused on the dimensions alone.
    assert!(!recipe.matches_crafting(
        &builtin_lookup(),
        &cells(
            2,
            2,
            &[
                VANILLA_ITEM_LINGERING_POTION_ID,
                VANILLA_ITEM_ARROW_ID,
                VANILLA_ITEM_ARROW_ID,
                VANILLA_ITEM_ARROW_ID,
            ],
        ),
        &tags,
        None
    ));
}

/// `:60` also needs all nine cells filled, and `:68-71` put the source in the
/// centre and the material everywhere else.
#[test]
fn imbue_refuses_a_hole_or_a_swapped_centre() {
    let recipe = tipped_arrow();
    let tags = tags();
    let mut holed = [VANILLA_ITEM_ARROW_ID; 9];
    holed[4] = VANILLA_ITEM_LINGERING_POTION_ID;
    holed[0] = EMPTY;
    assert!(!recipe.matches_crafting(&builtin_lookup(), &cells(3, 3, &holed), &tags, None));
    let mut swapped = [VANILLA_ITEM_ARROW_ID; 9];
    swapped[0] = VANILLA_ITEM_LINGERING_POTION_ID;
    assert!(!recipe.matches_crafting(&builtin_lookup(), &cells(3, 3, &swapped), &tags, None));
    let all_potion = [VANILLA_ITEM_LINGERING_POTION_ID; 9];
    assert!(!recipe.matches_crafting(&builtin_lookup(), &cells(3, 3, &all_potion), &tags, None));
}
