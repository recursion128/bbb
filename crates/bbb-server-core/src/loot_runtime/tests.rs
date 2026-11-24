//! Every assertion here describes the official server's answer for the exact
//! parameters `Block.dropResources(state, level, pos)` supplies — an empty
//! tool, no breaking entity, no block entity and no explosion — and runs
//! against the packaged vanilla datapack rather than a hand-written fixture.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde_json::Value;

use crate::random::GameRandom;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
    VANILLA_BLOCK_STATE_COUNT,
};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::item_types::vanilla_item_type;
use bbb_protocol::packets::BlockPos;

use super::*;
use bbb_server_datapack::loot::{
    parse_loot_condition, parse_loot_function, parse_loot_table, LoadedLootRegistry, LootElement,
    LootElementValue,
};
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

pub(super) fn inspect_test_loot(stack: &LootStack) -> ItemStackSummary {
    stack.clone()
}

fn block_drops_summary(
    tables: &LootRegistries,
    params: &BlockDropParams<'_>,
    random: &GameRandom,
) -> Result<Vec<ItemStackSummary>> {
    super::block_drops(tables, params, random)
        .map(|stacks| stacks.iter().map(inspect_test_loot).collect())
}

fn named_table_drops_summary(
    tables: &LootRegistries,
    table_id: &str,
    params: &BlockDropParams<'_>,
    random: &GameRandom,
) -> Result<Vec<ItemStackSummary>> {
    super::named_table_drops(tables, table_id, params, random)
        .map(|stacks| stacks.iter().map(inspect_test_loot).collect())
}

fn loot_table_drops_summary(
    tables: &LootRegistries,
    table_id: &str,
    params: &LootParams<'_>,
    random: &GameRandom,
) -> Result<Vec<ItemStackSummary>> {
    super::loot_table_drops(tables, table_id, params, random)
        .map(|stacks| stacks.iter().map(inspect_test_loot).collect())
}

/// Built-in `blocks/*` tables this evaluator cannot answer, each with the
/// mechanism it would need.
///
/// Empty for 26.1: the whole built-in block surface fits the declared node
/// types. The list stays so a table that stops fitting has to be admitted here
/// deliberately instead of failing silently.
const UNSUPPORTED_BUILTIN_BLOCK_TABLES: &[(&str, &str)] = &[];

fn datapacks() -> &'static DatapackStartup {
    static STARTUP: OnceLock<DatapackStartup> = OnceLock::new();
    STARTUP.get_or_init(|| {
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .expect("the embedded vanilla datapack loads")
    })
}

const ORIGIN: BlockPos = BlockPos { x: 4, y: 70, z: -9 };

/// `ItemStack.EMPTY`, the `LootContextParams.TOOL` the three-argument
/// `Block.getDrops` supplies (`Block.java:365-371`).
static EMPTY_TOOL: std::sync::LazyLock<LootTool> = std::sync::LazyLock::new(LootTool::empty);

static EMPTY_LEVEL: fn(BlockPos) -> Option<i32> = |_| None;

fn state_of(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties: BTreeMap<String, String> = properties
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    vanilla_block_state_id_by_name_and_properties(name, &properties)
        .unwrap_or_else(|| panic!("the generated registry carries {name} {properties:?}"))
}

/// `Block.defaultBlockState()` for a block named by its registry id.
fn default_state_of(name: &str) -> i32 {
    let block = vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("the generated registry carries {name}"));
    vanilla_block_default_state_id(block.id).unwrap_or_else(|| panic!("{name} has a default state"))
}

fn drops_with(
    block_state_id: i32,
    level: Option<&LootBlockStateAt<'_>>,
    random: &GameRandom,
) -> Vec<(String, i32)> {
    drops_with_tool(
        block_state_id,
        level,
        &EMPTY_TOOL,
        LootEntity::Absent,
        random,
    )
}

/// The same walk with the two parameters the six-argument `Block.getDrops`
/// (`Block.java:373-386`) fills differently.
fn drops_with_tool(
    block_state_id: i32,
    level: Option<&LootBlockStateAt<'_>>,
    tool: &LootTool,
    breaker: LootEntity<'_>,
    random: &GameRandom,
) -> Vec<(String, i32)> {
    let item_environment = RegistryItemEnvironment::new(crate::inventory::test_item_resolver());
    let drops = block_drops_summary(
        &datapacks().loot,
        &BlockDropParams {
            dimension: None,
            block_state_id,
            pos: ORIGIN,
            level,
            block_entity_data: None,
            tool,
            breaker,
            explosion_radius: None,
            block_entity: None,
            item_environment: Some(&item_environment),
        },
        random,
    )
    .expect("the built-in block tables are evaluable");
    drops
        .into_iter()
        .map(|stack| {
            let item = stack
                .item_id
                .and_then(vanilla_item_type)
                .map(|entry| entry.resource_id.to_string())
                .unwrap_or_else(|| "<empty>".to_string());
            (item, stack.count)
        })
        .collect()
}

fn drops(block_state_id: i32, seed: i64) -> Vec<(String, i32)> {
    let random = GameRandom::with_seed(seed);
    drops_with(block_state_id, Some(&EMPTY_LEVEL), &random)
}

fn direct_eval<'a>(random: &'a GameRandom, explosion_radius: Option<f32>) -> Eval<'a> {
    Eval {
        table_id: "minecraft:test",
        tables: &datapacks().loot,
        visited: Vec::new(),
        visited_functions: Vec::new(),
        visited_conditions: Vec::new(),
        context: LootParams {
            origin: Some(LootOrigin::at_center_of(ORIGIN)),
            block_state_id: Some(default_state_of("minecraft:stone")),
            tool: Some(&EMPTY_TOOL),
            explosion_radius,
            entities: LootEntities {
                this_entity: LootEntity::Opaque,
                ..LootEntities::default()
            },
            level: Some(&EMPTY_LEVEL),
            ..LootParams::new(param_sets::BLOCK)
        }
        .create()
        .expect("the block parameter set carries exactly these"),
        random,
    }
}

#[test]
fn survives_explosion_rolls_only_when_the_radius_is_present() {
    let condition = LootCondition::SurvivesExplosion;

    let random = GameRandom::with_seed(3);
    let survivors = |radius| {
        (0..4_000)
            .filter(|_| {
                direct_eval(&random, radius)
                    .condition(&condition, "condition")
                    .unwrap()
            })
            .count()
    };
    assert_eq!(survivors(None), 4_000);
    // `1 / radius`: 1 000 of 4 000 on average (sd ~27).
    assert!((880..=1_120).contains(&survivors(Some(4.0))));
}

#[test]
fn explosion_decay_rolls_survival_per_original_item() {
    let function = bbb_server_datapack::loot::parse_loot_function(
        &serde_json::json!({"function": "minecraft:explosion_decay"}),
        None,
    )
    .unwrap();
    let stack = ItemStackSummary {
        item_id: Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:stone")
                .unwrap()
                .id,
        ),
        count: 4,
        component_patch: Default::default(),
    };

    let random = GameRandom::with_seed(5);
    let unchanged = direct_eval(&random, None)
        .apply_loot_function(&function, stack.clone(), "function")
        .unwrap();
    assert_eq!(unchanged.count, 4);

    // Each of the four items survives at `1 / radius`: 1 000 of 4 000 over a
    // thousand stacks on average (sd ~27).
    let survivors: i32 = (0..1_000)
        .map(|_| {
            direct_eval(&random, Some(4.0))
                .apply_loot_function(&function, stack.clone(), "function")
                .unwrap()
                .count
        })
        .sum();
    assert!((880..=1_120).contains(&survivors), "{survivors}");
}

