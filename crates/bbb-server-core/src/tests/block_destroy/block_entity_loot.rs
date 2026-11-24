//! Block loot that reads the block entity (`LootContextParams.BLOCK_ENTITY`)
//! on every drop path, and the loot-table unpack a container runs before it
//! drops.

use super::*;

use super::player_overrides::{fast_tool, item, state, Dig, DIM, MINER};
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, PendingExplosion,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const IGLOO: &str = "minecraft:chests/igloo_chest";

fn field(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn text(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.into())
}

fn stone(count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count,
        ..ItemStackSummary::empty()
    }
}

/// Rewrites fields of the block entity at `pos` by reloading its own save
/// through `BlockEntity.loadStatic`, the way a chunk load would.
fn load_fields(world: &mut ServerWorld, pos: BlockPos, fields: Vec<NbtSummaryEntry>) {
    let block_state = world.block_state(DIM, pos).expect("a loaded position");
    let block_entity = world
        .level_block_entity_for_test(DIM, pos)
        .expect("a block entity");
    let mut record = crate::tests::save_native_fixture_block_entity(block_entity);
    for field in fields {
        record.retain(|entry| entry.name != field.name);
        record.push(field);
    }
    *block_entity = crate::tests::load_native_fixture_block_entity(pos, block_state, &record);
}

fn set_items(world: &mut ServerWorld, pos: BlockPos, items: &[(usize, ItemStackSummary)]) {
    let block_entity = world
        .level_block_entity_for_test(DIM, pos)
        .expect("a block entity");
    let (_, slots) = block_entity
        .randomizable_container_mut()
        .expect("a container");
    for (slot, stack) in items {
        slots[*slot] = crate::tests::native_stack(stack.clone());
    }
}

#[test]
fn dynamic_contents_edit_reaches_the_dropped_container_stack() {
    let table = serde_json::json!({
        "type": "minecraft:block",
        "pools": [{"rolls": 1, "conditions": [{
            "condition": "minecraft:entity_properties", "entity": "this",
            "predicate": {"team": "miners"}
        }, {
            "condition": "minecraft:entity_scores", "entity": "this",
            "scores": {"mining_points": 2}
        }], "entries": [{
            "type": "minecraft:dynamic", "name": "minecraft:contents",
            "functions": [{
                "function": "minecraft:filtered",
                "item_filter": {"items": "minecraft:stone"},
                "on_pass": {"function": "minecraft:set_count", "count": {
                    "type": "minecraft:score", "target": "this", "score": "mining_points", "scale": 1.5
                }}
            }]
        }]}]
    });
    let mut dig = Dig::with_config(
        ServerWorldConfig {
            game_type: GameType::Survival,
            ..crate::tests::datapack_config_with_loot_table("minecraft:blocks/shulker_box", &table)
        },
        true,
    );
    dig.command("team add miners");
    dig.command("team join miners miner");
    dig.command("scoreboard objectives add mining_points dummy");
    dig.command("scoreboard players set miner mining_points 2");
    let pos = BlockPos { x: 2, y: 64, z: 2 };
    let block = state("minecraft:shulker_box", &[]);
    dig.set(pos, block);
    set_items(&mut dig.world, pos, &[(0, stone(7))]);
    dig.hold(fast_tool(
        "minecraft:diamond_pickaxe",
        &["minecraft:shulker_box"],
        true,
    ));
    dig.dig(pos);
    let drops = dig.world.stored_items_for_test(DIM);
    assert_eq!(drops.len(), 1);
    let ground = dig.world.stored_entity_for_test(DIM, drops[0].0).unwrap();
    let crate::entity::EntityData::Item(item) = ground.data() else {
        panic!("a ground item");
    };
    assert_eq!(item.stack().count(), 3);

    let table = serde_json::json!({
        "type": "minecraft:block",
        "pools": [{"rolls": 1, "entries": [{
            "type": "minecraft:item", "name": "minecraft:shulker_box",
            "functions": [{
                "function": "minecraft:set_contents", "component": "minecraft:container",
                "entries": [{"type": "minecraft:slots", "slot_source": {
                    "type": "minecraft:slot_range", "source": "block_entity", "slots": "contents"
                }}]
            }]
        }]}]
    });
    let mut dig = Dig::with_config(
        ServerWorldConfig {
            game_type: GameType::Survival,
            ..crate::tests::datapack_config_with_loot_table("minecraft:blocks/jukebox", &table)
        },
        true,
    );
    dig.set(pos, state("minecraft:jukebox", &[]));
    let custom_data = NbtSummaryValue::Compound(vec![field("record", text("native"))]);
    load_fields(
        &mut dig.world,
        pos,
        vec![field(
            "RecordItem",
            NbtSummaryValue::Compound(vec![
                field("id", text("minecraft:music_disc_cat")),
                field("count", NbtSummaryValue::Int(1)),
                field(
                    "components",
                    NbtSummaryValue::Compound(vec![field(
                        "minecraft:custom_data",
                        custom_data.clone(),
                    )]),
                ),
            ]),
        )],
    );
    let saved_record = |dig: &mut Dig| {
        crate::tests::save_native_fixture_block_entity(
            dig.world.level_block_entity_for_test(DIM, pos).unwrap(),
        )
        .into_iter()
        .find(|entry| entry.name == "RecordItem")
        .expect("a saved record")
    };
    let source_record = saved_record(&mut dig);
    dig.command("loot spawn 5 70 5 mine 2 64 2");
    assert_eq!(dig.block(pos), Some(state("minecraft:jukebox", &[])));
    assert_eq!(saved_record(&mut dig), source_record);
    let drops = dropped(&dig.world);
    assert_eq!(drops.len(), 1, "{drops:#?}");
    let shulker = only_drop_of(&drops, "minecraft:shulker_box");
    let contents = &shulker.components["minecraft:container"];
    assert_eq!(
        container_slots(contents),
        vec![(0, "minecraft:music_disc_cat".to_string(), 1)]
    );
    let NbtSummaryValue::List(slots) = contents else {
        unreachable!()
    };
    let NbtSummaryValue::Compound(slot) = &slots[0] else {
        unreachable!()
    };
    let NbtSummaryValue::Compound(stack) = &slot
        .iter()
        .find(|entry| entry.name == "item")
        .unwrap()
        .value
    else {
        panic!("a contained record stack");
    };
    assert!(stack.contains(&field(
        "components",
        NbtSummaryValue::Compound(vec![field("minecraft:custom_data", custom_data)])
    )));
}

