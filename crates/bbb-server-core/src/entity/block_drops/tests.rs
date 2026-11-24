//! Every assertion here names the line of `Block.java` it holds the official
//! server to, and runs the loot half against the packaged vanilla datapack
//! rather than a hand-written table.

use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::{DataComponentPatchSummary, ItemEnchantmentSummary};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::*;
use crate::entity::test_items::{
    builtin_startup as datapacks, BuiltinItems as BuiltinEnchantmentPublication,
};
use bbb_server_types::item_stack::RuntimeItemStack;

const ORIGIN: BlockPos = BlockPos { x: 4, y: 70, z: -9 };

/// The seed `Level.random` is started from here.
const LEVEL_RANDOM_SEED: i64 = 99;

fn default_state_of(name: &str) -> i32 {
    let block = vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("the generated registry carries {name}"));
    vanilla_block_default_state_id(block.id).unwrap_or_else(|| panic!("{name} has a default state"))
}

fn item_id(name: &str) -> i32 {
    vanilla_item_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("the generated registry carries {name}"))
        .id
}

fn stack(name: &str, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id(name)),
        count,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

fn existing_stack(stack: ItemStackSummary) -> LootStack {
    RuntimeItemStack::from_summary(stack).into_summary()
}

/// One `Block.dropResources` against the packaged vanilla tables.
fn drops_of(block: &str) -> Vec<PendingPopResource> {
    let mut tables = BlockDropTables {
        tables: Some(&datapacks().loot),
        recipes: None,
        damage_type_tags: None,
    };
    let mut requests = Vec::new();
    drop_resources(
        &mut tables,
        &GameRandom::with_seed(LEVEL_RANDOM_SEED),
        BlockDropView::default(),
        ORIGIN,
        default_state_of(block),
        &mut requests,
    );
    requests
}

fn stack_with_custom_data(name: &str, count: i32, marker: i32) -> ItemStackSummary {
    let mut value = stack(name, count);
    value
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID);
    value.component_patch.custom_data = Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "marker".into(),
        value: NbtSummaryValue::Int(marker),
    }]));
    value
}

#[test]
fn explosion_collection_uses_direct_source_and_merges_across_blocks_without_popping() {
    let snow = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:snow",
        &std::collections::BTreeMap::from([("layers".to_string(), "8".to_string())]),
    )
    .unwrap();
    let mut tables = BlockDropTables {
        tables: Some(&datapacks().loot),
        recipes: None,
        damage_type_tags: None,
    };
    let second = BlockPos {
        x: ORIGIN.x + 1,
        ..ORIGIN
    };
    let mut collectors = Vec::new();
    let level_random = GameRandom::with_seed(LEVEL_RANDOM_SEED);

    collect_explosion_resources(
        &mut tables,
        &level_random,
        BlockDropView::default(),
        ORIGIN,
        snow,
        Some(1.0),
        LootEntity::Absent,
        &mut collectors,
    );
    assert!(
        collectors.is_empty(),
        "snow requires the direct source entity"
    );

    collect_explosion_resources(
        &mut tables,
        &level_random,
        BlockDropView::default(),
        ORIGIN,
        snow,
        Some(1.0),
        LootEntity::Opaque,
        &mut collectors,
    );
    collect_explosion_resources(
        &mut tables,
        &level_random,
        BlockDropView::default(),
        second,
        snow,
        Some(1.0),
        LootEntity::Opaque,
        &mut collectors,
    );

    assert_eq!(collectors.len(), 1);
    assert_eq!(collectors[0].pos, ORIGIN);
    assert_eq!(
        collectors[0].stacks,
        vec![existing_stack(stack("minecraft:snowball", 16))]
    );
}