#[test]
fn stone_drops_cobblestone_because_the_silk_touch_alternative_needs_an_enchanted_tool() {
    // `blocks/stone` is one `alternatives` entry: silk-touched stone first,
    // cobblestone otherwise. `MatchTool.test` (`MatchTool.java:28-31`) sees
    // `ItemStack.EMPTY`, whose component map is empty
    // (`ItemStack.java:220-222`), so `silk_touch` reads level 0 and
    // `EnchantmentPredicate.matchesEnchantment`
    // (`EnchantmentPredicate.java:53-60`) turns the first child down.
    let stone = default_state_of("minecraft:stone");
    for seed in [0, 1, 12345] {
        assert_eq!(
            drops(stone, seed),
            vec![("minecraft:cobblestone".to_string(), 1)]
        );
    }
}

#[test]
fn an_ordinary_block_drops_itself() {
    // `blocks/oak_log` is a single pool guarded by `survives_explosion` around
    // one `minecraft:item` entry.
    let log = default_state_of("minecraft:oak_log");
    assert_eq!(drops(log, 0), vec![("minecraft:oak_log".to_string(), 1)]);
}

#[test]
fn gravel_drops_flint_at_its_table_bonus_chance() {
    // `blocks/gravel` puts flint first behind `table_bonus`; the empty tool is
    // fortune level 0, so the chance is `chances[0]` = 0.1
    // (`BonusLevelTableCondition.java:37-42`).
    let gravel = default_state_of("minecraft:gravel");
    let random = GameRandom::with_seed(7);
    let mut flint = 0;
    for _ in 0..2000 {
        let drop = drops_with(gravel, Some(&EMPTY_LEVEL), &random);
        assert_eq!(drop.len(), 1);
        match drop[0].0.as_str() {
            "minecraft:flint" => flint += 1,
            "minecraft:gravel" => {}
            other => panic!("unexpected gravel drop {other}"),
        }
    }
    assert!((140..=260).contains(&flint), "{flint} flint in 2000 breaks");
}

#[test]
fn redstone_ore_counts_stay_within_the_uniform_range() {
    // `blocks/redstone_ore`: `set_count` `uniform{4,5}`, then `apply_bonus`
    // `uniform_bonus_count` that adds nothing at fortune level 0
    // (`ApplyBonusCount.java:163-165`).
    let ore = default_state_of("minecraft:redstone_ore");
    let random = GameRandom::with_seed(0);
    let counts: BTreeSet<i32> = (0..64)
        .map(|_| {
            let drop = drops_with(ore, Some(&EMPTY_LEVEL), &random);
            assert_eq!(drop.len(), 1);
            assert_eq!(drop[0].0, "minecraft:redstone");
            drop[0].1
        })
        .collect();
    assert_eq!(counts, BTreeSet::from([4, 5]));
}

#[test]
fn a_ripe_wheat_crop_rolls_a_seed_bonus_that_fortune_zero_does_not_disable() {
    // The second pool of `blocks/wheat` only runs at `age=7` and puts
    // `apply_bonus{binomial_with_bonus_count, extra: 3}` on its seeds
    // (`ApplyBonusCount.java:105-113`), so even a fortune-0 break can yield
    // one to four seeds.
    let ripe = state_of("minecraft:wheat", &[("age", "7")]);
    let random = GameRandom::with_seed(0);
    let mut seed_counts = BTreeSet::new();
    for _ in 0..256 {
        let drop = drops_with(ripe, Some(&EMPTY_LEVEL), &random);
        assert_eq!(drop.len(), 2);
        assert_eq!(drop[0], ("minecraft:wheat".to_string(), 1));
        assert_eq!(drop[1].0, "minecraft:wheat_seeds");
        assert!((1..=4).contains(&drop[1].1), "{drop:?}");
        seed_counts.insert(drop[1].1);
    }
    assert!(seed_counts.len() > 1, "{seed_counts:?}");

    // Unripe wheat takes the other alternative: the second pool's
    // `block_state_property` guard stops it.
    let young = state_of("minecraft:wheat", &[("age", "3")]);
    let random = GameRandom::with_seed(0);
    assert_eq!(
        drops_with(young, Some(&EMPTY_LEVEL), &random),
        vec![("minecraft:wheat_seeds".to_string(), 1)]
    );
}

#[test]
fn glowstone_clamps_its_uniform_count_through_limit_count() {
    // `blocks/glowstone` runs `set_count{uniform 2..4}`, then
    // `apply_bonus{uniform_bonus_count}`, then `limit_count{min: 1, max: 4}`
    // (`LimitCount.java:36-40` over `IntRange.clamp`, `IntRange.java:86-88`).
    let glowstone = default_state_of("minecraft:glowstone");
    let random = GameRandom::with_seed(0);
    let counts: BTreeSet<i32> = (0..64)
        .map(|_| {
            let drop = drops_with(glowstone, Some(&EMPTY_LEVEL), &random);
            assert_eq!(drop[0].0, "minecraft:glowstone_dust");
            drop[0].1
        })
        .collect();
    assert_eq!(counts, BTreeSet::from([2, 3, 4]));
}

#[test]
fn a_block_whose_key_has_no_table_drops_nothing() {
    // `noLootTable()` blocks are generated with no `blocks/<id>.json`, and
    // `ReloadableServerRegistries.Holder.getLootTable`
    // (`ReloadableServerRegistries.java:119-121`) answers `LootTable.EMPTY` for
    // a key the registry does not hold.
    for name in ["minecraft:air", "minecraft:bedrock", "minecraft:barrier"] {
        let state = default_state_of(name);
        assert_eq!(drops(state, 0), Vec::new());
    }
}

#[test]
fn a_wall_variant_drops_the_standing_blocks_item() {
    // `Blocks.wallVariant` (`Blocks.java:7022-7029`) gives the wall block the
    // standing block's loot table key, so `oak_wall_sign` runs `blocks/oak_sign`
    // even though no `blocks/oak_wall_sign.json` exists.
    let wall = default_state_of("minecraft:oak_wall_sign");
    assert_eq!(
        block_loot_table_id(wall).as_deref(),
        Some("minecraft:blocks/oak_sign")
    );
    assert_eq!(drops(wall, 0), vec![("minecraft:oak_sign".to_string(), 1)]);

    // The same for the other three shapes the override covers.
    for (wall_block, expected) in [
        ("minecraft:wall_torch", "minecraft:blocks/torch"),
        ("minecraft:red_wall_banner", "minecraft:blocks/red_banner"),
        (
            "minecraft:skeleton_wall_skull",
            "minecraft:blocks/skeleton_skull",
        ),
    ] {
        assert_eq!(
            block_loot_table_id(default_state_of(wall_block)).as_deref(),
            Some(expected)
        );
    }
}

#[test]
fn snow_drops_nothing_when_no_entity_broke_it() {
    // `blocks/snow`'s only pool is guarded by
    // `entity_properties {entity: this, predicate: {}}`. Under
    // `Block.dropResources(state, level, pos)` nothing sets `THIS_ENTITY`, and
    // `EntityPredicate.matches` returns false for a null entity before looking
    // at anything else (`EntityPredicate.java:92-95`), so the snow layer yields
    // no snowballs.
    for layers in ["1", "4", "8"] {
        let snow = state_of("minecraft:snow", &[("layers", layers)]);
        assert_eq!(drops(snow, 0), Vec::new());
    }

    // The six-argument overload does supply the parameter, and then the same
    // pool answers one snowball per layer.
    for layers in ["1", "4", "8"] {
        let snow = state_of("minecraft:snow", &[("layers", layers)]);
        let random = GameRandom::with_seed(0);
        assert_eq!(
            drops_with_tool(
                snow,
                Some(&EMPTY_LEVEL),
                &EMPTY_TOOL,
                LootEntity::Opaque,
                &random,
            ),
            vec![(
                "minecraft:snowball".to_string(),
                layers.parse::<i32>().unwrap()
            )]
        );
    }
}