#[test]
fn block_entity_loot_slots_unpack_copy_and_survive_a_real_restart() {
    use bbb_server_datapack::loot::{parse_loot_table, LootElementValue};

    let copy_table = serde_json::json!({
        "type": "minecraft:block",
        "pools": [{"rolls": 1, "entries": [{
            "type": "minecraft:item", "name": "minecraft:shulker_box",
            "functions": [{
                "function": "minecraft:set_contents", "component": "minecraft:container",
                "entries": [{"type": "minecraft:slots", "slot_source": {
                    "type": "minecraft:slot_range", "source": "block_entity", "slots": "container.*"
                }}]
            }]
        }]}]
    });
    let fill_table = serde_json::json!({
        "type": "minecraft:chest",
        "pools": [{"rolls": 1, "conditions": [{
            "condition": "minecraft:weather_check", "raining": false, "thundering": false
        }], "entries": [{
            "type": "minecraft:item", "name": "minecraft:stone",
            "functions": [{"function": "minecraft:set_custom_data", "tag": "{source:'pending'}"}]
        }]}, {"rolls": 1, "conditions": [{
            "condition": "minecraft:weather_check", "raining": true, "thundering": true
        }], "entries": [{
            "type": "minecraft:item", "name": "minecraft:emerald"
        }]}]
    });
    let invalid_slots_table = serde_json::json!({
        "type": "minecraft:block",
        "pools": [{"rolls": 1, "entries": [{
            "type": "minecraft:slots", "slot_source": {
                "type": "minecraft:slot_range", "source": "block_entity", "slots": "weapon"
            }
        }]}]
    });
    let raw_slots_table = serde_json::json!({
        "type": "minecraft:block",
        "pools": [{"rolls": 1, "entries": [{
            "type": "minecraft:slots", "slot_source": {
                "type": "minecraft:slot_range", "source": "block_entity", "slots": "contents"
            }
        }]}]
    });
    let mut config = datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let registry = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .unwrap();
    for (id, body) in [
        ("minecraft:blocks/chest", &copy_table),
        ("minecraft:blocks/shulker_box", &copy_table),
        ("minecraft:blocks/barrel", &invalid_slots_table),
        ("minecraft:blocks/crafter", &raw_slots_table),
        (IGLOO, &fill_table),
    ] {
        registry
            .elements
            .iter_mut()
            .find(|element| element.id == id)
            .unwrap()
            .value = LootElementValue::Table(parse_loot_table(body, None).unwrap());
    }
    let root = std::env::temp_dir().join(format!("bbb-block-loot-slots-{}", Uuid::new_v4()));
    let storage = created_fixture_save(&root, "bbb-block-loot-slots");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "miner", 1);
    assert!(!world.level_weather(DIM).is_raining());
    assert!(!world.level_weather(DIM).is_thundering());
    let positions = [
        BlockPos { x: 2, y: 64, z: 2 },
        BlockPos { x: 6, y: 64, z: 2 },
    ];
    let untouched = BlockPos { x: 10, y: 64, z: 2 };
    let raw_crafter = BlockPos { x: 14, y: 64, z: 2 };
    for (pos, block) in [
        (positions[0], "minecraft:chest"),
        (positions[1], "minecraft:shulker_box"),
        (untouched, "minecraft:barrel"),
        (raw_crafter, "minecraft:crafter"),
    ] {
        assert!(world.set_block(
            DIM,
            pos,
            state(block, &[]),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
        let body = world.level_block_entity_for_test(DIM, pos).unwrap();
        assert!(body.set_container_loot_table(IGLOO, 31));
        assert!(body
            .container_items()
            .unwrap()
            .iter()
            .all(|item| item.is_empty()));
    }
    let mine = |world: &mut ServerWorld, pos: BlockPos, expected| {
        let operation = world.publication().tick;
        let tick = console_command(
            &sender,
            &receiver,
            world,
            operation,
            &format!("loot spawn 5 70 5 mine {} {} {}", pos.x, pos.y, pos.z),
        );
        let outcome = admin_outcome(&tick);
        assert!(!outcome.replayed, "{outcome:?}");
        let Ok(AdminResult::Command(report)) = &outcome.result else {
            panic!("{outcome:?}");
        };
        assert_eq!(report.result, Some(expected), "{report:?}");
    };
    mine(&mut world, untouched, 0);
    mine(&mut world, raw_crafter, 0);
    assert!(dropped(&world).is_empty());
    for pos in [untouched, raw_crafter] {
        let body = world.level_block_entity_for_test(DIM, pos).unwrap();
        assert_eq!(body.container_loot_table_id(), Some(IGLOO));
        assert!(body
            .container_items()
            .unwrap()
            .iter()
            .all(|item| item.is_empty()));
    }
    let expected_data = NbtSummaryValue::Compound(vec![field("source", text("pending"))]);
    let mut original_slots = Vec::new();
    for (index, pos) in positions.into_iter().enumerate() {
        for copy in 0..2 {
            mine(&mut world, pos, 1);
            assert_ne!(
                world.block_state(DIM, pos),
                Some(block_state_id("minecraft:air"))
            );
            let body = world.level_block_entity_for_test(DIM, pos).unwrap();
            assert_eq!(body.container_loot_table_id(), None);
            let items = body.container_items().unwrap();
            assert_eq!(items.iter().map(|item| item.count()).sum::<i32>(), 1);
            if copy == 0 {
                original_slots.push(items.clone());
            } else {
                assert_eq!(items, original_slots[index]);
            }
        }
    }
    for pos in [untouched, raw_crafter] {
        let body = world.level_block_entity_for_test(DIM, pos).unwrap();
        assert_eq!(body.container_loot_table_id(), Some(IGLOO));
        assert!(body
            .container_items()
            .unwrap()
            .iter()
            .all(|item| item.is_empty()));
    }
    let drops = dropped(&world);
    assert_eq!(drops.iter().map(|drop| drop.count).sum::<i32>(), 4);
    for output in &drops {
        assert_eq!(output.item, "minecraft:shulker_box");
        let contents = output
            .components
            .get("minecraft:container")
            .unwrap_or_else(|| panic!("missing copied container: {drops:#?}"));
        assert_eq!(
            container_slots(contents),
            vec![(0, "minecraft:stone".to_string(), 1)]
        );
        let NbtSummaryValue::List(slots) = contents else {
            unreachable!()
        };
        let NbtSummaryValue::Compound(slot) = &slots[0] else {
            unreachable!()
        };
        let NbtSummaryValue::Compound(item) = &slot
            .iter()
            .find(|entry| entry.name == "item")
            .unwrap()
            .value
        else {
            unreachable!()
        };
        assert!(item.contains(&field(
            "components",
            NbtSummaryValue::Compound(vec![field("minecraft:custom_data", expected_data.clone())])
        )));
    }
    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reopened = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(config, &reopened);
    for (pos, expected) in positions.into_iter().zip(original_slots) {
        let body = world.level_block_entity_for_test(DIM, pos).unwrap();
        assert_eq!(body.container_loot_table_id(), None);
        assert_eq!(body.container_items().unwrap(), expected);
        let stack = expected.iter().find(|item| !item.is_empty()).unwrap();
        assert_eq!(
            crate::tests::saved_item_component(&world, stack, "minecraft:custom_data"),
            Some(expected_data.clone())
        );
    }
    for pos in [untouched, raw_crafter] {
        let body = world.level_block_entity_for_test(DIM, pos).unwrap();
        assert_eq!(body.container_loot_table_id(), Some(IGLOO));
        assert!(body
            .container_items()
            .unwrap()
            .iter()
            .all(|item| item.is_empty()));
    }
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

/// One dropped item entity as its persistent stack: item, count and the
/// component patch by component id.
#[derive(Debug)]
struct Dropped {
    item: String,
    count: i32,
    components: BTreeMap<String, NbtSummaryValue>,
}

fn dropped(world: &ServerWorld) -> Vec<Dropped> {
    world
        .stored_items_for_test(DIM)
        .into_iter()
        .map(|(id, _, item, count, _, _)| {
            let stack = world
                .stored_item_stack_for_test(DIM, id)
                .expect("a stored stack");
            let components = crate::tests::saved_item_components(world, &stack)
                .into_iter()
                .filter(|entry| !entry.name.to_jdk_string().starts_with('!'))
                .map(|entry| (entry.name.to_jdk_string(), entry.value))
                .collect();
            Dropped {
                item,
                count,
                components,
            }
        })
        .collect()
}

fn only_drop_of<'a>(drops: &'a [Dropped], item: &str) -> &'a Dropped {
    let matching: Vec<_> = drops.iter().filter(|drop| drop.item == item).collect();
    assert_eq!(matching.len(), 1, "{drops:#?}");
    matching[0]
}

