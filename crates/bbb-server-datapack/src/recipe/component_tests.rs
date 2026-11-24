//! The result component patch: what `DataComponentPatch.CODEC` accepts on a
//! recipe result, and what `TransmuteRecipe.createWithOriginalComponents` carries
//! out of the input stack.

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
    VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID, VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
    VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
    VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use bbb_protocol::item_component_value_types::{FireworkExplosionShapeSummary, ItemRaritySummary};
use bbb_protocol::item_types::{
    VANILLA_ITEM_BLACK_DYE_ID, VANILLA_ITEM_BLUE_DYE_ID, VANILLA_ITEM_BOWL_ID,
    VANILLA_ITEM_DIAMOND_ID, VANILLA_ITEM_FEATHER_ID, VANILLA_ITEM_FIREWORK_STAR_ID,
    VANILLA_ITEM_GUNPOWDER_ID, VANILLA_ITEM_LEATHER_HELMET_ID, VANILLA_ITEM_PAPER_ID,
    VANILLA_ITEM_RED_BANNER_ID, VANILLA_ITEM_RED_DYE_ID, VANILLA_ITEM_RED_SHULKER_BOX_ID,
    VANILLA_ITEM_SHIELD_ID, VANILLA_ITEM_SHULKER_BOX_ID, VANILLA_ITEM_STICK_ID,
    VANILLA_ITEM_SUSPICIOUS_STEW_ID, VANILLA_ITEM_WHITE_BANNER_ID, VANILLA_ITEM_WHITE_DYE_ID,
    VANILLA_ITEM_WRITTEN_BOOK_ID,
};
use bbb_protocol::mob_effect_types::{
    VANILLA_MOB_EFFECT_BLINDNESS_ID, VANILLA_MOB_EFFECT_NIGHT_VISION_ID,
    VANILLA_MOB_EFFECT_SATURATION_ID,
};
use bbb_protocol::nbt_summary::NbtSummaryValue;
use bbb_protocol::packets::{
    BannerPatternLayerSummary, DataComponentPatchSummary, FireworkExplosionSummary,
    ItemStackSummary, SuspiciousStewEffectSummary, WrittenBookContentSummary,
};
use serde_json::json;

use super::ingredient::{item_stack, stack_is_empty, ItemTagIndex};
use super::input::CraftingInput;
use super::test_support::{builtin_lookup, parse_recipe_with_builtin_lookup as parse_recipe};
use super::*;
use crate::item_stack_template::PrototypeDefaults;

fn tags() -> ItemTagIndex {
    let mut tags = ItemTagIndex::new();
    tags.insert(
        "minecraft:shulker_boxes",
        [VANILLA_ITEM_SHULKER_BOX_ID, VANILLA_ITEM_RED_SHULKER_BOX_ID],
    );
    tags
}

fn effect(effect_id: i32, duration: i32) -> SuspiciousStewEffectSummary {
    SuspiciousStewEffectSummary {
        effect_id,
        duration,
    }
}

fn stew_with(effects: Vec<SuspiciousStewEffectSummary>) -> ItemStackSummary {
    let mut stack = item_stack(VANILLA_ITEM_SUSPICIOUS_STEW_ID, 1);
    stack.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID];
    stack.component_patch.suspicious_stew_effects = effects;
    stack
}

fn patterned_banner(count: i32) -> ItemStackSummary {
    let mut stack = item_stack(VANILLA_ITEM_WHITE_BANNER_ID, count);
    stack.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID];
    stack.component_patch.banner_pattern_layers = vec![BannerPatternLayerSummary {
        registry_id: None,
        asset_id: Some("minecraft:stripe_bottom".to_string()),
        translation_key: Some("test.pattern".into()),
        color_id: 15,
    }];
    stack
}

fn written_book(count: i32) -> ItemStackSummary {
    let mut stack = item_stack(VANILLA_ITEM_WRITTEN_BOOK_ID, count);
    stack.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID];
    stack.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "Field Notes".into(),
        title_filter: None,
        author: "alice".into(),
        generation: 0,
        pages: vec!["page one".to_string()],
        page_filters: vec![None],
        resolved: false,
    });
    stack.component_patch.written_book_pages_nbt =
        vec![Some(NbtSummaryValue::String("page one".into()))];
    stack.component_patch.written_book_page_filters_nbt = vec![None];
    stack
}

/// `SuspiciousStewEffects.CODEC` (`SuspiciousStewEffects.java:27-29`) over
/// `Entry.CODEC` (`:59-65`): `duration` is optional and defaults to
/// `DEFAULT_DURATION` (`:26`).
#[test]
fn result_components_decode_the_declared_suspicious_stew_effects() {
    let recipe = parse_recipe(&json!({
        "type": "minecraft:crafting_shapeless",
        "ingredients": ["minecraft:bowl"],
        "result": {
            "id": "minecraft:suspicious_stew",
            "components": {
                "minecraft:suspicious_stew_effects": [
                    { "id": "minecraft:blindness", "duration": 220 },
                    { "id": "minecraft:night_vision" }
                ]
            }
        }
    }))
    .expect("the declared component kind decodes");
    let result = recipe.assemble_crafting(
        &CraftingInput::of(1, 1, vec![item_stack(VANILLA_ITEM_BOWL_ID, 1)]),
        &ItemTagIndex::new(),
    );
    assert_eq!(result.item_id, Some(VANILLA_ITEM_SUSPICIOUS_STEW_ID));
    assert_eq!(
        result.component_patch.suspicious_stew_effects,
        vec![
            effect(VANILLA_MOB_EFFECT_BLINDNESS_ID, 220),
            effect(VANILLA_MOB_EFFECT_NIGHT_VISION_ID, 160),
        ]
    );
    assert_eq!(result.component_patch.added_type_ids.len(), 1);
    assert_eq!(
        result.component_patch.added_type_ids,
        vec![VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID]
    );
}