#[test]
fn explosion_collectors_honor_components_stack_size_and_the_sixteen_item_ceiling() {
    let mut collectors = vec![PendingPopResource {
        pos: ORIGIN,
        stacks: vec![existing_stack(stack("minecraft:stone", 15))],
    }];
    let second = BlockPos {
        x: ORIGIN.x + 1,
        ..ORIGIN
    };
    append_explosion_stack(&mut collectors, second, stack("minecraft:stone", 4))
        .expect("the fresh drop is admitted");
    assert_eq!(collectors.len(), 2);
    assert_eq!(collectors[0].stacks[0].count, 16);
    assert_eq!(collectors[1].pos, second);
    assert_eq!(collectors[1].stacks[0].count, 3);

    let over_ceiling_pos = BlockPos {
        x: ORIGIN.x + 4,
        ..ORIGIN
    };
    let mut over_ceiling_collectors = vec![PendingPopResource {
        pos: ORIGIN,
        stacks: vec![existing_stack(stack("minecraft:stone", 20))],
    }];
    append_explosion_stack(
        &mut over_ceiling_collectors,
        over_ceiling_pos,
        stack("minecraft:stone", 1),
    )
    .expect("the fresh drop is admitted");
    assert_eq!(over_ceiling_collectors.len(), 2);
    assert_eq!(over_ceiling_collectors[0].stacks[0].count, 16);
    assert_eq!(over_ceiling_collectors[1].pos, over_ceiling_pos);
    assert_eq!(over_ceiling_collectors[1].stacks[0].count, 5);

    let component_pos = BlockPos {
        x: ORIGIN.x + 2,
        ..ORIGIN
    };
    let mut component_collectors = vec![PendingPopResource {
        pos: ORIGIN,
        stacks: vec![existing_stack(stack_with_custom_data(
            "minecraft:stone",
            1,
            1,
        ))],
    }];
    append_explosion_stack(
        &mut component_collectors,
        component_pos,
        stack_with_custom_data("minecraft:stone", 1, 2),
    )
    .expect("the fresh drop is admitted");
    assert_eq!(component_collectors.len(), 2);
    assert_eq!(component_collectors[1].pos, component_pos);

    let max_pos = BlockPos {
        x: ORIGIN.x + 3,
        ..ORIGIN
    };
    let mut max_collectors = vec![PendingPopResource {
        pos: ORIGIN,
        stacks: vec![existing_stack(stack("minecraft:ender_pearl", 8))],
    }];
    append_explosion_stack(
        &mut max_collectors,
        max_pos,
        stack("minecraft:ender_pearl", 9),
    )
    .expect("the fresh drop is admitted");
    assert_eq!(max_collectors.len(), 2);
    assert_eq!(max_collectors[0].stacks[0].count, 8);
    assert_eq!(max_collectors[1].stacks[0].count, 9);
}

/// `Block.popResource` (`Block.java:417-424`) places the stack at the block's
/// centre plus one `Mth.nextDouble(random, -0.25, 0.25)` per axis, with the
/// y coordinate lowered by half the item entity's own height (`:418`, `:421`).
#[test]
fn pop_resource_places_the_stack_at_the_official_offsets() {
    let random = GameRandom::with_seed(LEVEL_RANDOM_SEED);
    let mut spawns = Vec::new();
    pop_resource(
        ORIGIN,
        &stack("minecraft:stone", 1),
        &random,
        true,
        &mut spawns,
    );
    assert_eq!(spawns.len(), 1);

    let offset = |at: f64, centre: i32, lowered: f64| {
        (-0.25..=0.25).contains(&(at - f64::from(centre) - 0.5 + lowered))
    };
    let at = spawns[0].position;
    let lowered = f64::from(item_entity_height()) / 2.0;
    assert!(offset(at.x, ORIGIN.x, 0.0));
    assert!(offset(at.y, ORIGIN.y, lowered));
    assert!(offset(at.z, ORIGIN.z, 0.0));
    // `EntityType.ITEM.getHeight()` is a quarter of a block, so the drop starts
    // an eighth of a block below the centre.
    assert_eq!(item_entity_height(), 0.25);
}

/// `GameRules.BLOCK_DROPS` (`gamerules/GameRules.java:29`, `Block.java:443`).
#[test]
fn block_drops_off_spawns_nothing() {
    let random = GameRandom::with_seed(LEVEL_RANDOM_SEED);
    let mut spawns = Vec::new();
    pop_resource(
        ORIGIN,
        &stack("minecraft:stone", 1),
        &random,
        false,
        &mut spawns,
    );
    assert!(spawns.is_empty());
}

/// One item stack whose `minecraft:enchantments` component is a holder id, the
/// shape a loot table's `set_components` writes and a decoded wire stack
/// carries.
fn enchanted_stack() -> ItemStackSummary {
    let sharpness = builtin_enchantment_registry()
        .iter()
        .position(|entry| entry == "minecraft:sharpness")
        .and_then(|index| i32::try_from(index).ok())
        .expect("the built-in pack carries minecraft:sharpness");
    let mut stack = stack("minecraft:iron_sword", 1);
    stack.component_patch.enchantments = vec![ItemEnchantmentSummary {
        holder_id: sharpness,
        level: 3,
    }];
    stack
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
    stack
}