#[test]
fn tall_grass_reads_its_other_half_through_the_level() {
    // `blocks/tall_grass` pairs a `block_state_property` on `half` with a
    // `location_check` one block away, so only the half whose partner is really
    // there can drop anything (`LocationCheck.java:35-42` over
    // `BlockPredicate.matches`). The reachable entry is wheat seeds behind a
    // 0.125 `random_chance`.
    let lower = state_of("minecraft:tall_grass", &[("half", "lower")]);
    let upper = state_of("minecraft:tall_grass", &[("half", "upper")]);
    let blocks = BTreeMap::from([((ORIGIN.x, ORIGIN.y + 1, ORIGIN.z), upper)]);
    let level = |pos: BlockPos| blocks.get(&(pos.x, pos.y, pos.z)).copied();

    let random = GameRandom::with_seed(7);
    let dropped = (0..400)
        .filter(|_| !drops_with(lower, Some(&level), &random).is_empty())
        .count();
    assert!(
        (20..=80).contains(&dropped),
        "{dropped} drops in 400 breaks"
    );

    // With nothing above, the `location_check` fails and nothing drops.
    let random = GameRandom::with_seed(7);
    for _ in 0..64 {
        assert_eq!(drops_with(lower, Some(&EMPTY_LEVEL), &random), Vec::new());
    }
}

#[test]
fn a_cracked_decorated_pot_without_a_block_entity_emits_no_sherds() {
    // `blocks/decorated_pot` picks the `minecraft:dynamic` child when the pot is
    // cracked. `DynamicLoot.createItemStack` (`DynamicLoot.java:32-34`) asks
    // `LootParams.addDynamicDrops` (`LootParams.java:35-40`), and
    // `DecoratedPotBlock.getDrops` registers the `sherds` drop only for a pot
    // block entity (`DecoratedPotBlock.java:181-192`), so the entry emits
    // nothing — and because it expanded, the intact-pot child below it is not
    // reached.
    let cracked = state_of(
        "minecraft:decorated_pot",
        &[
            ("cracked", "true"),
            ("facing", "north"),
            ("waterlogged", "false"),
        ],
    );
    assert_eq!(drops(cracked, 0), Vec::new());

    let intact = state_of(
        "minecraft:decorated_pot",
        &[
            ("cracked", "false"),
            ("facing", "north"),
            ("waterlogged", "false"),
        ],
    );
    assert_eq!(
        drops(intact, 0),
        vec![("minecraft:decorated_pot".to_string(), 1)]
    );
}

#[test]
fn copy_state_only_patches_the_stack_when_the_pose_differs_from_the_items_prototype() {
    // `blocks/copper_golem_statue` copies `copper_golem_pose` onto the dropped
    // stack (`CopyBlockState.java:59-74`). The statue item's own prototype
    // already carries `copper_golem_pose=standing`, and
    // `PatchedDataComponentMap.set` (`PatchedDataComponentMap.java:69-80`) drops
    // a patch entry that equals the prototype, so a standing statue drops a
    // plain item and only the other poses carry a `block_state` patch.
    let standing = state_of(
        "minecraft:copper_golem_statue",
        &[
            ("copper_golem_pose", "standing"),
            ("facing", "north"),
            ("waterlogged", "false"),
        ],
    );
    let sitting = state_of(
        "minecraft:copper_golem_statue",
        &[
            ("copper_golem_pose", "sitting"),
            ("facing", "north"),
            ("waterlogged", "false"),
        ],
    );
    let random = GameRandom::with_seed(0);
    let standing_drop = block_drops_summary(
        &datapacks().loot,
        &BlockDropParams {
            dimension: None,
            block_state_id: standing,
            pos: ORIGIN,
            level: Some(&EMPTY_LEVEL),
            block_entity_data: None,
            tool: &EMPTY_TOOL,
            breaker: LootEntity::Absent,
            explosion_radius: None,
            block_entity: None,
            item_environment: None,
        },
        &random,
    )
    .unwrap();
    assert_eq!(standing_drop.len(), 1);
    assert!(standing_drop[0]
        .component_patch
        .block_state_properties
        .is_empty());
    assert!(standing_drop[0].component_patch.added_type_ids.is_empty());

    let sitting_drop = block_drops_summary(
        &datapacks().loot,
        &BlockDropParams {
            dimension: None,
            block_state_id: sitting,
            pos: ORIGIN,
            level: Some(&EMPTY_LEVEL),
            block_entity_data: None,
            tool: &EMPTY_TOOL,
            breaker: LootEntity::Absent,
            explosion_radius: None,
            block_entity: None,
            item_environment: None,
        },
        &random,
    )
    .unwrap();
    assert_eq!(sitting_drop.len(), 1);
    assert_eq!(
        sitting_drop[0].component_patch.block_state_properties,
        [("copper_golem_pose".into(), "sitting".into())].into()
    );
    assert_eq!(
        sitting_drop[0].component_patch.added_type_ids,
        vec![bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID]
    );
    // `facing` and `waterlogged` are on the state but not in the function's
    // property list, so they are not copied.
    assert_eq!(
        sitting_drop[0].component_patch.block_state_properties.len(),
        1
    );
}

#[test]
fn copy_state_overwrites_in_place_and_appends_new_properties() {
    let state = state_of(
        "minecraft:copper_golem_statue",
        &[
            ("copper_golem_pose", "sitting"),
            ("facing", "north"),
            ("waterlogged", "false"),
        ],
    );
    let function = parse_loot_function(
        &serde_json::json!({
            "function": "minecraft:copy_state",
            "block": "minecraft:copper_golem_statue",
            "properties": ["facing", "waterlogged"],
        }),
        None,
    )
    .unwrap();
    let stack = ItemStackSummary {
        item_id: None,
        count: 1,
        component_patch: bbb_protocol::packets::data_components::DataComponentPatchSummary {
            block_state_properties: [("facing".into(), "south".into())].into(),
            ..Default::default()
        }
        .into(),
    };
    let random = GameRandom::with_seed(0);
    let mut eval = Eval {
        table_id: "minecraft:test",
        tables: &datapacks().loot,
        visited: Vec::new(),
        visited_functions: Vec::new(),
        visited_conditions: Vec::new(),
        context: LootParams {
            origin: Some(LootOrigin::at_center_of(ORIGIN)),
            block_state_id: Some(state),
            tool: Some(&EMPTY_TOOL),
            level: Some(&EMPTY_LEVEL),
            ..LootParams::new(param_sets::BLOCK)
        }
        .create()
        .unwrap(),
        random: &random,
    };
    let copied = eval
        .apply_loot_function(&function, stack, "function")
        .unwrap();
    let entries: Vec<_> = copied
        .component_patch
        .block_state_properties
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    assert_eq!(
        entries,
        vec![
            ("facing".into(), "north".into()),
            ("waterlogged".into(), "false".into()),
        ]
    );
}

#[test]
fn every_builtin_block_table_is_evaluable() {
    let (table_ids, by_table) = builtin_block_tables();
    let registry = datapacks()
        .loot
        .registry("minecraft:loot_table")
        .expect("the loot table registry is loaded");
    let level = EMPTY_LEVEL;
    for table_id in table_ids {
        if UNSUPPORTED_BUILTIN_BLOCK_TABLES
            .iter()
            .any(|(id, _)| id == table_id)
        {
            // Declared out of scope with a reason; the walk below would fail.
            continue;
        }
        let element = registry
            .elements
            .iter()
            .find(|element| &element.id == table_id)
            .expect("the table id came from this registry");
        let LootElementValue::Table(table) = &element.value else {
            panic!("{table_id} decoded as something other than a table");
        };
        assert_eq!(
            table.param_set, "minecraft:block",
            "{table_id} declares a param set this evaluator was not written for"
        );
        // Every node, including the branches no seed happens to take.
        check_block_table(table_id, table)
            .unwrap_or_else(|error| panic!("{table_id} is not evaluable: {error}"));
        // And the branches a handful of seeds do take, end to end -- through
        // both parameter sets, because the six-argument one
        // (`Block.java:373-386`) reaches conditions the empty tool and the
        // absent breaker turn away before they are evaluated.
        let state = by_table[table_id];
        for (tool, breaker) in [
            (EMPTY_TOOL.clone(), LootEntity::Absent),
            (enchanted_pickaxe(), LootEntity::Opaque),
        ] {
            let item_environment =
                RegistryItemEnvironment::new(crate::inventory::test_item_resolver());
            for seed in [0_i64, 1, 7, 12345, -3] {
                let random = GameRandom::with_seed(seed);
                block_drops_summary(
                    &datapacks().loot,
                    &BlockDropParams {
                        dimension: None,
                        block_state_id: state,
                        pos: ORIGIN,
                        level: Some(&level),
                        block_entity_data: None,
                        tool: &tool,
                        breaker,
                        explosion_radius: None,
                        block_entity: None,
                        item_environment: Some(&item_environment),
                    },
                    &random,
                )
                .unwrap_or_else(|error| panic!("{table_id} failed at seed {seed}: {error}"));
            }
        }
    }
}