fn totals<'a>(stacks: impl Iterator<Item = (&'a str, i32)>) -> BTreeMap<String, i32> {
    let mut totals = BTreeMap::new();
    for (item, count) in stacks {
        *totals.entry(item.to_string()).or_insert(0) += count;
    }
    totals
}

fn resource_id(stack: &ItemStackSummary) -> &'static str {
    bbb_protocol::item_types::vanilla_item_type(stack.item_id.expect("a stack"))
        .expect("a registered item")
        .resource_id
}

/// `LootTable.fill` (`LootTable.java:146-165`) for a 27-slot container whose
/// non-zero seed picks `new GameRandom(seed)`, as the level runs it.
fn seeded_fill(world: &mut ServerWorld, pos: BlockPos, seed: i64) -> Vec<ItemStackSummary> {
    let slots = world.seeded_container_fill_for_test(DIM, pos, IGLOO, seed);
    assert!(
        slots.iter().any(|stack| stack.item_id().is_some()),
        "the fill draws something"
    );
    slots
        .into_iter()
        .map(|stack| stack.summary().clone())
        .collect()
}

/// `ItemContainerContents` as `Slot.CODEC` writes it (`ItemContainerContents.java:176-182`):
/// the occupied slots with their item, whose count of one is left out.
fn container_slots(value: &NbtSummaryValue) -> Vec<(i32, String, i32)> {
    let NbtSummaryValue::List(slots) = value else {
        panic!("a container is a slot list: {value:?}");
    };
    slots
        .iter()
        .map(|slot| {
            let NbtSummaryValue::Compound(fields) = slot else {
                panic!("a slot is a compound: {slot:?}");
            };
            let get = |name: &str| fields.iter().find(|entry| entry.name == name);
            let Some(NbtSummaryEntry {
                value: NbtSummaryValue::Int(index),
                ..
            }) = get("slot")
            else {
                panic!("{fields:?}");
            };
            let Some(NbtSummaryEntry {
                value: NbtSummaryValue::Compound(stack),
                ..
            }) = get("item")
            else {
                panic!("{fields:?}");
            };
            let id = stack
                .iter()
                .find_map(|entry| match (&entry.value, entry.name == "id") {
                    (NbtSummaryValue::String(id), true) => Some(id.try_to_string().unwrap()),
                    _ => None,
                })
                .expect("an item id");
            let count = stack
                .iter()
                .find_map(|entry| match (&entry.value, entry.name == "count") {
                    (NbtSummaryValue::Int(count), true) => Some(*count),
                    _ => None,
                })
                .unwrap_or(1);
            (*index, id, count)
        })
        .collect()
}

fn slot_view(slots: &[ItemStackSummary]) -> Vec<(i32, String, i32)> {
    slots
        .iter()
        .enumerate()
        .filter(|(_, stack)| stack.item_id.is_some() && stack.count > 0)
        .map(|(slot, stack)| (slot as i32, resource_id(stack).to_string(), stack.count))
        .collect()
}

/// `blocks/shulker_box` copies `custom_name` and `container` off
/// `collectComponents` (`BlockEntity.java:309-314`,
/// `BaseContainerBlockEntity.java:157-165`) for a survival break.
#[test]
fn a_survival_dig_drops_a_named_shulker_box_carrying_its_contents() {
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let table = startup
        .loot
        .registries
        .iter_mut()
        .flat_map(|registry| &mut registry.elements)
        .find(|entry| entry.id == "minecraft:blocks/shulker_box")
        .unwrap();
    let bbb_server_datapack::loot::LootElementValue::Table(table) = &mut table.value else {
        panic!("a block loot table");
    };
    let condition = bbb_server_datapack::loot::parse_loot_condition(&serde_json::json!({
        "condition":"minecraft:location_check","offsetX":1,
        "predicate":{"block":{"nbt":"{id:'minecraft:chest',x:3,y:64,z:2,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"}}
    }), None).unwrap();
    for pool in &mut table.pools {
        pool.conditions.push(condition.clone());
    }
    let mut dig = Dig::with_config(config, false);
    let pos = BlockPos { x: 2, y: 64, z: 2 };
    let chest = BlockPos { x: 3, y: 64, z: 2 };
    dig.set(chest, state("minecraft:chest", &[]));
    let mut diamonds = item("minecraft:diamond");
    diamonds.count = 3;
    set_items(&mut dig.world, chest, &[(0, diamonds)]);
    dig.set(pos, state("minecraft:shulker_box", &[]));
    load_fields(&mut dig.world, pos, vec![field("CustomName", text("Loot"))]);
    set_items(&mut dig.world, pos, &[(3, stone(7)), (20, stone(1))]);
    dig.hold(fast_tool(
        "minecraft:diamond_pickaxe",
        &["minecraft:shulker_box"],
        true,
    ));

    dig.dig(pos);
    assert_eq!(dig.block(pos), Some(block_state_id("minecraft:air")));
    let drops = dropped(&dig.world);
    assert_eq!(drops.len(), 1, "{drops:#?}");
    let shulker = only_drop_of(&drops, "minecraft:shulker_box");
    assert_eq!(shulker.count, 1);
    assert_eq!(
        shulker.components.get("minecraft:custom_name"),
        Some(&text("Loot"))
    );
    assert_eq!(
        container_slots(&shulker.components["minecraft:container"]),
        vec![
            (3, "minecraft:stone".to_string(), 7),
            (20, "minecraft:stone".to_string(), 1)
        ]
    );
    assert!(!shulker.components.contains_key("minecraft:container_loot"));
}

