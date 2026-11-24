use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_BOW_ID, VANILLA_ITEM_BRICK_ID, VANILLA_ITEM_DECORATED_POT_ID,
    VANILLA_ITEM_FIREWORK_ROCKET_ID, VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_FLINT_ID,
    VANILLA_ITEM_GUNPOWDER_ID, VANILLA_ITEM_IRON_BLOCK_ID, VANILLA_ITEM_IRON_INGOT_ID,
    VANILLA_ITEM_OAK_PLANKS_ID, VANILLA_ITEM_PAPER_ID, VANILLA_ITEM_STICK_ID,
    VANILLA_ITEM_STRING_ID, VANILLA_ITEM_WHITE_BANNER_ID, VANILLA_ITEM_WRITTEN_BOOK_ID,
};
use bbb_protocol::packets::ItemStackSummary;
use serde_json::json;

use super::ingredient::{item_stack, stack_is_empty, ItemTagIndex};
use super::input::CraftingInput;
use super::test_support::{builtin_lookup, parse_recipe_with_builtin_lookup as parse_recipe};
use super::*;

fn grid(width: i32, height: i32, cells: &[(i32, i32)]) -> CraftingInput {
    let mut items = vec![ItemStackSummary::empty(); (width * height) as usize];
    for (index, (item_id, count)) in cells.iter().enumerate() {
        if *item_id >= 0 {
            items[index] = item_stack(*item_id, *count);
        }
    }
    CraftingInput::of(width, height, items)
}

fn catalog(id: &str, value: serde_json::Value) -> RecipeSet {
    RecipeSet {
        recipes: vec![RecipeElement {
            id: id.to_string(),
            source_pack: "test".to_string(),
            recipe: parse_recipe(&value).expect("test recipe parses"),
        }],
        warnings: Vec::new(),
    }
}