#[test]
fn simple_recipe_values_fold_replace_and_remove_against_item_defaults() {
    let template = |components: serde_json::Value| {
        let bytes = serde_json::to_vec(&json!({
            "id": "minecraft:stick", "components": components
        }))
        .unwrap();
        let data = bbb_registry_codec::decode_direct_registry_json(&bytes).unwrap();
        super::source::template(data.root(), &builtin_lookup())
            .expect("persistent simple component decodes")
    };
    let default_value = template(json!({ "minecraft:repair_cost": 0 })).create();
    assert_eq!(default_value.item_id, Some(VANILLA_ITEM_STICK_ID));
    assert!(default_value.component_patch.added_type_ids.is_empty());
    assert_eq!(default_value.component_patch.repair_cost, None);

    let set = template(json!({ "minecraft:repair_cost": 3 }));
    let created = set.create();
    assert_eq!(created.component_patch.repair_cost, Some(3));
    assert_eq!(
        created.component_patch.added_type_ids,
        vec![VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID]
    );
    let carried = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID],
        repair_cost: Some(5),
        ..Default::default()
    };
    let replaced = set.apply(1, &carried).unwrap();
    assert_eq!(replaced.component_patch.repair_cost, Some(3));
    assert_eq!(
        replaced.component_patch.added_type_ids,
        vec![VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID]
    );

    let removed = template(json!({ "!minecraft:repair_cost": {} }))
        .apply(1, &carried)
        .unwrap();
    assert_eq!(removed.component_patch.repair_cost, None);
    assert!(removed.component_patch.added_type_ids.is_empty());
    assert_eq!(
        removed.component_patch.removed_type_ids,
        vec![VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID]
    );

    let stackable_damageable = template(json!({ "minecraft:max_damage": 20 })).create();
    assert!(stack_is_empty(&stackable_damageable));
    let single_damageable = template(json!({
        "minecraft:max_damage": 20,
        "minecraft:max_stack_size": 1
    }))
    .create();
    assert_eq!(single_damageable.component_patch.max_damage, Some(20));
    assert_eq!(single_damageable.component_patch.max_stack_size, Some(1));
}

/// `TransmuteRecipe.assemble` (`TransmuteRecipe.java:141-168`) through
/// `createWithOriginalComponents` (`:85-87`): the dyed box comes out of the
/// input's own patch, so an undyed box keeps whatever it carried.
#[test]
fn transmute_carries_the_input_patch_into_the_result() {
    let recipe = parse_recipe(&json!({
        "type": "minecraft:crafting_transmute",
        "input": "#minecraft:shulker_boxes",
        "material": "minecraft:red_dye",
        "result": { "id": "minecraft:red_shulker_box" }
    }))
    .expect("test recipe parses");
    let mut source = item_stack(VANILLA_ITEM_SHULKER_BOX_ID, 1);
    source.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID];
    source.component_patch.repair_cost = Some(5);
    let input = CraftingInput::of(
        2,
        1,
        vec![source.clone(), item_stack(VANILLA_ITEM_RED_DYE_ID, 1)],
    );
    let result = recipe.assemble_crafting(&input, &tags());
    assert_eq!(result.item_id, Some(VANILLA_ITEM_RED_SHULKER_BOX_ID));
    assert_eq!(result.count, 1);
    assert_eq!(result.component_patch, source.component_patch);
}

/// `ItemStackTemplate.apply` (`ItemStackTemplate.java:92-96`): the additional
/// patch builds the stack and the template's own declared components are laid
/// over it, so the declared value wins on a kind both carry.
#[test]
fn declared_result_components_win_over_the_input_patch() {
    let recipe = parse_recipe(&json!({
        "type": "minecraft:crafting_transmute",
        "input": "minecraft:suspicious_stew",
        "material": "minecraft:red_dye",
        "result": {
            "id": "minecraft:suspicious_stew",
            "components": {
                "minecraft:suspicious_stew_effects": [{ "id": "minecraft:saturation" }]
            }
        }
    }))
    .expect("test recipe parses");
    let source = stew_with(vec![effect(VANILLA_MOB_EFFECT_BLINDNESS_ID, 220)]);
    let input = CraftingInput::of(2, 1, vec![source, item_stack(VANILLA_ITEM_RED_DYE_ID, 1)]);
    let result = recipe.assemble_crafting(&input, &ItemTagIndex::new());
    assert_eq!(
        result.component_patch.suspicious_stew_effects,
        vec![effect(VANILLA_MOB_EFFECT_SATURATION_ID, 160)]
    );
    assert_eq!(result.component_patch.added_type_ids.len(), 1);
}