/// `Enchantment.CODEC` is a `RegistryFixedCodec` that writes each holder's
/// *identifier* (`Enchantment.java:71`, `RegistryFixedCodec.java:25-36`), so
/// the level's publication is what turns the drawn stack's holder id into the
/// `minecraft:sharpness` its save form names.
#[test]
fn a_holder_named_stack_lands_through_the_levels_publication() {
    let stack = enchanted_stack();
    let random = GameRandom::with_seed(LEVEL_RANDOM_SEED);
    let mut spawns = Vec::new();
    pop_resource(ORIGIN, &stack, &random, true, &mut spawns);
    assert_eq!(spawns.len(), 1);
    assert_eq!(
        spawns[0].item.item_resource_id(),
        Some("minecraft:iron_sword")
    );
    let enchantments = spawns[0]
        .item
        .component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
            &BuiltinEnchantmentPublication,
        )
        .unwrap()
        .expect("the enchantment is on the drop");
    assert!(
        format!("{enchantments:?}").contains("minecraft:sharpness"),
        "the holder id became the name the save form writes: {enchantments:?}"
    );
}

/// The `LootContextParams.TOOL` the six-argument overload fills, built from a
/// held stack the way [`loot_tool_for_stack`] builds it.
fn tool_of(item: &str, enchantments: &[(&str, i32)]) -> LootTool {
    let registry = builtin_enchantment_registry();
    let mut stack = stack(item, 1);
    stack.component_patch.enchantments = enchantments
        .iter()
        .map(|(id, level)| ItemEnchantmentSummary {
            holder_id: i32::try_from(
                registry
                    .iter()
                    .position(|entry| entry == id)
                    .unwrap_or_else(|| panic!("the built-in registry carries {id}")),
            )
            .expect("a registry is shorter than i32::MAX"),
            level: *level,
        })
        .collect();
    stack
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
    loot_tool_for_stack(&stack, registry)
}

/// One six-argument `Block.dropResources` against the packaged vanilla tables.
fn drops_of_with_tool(
    block: &str,
    tool: &LootTool,
    breaker: LootEntity<'_>,
) -> Vec<PendingPopResource> {
    let item_environment =
        crate::loot_runtime::RegistryItemEnvironment::new(&BuiltinEnchantmentPublication);
    let mut tables = BlockDropTables {
        tables: Some(&datapacks().loot),
        recipes: None,
        damage_type_tags: None,
    };
    let mut requests = Vec::new();
    drop_resources_with_tool(
        &mut tables,
        &GameRandom::with_seed(LEVEL_RANDOM_SEED),
        BlockDropView {
            item_environment: Some(&item_environment),
            ..Default::default()
        },
        ORIGIN,
        default_state_of(block),
        tool,
        breaker,
        &mut requests,
    );
    requests
}

/// The stacks one break of `block` produces, named by item and count.
fn named_drops(block: &str, tool: &LootTool, breaker: LootEntity<'_>) -> Vec<(&'static str, i32)> {
    let requests = drops_of_with_tool(block, tool, breaker);
    requests
        .into_iter()
        .flat_map(|request| request.stacks)
        .map(|stack| {
            let item = stack
                .item_id
                .and_then(bbb_protocol::item_types::vanilla_item_type)
                .map(|entry| entry.resource_id)
                .unwrap_or("<empty>");
            (item, stack.count)
        })
        .collect()
}

/// `minecraft:blocks/stone`'s first alternative is guarded by a `match_tool`
/// whose only term is a `minecraft:enchantments` predicate asking for
/// `silk_touch` at level 1 or more.
///
/// So the whole difference between a stone drop and a cobblestone drop is
/// whether `LootContextParams.TOOL` carries that enchantment
/// (`EnchantmentPredicate.matchesEnchantment`,
/// `EnchantmentPredicate.java:53-60`), and that is the thing the six-argument
/// `Block.getDrops` (`Block.java:373-386`) exists to supply.
#[test]
fn silk_touch_on_the_tool_is_what_turns_a_broken_stone_into_stone() {
    assert_eq!(
        named_drops(
            "minecraft:stone",
            &tool_of("minecraft:diamond_pickaxe", &[("minecraft:silk_touch", 1)]),
            LootEntity::Opaque,
        ),
        vec![("minecraft:stone", 1)]
    );
    // The same pick without the enchantment takes the second alternative, and
    // so does the empty tool the three-argument overload supplies.
    for tool in [tool_of("minecraft:diamond_pickaxe", &[]), LootTool::empty()] {
        assert_eq!(
            named_drops("minecraft:stone", &tool, LootEntity::Opaque),
            vec![("minecraft:cobblestone", 1)]
        );
    }
    // A `fortune` pick is not a `silk_touch` pick: the level is read per
    // enchantment, not "is enchanted at all".
    assert_eq!(
        named_drops(
            "minecraft:stone",
            &tool_of("minecraft:diamond_pickaxe", &[("minecraft:fortune", 3)]),
            LootEntity::Opaque,
        ),
        vec![("minecraft:cobblestone", 1)]
    );
}

