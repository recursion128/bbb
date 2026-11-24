//! Every 26.1 `chests/*` table unpacked into a real chest: the draws are this
//! server's own, so each fill is held to the items the table's entries name.

use super::*;

use crate::block_entity::BlockEntity;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/data/chest-loot-fills-26.1.json"
    )))
    .expect("the fixture is JSON")
}

/// Compounds compare as maps: `CompoundTag` keeps no order, and
/// `ItemEnchantments` iterates an identity-hashed map.
fn canonical(value: &NbtSummaryValue) -> NbtSummaryValue {
    match value {
        NbtSummaryValue::Compound(entries) => {
            let mut entries: Vec<NbtSummaryEntry> = entries
                .iter()
                .map(|entry| NbtSummaryEntry {
                    name: entry.name.clone(),
                    value: canonical(&entry.value),
                })
                .collect();
            entries.sort_by_key(|entry| entry.name.to_jdk_string());
            NbtSummaryValue::Compound(entries)
        }
        NbtSummaryValue::List(values) => {
            NbtSummaryValue::List(values.iter().map(canonical).collect())
        }
        other => other.clone(),
    }
}

/// `ContainerHelper.saveAllItems` (`ContainerHelper.java:23-38`): one
/// `ItemStackWithSlot` per occupied slot, its stack fields beside `Slot`.
fn saved_slots(record: &[NbtSummaryEntry]) -> BTreeMap<i32, NbtSummaryValue> {
    let Some(NbtSummaryValue::List(items)) = record
        .iter()
        .find(|entry| entry.name == "Items")
        .map(|entry| &entry.value)
    else {
        return BTreeMap::new();
    };
    items
        .iter()
        .map(|item| {
            let NbtSummaryValue::Compound(fields) = item else {
                panic!("a saved slot is a compound: {item:?}");
            };
            let slot = fields
                .iter()
                .find_map(|entry| match (&entry.value, entry.name == "Slot") {
                    (NbtSummaryValue::Byte(slot), true) => Some(i32::from(*slot)),
                    _ => None,
                })
                .expect("a slot index");
            let stack = fields
                .iter()
                .filter(|entry| entry.name != "Slot")
                .cloned()
                .collect();
            (slot, canonical(&NbtSummaryValue::Compound(stack)))
        })
        .collect()
}

/// Every `minecraft:item` entry name in the packaged `table`.
fn table_items(table: &str) -> std::collections::BTreeSet<String> {
    fn walk(value: &serde_json::Value, names: &mut std::collections::BTreeSet<String>) {
        match value {
            serde_json::Value::Object(map) => {
                let entry = map.get("type").and_then(serde_json::Value::as_str);
                if entry == Some("minecraft:item") {
                    if let Some(name) = map.get("name").and_then(serde_json::Value::as_str) {
                        names.insert(name.to_string());
                    }
                }
                if entry == Some("minecraft:loot_table") {
                    if let Some(table) = map.get("value").and_then(serde_json::Value::as_str) {
                        names.extend(table_items(table));
                    }
                }
                map.values().for_each(|value| walk(value, names));
            }
            serde_json::Value::Array(values) => values.iter().for_each(|value| walk(value, names)),
            _ => {}
        }
    }
    let snapshot = bbb_server_datapack::vanilla_snapshot::VanillaDatapackSnapshot::embedded()
        .expect("the embedded snapshot loads");
    let path = table.strip_prefix("minecraft:").expect("a vanilla table");
    let bytes = snapshot
        .read_path(&format!("loot_table/{path}.json"))
        .unwrap_or_else(|| panic!("the built-in pack carries {table}"));
    let mut names = std::collections::BTreeSet::new();
    walk(
        &serde_json::from_slice(bytes).expect("a table is JSON"),
        &mut names,
    );
    // Enchanting a book turns it into an enchanted book.
    if names.contains("minecraft:book") {
        names.insert("minecraft:enchanted_book".to_string());
    }
    names
}

/// A saved stack's `id`.
fn item_id(stack: &NbtSummaryValue) -> String {
    let NbtSummaryValue::Compound(fields) = stack else {
        panic!("a stack is a compound: {stack:?}");
    };
    fields
        .iter()
        .find_map(|entry| match (&entry.value, entry.name == "id") {
            (NbtSummaryValue::String(id), true) => Some(id.try_to_string().unwrap()),
            _ => None,
        })
        .expect("a stack names its item")
}

#[test]
fn every_chest_table_fills_a_chest_with_the_items_it_names() {
    let oracle = oracle();
    let origin = oracle["origin"].as_array().expect("an origin");
    let pos = BlockPos {
        x: origin[0].as_f64().unwrap().floor() as i32,
        y: origin[1].as_f64().unwrap().floor() as i32,
        z: origin[2].as_f64().unwrap().floor() as i32,
    };
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "looter", 1);
    world.tick(receiver.drain());
    let chest = vanilla_block_default_state_id(
        vanilla_block_type_for_resource_id("minecraft:chest")
            .expect("a chest")
            .id,
    )
    .expect("a default state");
    assert!(world.set_block(DIM, pos, chest, SET_BLOCK_UPDATE_ALL));

    let tables = oracle["tables"].as_object().expect("the tables");
    assert_eq!(tables.len(), 56);
    let registered: Vec<_> = world
        .datapacks()
        .expect("datapack generation")
        .loot
        .registries
        .iter()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .expect("the loot table registry")
        .elements
        .iter()
        .map(|element| element.id.clone())
        .filter(|id| id.starts_with("minecraft:chests/"))
        .collect();
    assert_eq!(registered, tables.keys().cloned().collect::<Vec<_>>());

    let mut mismatches = Vec::new();
    for (table, _) in tables {
        let named = table_items(table);
        for seed in oracle["seeds"].as_array().expect("the seeds") {
            let seed = seed.as_str().expect("a seed");
            let mut record = world
                .block_entity_save_for_test(DIM, pos)
                .expect("the chest record saves");
            record.retain(|entry| {
                !["Items", "LootTable", "LootTableSeed"]
                    .contains(&entry.name.to_jdk_string().as_str())
            });
            record.push(NbtSummaryEntry {
                name: "LootTable".into(),
                value: NbtSummaryValue::String(table.as_str().into()),
            });
            record.push(NbtSummaryEntry {
                name: "LootTableSeed".into(),
                value: NbtSummaryValue::Long(seed.parse().expect("an i64 seed")),
            });
            *world
                .level_block_entity_for_test(DIM, pos)
                .expect("the chest") = BlockEntity::load_static(pos, chest, &record)
                .expect("the record loads")
                .expect("a chest record names a block entity");

            world.unpack_container_loot_for_test(DIM, pos);
            let actual = saved_slots(
                &world
                    .block_entity_save_for_test(DIM, pos)
                    .unwrap_or_else(|| panic!("{table} {seed}: the chest saves")),
            );
            let foreign: Vec<_> = actual
                .iter()
                .filter(|(slot, stack)| {
                    !(0..27).contains(*slot) || !named.contains(&item_id(stack))
                })
                .collect();
            if !foreign.is_empty() {
                mismatches.push(format!("{table} {seed}: {foreign:?}"));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} fills hold stacks the table never names:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