/// A tool that carries both enchantments the built-in `blocks/*` tables ever
/// read, so the `match_tool`, `table_bonus` and `apply_bonus` branches an
/// unenchanted break skips are all walked too.
fn enchanted_pickaxe() -> LootTool {
    let mut enchantments = BTreeMap::new();
    enchantments.insert("minecraft:silk_touch".to_string(), 1);
    enchantments.insert("minecraft:fortune".to_string(), 3);
    let mut tool = LootTool {
        stack: Default::default(),
        item_id: Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                "minecraft:diamond_pickaxe",
            )
            .expect("the generated registry carries the diamond pickaxe")
            .id,
        ),
        count: 1,
        enchantments,
    };
    tool.stack = bbb_server_types::item_stack::RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: tool.item_id,
        count: tool.count,
        component_patch: bbb_registry_codec::read_item_component(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
            &bbb_protocol::json_value_to_nbt_summary(
                &serde_json::json!({"minecraft:silk_touch":1,"minecraft:fortune":3}),
            )
            .unwrap(),
            crate::inventory::test_item_resolver(),
        )
        .unwrap()
        .into(),
    });
    tool
}

/// Every built-in `blocks/*` loot table id, plus one block state that resolves
/// to each.
fn builtin_block_tables() -> &'static (Vec<String>, BTreeMap<String, i32>) {
    static TABLES: OnceLock<(Vec<String>, BTreeMap<String, i32>)> = OnceLock::new();
    TABLES.get_or_init(|| {
        let registry = datapacks()
            .loot
            .registry("minecraft:loot_table")
            .expect("the loot table registry is loaded");
        let table_ids: BTreeSet<String> = registry
            .elements
            .iter()
            .filter(|element| element.id.starts_with("minecraft:blocks/"))
            .map(|element| element.id.clone())
            .collect();
        let mut by_table: BTreeMap<String, i32> = BTreeMap::new();
        for state in 0..VANILLA_BLOCK_STATE_COUNT {
            let Some(table_id) = block_loot_table_id(state) else {
                continue;
            };
            if table_ids.contains(&table_id) {
                by_table.entry(table_id).or_insert(state);
            }
        }
        (table_ids.into_iter().collect(), by_table)
    })
}

/// `BuiltInLootTables.HARVEST_BEEHIVE` (`BuiltInLootTables.java:107`): one item
/// entry, `set_count` 3, no draws.
#[test]
fn harvest_beehive_drops_three_honeycomb() {
    let nest = state_of(
        "minecraft:bee_nest",
        &[("facing", "north"), ("honey_level", "5")],
    );
    let random = GameRandom::with_seed(0);
    let drops = named_table_drops_summary(
        &datapacks().loot,
        "minecraft:harvest/beehive",
        &BlockDropParams {
            dimension: None,
            block_state_id: nest,
            pos: ORIGIN,
            level: Some(&EMPTY_LEVEL),
            block_entity_data: None,
            tool: &EMPTY_TOOL,
            breaker: LootEntity::Absent,
            explosion_radius: None,
            block_entity: None,
            item_environment: None,
        },
        &random,
    )
    .expect("harvest/beehive is evaluable");
    assert_eq!(drops.len(), 1);
    assert_eq!(
        drops[0].item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_HONEYCOMB_ID)
    );
    assert_eq!(drops[0].count, 3);
}

/// `/loot give <players> loot minecraft:blocks/stone`, bound the way
/// `LootCommand.dropChestLoot` (`LootCommand.java:467-474`) binds it. One
/// `alternatives` reaches both branch shapes at once: `MatchTool.test`
/// (`MatchTool.java:28-31`) is `false` without `TOOL` so the silk-touch child
/// is skipped, and `ExplosionCondition.test` is `true` without
/// `EXPLOSION_RADIUS` so the cobblestone one is taken.
#[test]
fn the_stone_table_drops_cobblestone_under_the_chest_set() {
    for seed in [0, 1, 12345] {
        let random = GameRandom::with_seed(seed);
        let drops = loot_table_drops_summary(
            &datapacks().loot,
            "minecraft:blocks/stone",
            &chest_params(),
            &random,
        )
        .expect("neither missing key refuses the roll");
        assert_eq!(
            item_ids(&drops),
            vec!["minecraft:cobblestone"],
            "seed {seed}"
        );
    }
}

/// Every condition that can now reach a `None` it could not reach before, and
/// the branch 26.1 takes there. A blanket `false` fails rows one and four, a
/// blanket `true` fails rows two and three.
#[test]
fn each_condition_takes_its_own_absent_branch_under_the_chest_set() {
    for (body, expected) in [
        // `ExplosionCondition.test` (`ExplosionCondition.java:27-36`).
        (
            serde_json::json!({"condition": "minecraft:survives_explosion"}),
            true,
        ),
        // `MatchTool.test` (`MatchTool.java:28-31`).
        (
            serde_json::json!({"condition": "minecraft:match_tool"}),
            false,
        ),
        // `LootItemBlockStatePropertyCondition.test`
        // (`LootItemBlockStatePropertyCondition.java:44-47`).
        (
            serde_json::json!({
                "condition": "minecraft:block_state_property",
                "block": "minecraft:stone",
            }),
            false,
        ),
        // `BonusLevelTableCondition.test`
        // (`BonusLevelTableCondition.java:37-42`): a null tool is level 0 and
        // the chance still rolls, so this row is neither a refusal nor a skip.
        (
            serde_json::json!({
                "condition": "minecraft:table_bonus",
                "enchantment": "minecraft:fortune",
                "chances": [1.0, 0.0],
            }),
            true,
        ),
    ] {
        assert_eq!(chest_condition(&body), expected, "{body}");
    }
}

/// The three functions that read a context key each leave the stack alone when
/// the set does not carry it — none of them throws or reads through the null.
#[test]
fn each_function_leaves_the_stack_alone_under_the_chest_set() {
    for body in [
        // `ApplyBonusCount.run` (`ApplyBonusCount.java:63-72`), no `TOOL`.
        serde_json::json!({
            "function": "minecraft:apply_bonus",
            "enchantment": "minecraft:fortune",
            "formula": "minecraft:ore_drops",
        }),
        // `CopyBlockState.run` (`CopyBlockState.java:59-74`), no `BLOCK_STATE`.
        serde_json::json!({
            "function": "minecraft:copy_state",
            "block": "minecraft:stone",
            "properties": ["facing"],
        }),
        // `CopyComponentsFunction.run`
        // (`CopyComponentsFunction.java:77-96`) over
        // `LootContextArg.SimpleGetter.get` (`LootContextArg.java:87-95`), no
        // `BLOCK_ENTITY`.
        serde_json::json!({"function": "minecraft:copy_components", "source": "block_entity"}),
    ] {
        let function = parse_loot_function(&body, None).expect("a declared function type");
        let stack = ItemStackSummary {
            item_id: Some(
                bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:stone")
                    .unwrap()
                    .id,
            ),
            count: 3,
            component_patch: Default::default(),
        };
        let random = GameRandom::with_seed(0);
        let mut eval = Eval {
            table_id: "minecraft:test",
            tables: &datapacks().loot,
            visited: Vec::new(),
            visited_functions: Vec::new(),
            visited_conditions: Vec::new(),
            context: chest_params().create().expect("chest needs only an origin"),
            random: &random,
        };
        let result = eval
            .apply_loot_function(&function, stack.clone(), "function")
            .unwrap_or_else(|error| panic!("{body}: {error}"));
        assert_eq!(result.count, stack.count, "{body}");
        assert_eq!(result.component_patch, stack.component_patch, "{body}");
    }
}