/// `minecraft:blocks/short_grass`'s first alternative is a `match_tool` naming
/// one item id outright, which is `ItemStack.is(HolderSet)`
/// (`ItemPredicate.java:27`) against the tool's own holder.
#[test]
fn shears_are_what_make_short_grass_drop_short_grass() {
    let shears = tool_of("minecraft:shears", &[]);
    let pickaxe = tool_of("minecraft:diamond_pickaxe", &[]);
    for _ in 0..16 {
        // Shears take the first alternative every time, and it makes no draw.
        assert_eq!(
            drops_of_with_tool("minecraft:short_grass", &shears, LootEntity::Opaque,)[0].stacks,
            vec![stack("minecraft:short_grass", 1)]
        );
        // Anything else falls through to the seed branch, which is a
        // `random_chance` and never the grass itself.
        for dropped in
            &drops_of_with_tool("minecraft:short_grass", &pickaxe, LootEntity::Opaque)[0].stacks
        {
            assert_eq!(dropped.item_id, Some(item_id("minecraft:wheat_seeds")));
        }
    }
}

/// `minecraft:blocks/amethyst_cluster` names its harvesting tools by the
/// `#minecraft:cluster_max_harvestables` item tag rather than by id, so
/// answering it needs the tag expanded and not just compared.
#[test]
fn an_item_tag_decides_how_many_shards_an_amethyst_cluster_drops() {
    // A pickaxe is in the tag: four shards, before the `ore_drops` bonus.
    assert_eq!(
        named_drops(
            "minecraft:amethyst_cluster",
            &tool_of("minecraft:wooden_pickaxe", &[]),
            LootEntity::Opaque,
        ),
        vec![("minecraft:amethyst_shard", 4)]
    );
    // A shovel is not, so the last alternative gives two.
    assert_eq!(
        named_drops(
            "minecraft:amethyst_cluster",
            &tool_of("minecraft:diamond_shovel", &[]),
            LootEntity::Opaque,
        ),
        vec![("minecraft:amethyst_shard", 2)]
    );
    // And silk touch outranks both.
    assert_eq!(
        named_drops(
            "minecraft:amethyst_cluster",
            &tool_of("minecraft:wooden_pickaxe", &[("minecraft:silk_touch", 1)]),
            LootEntity::Opaque,
        ),
        vec![("minecraft:amethyst_cluster", 1)]
    );
    // The tag membership is the packaged datapack's, re-read here rather than
    // taken from the generated catalog on trust.
    let members = tag_members_from_datapack("cluster_max_harvestables");
    assert!(
        members.contains(&"minecraft:wooden_pickaxe".to_string()),
        "{members:?}"
    );
    assert!(
        !members.contains(&"minecraft:diamond_shovel".to_string()),
        "{members:?}"
    );
    assert_eq!(
        match bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1(
            "minecraft:item",
            "minecraft:cluster_max_harvestables",
        ) {
            bbb_protocol::PersistentRegistryTagMembers::Present(generated) => generated,
            other => panic!("the generated facts carry the tag: {other:?}"),
        },
        members,
        "the generated item tag facts and the packaged datapack must agree"
    );
}

/// One item tag of the packaged built-in datapack, fully expanded.
fn tag_members_from_datapack(tag: &str) -> Vec<String> {
    let snapshot = bbb_server_datapack::vanilla_snapshot::VanillaDatapackSnapshot::embedded()
        .expect("the embedded snapshot loads");
    let bytes = snapshot
        .read_path(&format!("tags/item/{tag}.json"))
        .unwrap_or_else(|| panic!("the built-in pack carries tags/item/{tag}.json"));
    let value: serde_json::Value = serde_json::from_slice(bytes).expect("a tag file is JSON");
    let mut members = Vec::new();
    for entry in value
        .get("values")
        .and_then(serde_json::Value::as_array)
        .expect("a tag file has values")
    {
        let id = match entry {
            serde_json::Value::String(id) => id.clone(),
            serde_json::Value::Object(map) => map
                .get("id")
                .and_then(serde_json::Value::as_str)
                .expect("a tag entry names an id")
                .to_string(),
            other => panic!("unexpected tag entry {other:?}"),
        };
        match id.strip_prefix('#') {
            Some(nested) => members.extend(tag_members_from_datapack(
                nested.trim_start_matches("minecraft:"),
            )),
            None => members.push(id),
        }
    }
    members
}