/// `TransmuteRecipe.matches` (`:126-132`) refuses a single-count result only
/// when `ItemStack.isSameItemSameComponents` holds, including prototype defaults.
#[test]
fn transmute_matches_only_a_change_to_effective_components_for_the_same_item() {
    let recipe = parse_recipe(&json!({
        "type": "minecraft:crafting_transmute",
        "input": "minecraft:suspicious_stew",
        "material": "minecraft:red_dye",
        "result": {
            "id": "minecraft:suspicious_stew",
            "components": {
                "minecraft:suspicious_stew_effects": [{ "id": "minecraft:saturation" }]
            }
        }
    }))
    .expect("test recipe parses");
    let tags = ItemTagIndex::new();
    let dye = item_stack(VANILLA_ITEM_RED_DYE_ID, 1);
    let differing = CraftingInput::of(
        2,
        1,
        vec![
            stew_with(vec![effect(VANILLA_MOB_EFFECT_BLINDNESS_ID, 220)]),
            dye.clone(),
        ],
    );
    let RecipeKind::Transmute(transmute) = &recipe.kind else {
        unreachable!()
    };
    assert!(transmute.matches(&builtin_lookup(), &differing, &tags));
    let identical = CraftingInput::of(
        2,
        1,
        vec![
            stew_with(vec![effect(VANILLA_MOB_EFFECT_SATURATION_ID, 160)]),
            dye,
        ],
    );
    assert!(!transmute.matches(&builtin_lookup(), &identical, &tags));

    let unchanged = parse_recipe(&json!({
        "type": "minecraft:crafting_transmute",
        "input": "minecraft:stick",
        "material": "minecraft:red_dye",
        "result": { "id": "minecraft:stick" }
    }))
    .expect("same-item recipe parses");
    let mut source = item_stack(VANILLA_ITEM_STICK_ID, 1);
    source.component_patch = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_RARITY_TYPE_ID],
        rarity: Some(ItemRaritySummary::Common),
        ..Default::default()
    }
    .into();
    let explicit_default =
        CraftingInput::of(2, 1, vec![source, item_stack(VANILLA_ITEM_RED_DYE_ID, 1)]);
    assert!(!unchanged.matches_crafting(&builtin_lookup(), &explicit_default, &tags, None));

    let removal = parse_recipe(&json!({
        "type": "minecraft:crafting_transmute",
        "input": "minecraft:stick",
        "material": "minecraft:red_dye",
        "result": {
            "id": "minecraft:stick",
            "components": { "!minecraft:rarity": {} }
        }
    }))
    .expect("component removal recipe parses");
    assert!(removal.matches_crafting(&builtin_lookup(), &explicit_default, &tags, None));
}

/// `BannerDuplicateRecipe.assemble` (`BannerDuplicateRecipe.java:79-91`): `:85`
/// rebuilds the copy from the patterned banner's own patch.
#[test]
fn banner_duplicate_copies_the_patterns_onto_the_result() {
    let recipe = parse_recipe(&json!({
        "type": "minecraft:crafting_special_bannerduplicate",
        "banner": "minecraft:white_banner",
        "result": { "id": "minecraft:white_banner" }
    }))
    .expect("test recipe parses");
    let source = patterned_banner(3);
    let input = CraftingInput::of(
        2,
        1,
        vec![source.clone(), item_stack(VANILLA_ITEM_WHITE_BANNER_ID, 1)],
    );
    let result = recipe.assemble_crafting(&input, &ItemTagIndex::new());
    assert_eq!(result.item_id, Some(VANILLA_ITEM_WHITE_BANNER_ID));
    assert_eq!(result.count, 1);
    assert_eq!(
        result.component_patch.banner_pattern_layers,
        source.component_patch.banner_pattern_layers
    );
}

/// `BookCloningRecipe.assemble` (`BookCloningRecipe.java:93-125`): `:122` passes
/// `count - 1` as the extra count and rebuilds from the source book's patch, so
/// two sheets of paper yield two books that both carry the pages, and `:121`
/// hands each one the next generation.
#[test]
fn book_cloning_copies_the_pages_and_counts_the_material() {
    let recipe = parse_recipe(&json!({
        "type": "minecraft:crafting_special_bookcloning",
        "source": "minecraft:written_book",
        "material": "minecraft:paper",
        "result": { "id": "minecraft:written_book" }
    }))
    .expect("test recipe parses");
    let book = written_book(1);
    let input = CraftingInput::of(
        3,
        1,
        vec![
            book.clone(),
            item_stack(VANILLA_ITEM_PAPER_ID, 1),
            item_stack(VANILLA_ITEM_PAPER_ID, 1),
        ],
    );
    let result = recipe.assemble_crafting(&input, &ItemTagIndex::new());
    assert_eq!(result.item_id, Some(VANILLA_ITEM_WRITTEN_BOOK_ID));
    assert_eq!(result.count, 2);
    assert_eq!(
        result.component_patch.written_book,
        Some(WrittenBookContentSummary {
            generation: 1,
            ..book.component_patch.written_book.clone().unwrap()
        })
    );
    assert_eq!(
        result.component_patch.added_type_ids,
        vec![VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID]
    );
    let remainders = recipe.remaining_items(&input);
    assert_eq!(remainders[0], book.clone());
}