#[test]
fn pack_overlay_loads_shaped_and_shapeless_component_results() {
    use crate::pack::ServerPackResources;
    use crate::registry_data_loader::DatapackLoadLimits;
    use crate::resource_overlay::{MergedServerDataView, PackOverlayInput};
    use std::collections::BTreeMap;

    struct RawPack(BTreeMap<String, Vec<u8>>);
    impl ServerPackResources for RawPack {
        fn provided_namespaces(&self) -> Vec<String> {
            vec!["test".into()]
        }
        fn list_paths(&self, namespace: &str, dir: &str) -> Vec<String> {
            if namespace != "test" {
                return Vec::new();
            }
            self.0
                .keys()
                .filter(|path| path.starts_with(dir))
                .cloned()
                .collect()
        }
        fn read(&self, namespace: &str, path: &str) -> Option<Vec<u8>> {
            (namespace == "test")
                .then(|| self.0.get(path).cloned())
                .flatten()
        }
        fn read_root(&self, _path: &str) -> Option<Vec<u8>> {
            None
        }
    }

    let low = RawPack(BTreeMap::from([
        ("recipe/a_shaped.json".into(), br#"{"type":"crafting_shaped","pattern":["X"],"key":{"X":"stick"},"result":{"id":"white_banner","components":{"banner_patterns":[{"pattern":{"asset_id":"test:swirl","translation_key":"t"},"color":"white"}]}}}"#.to_vec()),
        ("recipe/b_shapeless.json".into(), br#"{"type":"crafting_shapeless","ingredients":["stick"],"result":"torch"}"#.to_vec()),
        ("recipe/c_missing.json".into(), br#"{"type":"crafting_shapeless","ingredients":["stick"],"result":"torch"}"#.to_vec()),
    ]));
    let high = RawPack(BTreeMap::from([
        ("recipe/b_shapeless.json".into(), br#"{"type":"crafting_shapeless","ingredients":["stick"],"result":{"id":"written_book","components":{"written_book_content":{"title":"Title","author":"Author","pages":[]},"custom_name":"Name","lore":["Lore"],"item_model":"test:book","max_stack_size":8}}}"#.to_vec()),
        ("recipe/c_missing.json".into(), br#"{"type":"crafting_shapeless","ingredients":["stick"],"result":{"id":"white_banner","components":{"banner_patterns":[{"pattern":"test:missing","color":"white"}]}}}"#.to_vec()),
    ]));
    let inputs = [
        PackOverlayInput {
            pack_id: "low",
            resources: &low,
            filter: None,
        },
        PackOverlayInput {
            pack_id: "high",
            resources: &high,
            filter: None,
        },
    ];
    let view = MergedServerDataView::merge(&inputs);
    let set = super::test_support::load_recipes_with_builtin_lookup(
        &view,
        &inputs,
        DatapackLoadLimits::default(),
    );
    assert_eq!(set.warnings.len(), 1);
    assert_eq!(set.warnings[0].element_id, "test:c_missing");
    assert_eq!(set.warnings[0].pack, "high");
    assert!(set.warnings[0].message.contains("test:missing"));
    assert!(set.recipe("test:c_missing").is_none());
    let shaped = set.recipe("test:a_shaped").unwrap();
    assert_eq!(shaped.source_pack, "low");
    let RecipeKind::Shaped(shaped) = &shaped.recipe.kind else {
        panic!("shaped winner");
    };
    assert_eq!(
        shaped.result.components.banner_pattern_layers[0]
            .translation_key
            .as_ref()
            .unwrap(),
        "t"
    );
    let shapeless = set.recipe("test:b_shapeless").unwrap();
    assert_eq!(shapeless.source_pack, "high");
    let RecipeKind::Shapeless(shapeless) = &shapeless.recipe.kind else {
        panic!("shapeless winner");
    };
    let book = shapeless.result.components.written_book.as_ref().unwrap();
    assert_eq!(book.title, "Title");
    assert_eq!(book.author, "Author");
    let result = shapeless.result.create();
    assert_eq!(
        result.component_patch.custom_name_nbt,
        Some(bbb_protocol::NbtSummaryValue::String("Name".into()))
    );
    assert_eq!(
        result.component_patch.lore_nbt,
        [bbb_protocol::NbtSummaryValue::String("Lore".into())]
    );
    assert_eq!(
        result.component_patch.item_model.as_deref(),
        Some("test:book")
    );
    assert_eq!(result.component_patch.max_stack_size, Some(8));
}

#[test]
fn whitelist_matches_vanilla_26_1_count() {
    assert_eq!(RECIPE_SERIALIZER_TYPES_26_1.len(), 21);
}

#[test]
fn parses_typed_recipe_and_defaults_namespace() {
    let recipe = parse_recipe(&json!({
        "type": "crafting_shapeless",
        "ingredients": ["minecraft:flint", "minecraft:iron_ingot"],
        "result": { "id": "minecraft:flint_and_steel" }
    }))
    .unwrap();
    assert_eq!(recipe.type_id, "minecraft:crafting_shapeless");
    let recipe = parse_recipe(&json!({
        "type": "minecraft:stonecutting",
        "ingredient": "minecraft:tuff",
        "result": { "id": "minecraft:chiseled_tuff" }
    }))
    .unwrap();
    assert_eq!(recipe.type_id, "minecraft:stonecutting");
}

#[test]
fn shaped_matches_including_x_flip() {
    let recipes = catalog(
        "test:asymmetric",
        json!({
            "type": "minecraft:crafting_shaped",
            "key": { "#": "minecraft:oak_planks" },
            "pattern": ["# ", "##"],
            "result": { "id": "minecraft:stick", "count": 1 }
        }),
    );
    let tags = ItemTagIndex::new();
    let normal = grid(
        2,
        2,
        &[
            (VANILLA_ITEM_OAK_PLANKS_ID, 1),
            (-1, 0),
            (VANILLA_ITEM_OAK_PLANKS_ID, 1),
            (VANILLA_ITEM_OAK_PLANKS_ID, 1),
        ],
    );
    let flipped = grid(
        2,
        2,
        &[
            (-1, 0),
            (VANILLA_ITEM_OAK_PLANKS_ID, 1),
            (VANILLA_ITEM_OAK_PLANKS_ID, 1),
            (VANILLA_ITEM_OAK_PLANKS_ID, 1),
        ],
    );
    let found = recipes
        .get_recipe_for_crafting(&builtin_lookup(), &normal, None, &tags, None)
        .expect("normal orientation matches");
    assert_eq!(
        found.recipe.assemble_crafting(&normal, &tags).item_id,
        Some(VANILLA_ITEM_STICK_ID)
    );
    assert!(recipes
        .get_recipe_for_crafting(&builtin_lookup(), &flipped, None, &tags, None)
        .is_some());
}

#[test]
fn shaped_3x3_matches_filled_grid_and_refuses_2x2() {
    let recipes = catalog(
        "minecraft:iron_block",
        json!({
            "type": "minecraft:crafting_shaped",
            "key": { "#": "minecraft:iron_ingot" },
            "pattern": ["###", "###", "###"],
            "result": { "id": "minecraft:iron_block" }
        }),
    );
    let tags = ItemTagIndex::new();
    let filled = grid(3, 3, &[(VANILLA_ITEM_IRON_INGOT_ID, 1); 9]);
    let quarter = grid(2, 2, &[(VANILLA_ITEM_IRON_INGOT_ID, 1); 4]);
    let found = recipes
        .get_recipe_for_crafting(&builtin_lookup(), &filled, None, &tags, None)
        .expect("a filled 3x3 grid matches");
    assert_eq!(
        found.recipe.assemble_crafting(&filled, &tags).item_id,
        Some(VANILLA_ITEM_IRON_BLOCK_ID)
    );
    // A pattern only matches an input of its own size (`ShapedRecipePattern.java:155-171`).
    assert!(recipes
        .get_recipe_for_crafting(&builtin_lookup(), &quarter, None, &tags, None)
        .is_none());
}

#[test]
fn shaped_3x3_asymmetric_matches_both_orientations() {
    let recipes = catalog(
        "minecraft:bow",
        json!({
            "type": "minecraft:crafting_shaped",
            "key": { "#": "minecraft:stick", "X": "minecraft:string" },
            "pattern": [" #X", "# X", " #X"],
            "result": { "id": "minecraft:bow" }
        }),
    );
    let tags = ItemTagIndex::new();
    let stick = (VANILLA_ITEM_STICK_ID, 1);
    let string = (VANILLA_ITEM_STRING_ID, 1);
    let empty = (-1, 0);
    let normal = grid(
        3,
        3,
        &[
            empty, stick, string, stick, empty, string, empty, stick, string,
        ],
    );
    // The flipped arm reads each row right to left (`ShapedRecipePattern.java:173`).
    let flipped = grid(
        3,
        3,
        &[
            string, stick, empty, string, empty, stick, string, stick, empty,
        ],
    );
    let swapped = grid(
        3,
        3,
        &[
            empty, string, stick, string, empty, stick, empty, string, stick,
        ],
    );
    let found = recipes
        .get_recipe_for_crafting(&builtin_lookup(), &normal, None, &tags, None)
        .expect("the bow shape matches");
    assert_eq!(
        found.recipe.assemble_crafting(&normal, &tags).item_id,
        Some(VANILLA_ITEM_BOW_ID)
    );
    assert!(recipes
        .get_recipe_for_crafting(&builtin_lookup(), &flipped, None, &tags, None)
        .is_some());
    assert!(recipes
        .get_recipe_for_crafting(&builtin_lookup(), &swapped, None, &tags, None)
        .is_none());
}

#[test]
fn shapeless_matches_and_refuses_wrong_count() {
    let recipes = catalog(
        "minecraft:flint_and_steel",
        json!({
            "type": "minecraft:crafting_shapeless",
            "ingredients": ["minecraft:iron_ingot", "minecraft:flint"],
            "result": { "id": "minecraft:flint_and_steel" }
        }),
    );
    let tags = ItemTagIndex::new();
    let ok = grid(
        2,
        2,
        &[
            (VANILLA_ITEM_IRON_INGOT_ID, 1),
            (VANILLA_ITEM_FLINT_ID, 1),
            (-1, 0),
            (-1, 0),
        ],
    );
    let too_few = grid(
        2,
        2,
        &[(VANILLA_ITEM_IRON_INGOT_ID, 1), (-1, 0), (-1, 0), (-1, 0)],
    );
    let too_many = grid(
        2,
        2,
        &[
            (VANILLA_ITEM_IRON_INGOT_ID, 1),
            (VANILLA_ITEM_FLINT_ID, 1),
            (VANILLA_ITEM_OAK_PLANKS_ID, 1),
            (-1, 0),
        ],
    );
    let found = recipes
        .get_recipe_for_crafting(&builtin_lookup(), &ok, None, &tags, None)
        .expect("flint and steel matches");
    assert_eq!(
        found.recipe.assemble_crafting(&ok, &tags).item_id,
        Some(VANILLA_ITEM_FLINT_AND_STEEL_ID)
    );
    assert!(recipes
        .get_recipe_for_crafting(&builtin_lookup(), &too_few, None, &tags, None)
        .is_none());
    assert!(recipes
        .get_recipe_for_crafting(&builtin_lookup(), &too_many, None, &tags, None)
        .is_none());
}

#[test]
fn firework_rocket_special_matches_3x3_and_caps_fuel() {
    let recipes = catalog(
        "minecraft:firework_rocket",
        json!({
            "type": "minecraft:crafting_special_firework_rocket",
            "fuel": "minecraft:gunpowder",
            "result": { "count": 3, "id": "minecraft:firework_rocket" },
            "shell": "minecraft:paper",
            "star": "minecraft:firework_star"
        }),
    );
    let tags = ItemTagIndex::new();
    let paper = (VANILLA_ITEM_PAPER_ID, 1);
    let fuel = (VANILLA_ITEM_GUNPOWDER_ID, 1);
    let empty = (-1, 0);
    let three_fuel = grid(
        3,
        3,
        &[paper, empty, fuel, empty, empty, empty, fuel, empty, fuel],
    );
    // Three is the fuel ceiling, and a fourth only fits on a 3x3 grid
    // (`FireworkRocketRecipe.java:51-79`).
    let four_fuel = grid(
        3,
        3,
        &[paper, empty, fuel, empty, fuel, empty, fuel, empty, fuel],
    );
    let found = recipes
        .get_recipe_for_crafting(&builtin_lookup(), &three_fuel, None, &tags, None)
        .expect("a scattered rocket matches");
    let result = found.recipe.assemble_crafting(&three_fuel, &tags);
    assert_eq!(result.item_id, Some(VANILLA_ITEM_FIREWORK_ROCKET_ID));
    assert_eq!(result.count, 3);
    assert!(recipes
        .get_recipe_for_crafting(&builtin_lookup(), &four_fuel, None, &tags, None)
        .is_none());
}

#[test]
fn decorated_pot_special_matches_3x3_plus() {
    let recipes = catalog(
        "minecraft:decorated_pot",
        json!({
            "type": "minecraft:crafting_decorated_pot",
            "back": "#minecraft:decorated_pot_ingredients",
            "front": "#minecraft:decorated_pot_ingredients",
            "left": "#minecraft:decorated_pot_ingredients",
            "right": "#minecraft:decorated_pot_ingredients",
            "result": { "id": "minecraft:decorated_pot" }
        }),
    );
    let mut tags = ItemTagIndex::new();
    tags.insert(
        "minecraft:decorated_pot_ingredients",
        [VANILLA_ITEM_BRICK_ID],
    );
    let mut items = vec![ItemStackSummary::empty(); 9];
    items[1] = item_stack(VANILLA_ITEM_BRICK_ID, 1);
    items[3] = item_stack(VANILLA_ITEM_BRICK_ID, 1);
    items[5] = item_stack(VANILLA_ITEM_BRICK_ID, 1);
    items[7] = item_stack(VANILLA_ITEM_BRICK_ID, 1);
    let input = CraftingInput::of(3, 3, items);
    let found = recipes
        .get_recipe_for_crafting(&builtin_lookup(), &input, None, &tags, None)
        .expect("decorated pot matches a 3x3 plus");
    assert_eq!(
        found.recipe.assemble_crafting(&input, &tags).item_id,
        Some(VANILLA_ITEM_DECORATED_POT_ID)
    );
}

#[test]
fn builtin_vanilla_recipes_decode_typed_bodies() {
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
    assert_eq!(recipes.recipes.len(), 1515);
    let shaped = recipes
        .recipes
        .iter()
        .filter(|element| matches!(element.recipe.kind, RecipeKind::Shaped(_)))
        .count();
    let shapeless = recipes
        .recipes
        .iter()
        .filter(|element| matches!(element.recipe.kind, RecipeKind::Shapeless(_)))
        .count();
    let special = recipes
        .recipes
        .iter()
        .filter(|element| element.recipe.is_special())
        .count();
    assert_eq!(shaped, 708);
    assert_eq!(shapeless, 322);
    assert_eq!(special, 24);
}

fn patterned_banner(item_id: i32, count: i32) -> ItemStackSummary {
    let mut stack = item_stack(item_id, count);
    stack.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID];
    stack.component_patch.banner_pattern_layers =
        vec![bbb_protocol::packets::BannerPatternLayerSummary {
            registry_id: None,
            asset_id: Some("minecraft:stripe_bottom".to_string()),
            translation_key: None,
            color_id: 15,
        }];
    stack
}

fn written_book(count: i32) -> ItemStackSummary {
    let mut stack = item_stack(VANILLA_ITEM_WRITTEN_BOOK_ID, count);
    stack.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID];
    stack.component_patch.written_book = Some(bbb_protocol::packets::WrittenBookContentSummary {
        title: "Field Notes".into(),
        title_filter: None,
        author: "alice".into(),
        generation: 0,
        pages: vec!["page one".to_string()],
        page_filters: Vec::new(),
        resolved: false,
    });
    stack
}

/// `BannerDuplicateRecipe.getRemainingItems` (`BannerDuplicateRecipe.java:94-110`):
/// `:104` hands the patterned banner *itself* back with count one
/// (`ItemStack.copyWithCount`, `ItemStack.java:590-598`), so its layers survive
/// the craft. The plain target has no remainder at all (`:103` is the only
/// arm that fires without a crafting remainder).
#[test]
fn banner_duplicate_returns_the_patterned_banner_itself_as_the_remainder() {
    let recipes = catalog(
        "minecraft:white_banner_duplicate",
        json!({
            "type": "minecraft:crafting_special_bannerduplicate",
            "banner": "minecraft:white_banner",
            "result": { "id": "minecraft:white_banner" }
        }),
    );
    let source = patterned_banner(VANILLA_ITEM_WHITE_BANNER_ID, 3);
    let target = item_stack(VANILLA_ITEM_WHITE_BANNER_ID, 1);
    let input = CraftingInput::of(2, 1, vec![source.clone(), target]);
    let remainders = recipes.recipes[0].recipe.remaining_items(&input);
    assert_eq!(remainders.len(), 2);
    assert_eq!(
        remainders[0],
        patterned_banner(VANILLA_ITEM_WHITE_BANNER_ID, 1)
    );
    assert_eq!(
        remainders[0].component_patch.banner_pattern_layers,
        source.component_patch.banner_pattern_layers
    );
    assert!(stack_is_empty(&remainders[1]));
}

/// `BookCloningRecipe.getRemainingItems` (`BookCloningRecipe.java:128-143`):
/// `:137` hands the source book itself back, and `:138` breaks, so a second
/// written book in the grid gets no remainder.
#[test]
fn book_cloning_returns_the_written_book_itself_and_stops_at_the_first() {
    let recipes = catalog(
        "minecraft:book_cloning",
        json!({
            "type": "minecraft:crafting_special_bookcloning",
            "source": "minecraft:written_book",
            "material": "minecraft:paper",
            "result": { "id": "minecraft:written_book" }
        }),
    );
    let book = written_book(2);
    let input = CraftingInput::of(
        2,
        2,
        vec![
            book.clone(),
            item_stack(VANILLA_ITEM_PAPER_ID, 1),
            written_book(1),
            ItemStackSummary::empty(),
        ],
    );
    let remainders = recipes.recipes[0].recipe.remaining_items(&input);
    assert_eq!(remainders.len(), 4);
    assert_eq!(remainders[0], written_book(1));
    assert_eq!(
        remainders[0].component_patch.written_book,
        book.component_patch.written_book
    );
    assert!(stack_is_empty(&remainders[1]));
    assert!(stack_is_empty(&remainders[2]));
    assert!(stack_is_empty(&remainders[3]));
}

mod special_assembly {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    };
    use bbb_protocol::item_component_value_types::FireworkExplosionShapeSummary;
    use bbb_protocol::item_types::{
        VANILLA_ITEM_ANGLER_POTTERY_SHERD_ID, VANILLA_ITEM_ARROW_ID, VANILLA_ITEM_BRICK_ID,
        VANILLA_ITEM_FIREWORK_STAR_ID, VANILLA_ITEM_GUNPOWDER_ID, VANILLA_ITEM_IRON_PICKAXE_ID,
        VANILLA_ITEM_LINGERING_POTION_ID, VANILLA_ITEM_PAPER_ID,
    };
    use bbb_protocol::packets::{
        FireworkExplosionSummary, ItemEnchantmentSummary, ItemStackSummary,
    };
    use bbb_registry_codec::PersistentItemStackHolderResolver;

    use super::super::ingredient::{item_stack, ItemTagIndex};
    use super::super::input::CraftingInput;
    use super::super::test_support::{builtin_lookup, builtin_publication};

    fn tags() -> ItemTagIndex {
        let startup = builtin_publication();
        ItemTagIndex::from_loaded(startup.tags.registry("minecraft:item").unwrap())
            .with_enchantment_curses(startup)
    }

    fn craft(items: Vec<ItemStackSummary>) -> ItemStackSummary {
        let input = CraftingInput::of(3, 3, items);
        let tags = tags();
        builtin_publication()
            .recipes
            .get_recipe_for_crafting(&builtin_lookup(), &input, None, &tags, None)
            .expect("a vanilla recipe matches")
            .recipe
            .assemble_crafting(&input, &tags)
    }

    fn with_component(mut stack: ItemStackSummary, type_id: i32) -> ItemStackSummary {
        stack.component_patch.added_type_ids.push(type_id);
        stack
    }

    fn empty() -> ItemStackSummary {
        ItemStackSummary::empty()
    }

    #[test]
    fn firework_rocket_flies_per_gunpowder_and_carries_star_explosions() {
        let explosion = FireworkExplosionSummary {
            shape: FireworkExplosionShapeSummary::Creeper,
            colors: vec![0xFF0000],
            fade_colors: Vec::new(),
            has_trail: true,
            has_twinkle: false,
        };
        let mut star = with_component(
            item_stack(VANILLA_ITEM_FIREWORK_STAR_ID, 1),
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
        );
        star.component_patch.firework_explosion = Some(explosion.clone());
        let rocket = craft(vec![
            item_stack(VANILLA_ITEM_PAPER_ID, 1),
            item_stack(VANILLA_ITEM_GUNPOWDER_ID, 1),
            item_stack(VANILLA_ITEM_GUNPOWDER_ID, 1),
            star,
            empty(),
            empty(),
            empty(),
            empty(),
            empty(),
        ]);
        assert_eq!(rocket.count, 3);
        assert_eq!(rocket.component_patch.fireworks_flight_duration, Some(2));
        assert_eq!(rocket.component_patch.fireworks_explosions, vec![explosion]);
    }

    #[test]
    fn tipped_arrow_takes_the_lingering_potion_contents() {
        let mut potion = with_component(
            item_stack(VANILLA_ITEM_LINGERING_POTION_ID, 1),
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        );
        potion.component_patch.potion_contents.potion_id = Some(7);
        let mut items = vec![item_stack(VANILLA_ITEM_ARROW_ID, 1); 9];
        items[4] = potion;
        let arrows = craft(items);
        assert_eq!(arrows.count, 8);
        assert_eq!(arrows.component_patch.potion_contents.potion_id, Some(7));
    }

    #[test]
    fn decorated_pot_keeps_its_sherds() {
        let sherd = item_stack(VANILLA_ITEM_ANGLER_POTTERY_SHERD_ID, 1);
        let brick = item_stack(VANILLA_ITEM_BRICK_ID, 1);
        let pot = craft(vec![
            empty(),
            sherd.clone(),
            empty(),
            brick.clone(),
            empty(),
            brick,
            empty(),
            sherd,
            empty(),
        ]);
        assert_eq!(
            pot.component_patch.pot_decorations_item_ids,
            vec![
                VANILLA_ITEM_ANGLER_POTTERY_SHERD_ID,
                VANILLA_ITEM_BRICK_ID,
                VANILLA_ITEM_BRICK_ID,
                VANILLA_ITEM_ANGLER_POTTERY_SHERD_ID,
            ]
        );
    }

    #[test]
    fn repair_combines_durability_and_keeps_only_curses() {
        let lookup = builtin_lookup();
        let enchantment = |id: &str, level| ItemEnchantmentSummary {
            holder_id: lookup.holder_id("minecraft:enchantment", id).unwrap(),
            level,
        };
        let pickaxe = |damage, enchantments: Vec<ItemEnchantmentSummary>| {
            let mut stack = with_component(
                item_stack(VANILLA_ITEM_IRON_PICKAXE_ID, 1),
                VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            );
            stack.component_patch.damage = Some(damage);
            stack = with_component(stack, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
            stack.component_patch.enchantments = enchantments;
            stack
        };
        let mut items = vec![empty(); 9];
        items[0] = pickaxe(200, vec![enchantment("minecraft:efficiency", 5)]);
        items[1] = pickaxe(
            150,
            vec![
                enchantment("minecraft:vanishing_curse", 1),
                enchantment("minecraft:unbreaking", 3),
            ],
        );
        let repaired = craft(items);
        // 250 max: (50 + 100 + 250 * 5 / 100) remaining leaves 88 damage.
        assert_eq!(repaired.component_patch.damage, Some(88));
        assert_eq!(
            repaired.component_patch.enchantments,
            vec![enchantment("minecraft:vanishing_curse", 1)]
        );
    }
}