#[test]
fn a_survival_dig_fills_a_shulker_loot_table_into_the_dropped_box() {
    for seed in [8_675_309, 0] {
        let mut dig = Dig::new(GameType::Survival, false);
        let pos = BlockPos { x: 2, y: 64, z: 2 };
        dig.set(pos, state("minecraft:shulker_box", &[]));
        load_fields(
            &mut dig.world,
            pos,
            vec![
                field("LootTable", text(IGLOO)),
                field("LootTableSeed", NbtSummaryValue::Long(seed)),
            ],
        );
        let expected = (seed != 0).then(|| slot_view(&seeded_fill(&mut dig.world, pos, seed)));
        dig.hold(fast_tool(
            "minecraft:diamond_pickaxe",
            &["minecraft:shulker_box"],
            true,
        ));

        dig.dig(pos);
        let drops = dropped(&dig.world);
        assert_eq!(drops.len(), 1, "{drops:#?}");
        let shulker = only_drop_of(&drops, "minecraft:shulker_box");
        let slots = container_slots(&shulker.components["minecraft:container"]);
        if let Some(expected) = expected {
            assert_eq!(slots, expected);
        } else {
            assert!(!slots.is_empty());
            let startup = crate::registry_sync::builtin_datapack_startup_shared();
            let table = crate::loot_runtime::loot_table_by_id(&startup.loot, IGLOO).unwrap();
            assert!(table.random_sequence.is_some());
        }
        assert!(!shulker.components.contains_key("minecraft:container_loot"));
    }
}

/// `ShulkerBoxBlockEntity.saveAdditional` / `loadAdditional` (`:202-213`) over
/// `BaseContainerBlockEntity`'s `CustomName` and the loot-table pair; the
/// explosion drop of a box that still names its table carries it as
/// `container_loot` instead of unpacking it (`preRemoveSideEffects` is empty).
#[test]
fn a_packed_shulker_box_keeps_its_name_and_seed_and_explodes_into_container_loot() {
    let mut dig = Dig::new(GameType::Survival, false);
    let pos = BlockPos { x: 2, y: 80, z: 2 };
    dig.set(pos, state("minecraft:shulker_box", &[]));
    load_fields(
        &mut dig.world,
        pos,
        vec![
            field("CustomName", text("Sealed")),
            field("LootTable", text(IGLOO)),
            field("LootTableSeed", NbtSummaryValue::Long(-42)),
        ],
    );
    let record = dig
        .world
        .level_block_entity_for_test(DIM, pos)
        .expect("a shulker box")
        .save_with_full_metadata_with_resolver(
            crate::world::registry_runtime::item_stack_resolver_for_test(),
        )
        .expect("the fixture record saves through its registry publication");
    let saved = |name: &str| {
        record
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.value.clone())
    };
    assert_eq!(saved("CustomName"), Some(text("Sealed")));
    assert_eq!(saved("LootTable"), Some(text(IGLOO)));
    assert_eq!(saved("LootTableSeed"), Some(NbtSummaryValue::Long(-42)));
    assert_eq!(saved("Items"), None);

    explode(&mut dig.world, pos, 3.0);
    assert_eq!(dig.block(pos), Some(block_state_id("minecraft:air")));
    let drops = dropped(&dig.world);
    let shulker = only_drop_of(&drops, "minecraft:shulker_box");
    assert_eq!(
        shulker.components.get("minecraft:custom_name"),
        Some(&text("Sealed"))
    );
    assert_eq!(
        shulker.components.get("minecraft:container_loot"),
        Some(&NbtSummaryValue::Compound(vec![
            field("loot_table", text(IGLOO)),
            field("seed", NbtSummaryValue::Long(-42)),
        ]))
    );
    assert_eq!(drops.len(), 1, "{drops:#?}");
}

fn explode(world: &mut ServerWorld, pos: BlockPos, radius: f32) {
    let mut events = Vec::new();
    world.explode(
        DIM,
        PendingExplosion {
            source: None,
            center: Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
            radius,
            fire: false,
            // `tnt_explosion_drop_decay` is off by default, so every block drops.
            interaction: ExplosionInteraction::Tnt,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        },
        &mut events,
    );
}

/// `BlockBehaviour.onExplosionHit` (`BlockBehaviour.java:177-201`) reads the
/// block entity before `getDrops` and the removal: the named shulker box keeps
/// its contents in the item, and the chest's table fills during the removal's
/// `Containers.dropContents` before its slots scatter.
#[test]
fn an_explosion_drops_block_entity_components_and_unpacks_a_loot_chest() {
    let mut dig = Dig::new(GameType::Survival, false);
    let shulker_pos = BlockPos { x: 2, y: 80, z: 2 };
    let chest_pos = BlockPos { x: 3, y: 80, z: 2 };
    dig.set(shulker_pos, state("minecraft:shulker_box", &[]));
    load_fields(
        &mut dig.world,
        shulker_pos,
        vec![field("CustomName", text("Blown"))],
    );
    set_items(&mut dig.world, shulker_pos, &[(0, stone(5))]);
    dig.set(chest_pos, state("minecraft:chest", &[]));
    load_fields(
        &mut dig.world,
        chest_pos,
        vec![
            field("CustomName", text("Stash")),
            field("LootTable", text(IGLOO)),
            field("LootTableSeed", NbtSummaryValue::Long(77)),
        ],
    );
    let expected = seeded_fill(&mut dig.world, chest_pos, 77);

    explode(&mut dig.world, shulker_pos, 4.0);
    assert_eq!(
        dig.block(shulker_pos),
        Some(block_state_id("minecraft:air"))
    );
    assert_eq!(dig.block(chest_pos), Some(block_state_id("minecraft:air")));
    let drops = dropped(&dig.world);
    let shulker = only_drop_of(&drops, "minecraft:shulker_box");
    assert_eq!(
        shulker.components.get("minecraft:custom_name"),
        Some(&text("Blown"))
    );
    assert_eq!(
        container_slots(&shulker.components["minecraft:container"]),
        vec![(0, "minecraft:stone".to_string(), 5)]
    );
    let chest = only_drop_of(&drops, "minecraft:chest");
    assert_eq!(
        chest.components.get("minecraft:custom_name"),
        Some(&text("Stash"))
    );
    let scattered = totals(
        drops
            .iter()
            .filter(|drop| drop.item != "minecraft:chest" && drop.item != "minecraft:shulker_box")
            .map(|drop| (drop.item.as_str(), drop.count)),
    );
    let filled = totals(
        expected
            .iter()
            .filter(|stack| stack.item_id.is_some())
            .map(|stack| (resource_id(stack), stack.count)),
    );
    assert_eq!(scattered, filled);
}