/// `DyeRecipe.assemble` (`DyeRecipe.java:117-120`): `DyedItemColor.applyDyes`
/// (`DyedItemColor.java:40-79`) averages the target's own colour with every dye
/// on the grid. The three expected values come from that method run on the
/// Vanilla 26.1 jar.
#[test]
fn dye_recipe_averages_the_target_colour_with_every_grid_dye() {
    let recipe = super::test_support::parse_recipe_with_builtin_lookup(&json!({
        "type": "minecraft:crafting_dye",
        "target": "minecraft:leather_helmet",
        "dye": ["minecraft:red_dye", "minecraft:blue_dye", "minecraft:white_dye"],
        "result": { "id": "minecraft:leather_helmet" }
    }))
    .expect("test recipe parses");
    let tags = ItemTagIndex::new();
    let helmet = item_stack(VANILLA_ITEM_LEATHER_HELMET_ID, 1);
    let dyed_once = recipe.assemble_crafting(
        &CraftingInput::of(
            2,
            1,
            vec![helmet.clone(), item_stack(VANILLA_ITEM_RED_DYE_ID, 1)],
        ),
        &tags,
    );
    assert_eq!(dyed_once.item_id, Some(VANILLA_ITEM_LEATHER_HELMET_ID));
    assert_eq!(dyed_once.component_patch.dyed_color, Some(11_546_150));
    assert_eq!(
        dyed_once.component_patch.added_type_ids,
        vec![VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID]
    );

    let dyed_twice = recipe.assemble_crafting(
        &CraftingInput::of(
            3,
            1,
            vec![
                helmet.clone(),
                item_stack(VANILLA_ITEM_RED_DYE_ID, 1),
                item_stack(VANILLA_ITEM_BLUE_DYE_ID, 1),
            ],
        ),
        &tags,
    );
    assert_eq!(dyed_twice.component_patch.dyed_color, Some(11_359_128));

    // `:117` reads the target's own `DYED_COLOR`, so re-dyeing mixes rather
    // than replaces.
    let mut already_dyed = helmet;
    already_dyed.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID];
    already_dyed.component_patch.dyed_color = Some(0xFF_0000);
    let remixed = recipe.assemble_crafting(
        &CraftingInput::of(
            2,
            1,
            vec![already_dyed, item_stack(VANILLA_ITEM_WHITE_DYE_ID, 1)],
        ),
        &tags,
    );
    assert_eq!(remixed.component_patch.dyed_color, Some(16_744_576));
}

/// `FireworkStarFadeRecipe.assemble` (`FireworkStarFadeRecipe.java:91`):
/// `ItemStack.update` (`ItemStack.java:773-775`) reads the explosion off the
/// rebuilt result and `withFadeColors` (`FireworkExplosion.java:101-103`)
/// replaces only its fade colours.
#[test]
fn firework_star_fade_replaces_only_the_fade_colours() {
    let recipe = parse_recipe(&json!({
        "type": "minecraft:crafting_special_firework_star_fade",
        "target": "minecraft:firework_star",
        "dye": ["minecraft:red_dye", "minecraft:blue_dye"],
        "result": { "id": "minecraft:firework_star" }
    }))
    .expect("test recipe parses");
    let tags = ItemTagIndex::new();
    let mut star = item_stack(VANILLA_ITEM_FIREWORK_STAR_ID, 1);
    star.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID];
    star.component_patch.firework_explosion = Some(FireworkExplosionSummary {
        shape: FireworkExplosionShapeSummary::Star,
        colors: vec![7],
        fade_colors: vec![9],
        has_trail: true,
        has_twinkle: false,
    });
    let faded = recipe.assemble_crafting(
        &CraftingInput::of(
            3,
            1,
            vec![
                star,
                item_stack(VANILLA_ITEM_RED_DYE_ID, 1),
                item_stack(VANILLA_ITEM_BLUE_DYE_ID, 1),
            ],
        ),
        &tags,
    );
    // `DyeColor.getFireworkColor` (`DyeColor.java:88-90`) for RED and BLUE.
    let explosion = faded.component_patch.firework_explosion.as_ref().unwrap();
    assert_eq!(explosion.fade_colors, vec![11_743_532, 2_437_522]);
    assert_eq!(explosion.shape, FireworkExplosionShapeSummary::Star);
    assert_eq!(explosion.colors, vec![7]);
    assert!(explosion.has_trail);

    // A star with no explosion of its own falls back to
    // `FireworkExplosion.DEFAULT` (`FireworkExplosion.java:26`).
    let bare = recipe.assemble_crafting(
        &CraftingInput::of(
            2,
            1,
            vec![
                item_stack(VANILLA_ITEM_FIREWORK_STAR_ID, 1),
                item_stack(VANILLA_ITEM_RED_DYE_ID, 1),
            ],
        ),
        &tags,
    );
    let explosion = bare.component_patch.firework_explosion.as_ref().unwrap();
    assert_eq!(explosion.shape, FireworkExplosionShapeSummary::SmallBall);
    assert!(explosion.colors.is_empty());
    assert_eq!(explosion.fade_colors, vec![11_743_532]);
    assert_eq!(
        bare.component_patch.added_type_ids,
        vec![VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID]
    );
}