/// `LootCommand.dropChestLoot` (`LootCommand.java:467-474`): `ORIGIN` and an
/// optional `THIS_ENTITY` is all `CHEST` carries
/// (`LootContextParamSets.java:22`).
fn chest_params<'a>() -> LootParams<'a> {
    LootParams {
        origin: Some(LootOrigin::at_center_of(ORIGIN)),
        level: Some(&EMPTY_LEVEL),
        ..LootParams::new(param_sets::CHEST)
    }
}

/// One condition under [`chest_params`].
fn chest_condition(body: &Value) -> bool {
    let condition = parse_loot_condition(body, None).expect("a declared condition type");
    let random = GameRandom::with_seed(0);
    let mut eval = Eval {
        table_id: "minecraft:test",
        tables: &datapacks().loot,
        visited: Vec::new(),
        visited_functions: Vec::new(),
        visited_conditions: Vec::new(),
        context: chest_params().create().expect("chest needs only an origin"),
        random: &random,
    };
    eval.condition(&condition, "condition")
        .unwrap_or_else(|error| panic!("{body}: {error}"))
}

fn item_ids(drops: &[ItemStackSummary]) -> Vec<&'static str> {
    drops
        .iter()
        .map(|stack| {
            stack
                .item_id
                .and_then(vanilla_item_type)
                .map_or("<empty>", |entry| entry.resource_id)
        })
        .collect()
}

/// `Sheep.shear` (`Sheep.java:155-178`) through
/// `LivingEntity.dropFromShearingLootTable` (`LivingEntity.java:1581-1593`):
/// `ORIGIN`, `THIS_ENTITY` and `TOOL` against
/// `LootContextParamSets.SHEARING` (`LootContextParamSets.java:97-99`).
///
/// The per-colour table is one item entry under `rolls` uniform 1..3, so the
/// drop list is that many one-wool stacks.
#[test]
fn a_sheep_wool_table_evaluates_under_the_shearing_parameter_set() {
    let shears = shears_tool();
    for seed in [0_i64, 1, 7, 12345] {
        let random = GameRandom::with_seed(seed);
        let drops = loot_table_drops_summary(
            &datapacks().loot,
            "minecraft:shearing/sheep/white",
            &shearing_params(&shears),
            &random,
        )
        .expect("the per-colour shearing tables are evaluable");
        assert!((1..=3).contains(&drops.len()), "seed {seed}");
        for stack in &drops {
            assert_eq!(
                stack
                    .item_id
                    .and_then(vanilla_item_type)
                    .map(|e| e.resource_id),
                Some("minecraft:white_wool")
            );
            assert_eq!(stack.count, 1);
        }
    }

    // Every colour resolves, and each is its own wool.
    {
        let colour = "black";
        let random = GameRandom::with_seed(0);
        let drops = loot_table_drops_summary(
            &datapacks().loot,
            &format!("minecraft:shearing/sheep/{colour}"),
            &shearing_params(&shears),
            &random,
        )
        .expect("every colour table is evaluable");
        assert_eq!(
            drops[0]
                .item_id
                .and_then(vanilla_item_type)
                .map(|e| e.resource_id),
            Some(format!("minecraft:{colour}_wool").as_str())
        );
    }
}

/// The other three `shearing/*` tables an entity reaches, for the same reason:
/// one item entry each, so the seam carries them too
/// (`SnowGolem.java:156`, `Bogged.java:134`, `MushroomCow.java:176`).
#[test]
fn the_flat_shearing_tables_evaluate_under_the_same_seam() {
    let shears = shears_tool();
    for (table_id, item, count) in [
        (
            "minecraft:shearing/snow_golem",
            "minecraft:carved_pumpkin",
            1,
        ),
        (
            "minecraft:shearing/mooshroom/red",
            "minecraft:red_mushroom",
            5,
        ),
        (
            "minecraft:shearing/mooshroom/brown",
            "minecraft:brown_mushroom",
            5,
        ),
    ] {
        let random = GameRandom::with_seed(0);
        let drops = loot_table_drops_summary(
            &datapacks().loot,
            table_id,
            &shearing_params(&shears),
            &random,
        )
        .unwrap_or_else(|error| panic!("{table_id}: {error}"));
        assert_eq!(drops.len(), count, "{table_id}");
        for stack in &drops {
            assert_eq!(
                stack
                    .item_id
                    .and_then(vanilla_item_type)
                    .map(|e| e.resource_id),
                Some(item)
            );
        }
    }
}

/// The colour-dispatch tables `Sheep.shear` and `MushroomCow.shear` name, end
/// to end. Each is one `alternatives` entry over sixteen (or two)
/// `minecraft:loot_table` children, and the child that runs is chosen by an
/// `entity_properties` predicate reading the animal's own colour component.
#[test]
fn the_shearing_dispatch_tables_draw_by_the_live_animals_colour() {
    let shears = shears_tool();
    for (table_id, type_id, item) in [
        (
            "minecraft:shearing/sheep",
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHEEP_ID,
            "minecraft:white_wool",
        ),
        (
            "minecraft:shearing/mooshroom",
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
            "minecraft:red_mushroom",
        ),
    ] {
        let (store, id) = animal(type_id);
        let random = GameRandom::with_seed(0);
        let drops = loot_table_drops_summary(
            &datapacks().loot,
            table_id,
            &shearing_params_for(&shears, stored(&store, id)),
            &random,
        )
        .unwrap_or_else(|error| panic!("{table_id}: {error}"));
        assert!(!drops.is_empty(), "{table_id} drew nothing");
        for stack in &drops {
            assert_eq!(
                stack
                    .item_id
                    .and_then(vanilla_item_type)
                    .map(|entry| entry.resource_id),
                Some(item)
            );
        }
    }
}

/// `SheepPredicate.matches` (`SheepPredicate.java:24-26`) is the other half of
/// the sheep dispatch: every child of `shearing/sheep` also asks for
/// `sheared: false`, so a sheared sheep, or an animal that is not a sheep,
/// matches no child at all and the pool contributes nothing.
#[test]
fn a_sheared_sheep_matches_no_colour_child_and_drops_nothing() {
    let shears = shears_tool();
    let mut sheared = animal(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHEEP_ID);
    let bbb_protocol::NbtSummaryValue::Compound(mut record) =
        sheared.0.command_data(sheared.1).expect("sheep data")
    else {
        unreachable!("entity data is a compound")
    };
    record.retain(|entry| entry.name != "Sheared");
    record.push(bbb_protocol::NbtSummaryEntry {
        name: "Sheared".into(),
        value: bbb_protocol::NbtSummaryValue::Byte(1),
    });
    sheared
        .0
        .load_command_data(sheared.1, &record)
        .expect("sheared sheep data loads");
    let not_a_sheep = animal(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MOOSHROOM_ID);
    for (store, id) in [&sheared, &not_a_sheep] {
        let random = GameRandom::with_seed(0);
        assert_eq!(
            loot_table_drops_summary(
                &datapacks().loot,
                "minecraft:shearing/sheep",
                &shearing_params_for(&shears, stored(store, *id)),
                &random,
            )
            .expect("the table is evaluable, it just selects nothing"),
            Vec::new()
        );
    }
}