/// A survival break of a packed chest: `Containers.dropContents` reads every
/// slot through `RandomizableContainerBlockEntity.getItem`, which unpacks the
/// table with no player (`RandomizableContainerBlockEntity.java:54-57`) before
/// the chest item itself drops.
#[test]
fn a_survival_dig_of_a_loot_chest_fills_it_before_its_contents_drop() {
    let mut dig = Dig::new(GameType::Survival, false);
    let pos = BlockPos { x: 2, y: 64, z: 2 };
    dig.set(pos, state("minecraft:chest", &[]));
    load_fields(
        &mut dig.world,
        pos,
        vec![
            field("LootTable", text(IGLOO)),
            field("LootTableSeed", NbtSummaryValue::Long(1_234_567)),
        ],
    );
    let expected = seeded_fill(&mut dig.world, pos, 1_234_567);
    dig.hold(fast_tool(
        "minecraft:diamond_axe",
        &["minecraft:chest"],
        true,
    ));

    dig.dig(pos);
    let drops = dropped(&dig.world);
    let chest = only_drop_of(&drops, "minecraft:chest");
    assert!(chest.components.is_empty(), "{chest:?}");
    let scattered = totals(
        drops
            .iter()
            .filter(|drop| drop.item != "minecraft:chest")
            .map(|drop| (drop.item.as_str(), drop.count)),
    );
    let filled = totals(
        expected
            .iter()
            .filter(|stack| stack.item_id.is_some())
            .map(|stack| (resource_id(stack), stack.count)),
    );
    assert_eq!(scattered, filled);
}

/// `blocks/chest` and `blocks/furnace` copy `custom_name`
/// (`BaseContainerBlockEntity.java:157-165`); the contents leave through the
/// removal, not the item.
#[test]
fn named_containers_drop_their_name_on_the_item_and_their_contents_beside_it() {
    let table = serde_json::json!({"type":"minecraft:block", "pools":[{"rolls":1,"entries":[{
        "type":"minecraft:item","name":"minecraft:chest",
        "functions":[{"function":"minecraft:copy_name","source":"block_entity"}]
    }]}]});
    let mut dig = Dig::with_config(
        ServerWorldConfig {
            game_type: GameType::Survival,
            ..crate::tests::datapack_config_with_loot_table("minecraft:blocks/chest", &table)
        },
        false,
    );
    let chest = BlockPos { x: 2, y: 64, z: 2 };
    let furnace = BlockPos { x: 5, y: 64, z: 2 };
    dig.set(chest, state("minecraft:chest", &[]));
    load_fields(
        &mut dig.world,
        chest,
        vec![field("CustomName", text("Stash"))],
    );
    set_items(&mut dig.world, chest, &[(0, stone(5))]);
    dig.set(furnace, state("minecraft:furnace", &[]));
    load_fields(
        &mut dig.world,
        furnace,
        vec![field("CustomName", text("Oven"))],
    );
    dig.hold(fast_tool(
        "minecraft:diamond_pickaxe",
        &["minecraft:chest", "minecraft:furnace"],
        true,
    ));

    dig.dig(chest);
    dig.dig(furnace);
    let drops = dropped(&dig.world);
    assert_eq!(
        only_drop_of(&drops, "minecraft:chest")
            .components
            .get("minecraft:custom_name"),
        Some(&text("Stash"))
    );
    assert_eq!(
        only_drop_of(&drops, "minecraft:furnace")
            .components
            .get("minecraft:custom_name"),
        Some(&text("Oven"))
    );
    let stone = only_drop_of(&drops, "minecraft:stone");
    assert_eq!(stone.count, 5);
    assert!(stone.components.is_empty(), "{stone:?}");
    assert_eq!(drops.len(), 3, "{drops:#?}");
}

fn red_stripe() -> NbtSummaryValue {
    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
        field("pattern", text("minecraft:stripe_top")),
        field("color", text("red")),
    ])])
}

/// `blocks/white_banner` copies `custom_name` and `banner_patterns`
/// (`BannerBlockEntity.java:96-100`).
#[test]
fn a_survival_dig_drops_a_banner_with_its_patterns_and_name() {
    let mut dig = Dig::new(GameType::Survival, false);
    let pos = BlockPos { x: 2, y: 64, z: 2 };
    dig.set(pos, state("minecraft:white_banner", &[]));
    load_fields(
        &mut dig.world,
        pos,
        vec![
            field("patterns", red_stripe()),
            field("CustomName", text("Flag")),
        ],
    );
    dig.hold(fast_tool(
        "minecraft:diamond_axe",
        &["minecraft:white_banner"],
        true,
    ));

    dig.dig(pos);
    let drops = dropped(&dig.world);
    assert_eq!(drops.len(), 1, "{drops:#?}");
    let banner = only_drop_of(&drops, "minecraft:white_banner");
    assert_eq!(
        banner.components.get("minecraft:banner_patterns"),
        Some(&red_stripe())
    );
    assert_eq!(
        banner.components.get("minecraft:custom_name"),
        Some(&text("Flag"))
    );
}