#[test]
fn firework_star_writes_input_explosion_over_template_and_keeps_other_components() {
    let recipe = parse_recipe(&json!({
        "type": "minecraft:crafting_special_firework_star",
        "shapes": { "star": "minecraft:stick" },
        "trail": "minecraft:diamond",
        "twinkle": "minecraft:feather",
        "fuel": "minecraft:gunpowder",
        "dye": ["minecraft:red_dye", "minecraft:blue_dye"],
        "result": {
            "id": "minecraft:firework_star",
            "components": {
                "minecraft:dyed_color": 123456,
                "minecraft:firework_explosion": {
                    "shape": "burst", "colors": [3], "fade_colors": [4],
                    "has_trail": false, "has_twinkle": false
                }
            }
        }
    }))
    .unwrap();
    let input = CraftingInput::of(
        3,
        3,
        vec![
            item_stack(VANILLA_ITEM_RED_DYE_ID, 1),
            item_stack(VANILLA_ITEM_STICK_ID, 1),
            item_stack(VANILLA_ITEM_FEATHER_ID, 1),
            item_stack(VANILLA_ITEM_GUNPOWDER_ID, 1),
            item_stack(VANILLA_ITEM_BLUE_DYE_ID, 1),
            item_stack(VANILLA_ITEM_DIAMOND_ID, 1),
            ItemStackSummary::empty(),
            ItemStackSummary::empty(),
            ItemStackSummary::empty(),
        ],
    );
    let tags = ItemTagIndex::new();
    assert!(recipe.matches_crafting(&builtin_lookup(), &input, &tags, None));
    let result = recipe.assemble_crafting(&input, &tags);
    let explosion = result.component_patch.firework_explosion.as_ref().unwrap();
    assert_eq!(explosion.shape, FireworkExplosionShapeSummary::Star);
    assert_eq!(explosion.colors, vec![11_743_532, 2_437_522]);
    assert!(explosion.fade_colors.is_empty());
    assert!(explosion.has_trail);
    assert!(explosion.has_twinkle);
    assert_eq!(result.component_patch.dyed_color, Some(123456));
    assert_eq!(result.component_patch.added_type_ids.len(), 2);

    let default = recipe.assemble_crafting(
        &CraftingInput::of(
            2,
            1,
            vec![
                item_stack(VANILLA_ITEM_GUNPOWDER_ID, 1),
                item_stack(VANILLA_ITEM_RED_DYE_ID, 1),
            ],
        ),
        &tags,
    );
    let explosion = default.component_patch.firework_explosion.as_ref().unwrap();
    assert_eq!(explosion.shape, FireworkExplosionShapeSummary::SmallBall);
    assert!(!explosion.has_trail);
    assert!(!explosion.has_twinkle);
}

/// `ShieldDecorationRecipe.assemble` (`ShieldDecorationRecipe.java:101-102`):
/// the shield takes the banner's layers and the banner item's own registered
/// colour (`BannerItem.java:15-17`).
#[test]
fn shield_decoration_takes_the_banner_patterns_and_base_colour() {
    let recipe = super::test_support::parse_recipe_with_builtin_lookup(&json!({
        "type": "minecraft:crafting_special_shielddecoration",
        "banner": ["minecraft:white_banner", "minecraft:red_banner"],
        "target": "minecraft:shield",
        "result": { "id": "minecraft:shield" }
    }))
    .expect("test recipe parses");
    let tags = ItemTagIndex::new();
    let mut banner = patterned_banner(1);
    banner.item_id = Some(VANILLA_ITEM_RED_BANNER_ID);
    let decorated = recipe.assemble_crafting(
        &CraftingInput::of(
            2,
            1,
            vec![banner.clone(), item_stack(VANILLA_ITEM_SHIELD_ID, 1)],
        ),
        &tags,
    );
    assert_eq!(decorated.item_id, Some(VANILLA_ITEM_SHIELD_ID));
    assert_eq!(
        decorated.component_patch.banner_pattern_layers,
        banner.component_patch.banner_pattern_layers
    );
    assert_eq!(decorated.component_patch.base_color_id, Some(14));
    assert_eq!(
        decorated.component_patch.added_type_ids,
        vec![
            VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
            VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID
        ]
    );

    // `PatchedDataComponentMap.set` (`PatchedDataComponentMap.java:73-74`)
    // drops a value equal to the prototype default, and the shield registers
    // `BannerPatternLayers.EMPTY` (`Items.java:1727-1731`).
    let plain = recipe.assemble_crafting(
        &CraftingInput::of(
            2,
            1,
            vec![
                item_stack(VANILLA_ITEM_WHITE_BANNER_ID, 1),
                item_stack(VANILLA_ITEM_SHIELD_ID, 1),
            ],
        ),
        &tags,
    );
    assert!(plain.component_patch.banner_pattern_layers.is_empty());
    assert_eq!(
        plain.component_patch.added_type_ids,
        vec![VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID]
    );
    assert_eq!(plain.component_patch.base_color_id, Some(0));
}

/// `WrittenBookContent.craftCopy` (`WrittenBookContent.java:88-90`) hands the
/// record constructor (`:68-78`) one generation more, which throws past
/// `MAX_GENERATION` (`:41`). `BookCloningRecipe.assemble` (`:93-125`) reads the
/// content back without the `allowed_generations` gate `matches` (`:70-73`)
/// applied, so a source past the cap reaches it and no book is handed out.
#[test]
fn book_cloning_refuses_a_copy_past_the_max_generation() {
    let recipe = parse_recipe(&json!({
        "type": "minecraft:crafting_special_bookcloning",
        "source": "minecraft:written_book",
        "material": "minecraft:paper",
        "result": { "id": "minecraft:written_book" }
    }))
    .expect("test recipe parses");
    let mut book = written_book(1);
    book.component_patch
        .written_book
        .as_mut()
        .unwrap()
        .generation = 3;
    let input = CraftingInput::of(2, 1, vec![book, item_stack(VANILLA_ITEM_PAPER_ID, 1)]);
    let tags = ItemTagIndex::new();
    assert!(!recipe.matches_crafting(&builtin_lookup(), &input, &tags, None));
    assert_eq!(
        recipe.assemble_crafting(&input, &tags),
        ItemStackSummary::empty()
    );
}