/// A `minecraft:loot_table` registry holding exactly the tables written here,
/// which is what `LootContext.getResolver` (`LootContext.java:63-65`) answers a
/// reference entry's key from.
///
/// Ids arrive sorted because the real merged listing is a `TreeMap` and the
/// lookup binary-searches it.
fn registry_of(tables: &[(&str, Value)]) -> LootRegistries {
    let ids: Vec<&str> = tables.iter().map(|(id, _)| *id).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "the element list is ordered by id");
    LootRegistries {
        registries: vec![LoadedLootRegistry {
            registry_id: "minecraft:loot_table".to_string(),
            elements: tables
                .iter()
                .map(|(id, value)| LootElement {
                    id: (*id).to_string(),
                    source_pack: "test".to_string(),
                    value: LootElementValue::Table(
                        parse_loot_table(value, None)
                            .unwrap_or_else(|error| panic!("{id}: {error}")),
                    ),
                })
                .collect(),
            tags: None,
        }],
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    }
}

/// `LootParams.getLuck` (`LootParams.java:42-44`) as the two nodes that read it
/// see it: `LootPool.addRandomItems` adds `Mth.floor(bonus_rolls * luck)` rolls
/// on top of the pool's own `rolls` (`LootPool.java:96-105`).
///
/// The built-in `minecraft:entities/*` tables declare `bonus_rolls: 0.0` and no
/// `quality` at all, so the death path's `killerPlayer.getLuck()` is only
/// observable on a table that spells them — which is what this fixture is for.
#[test]
fn the_loot_parameters_luck_scales_bonus_rolls() {
    let tables = registry_of(&[(
        "test:luck",
        serde_json::json!({
            "type": "minecraft:empty",
            "pools": [{
                "rolls": 1,
                "bonus_rolls": 2.0,
                "entries": [stone_entry()],
            }],
        }),
    )]);
    // `EMPTY` carries no key at all, so the draw is the pool's own terms only.
    let params = |luck: f32| LootParams {
        luck,
        ..LootParams::new(param_sets::EMPTY)
    };
    let random = GameRandom::with_seed(7);
    assert_eq!(
        loot_table_drops_summary(&tables, "test:luck", &params(0.0), &random)
            .unwrap()
            .len(),
        1,
        "one roll with no luck"
    );
    let random = GameRandom::with_seed(7);
    assert_eq!(
        loot_table_drops_summary(&tables, "test:luck", &params(1.5), &random)
            .unwrap()
            .len(),
        4,
        "1 + Mth.floor(2.0 * 1.5) rolls"
    );
}

/// One stone item entry, the leaf every table below draws.
fn stone_entry() -> Value {
    serde_json::json!({"type": "minecraft:item", "name": "minecraft:stone"})
}

/// Draw `table_id` out of `tables` under `LootContextParamSets.EMPTY`
/// (`LootContextParamSets.java:21`), which carries no key at all, so nothing
/// below is answering a parameter question.
fn drops_of(tables: &LootRegistries, table_id: &str) -> Vec<ItemStackSummary> {
    let random = GameRandom::with_seed(0);
    loot_table_drops_summary(
        tables,
        table_id,
        &LootParams::new(param_sets::EMPTY),
        &random,
    )
    .unwrap_or_else(|error| panic!("{table_id}: {error}"))
}

/// `NestedLootTable.createItemStack` (`NestedLootTable.java:51-55`): the
/// referenced table is drawn *raw*, so its own stacks come out through its own
/// functions and then through every nest the reference entry sits in —
/// `LootPoolSingletonContainer.java:29-34`, `LootPool.java:96-105`,
/// `LootTable.java:88-101`, innermost first.
///
/// The counts pin that order rather than just the membership: the entry's
/// `set_count` is absolute, so it can only produce 13 by running *after* the
/// nested table's own absolute 2 and *before* the two additive ones. Reversed,
/// the same four functions give 5.
#[test]
fn a_reference_entry_draws_the_named_table_through_every_function_nest() {
    let tables = registry_of(&[
        (
            "test:inner",
            serde_json::json!({
                "type": "minecraft:empty",
                "functions": [{"function": "minecraft:set_count", "count": 2}],
                "pools": [{"rolls": 1, "entries": [stone_entry()]}]
            }),
        ),
        (
            "test:outer",
            serde_json::json!({
                "type": "minecraft:empty",
                "functions": [{"function": "minecraft:set_count", "count": 1, "add": true}],
                "pools": [{
                    "rolls": 1,
                    "functions": [{"function": "minecraft:set_count", "count": 2, "add": true}],
                    "entries": [{
                        "type": "minecraft:loot_table",
                        "value": "test:inner",
                        "functions": [{"function": "minecraft:set_count", "count": 10}]
                    }]
                }]
            }),
        ),
    ]);

    let drops = drops_of(&tables, "test:outer");
    assert_eq!(drops.len(), 1);
    assert_eq!(drops[0].count, 13);
}

/// `NestedLootTable.java:53`: a key the resolver has no table for is
/// `LootTable.EMPTY`, which contributes nothing and stops nothing else.
#[test]
fn a_reference_to_a_key_the_registry_has_no_table_for_draws_nothing() {
    let tables = registry_of(&[(
        "test:outer",
        serde_json::json!({
            "type": "minecraft:empty",
            "pools": [
                {"rolls": 1, "entries": [stone_entry()]},
                {"rolls": 1, "entries": [{"type": "minecraft:loot_table", "value": "test:absent"}]}
            ]
        }),
    )]);

    let drops = drops_of(&tables, "test:outer");
    assert_eq!(drops.len(), 1, "only the item pool contributed");
    assert_eq!(drops[0].count, 1);
}

/// `LootContext.pushVisitedElement` (`LootContext.java:55-57`) answers false for
/// a table already being drawn and `LootTable.getRandomItemsRaw`
/// (`LootTable.java:88-101`) then emits nothing at all, so a cycle is a finite
/// draw rather than a stack overflow: `test:a` reaches `test:b`, whose item pool
/// draws and whose reference back to `test:a` contributes nothing.
#[test]
fn a_reference_cycle_draws_each_table_once() {
    let tables = registry_of(&[
        (
            "test:a",
            serde_json::json!({
                "type": "minecraft:empty",
                "pools": [{"rolls": 1, "entries": [
                    {"type": "minecraft:loot_table", "value": "test:b"}
                ]}]
            }),
        ),
        (
            "test:b",
            serde_json::json!({
                "type": "minecraft:empty",
                "pools": [
                    {"rolls": 1, "entries": [stone_entry()]},
                    {"rolls": 1, "entries": [
                        {"type": "minecraft:loot_table", "value": "test:a"}
                    ]}
                ]
            }),
        ),
    ]);

    let drops = drops_of(&tables, "test:a");
    assert_eq!(
        drops.len(),
        1,
        "the cycle contributed nothing a second time"
    );
}

/// `LootContext.popVisitedElement` (`LootContext.java:59-61`) and the matching
/// pop in `Validatable.validateReference`'s child context: the visited set is
/// the path down, not everything ever seen, so two sibling references to one
/// table are two draws and no cycle.
#[test]
fn a_table_referenced_twice_side_by_side_is_drawn_twice() {
    let tables = registry_of(&[
        (
            "test:inner",
            serde_json::json!({
                "type": "minecraft:empty",
                "pools": [{"rolls": 1, "entries": [stone_entry()]}]
            }),
        ),
        (
            "test:outer",
            serde_json::json!({
                "type": "minecraft:empty",
                "pools": [
                    {"rolls": 1, "entries": [
                        {"type": "minecraft:loot_table", "value": "test:inner"}
                    ]},
                    {"rolls": 1, "entries": [
                        {"type": "minecraft:loot_table", "value": "test:inner"}
                    ]}
                ]
            }),
        ),
    ]);

    assert_eq!(drops_of(&tables, "test:outer").len(), 2);
}

/// `LootParams` for one shearing call (`LivingEntity.java:1581-1593`), with an
/// opaque entity.
fn shearing_params(tool: &LootTool) -> LootParams<'_> {
    shearing_params_for(tool, LootEntity::Opaque)
}