/// `WaterFluid.beforeDestroyingBlock` (`WaterFluid.java:81-84`) reads the block
/// entity before the flow replaces a head, so the washed-away head keeps its
/// profile and name.
#[test]
fn flowing_water_washes_a_player_head_away_with_its_profile() {
    let mut dig = Dig::new(GameType::Survival, false);
    let pos = BlockPos { x: 2, y: 64, z: 8 };
    let head = state("minecraft:player_head", &[]);
    dig.set(pos, head);
    load_fields(
        &mut dig.world,
        pos,
        vec![
            field("profile", steve()),
            field("custom_name", text("Trophy")),
        ],
    );
    dig.set(BlockPos { z: 9, ..pos }, state("minecraft:water", &[]));
    for _ in 0..12 {
        dig.world.tick(dig.receiver.drain());
    }

    assert_ne!(dig.block(pos), Some(head), "the flow reached the head");
    let drops = dropped(&dig.world);
    assert_eq!(drops.len(), 1, "{drops:#?}");
    let dropped_head = only_drop_of(&drops, "minecraft:player_head");
    assert_eq!(
        dropped_head.components.get("minecraft:profile"),
        Some(&steve())
    );
    assert_eq!(
        dropped_head.components.get("minecraft:custom_name"),
        Some(&text("Trophy"))
    );
}

/// `PistonBaseBlock.moveBlocks` (`PistonBaseBlock.java:302-303`) destroys a
/// `PushReaction.DESTROY` block through the four-argument `dropResources`,
/// which reads the block entity first.
#[test]
fn a_piston_destroying_a_shulker_box_drops_it_with_its_contents() {
    let mut dig = Dig::new(GameType::Survival, false);
    let piston = BlockPos { x: 4, y: 64, z: 6 };
    let pushed = BlockPos { x: 5, y: 64, z: 6 };
    dig.set(piston, state("minecraft:piston", &[("facing", "east")]));
    dig.set(pushed, state("minecraft:shulker_box", &[]));
    load_fields(
        &mut dig.world,
        pushed,
        vec![field("CustomName", text("Pushed"))],
    );
    set_items(&mut dig.world, pushed, &[(13, stone(9))]);
    dig.set(
        BlockPos { x: 3, y: 64, z: 6 },
        block_state_id("minecraft:redstone_block"),
    );
    for _ in 0..4 {
        dig.world.tick(dig.receiver.drain());
    }

    assert_ne!(
        dig.block(pushed),
        Some(state("minecraft:shulker_box", &[])),
        "the piston extended into the box"
    );
    let drops = dropped(&dig.world);
    assert_eq!(drops.len(), 1, "{drops:#?}");
    let shulker = only_drop_of(&drops, "minecraft:shulker_box");
    assert_eq!(
        shulker.components.get("minecraft:custom_name"),
        Some(&text("Pushed"))
    );
    assert_eq!(
        container_slots(&shulker.components["minecraft:container"]),
        vec![(13, "minecraft:stone".to_string(), 9)]
    );
}

/// `blocks/beehive` copies `bees` under silk touch
/// (`BeehiveBlockEntity.java:318-321`), which also keeps the occupants in.
#[test]
fn a_silk_touch_dig_keeps_the_bees_in_the_dropped_beehive() {
    let mut dig = Dig::new(GameType::Survival, false);
    let pos = BlockPos { x: 2, y: 64, z: 2 };
    dig.set(pos, state("minecraft:beehive", &[("honey_level", "5")]));
    let bees = NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
        field(
            "entity_data",
            NbtSummaryValue::Compound(vec![field("id", text("minecraft:bee"))]),
        ),
        field("ticks_in_hive", NbtSummaryValue::Int(37)),
        field("min_ticks_in_hive", NbtSummaryValue::Int(600)),
    ])]);
    load_fields(&mut dig.world, pos, vec![field("bees", bees.clone())]);
    let silk_axe = dig.enchanted(
        fast_tool("minecraft:diamond_axe", &["minecraft:beehive"], true),
        "minecraft:silk_touch",
    );
    dig.hold(silk_axe);

    let before = dig
        .world
        .level_block_entity_for_test(DIM, pos)
        .unwrap()
        .beehive()
        .unwrap()
        .release_entries()
        .remove(0)
        .unwrap();
    dig.dig(pos);
    let drops = dropped(&dig.world);
    assert_eq!(drops.len(), 1, "{drops:#?}");
    let hive = only_drop_of(&drops, "minecraft:beehive");
    let Some(NbtSummaryValue::List(dropped_bees)) = hive.components.get("minecraft:bees") else {
        panic!("the hive keeps its occupants");
    };
    let NbtSummaryValue::Compound(dropped_bee) = &dropped_bees[0] else {
        panic!("a saved occupant");
    };
    let ticks = &dropped_bee
        .iter()
        .find(|entry| entry.name == "ticks_in_hive")
        .unwrap()
        .value;
    assert!(bbb_protocol::nbt_summary::codec_int_value(ticks).unwrap() >= before.ticks_in_hive);
    let NbtSummaryValue::Compound(mut expected) = before.saved.clone() else {
        unreachable!();
    };
    expected
        .iter_mut()
        .find(|entry| entry.name == "ticks_in_hive")
        .unwrap()
        .value = ticks.clone();
    assert_eq!(dropped_bees, &vec![NbtSummaryValue::Compound(expected)]);
    assert_eq!(
        hive.components.get("minecraft:block_state"),
        Some(&honey_state(5))
    );
    assert!(released_bees(&dig.world).is_empty());
    let id = dig.world.stored_items_for_test(DIM)[0].0;
    let stack = dig.world.stored_item_stack_for_test(DIM, id).unwrap();
    *dig.world
        .player_inventory_for_test(MINER)
        .selected_item_mut() = stack;
    let support = BlockPos {
        y: pos.y - 1,
        ..pos
    };
    dig.world.set_block(
        DIM,
        support,
        state("minecraft:stone", &[]),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    dig.place(support);
    assert_eq!(
        crate::block_behavior::beehive::honey_level(dig.block(pos).unwrap()),
        5
    );
    let placed = dig
        .world
        .level_block_entity_for_test(DIM, pos)
        .unwrap()
        .beehive()
        .unwrap()
        .release_entries();
    assert_eq!(placed.len(), 1);
    let placed = placed.into_iter().next().unwrap().unwrap();
    assert_eq!(placed.entity_data, before.entity_data);
    assert_eq!(placed.min_ticks_in_hive, before.min_ticks_in_hive);
    assert!(placed.ticks_in_hive >= before.ticks_in_hive);
}

fn honey_state(honey: i32) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![field("honey_level", text(&honey.to_string()))])
}

fn occupy(dig: &mut Dig, pos: BlockPos) {
    dig.world
        .level_block_entity_for_test(DIM, pos)
        .unwrap()
        .beehive_mut()
        .unwrap()
        .store_bee(37);
}