/// `PatchedDataComponentMap.applyPatch` (`PatchedDataComponentMap.java:108`)
/// compares against `prototype.get(type)`, so every default a 26.1 item
/// registers for a kind this decoder models has to be readable here. Three of
/// the seven kinds are registered at all, and each registration decodes.
#[test]
fn every_registered_default_the_result_path_compares_against_decodes() {
    let lookup = super::test_support::builtin_lookup();
    let catalog = bbb_protocol::vanilla_item_prototype_component_catalog()
        .expect("the embedded prototype catalog loads");
    let mut registrations = std::collections::BTreeMap::new();
    for (item_id, components) in catalog
        .component_codec_values_by_item_id()
        .iter()
        .enumerate()
    {
        if item_id != 0 {
            PrototypeDefaults::of_item(item_id as i32, &lookup)
                .unwrap_or_else(|error| panic!("item {item_id} default: {}", error.message));
        }
        for type_id in [
            VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
            VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
            VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
            VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID,
            VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
        ] {
            if components.contains_key(&type_id) {
                *registrations.entry(type_id).or_insert(0usize) += 1;
            }
        }
    }
    assert_eq!(
        registrations,
        std::collections::BTreeMap::from([
            (VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID, 1),
            (VANILLA_DATA_COMPONENT_DYE_TYPE_ID, 16),
            (VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, 17),
        ])
    );
}

/// `applyPatch` (`PatchedDataComponentMap.java:110-111`) drops a declared value
/// equal to the item's prototype default, whatever kind carries it: the dye a
/// dye item registers (`Items.java` `DyeItem`) and the empty effect list
/// `minecraft:suspicious_stew` registers both collapse.
#[test]
fn a_declared_result_value_equal_to_the_prototype_default_collapses() {
    let dyed = |color: &str| {
        parse_recipe(&json!({
            "type": "minecraft:crafting_shapeless",
            "ingredients": ["minecraft:bowl"],
            "result": {
                "id": "minecraft:black_dye",
                "components": { "minecraft:dye": color }
            }
        }))
        .expect("test recipe parses")
        .assemble_crafting(
            &CraftingInput::of(1, 1, vec![item_stack(VANILLA_ITEM_BOWL_ID, 1)]),
            &ItemTagIndex::new(),
        )
    };
    let collapsed = dyed("black");
    assert_eq!(collapsed.item_id, Some(VANILLA_ITEM_BLACK_DYE_ID));
    assert!(collapsed.component_patch.added_type_ids.is_empty());
    assert_eq!(collapsed.component_patch.dye_color_id, None);
    let kept = dyed("red");
    assert_eq!(
        kept.component_patch.added_type_ids,
        vec![VANILLA_DATA_COMPONENT_DYE_TYPE_ID]
    );

    let stew = parse_recipe(&json!({
        "type": "minecraft:crafting_shapeless",
        "ingredients": ["minecraft:bowl"],
        "result": {
            "id": "minecraft:suspicious_stew",
            "components": { "minecraft:suspicious_stew_effects": [] }
        }
    }))
    .expect("test recipe parses")
    .assemble_crafting(
        &CraftingInput::of(1, 1, vec![item_stack(VANILLA_ITEM_BOWL_ID, 1)]),
        &ItemTagIndex::new(),
    );
    assert_eq!(stew.item_id, Some(VANILLA_ITEM_SUSPICIOUS_STEW_ID));
    assert!(stew.component_patch.added_type_ids.is_empty());
}

/// Every `BannerItem` resolves the `DyeColor` its block registered
/// (`Blocks.java:3389-3580`), so the shield overlay never falls back to white
/// for a banner Vanilla would have coloured.
#[test]
fn every_banner_item_resolves_its_registered_dye_colour() {
    let catalog = bbb_protocol::vanilla_item_prototype_component_catalog()
        .expect("the embedded prototype catalog loads");
    let banners: Vec<_> = (0..catalog.item_resource_ids().len() as i32)
        .filter(|item_id| crate::recipe::ingredient::is_banner_item(*item_id))
        .collect();
    assert_eq!(banners.len(), 16);
    for (index, item_id) in banners.into_iter().enumerate() {
        let color = crate::recipe::ingredient::banner_item_color(item_id)
            .unwrap_or_else(|| panic!("banner {item_id} has no colour"));
        // `Items.java:1727-1731` onward registers the banners in `DyeColor`
        // order, so the nth banner item carries the nth colour.
        assert_eq!(color, index as i32);
        assert_eq!(
            catalog.item_resource_ids()[item_id as usize],
            format!(
                "minecraft:{}_banner",
                bbb_protocol::builtin_enum_component_values::vanilla_dye_color_name(color)
            )
        );
    }
}