/// The same call with a chosen `THIS_ENTITY`.
fn shearing_params_for<'a>(tool: &'a LootTool, this_entity: LootEntity<'a>) -> LootParams<'a> {
    LootParams {
        origin: Some(LootOrigin {
            x: 4.5,
            y: 70.0,
            z: -9.5,
        }),
        tool: Some(tool),
        entities: LootEntities {
            this_entity,
            ..LootEntities::default()
        },
        ..LootParams::new(param_sets::SHEARING)
    }
}

/// A live animal in its own store.
fn animal(type_id: i32) -> (crate::entity::EntityStore, i32) {
    let mut store = crate::entity::EntityStore::new(0, crate::entity::EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            type_id,
            bbb_protocol::packets::Vec3d {
                x: 4.5,
                y: 70.0,
                z: -9.5,
            },
        )
        .expect("a stored mob");
    (store, id)
}

fn stored(store: &crate::entity::EntityStore, id: i32) -> LootEntity<'_> {
    LootEntity::Stored(StoredLootEntity {
        entity: store.get(id).expect("the mob is filed"),
        bodies: Some(crate::entity::store::EntityBodyReadView::new(
            store, None, None,
        )),
        raid_at_position: None,
    })
}

/// The `ItemStack` `Sheep.mobInteract` passes on (`Sheep.java:138-152`).
fn shears_tool() -> LootTool {
    LootTool {
        stack: Default::default(),
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID),
        count: 1,
        enchantments: BTreeMap::new(),
    }
}

/// `DecoratedPotBlock.getDrops` (`DecoratedPotBlock.java:181-192`): the
/// `sherds` dynamic drop is each face's default stack in `PotDecorations.ordered`
/// order, a brick for a face the record leaves out.
#[test]
fn a_cracked_decorated_pot_emits_one_stack_per_face_from_its_block_entity() {
    use crate::block_entity::{BlockEntity, BlockEntityItemProjection};

    let cracked = state_of(
        "minecraft:decorated_pot",
        &[
            ("cracked", "true"),
            ("facing", "north"),
            ("waterlogged", "false"),
        ],
    );
    let record = [
        bbb_protocol::NbtSummaryEntry {
            name: "id".into(),
            value: bbb_protocol::NbtSummaryValue::String("minecraft:decorated_pot".into()),
        },
        bbb_protocol::NbtSummaryEntry {
            name: "sherds".into(),
            value: bbb_protocol::NbtSummaryValue::List(vec![
                bbb_protocol::NbtSummaryValue::String("minecraft:brick".into()),
                bbb_protocol::NbtSummaryValue::String("minecraft:skull_pottery_sherd".into()),
            ]),
        },
    ];
    let pot = BlockEntity::load_static(ORIGIN, cracked, &record)
        .expect("load pot record")
        .expect("a pot record");
    let drops = block_drops_summary(
        &datapacks().loot,
        &BlockDropParams {
            dimension: None,
            block_state_id: cracked,
            pos: ORIGIN,
            level: None,
            block_entity_data: None,
            tool: &EMPTY_TOOL,
            breaker: LootEntity::Absent,
            explosion_radius: None,
            block_entity: Some(LootBlockEntity {
                block_entity: &pot,
                projection: BlockEntityItemProjection::without_resolver(),
            }),
            item_environment: None,
        },
        &GameRandom::with_seed(0),
    )
    .expect("the cracked pot table evaluates");
    let names: Vec<_> = drops
        .iter()
        .map(|stack| {
            vanilla_item_type(stack.item_id.expect("a stack"))
                .expect("a registered item")
                .resource_id
        })
        .collect();
    assert_eq!(
        names,
        [
            "minecraft:brick",
            "minecraft:skull_pottery_sherd",
            "minecraft:brick",
            "minecraft:brick",
        ]
    );
    assert!(drops.iter().all(|stack| stack.count == 1));
}

/// `WeatherCheck.test` (`WeatherCheck.java:24-29`): each present term must
/// equal the level's own `isRaining` / `isThundering`.
#[test]
fn weather_check_compares_each_present_term_with_the_level_weather() {
    let condition = parse_loot_condition(
        &serde_json::json!({
            "condition": "minecraft:weather_check",
            "raining": true,
            "thundering": false,
        }),
        None,
    )
    .expect("the condition parses");
    let check = |weather: Option<LootWeather>| {
        matches_conditions(
            &datapacks().loot,
            "weather",
            std::slice::from_ref(&condition),
            &LootParams {
                weather,
                ..LootParams::new(param_sets::EMPTY)
            },
            &GameRandom::with_seed(0),
        )
    };
    let weather = |raining, thundering| {
        Some(LootWeather {
            raining,
            thundering,
        })
    };
    assert_eq!(check(weather(true, false)), Ok(true));
    assert_eq!(check(weather(true, true)), Ok(false));
    assert_eq!(check(weather(false, false)), Ok(false));
}

/// `SetNameFunction.createResolver` (`SetNameFunction.java:69-88`) never fails
/// the table: with no command source to resolve against, the name stays as
/// written.
#[test]
fn an_entity_resolved_name_without_a_text_resolver_keeps_the_drop() {
    let tables = registry_of(&[(
        "test:named",
        serde_json::json!({
            "type": "minecraft:shearing",
            "pools": [{
                "rolls": 1,
                "entries": [stone_entry()],
                "functions": [{
                    "function": "minecraft:set_name",
                    "entity": "this",
                    "name": {"selector": "@s"},
                }],
            }],
        }),
    )]);
    let (store, id) = animal(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHEEP_ID);
    let tool = shears_tool();
    let random = GameRandom::with_seed(0);
    let item_environment = RegistryItemEnvironment::new(crate::inventory::test_item_resolver());
    let drops = loot_table_drops_summary(
        &tables,
        "test:named",
        &LootParams {
            item_environment: Some(&item_environment),
            ..shearing_params_for(&tool, stored(&store, id))
        },
        &random,
    )
    .expect("the table evaluates");
    assert_eq!(drops.len(), 1);
}

#[test]
fn integer_environment_attribute_check_validates_and_evaluates() {
    use bbb_facts::environment_attribute_types::{
        vanilla_environment_attribute, VanillaEnvironmentAttributeRange,
        VanillaEnvironmentAttributeType,
    };
    use bbb_server_datapack::loot::EnvironmentAttributeExpectedValue;

    let mut base = *vanilla_environment_attribute("minecraft:gameplay/sky_light_level").unwrap();
    base.value_type = VanillaEnvironmentAttributeType::Integer;
    base.value_range = VanillaEnvironmentAttributeRange::SkyLight;
    base.scalar_default = None;
    let attribute: &'static _ = Box::leak(Box::new(base));

    struct IntAttrEnv {
        base: RegistryItemEnvironment<'static>,
        actual: i32,
    }
    impl LootItemEnvironment for IntAttrEnv {
        fn item_resolver(&self) -> &dyn bbb_registry_codec::PersistentItemStackHolderResolver {
            self.base.item_resolver()
        }
        fn environment_attribute(
            &self,
            _attribute: &'static bbb_facts::environment_attribute_types::VanillaEnvironmentAttribute,
            _origin: Option<LootOrigin>,
        ) -> std::result::Result<LootEnvironmentAttributeValue<'_>, LootRuntimeReason> {
            Ok(LootEnvironmentAttributeValue::Integer(self.actual))
        }
        fn enchant_with_levels(
            &self,
            stack: ItemStackSummary,
            levels: i32,
            options: Option<&bbb_server_datapack::loot::HolderSet>,
            random: &GameRandom,
        ) -> std::result::Result<ItemStackSummary, LootRuntimeReason> {
            self.base
                .enchant_with_levels(stack, levels, options, random)
        }
        fn matches_item(
            &self,
            predicate: &bbb_server_datapack::loot::ItemPredicate,
            stack: &ItemStackSummary,
        ) -> std::result::Result<bool, LootRuntimeReason> {
            self.base.matches_item(predicate, stack)
        }
        fn smelt_result(
            &self,
            stack: &ItemStackSummary,
        ) -> std::result::Result<Option<RuntimeItemStack>, LootRuntimeReason> {
            self.base.smelt_result(stack)
        }
    }

    let check_cond = LootCondition::EnvironmentAttributeCheck {
        attribute,
        value: EnvironmentAttributeExpectedValue::Integer(7),
    };
    let mut value_cond = bbb_server_datapack::loot::parse_loot_condition(
        &serde_json::json!({
            "condition": "minecraft:value_check",
            "value": 0.0,
            "range": 7
        }),
        None,
    )
    .unwrap();
    if let LootCondition::ValueCheck { value, .. } = &mut value_cond {
        *value = NumberProvider::EnvironmentAttribute(attribute);
    }
    let conditions = [check_cond, value_cond];
    check_conditions("test:int_attr", &conditions, param_sets::CHEST)
        .expect("integer environment attribute condition validates");

    let tables = registry_of(&[]);
    let eval = |actual: i32| {
        let env = IntAttrEnv {
            base: RegistryItemEnvironment::new(crate::inventory::test_item_resolver()),
            actual,
        };
        matches_conditions(
            &tables,
            "test:int_attr",
            &conditions,
            &LootParams {
                item_environment: Some(&env),
                origin: Some(LootOrigin {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5,
                }),
                ..LootParams::new(param_sets::CHEST)
            },
            &GameRandom::with_seed(0),
        )
    };
    assert_eq!(eval(7), Ok(true));
    assert_eq!(eval(6), Ok(false));
}