fn released_bees(world: &ServerWorld) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID
        })
        .collect()
}

#[test]
fn mining_hives_releases_bees_and_targets_nearby_players_even_with_smoke() {
    for smoke in [false, true] {
        let mut dig = Dig::new(GameType::Survival, false);
        let pos = BlockPos { x: 2, y: 64, z: 2 };
        dig.set(pos, state("minecraft:bee_nest", &[]));
        occupy(&mut dig, pos);
        if smoke {
            dig.set(
                BlockPos { y: 63, ..pos },
                state("minecraft:campfire", &[("lit", "true")]),
            );
        }
        dig.hold(fast_tool(
            "minecraft:diamond_axe",
            &["minecraft:bee_nest"],
            true,
        ));
        dig.dig(pos);
        let bees = released_bees(&dig.world);
        assert_eq!(bees.len(), 1);
        dig.world.tick(dig.receiver.drain());
        let crate::entity::EntityData::GeneralLiving(data) = dig
            .world
            .stored_entity_for_test(DIM, bees[0])
            .unwrap()
            .data()
        else {
            panic!("a bee");
        };
        let target = data.bee_retained().unwrap().persistent_anger().1;
        // `BeehiveBlockEntity.emptyAllLivingFromHive` (`:125-136`): a sedated
        // hive keeps the nearby player's bees out for 400 ticks.
        let Some(NbtSummaryValue::Compound(saved)) =
            dig.world.stored_entity_command_data_for_test(DIM, bees[0])
        else {
            panic!("a bee saves a compound");
        };
        let stay_out = saved
            .iter()
            .filter(|entry| entry.name == "CannotEnterHiveTicks")
            .find_map(|entry| match entry.value {
                NbtSummaryValue::Int(ticks) => Some(ticks),
                _ => None,
            });
        assert_eq!(
            stay_out.is_some_and(|ticks| ticks > 390),
            smoke,
            "{stay_out:?}"
        );
        if smoke {
            assert!(
                matches!(target, Some(uuid) if uuid == uuid::Uuid::from_u128(1) || uuid == uuid::Uuid::from_u128(2))
            );
        } else {
            assert_eq!(target, Some(uuid::Uuid::from_u128(1)));
        }
    }
}