/// The 26.1 built-in recipe pack declares exactly one component kind, on the
/// seventeen suspicious stew recipes, and every one of them now assembles with
/// its effect rather than a plain bowl of stew.
#[test]
fn the_builtin_pack_assembles_every_suspicious_stew_with_its_effect() {
    use crate::registry_data_loader::DatapackLoadLimits;
    use crate::resource_overlay::{MergedServerDataView, PackOverlayInput};
    use crate::vanilla_snapshot::VanillaDatapackSnapshot;
    use std::sync::Arc;

    let snapshot = Arc::new(VanillaDatapackSnapshot::embedded().unwrap());
    let pack = snapshot.vanilla_pack();
    let inputs = [PackOverlayInput {
        pack_id: "vanilla",
        resources: &pack,
        filter: None,
    }];
    let view = MergedServerDataView::merge(&inputs);
    let recipes = super::test_support::load_recipes_with_builtin_lookup(
        &view,
        &inputs,
        DatapackLoadLimits::default(),
    );
    assert_eq!(recipes.warnings, Vec::<RecipeLoadWarning>::new());
    let stews: Vec<_> = recipes
        .recipes
        .iter()
        .filter(|element| match &element.recipe.kind {
            RecipeKind::Shapeless(shapeless) => !shapeless
                .result
                .components
                .suspicious_stew_effects
                .is_empty(),
            _ => false,
        })
        .collect();
    assert_eq!(stews.len(), 17);
    let blue_orchid = recipes
        .recipe("minecraft:suspicious_stew_from_blue_orchid")
        .expect("the built-in pack carries it");
    assert_eq!(
        blue_orchid
            .recipe
            .assemble_crafting(
                &CraftingInput::of(1, 1, vec![item_stack(VANILLA_ITEM_BOWL_ID, 1)]),
                &ItemTagIndex::new()
            )
            .component_patch
            .suspicious_stew_effects,
        vec![effect(VANILLA_MOB_EFFECT_SATURATION_ID, 7)]
    );
}

/// `WrittenBookContent.CODEC` (`WrittenBookContent.java:44-53`) on a declared
/// result: `Filterable.codec` (`Filterable.java:12-19`) reads the title and each
/// page either bare or as a `{raw, filtered}` record, and a page is a
/// `Component` the plain-string arm of `ComponentSerialization.CODEC`
/// (`ComponentSerialization.java:107-114`) reads as a literal.
#[test]
fn written_book_content_decodes_its_title_and_plain_string_pages() {
    let decoded = parse_recipe(&json!({
        "type": "minecraft:crafting_shapeless",
        "ingredients": ["minecraft:bowl"],
        "result": {
            "id": "minecraft:written_book",
            "components": {
                "minecraft:written_book_content": {
                    "title": { "raw": "Field Notes", "filtered": "Notes" },
                    "author": "alice",
                    "generation": 2,
                    "pages": ["first", { "raw": "second", "filtered": "###### " }],
                    "resolved": true
                }
            }
        }
    }))
    .expect("a declared written book decodes")
    .assemble_crafting(
        &CraftingInput::of(1, 1, vec![item_stack(VANILLA_ITEM_BOWL_ID, 1)]),
        &ItemTagIndex::new(),
    );
    let patch = &decoded.component_patch;
    assert_eq!(
        patch.written_book,
        Some(WrittenBookContentSummary {
            title: "Field Notes".into(),
            title_filter: Some("Notes".into()),
            author: "alice".into(),
            generation: 2,
            pages: vec!["first".to_string(), "second".to_string()],
            page_filters: vec![None, Some("###### ".to_string())],
            resolved: true,
        })
    );
    // The wire encoder writes the page projections beside the summary and
    // refuses a misaligned or noncanonical one, so the decode must fill both.
    assert_eq!(
        patch.written_book_pages_nbt,
        vec![
            Some(NbtSummaryValue::string("first")),
            Some(NbtSummaryValue::string("second")),
        ]
    );
    assert_eq!(
        patch.written_book_page_filters_nbt,
        vec![None, Some(NbtSummaryValue::string("###### "))]
    );
}

/// The optional fields default the way `optionalFieldOf` does at
/// `WrittenBookContent.java:48-50`.
#[test]
fn written_book_content_defaults_generation_pages_and_resolved() {
    let decoded = parse_recipe(&json!({
        "type": "minecraft:crafting_shapeless",
        "ingredients": ["minecraft:bowl"],
        "result": {
            "id": "minecraft:written_book",
            "components": {
                "minecraft:written_book_content": { "title": "Blank", "author": "bob" }
            }
        }
    }))
    .expect("the required pair alone decodes")
    .assemble_crafting(
        &CraftingInput::of(1, 1, vec![item_stack(VANILLA_ITEM_BOWL_ID, 1)]),
        &ItemTagIndex::new(),
    );
    assert_eq!(
        decoded.component_patch.written_book,
        Some(WrittenBookContentSummary {
            title: "Blank".into(),
            title_filter: None,
            author: "bob".into(),
            generation: 0,
            pages: Vec::new(),
            page_filters: Vec::new(),
            resolved: false,
        })
    );
}