#[test]
fn pending_vehicle_cargo_unpacks_for_stored_and_facts_slot_reads() {
    let tables = registry_of(&[
        (
            "test:cargo",
            serde_json::json!({
                "type": "minecraft:chest",
                "pools": [{
                    "rolls": 1,
                    "entries": [{"type": "minecraft:item", "name": "minecraft:emerald"}]
                }]
            }),
        ),
        (
            "test:read_vehicle_slots",
            serde_json::json!({
                "type": "minecraft:entity",
                "pools": [{
                    "rolls": 1,
                    "entries": [{
                        "type": "minecraft:slots",
                        "slot_source": {
                            "type": "minecraft:slot_range",
                            "source": "this",
                            "slots": "container.*"
                        }
                    }]
                }]
            }),
        ),
    ]);
    let resolver = crate::inventory::test_item_resolver();
    let condition = bbb_server_datapack::loot::parse_loot_condition(
        &serde_json::json!({
            "condition": "minecraft:entity_properties",
            "entity": "this",
            "predicate": {
                "slots": {
                    "container.*": {"items": "minecraft:emerald"}
                }
            }
        }),
        None,
    )
    .unwrap();

    struct VehicleEnv<'a> {
        base: RegistryItemEnvironment<'static>,
        store: &'a crate::entity::EntityStore,
    }
    impl LootItemEnvironment for VehicleEnv<'_> {
        fn item_resolver(&self) -> &dyn bbb_registry_codec::PersistentItemStackHolderResolver {
            self.base.item_resolver()
        }
        fn entity_for_relations<'s>(
            &'s self,
            entity: LootEntity<'_>,
            _bodies: Option<crate::entity::store::EntityBodyReadView<'s>>,
        ) -> Option<StoredLootEntity<'s>> {
            let uuid = match entity {
                LootEntity::Stored(stored) => Some(stored.entity.uuid()),
                LootEntity::Facts(facts) => facts.entity_uuid,
                _ => None,
            }?;
            let id = self.store.entity_id_for_uuid(uuid)?;
            Some(StoredLootEntity {
                entity: self.store.get(id)?,
                bodies: Some(crate::entity::store::EntityBodyReadView::new(
                    self.store, None, None,
                )),
                raid_at_position: None,
            })
        }
        fn enchant_with_levels(
            &self,
            stack: ItemStackSummary,
            levels: i32,
            options: Option<&bbb_server_datapack::loot::HolderSet>,
            random: &GameRandom,
        ) -> std::result::Result<ItemStackSummary, LootRuntimeReason> {
            self.base
                .enchant_with_levels(stack, levels, options, random)
        }
        fn matches_item(
            &self,
            predicate: &bbb_server_datapack::loot::ItemPredicate,
            stack: &ItemStackSummary,
        ) -> std::result::Result<bool, LootRuntimeReason> {
            self.base.matches_item(predicate, stack)
        }
        fn smelt_result(
            &self,
            stack: &ItemStackSummary,
        ) -> std::result::Result<Option<RuntimeItemStack>, LootRuntimeReason> {
            self.base.smelt_result(stack)
        }
    }

    for use_facts in [false, true] {
        let mut store = crate::entity::EntityStore::new(0, crate::entity::EntityIdCounter::new());
        let boat_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID;
        let mut boat = crate::entity::ChestBoatData::new();
        boat.set_loot_table("test:cargo".into(), 11).unwrap();
        let id = store.spawn(
            boat_type,
            bbb_protocol::packets::Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            crate::entity::EntityData::ChestBoat(boat),
        );
        assert!(crate::entity::item_slots::container_has_loot(
            store.get(id).unwrap()
        ));

        let bodies = crate::entity::store::EntityBodyReadView::new(&store, None, None);
        let stored_view = StoredLootEntity {
            entity: store.get(id).unwrap(),
            bodies: Some(bodies),
            raid_at_position: None,
        };
        let snapshot = stored_view.snapshot(resolver);
        let this_entity = if use_facts {
            LootEntity::Facts(&snapshot)
        } else {
            LootEntity::Stored(stored_view)
        };
        let env = VehicleEnv {
            base: RegistryItemEnvironment::new(resolver),
            store: &store,
        };
        let unpack = |requested: &crate::entity::Entity, selected: &[i32]| {
            let actual = bodies
                .get(requested.id())
                .filter(|actual| actual.uuid() == requested.uuid())
                .ok_or_else(|| {
                    LootRuntimeReason::Catalog("vehicle slots need source body".into())
                })?;
            crate::entity::item_slots::unpack_vehicle_cargo(
                actual,
                |position, request, available| {
                    let params = LootParams {
                        item_environment: Some(&env),
                        origin: Some(LootOrigin {
                            x: position.x,
                            y: position.y,
                            z: position.z,
                        }),
                        ..LootParams::new(param_sets::CHEST)
                    };
                    fill_container(
                        &tables,
                        &request.table_id,
                        &params,
                        &GameRandom::with_seed(request.seed),
                        available,
                    )
                    .map_err(|error| error.to_string())
                },
            )
            .map_err(LootRuntimeReason::Catalog)?;
            selected.iter().try_fold(Vec::new(), |mut output, slot| {
                if let Some(stack) = crate::entity::item_slots::read(actual, *slot)
                    .map_err(LootRuntimeReason::Catalog)?
                {
                    output.push(stack);
                }
                Ok(output)
            })
        };
        let params = LootParams {
            item_environment: Some(&env),
            vehicle_slots: Some(&unpack),
            origin: Some(LootOrigin {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            }),
            entities: LootEntities {
                this_entity,
                ..Default::default()
            },
            damage_source: LootDamageSource::Present(LootDamageSourceFacts {
                damage_type: "minecraft:generic",
                is_direct: Some(true),
                tags: None,
                causing: LootEntity::Absent,
                direct: None,
            }),
            ..LootParams::new(param_sets::ENTITY)
        };

        assert_eq!(
            matches_conditions(
                &tables,
                "test:pred",
                std::slice::from_ref(&condition),
                &params,
                &GameRandom::with_seed(0),
            ),
            Ok(true),
            "use_facts={use_facts}"
        );
        let drops = loot_table_drops_summary(
            &tables,
            "test:read_vehicle_slots",
            &params,
            &GameRandom::with_seed(0),
        )
        .expect("slot_source unpacks vehicle loot");
        assert_eq!(drops.len(), 1, "use_facts={use_facts}");
        assert_eq!(
            drops[0].item_id,
            Some(
                bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:emerald")
                    .unwrap()
                    .id
            )
        );
    }
}