#[test]
fn a_survival_hive_dig_clears_a_comparator_reading_honey_through_stone() {
    let mut dig = Dig::new(GameType::Survival, false);
    let hive = BlockPos { x: 2, y: 64, z: 2 };
    let conductor = BlockPos { x: 3, ..hive };
    let comparator = BlockPos { x: 4, ..hive };
    dig.set(hive, state("minecraft:beehive", &[("honey_level", "5")]));
    occupy(&mut dig, hive);
    dig.set(conductor, state("minecraft:stone", &[]));
    dig.world.set_block(
        DIM,
        BlockPos {
            y: comparator.y - 1,
            ..comparator
        },
        state("minecraft:stone", &[]),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    dig.set(
        comparator,
        state("minecraft:comparator", &[("facing", "west")]),
    );
    dig.set(
        BlockPos {
            y: comparator.y + 1,
            ..comparator
        },
        state("minecraft:stone", &[]),
    );
    for _ in 0..8 {
        dig.world.tick(dig.receiver.drain());
    }
    let output = |dig: &mut Dig| {
        dig.world
            .level_block_entity_for_test(DIM, comparator)
            .unwrap()
            .comparator()
            .unwrap()
            .output_signal()
    };
    assert_eq!(output(&mut dig), 5);
    dig.hold(fast_tool(
        "minecraft:diamond_axe",
        &["minecraft:beehive"],
        true,
    ));
    dig.dig(hive);
    for _ in 0..8 {
        dig.world.tick(dig.receiver.drain());
    }
    assert_eq!(dig.block(hive), Some(state("minecraft:air", &[])));
    assert_eq!(output(&mut dig), 0);
}

#[test]
fn mining_hive_spawn_prevention_uses_the_live_enchantment_tag() {
    let mut dig = Dig::new(GameType::Survival, false);
    dig.world.install_level_registry_tags(Arc::new(
        crate::registry_tags::RegistryTagSnapshot::new([
            (
                "minecraft:enchantment".into(),
                "minecraft:prevents_bee_spawns_when_mining".into(),
                vec!["minecraft:fortune".into()],
            ),
            (
                "minecraft:entity_type".into(),
                "minecraft:beehive_inhabitors".into(),
                vec!["minecraft:bee".into()],
            ),
        ]),
    ));
    for (x, enchantment, released) in [(2, "minecraft:fortune", 0), (5, "minecraft:silk_touch", 1)]
    {
        let pos = BlockPos { x, y: 64, z: 2 };
        dig.set(pos, state("minecraft:beehive", &[]));
        occupy(&mut dig, pos);
        dig.hold(dig.enchanted(
            fast_tool("minecraft:diamond_axe", &["minecraft:beehive"], true),
            enchantment,
        ));
        dig.dig(pos);
        assert_eq!(released_bees(&dig.world).len(), released);
    }
}

#[test]
fn creative_hive_drops_keep_bees_and_honey_only_when_block_drops_are_enabled() {
    let mut dig = Dig::new(GameType::Creative, true);
    let pos = BlockPos { x: 2, y: 64, z: 2 };
    for (honey, occupied, drops) in [(0, false, 0), (0, true, 1), (4, false, 1)] {
        dig.set(
            pos,
            state("minecraft:beehive", &[("honey_level", &honey.to_string())]),
        );
        if occupied {
            occupy(&mut dig, pos);
        }
        let before: Vec<_> = dig
            .world
            .stored_items_for_test(DIM)
            .into_iter()
            .map(|entry| entry.0)
            .collect();
        dig.dig(pos);
        let fresh: Vec<_> = dig
            .world
            .stored_items_for_test(DIM)
            .into_iter()
            .filter(|entry| !before.contains(&entry.0))
            .collect();
        assert_eq!(fresh.len(), drops);
        assert!(released_bees(&dig.world).is_empty());
        if let Some(drop) = fresh.first() {
            // The drop has already taken its first physics step.
            assert!((drop.1.x - 2.0).abs() < 0.125 && (drop.1.z - 2.0).abs() < 0.125);
            assert!((64.0..64.25).contains(&drop.1.y));
            let stack = dig.world.stored_item_stack_for_test(DIM, drop.0).unwrap();
            let components: BTreeMap<_, _> =
                crate::tests::saved_item_components(&dig.world, &stack)
                    .into_iter()
                    .map(|entry| (entry.name.to_jdk_string(), entry.value))
                    .collect();
            assert_eq!(
                components.get("minecraft:block_state"),
                Some(&honey_state(honey))
            );
            let NbtSummaryValue::List(bees) = &components["minecraft:bees"] else {
                panic!("bees component");
            };
            assert_eq!(bees.len(), usize::from(occupied));
        }
    }
    dig.command("gamerule block_drops false");
    dig.set(pos, state("minecraft:beehive", &[("honey_level", "5")]));
    occupy(&mut dig, pos);
    let count = dig.world.stored_items_for_test(DIM).len();
    dig.dig(pos);
    assert_eq!(dig.world.stored_items_for_test(DIM).len(), count);
}

#[test]
fn hive_destroy_advancement_matches_player_block_tool_and_remaining_bees_once() {
    use crate::tests::parse_advancement;
    use bbb_server_datapack::advancement::{AdvancementElement, AdvancementSet};
    use serde_json::json;
    for (mode, silk, remaining) in [
        (GameType::Survival, false, 0),
        (GameType::Survival, true, 1),
        (GameType::Creative, false, 0),
    ] {
        let mut config = datapack_config();
        config.game_type = mode;
        let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
        let base = json!({"player":{"type":"minecraft:player"}, "block":"minecraft:beehive", "item":{"items":"minecraft:diamond_axe"}, "num_bees_inside":remaining});
        let mut definitions = vec![("match", base.clone(), 7)];
        for (name, field, value) in [
            ("player", "player", json!({"type":"minecraft:cow"})),
            ("block", "block", json!("minecraft:bee_nest")),
            ("item", "item", json!({"items":"minecraft:wooden_axe"})),
            ("bees", "num_bees_inside", json!(1 - remaining)),
        ] {
            let mut condition = base.clone();
            condition[field] = value;
            definitions.push((name, condition, 100));
        }
        startup.advancements = AdvancementSet {
            advancements: definitions.into_iter().map(|(name, conditions, reward)| AdvancementElement {
                id: format!("test:{name}"), source_pack: "test".into(),
                advancement: parse_advancement(&json!({"criteria":{"mined":{"trigger":"minecraft:bee_nest_destroyed", "conditions":conditions}}, "rewards":{"experience":reward}})).unwrap(),
            }).collect(), warnings: Vec::new(),
        };
        let mut dig = Dig::with_config(config, false);
        let tool = fast_tool("minecraft:diamond_axe", &["minecraft:beehive"], true);
        let tool = if silk {
            dig.enchanted(tool, "minecraft:silk_touch")
        } else {
            tool
        };
        dig.hold(tool);
        for x in [2, 5] {
            let pos = BlockPos { x, y: 64, z: 2 };
            dig.set(pos, state("minecraft:beehive", &[]));
            occupy(&mut dig, pos);
            dig.dig(pos);
            assert_eq!(
                dig.world.experience_total_for_test(MINER),
                if mode == GameType::Creative { 0 } else { 7 }
            );
        }
    }
}

fn sherds() -> NbtSummaryValue {
    NbtSummaryValue::List(
        [
            "minecraft:angler_pottery_sherd",
            "minecraft:brick",
            "minecraft:brick",
            "minecraft:arms_up_pottery_sherd",
        ]
        .into_iter()
        .map(text)
        .collect(),
    )
}

/// `blocks/decorated_pot`: an intact pot copies `pot_decorations`
/// (`DecoratedPotBlockEntity.java:112-116`), which a plain pot's prototype
/// already holds, and a cracked one emits `DecoratedPotBlock.getDrops`'
/// dynamic `sherds` (`DecoratedPotBlock.java:181-192`) face by face.
#[test]
fn decorated_pots_drop_their_decorations_whole_or_as_sherds() {
    let mut dig = Dig::new(GameType::Survival, false);
    let intact = BlockPos { x: 2, y: 64, z: 2 };
    let plain = BlockPos { x: 5, y: 64, z: 2 };
    let cracked = BlockPos { x: 8, y: 64, z: 2 };
    for pos in [intact, plain, cracked] {
        dig.set(pos, state("minecraft:decorated_pot", &[]));
    }
    load_fields(&mut dig.world, intact, vec![field("sherds", sherds())]);
    load_fields(&mut dig.world, cracked, vec![field("sherds", sherds())]);

    dig.dig(intact);
    dig.dig(plain);
    let drops = dropped(&dig.world);
    assert_eq!(drops.len(), 2, "{drops:#?}");
    assert!(drops
        .iter()
        .all(|drop| drop.item == "minecraft:decorated_pot"));
    let decorated: Vec<_> = drops
        .iter()
        .map(|drop| drop.components.get("minecraft:pot_decorations"))
        .collect();
    assert!(decorated.contains(&Some(&sherds())), "{drops:#?}");
    assert!(decorated.contains(&None), "{drops:#?}");

    dig.hold(item("minecraft:diamond_pickaxe"));
    dig.dig(cracked);
    let drops = dropped(&dig.world);
    let shattered = totals(
        drops
            .iter()
            .filter(|drop| drop.item != "minecraft:decorated_pot")
            .map(|drop| (drop.item.as_str(), drop.count)),
    );
    assert_eq!(
        shattered,
        BTreeMap::from([
            ("minecraft:angler_pottery_sherd".to_string(), 1),
            ("minecraft:arms_up_pottery_sherd".to_string(), 1),
            ("minecraft:brick".to_string(), 2),
        ])
    );
}

fn steve() -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![field("name", text("Steve"))])
}

/// `blocks/player_head` copies `profile` and `custom_name`
/// (`SkullBlockEntity.java:90-95`).
#[test]
fn a_survival_dig_drops_a_player_head_with_its_profile() {
    let mut dig = Dig::new(GameType::Survival, false);
    let pos = BlockPos { x: 2, y: 64, z: 2 };
    dig.set(pos, state("minecraft:player_head", &[]));
    load_fields(
        &mut dig.world,
        pos,
        vec![
            field("profile", steve()),
            field("custom_name", text("Trophy")),
        ],
    );
    dig.hold(fast_tool(
        "minecraft:diamond_pickaxe",
        &["minecraft:player_head"],
        true,
    ));

    dig.dig(pos);
    let drops = dropped(&dig.world);
    assert_eq!(drops.len(), 1, "{drops:#?}");
    let head = only_drop_of(&drops, "minecraft:player_head");
    assert_eq!(head.components.get("minecraft:profile"), Some(&steve()));
    assert_eq!(
        head.components.get("minecraft:custom_name"),
        Some(&text("Trophy"))
    );
}