/// `BannerPatternLayers.CODEC` (`BannerPatternLayers.java:29`) on a declared
/// result. `Layer.CODEC` (`:81-87`) reads `pattern` through
/// `BannerPattern.CODEC` (`BannerPattern.java:24`), which is a
/// `RegistryFileCodec`: a string is a key the loaded registry answers
/// (`RegistryFileCodec.java:64-70`) and anything else is an inline direct
/// holder (`:59-61`).
#[test]
fn banner_pattern_layers_resolve_a_named_holder_and_keep_an_inline_one() {
    let generation = banner_pattern_generation();
    let mut set = banner_recipe_set(
        json!([
            { "pattern": "minecraft:stripe_bottom", "color": "red" },
            {
                "pattern": {
                    "asset_id": "test:swirl",
                    "translation_key": "block.test.banner.swirl"
                },
                "color": "light_blue"
            },
            { "pattern": "test:extra", "color": "black" }
        ]),
        &generation,
    );
    set.finalize_client_catalog(&ItemTagIndex::new(), &generation);
    assert_eq!(set.warnings, Vec::<RecipeLoadWarning>::new());
    let RecipeKind::Shapeless(shapeless) = &set.recipes[0].recipe.kind else {
        panic!("the fixture is shapeless");
    };
    assert_eq!(
        shapeless.result.components.banner_pattern_layers,
        vec![
            BannerPatternLayerSummary {
                registry_id: Some(1),
                asset_id: None,
                translation_key: None,
                color_id: 14,
            },
            BannerPatternLayerSummary {
                registry_id: None,
                asset_id: Some("test:swirl".to_string()),
                translation_key: Some("block.test.banner.swirl".into()),
                color_id: 3,
            },
            BannerPatternLayerSummary {
                registry_id: Some(2),
                asset_id: None,
                translation_key: None,
                color_id: 15,
            },
        ]
    );
}

/// Every one of the built-in pack's own banner patterns is a key the generation
/// it loaded from answers.
#[test]
fn the_builtin_banner_pattern_registry_answers_each_of_its_own_keys() {
    use crate::registry_data_loader::{load_dynamic_registries, DatapackLoadLimits};
    use crate::resource_overlay::{MergedServerDataView, PackOverlayInput};
    use crate::vanilla_snapshot::VanillaDatapackSnapshot;
    use std::sync::Arc;

    let snapshot = Arc::new(VanillaDatapackSnapshot::embedded().unwrap());
    let pack = snapshot.vanilla_pack();
    let inputs = [PackOverlayInput {
        pack_id: "vanilla",
        resources: &pack,
        filter: None,
    }];
    let view = MergedServerDataView::merge(&inputs);
    let generation =
        load_dynamic_registries(&view, &inputs, DatapackLoadLimits::default()).unwrap();
    let patterns = generation
        .registry("minecraft:banner_pattern")
        .expect("the built-in pack loads the registry");
    assert_eq!(patterns.elements.len(), 43);
    for (index, element) in patterns.elements.iter().enumerate() {
        let mut set = banner_recipe_set(
            json!([{ "pattern": element.id, "color": "white" }]),
            &generation,
        );
        set.finalize_client_catalog(&ItemTagIndex::new(), &generation);
        assert_eq!(set.warnings, Vec::<RecipeLoadWarning>::new());
        let RecipeKind::Shapeless(shapeless) = &set.recipes[0].recipe.kind else {
            panic!("the fixture is shapeless");
        };
        assert_eq!(
            shapeless.result.components.banner_pattern_layers[0].registry_id,
            Some(index as i32)
        );
    }
}

/// A generation whose `banner_pattern` registry answers three keys in load
/// order, which is the order that gives each its network id.
fn banner_pattern_generation() -> crate::dynamic_registry::DatapackGeneration {
    use crate::dynamic_registry::{DatapackGeneration, LoadedDynamicRegistry, RawRegistryElement};
    DatapackGeneration {
        registries: vec![LoadedDynamicRegistry {
            registry_id: "minecraft:banner_pattern".to_string(),
            elements: ["minecraft:base", "minecraft:stripe_bottom", "test:extra"]
                .into_iter()
                .map(|id| RawRegistryElement {
                    id: id.to_string(),
                    source_pack: "test".to_string(),
                    source_json: std::sync::Arc::from(b"{}".as_slice()),
                    source: Some(
                        bbb_registry_codec::decode_direct_registry_json(
                            &serde_json::to_vec(&(json!({}))).unwrap(),
                        )
                        .unwrap(),
                    ),
                    codec_value: None,
                })
                .collect(),
        }],
        worldgen: Default::default(),
    }
}

fn parse_banner_recipe(
    layers: serde_json::Value,
    generation: &crate::dynamic_registry::DatapackGeneration,
) -> Result<Recipe, RecipeDecodeError> {
    let value = json!({
        "type": "minecraft:crafting_shapeless",
        "ingredients": ["minecraft:bowl"],
        "result": {
            "id": "minecraft:white_banner",
            "components": { "minecraft:banner_patterns": layers }
        }
    });
    let bytes = serde_json::to_vec(&value).expect("fixture JSON");
    let decoded = bbb_registry_codec::decode_direct_registry_json(&bytes).expect("fixture source");
    super::test_support::with_generation_lookup(generation, |lookup| {
        parse_recipe_source_with_lookup(decoded.root(), lookup)
    })
}

fn banner_recipe_set(
    layers: serde_json::Value,
    generation: &crate::dynamic_registry::DatapackGeneration,
) -> RecipeSet {
    RecipeSet {
        recipes: vec![RecipeElement {
            id: "test:banner".to_string(),
            source_pack: "test".to_string(),
            recipe: parse_banner_recipe(layers, generation).expect("the declared layers decode"),
        }],
        warnings: Vec::new(),
    }
}