/// `ApplyBonusCount.OreDrops.calculateNewCount` (`ApplyBonusCount.java:136-147`)
/// reads the tool's `fortune` level and multiplies by `max(nextInt(level + 2) - 1, 0) + 1`.
#[test]
fn fortune_reaches_the_ore_drops_formula() {
    let level = 3;
    let drops = drops_of_with_tool(
        "minecraft:amethyst_cluster",
        &tool_of("minecraft:wooden_pickaxe", &[("minecraft:fortune", level)]),
        LootEntity::Opaque,
    );

    let count = drops[0].stacks[0].count;
    assert!(count % 4 == 0 && (4..=4 * (level + 1)).contains(&count));

    let drops = drops_of_with_tool(
        "minecraft:amethyst_cluster",
        &tool_of("minecraft:wooden_pickaxe", &[]),
        LootEntity::Opaque,
    );
    assert_eq!(drops[0].stacks[0].count, 4);
}

/// `minecraft:blocks/snow`'s single pool is guarded by an `entity_properties`
/// condition on `this` with an empty predicate, and
/// `EntityPredicate.matches` rejects a null entity before it reads a field
/// (`EntityPredicate.java:92-95`).
///
/// So snow drops nothing at all through the three-argument overload and a
/// snowball through the six-argument one — the plainest thing
/// `LootContextParams.THIS_ENTITY` decides.
#[test]
fn a_breaker_is_the_whole_difference_between_snow_dropping_and_not() {
    let shovel = tool_of("minecraft:diamond_shovel", &[]);
    assert_eq!(
        named_drops("minecraft:snow", &shovel, LootEntity::Opaque),
        vec![("minecraft:snowball", 1)]
    );
    assert_eq!(
        named_drops("minecraft:snow", &shovel, LootEntity::Absent),
        Vec::new()
    );
    // The three-argument overload is the absent-breaker one.
    assert_eq!(drops_of("minecraft:snow")[0].stacks, Vec::new());
    // A silk-touched shovel takes the other inner alternative and lifts the
    // layer itself, which is the same `TOOL` read as `blocks/stone`'s.
    assert_eq!(
        named_drops(
            "minecraft:snow",
            &tool_of("minecraft:diamond_shovel", &[("minecraft:silk_touch", 1)]),
            LootEntity::Opaque,
        ),
        vec![("minecraft:snow", 1)]
    );
}

/// The `minecraft:enchantment` registry ids are handed out in the order
/// `RegistryDataLoader` lists the directory, which is *not* the
/// `Enchantments.java` bootstrap order the generated
/// `bbb_protocol::enchantment_types` catalog carries. Reading a holder id
/// against the wrong one silently names the wrong enchantment.
#[test]
fn the_builtin_enchantment_registry_is_directory_order_and_not_bootstrap_order() {
    let registry = builtin_enchantment_registry();
    assert_eq!(registry.len(), 43);
    let mut sorted = registry.to_vec();
    sorted.sort();
    assert_eq!(registry, sorted.as_slice(), "ids follow the listed paths");

    // The loaded generation agrees with the built-in fallback for a world with
    // no extra datapack, which is what makes the fallback usable at all.
    let loaded: Vec<String> = datapacks()
        .registries
        .registry("minecraft:enchantment")
        .expect("the generation carries the enchantment registry")
        .elements
        .iter()
        .map(|element| element.id.clone())
        .collect();
    assert_eq!(loaded, registry);

    // And it is a different order from the generated bootstrap catalog, so the
    // two are not interchangeable.
    let bootstrap: Vec<&str> = (0..registry.len())
        .map(|index| {
            bbb_protocol::enchantment_types::VanillaEnchantmentIdentity::for_vanilla_registry_id(
                i32::try_from(index).unwrap(),
            )
            .expect("the bootstrap catalog is the same length")
            .resource_id()
        })
        .collect();
    assert_ne!(
        bootstrap,
        registry.iter().map(String::as_str).collect::<Vec<_>>()
    );
}
