use super::*;

use super::helpers::entity_command_record as record;
use super::nbt as entry;
use crate::tests::entities::restore_entity_writes;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_DONKEY_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const POSITION: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};

fn stack(id: &str, count: i32) -> Vec<NbtSummaryEntry> {
    vec![
        entry("id", NbtSummaryValue::string(id)),
        entry("count", NbtSummaryValue::Int(count)),
    ]
}

fn slotted(slot: i8, id: &str, count: i32) -> NbtSummaryValue {
    let mut fields = vec![entry("Slot", NbtSummaryValue::Byte(slot))];
    fields.extend(stack(id, count));
    NbtSummaryValue::Compound(fields)
}

fn spawn(world: &mut ServerWorld, kind: i32) -> (i32, uuid::Uuid) {
    helpers::spawn_living_at(world, kind, POSITION)
}

fn load(world: &mut ServerWorld, id: i32, entries: &[NbtSummaryEntry]) {
    let uuid = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
        .unwrap()
        .uuid();
    let mut complete = record(world, uuid);
    for update in entries {
        if let Some(existing) = complete.iter_mut().find(|entry| entry.name == update.name) {
            existing.value = update.value.clone();
        } else {
            complete.push(update.clone());
        }
    }
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, id, &complete);
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, id)
        .unwrap()
        .set_pos(POSITION);
}

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> &'a NbtSummaryValue {
    helpers::nbt_field(entries, name).unwrap()
}

fn kill(world: &mut ServerWorld, id: i32) {
    crate::entity::living::set_health(
        world
            .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap(),
        0.1,
    );
    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            Java26BuiltinDimension::Overworld,
            id,
            "minecraft:generic",
            40.0,
            None,
        )
        .unwrap_or_else(|| panic!("hurt refused for {id}"));
    assert!(outcome.applied && outcome.succeeded);
}

#[test]
fn allay_and_chested_mount_cargo_survive_region_reload_and_drop_in_override_order() {
    let mut world = spawn_pinned_world(datapack_config());
    let (allay, allay_uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_ALLAY_ID);
    let (donkey, donkey_uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_DONKEY_ID);
    let (llama, llama_uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_LLAMA_ID);

    let mut sword = stack("minecraft:diamond_sword", 1);
    sword.push(entry(
        "components",
        NbtSummaryValue::Compound(vec![
            entry("minecraft:damage", NbtSummaryValue::Int(7)),
            entry(
                "minecraft:enchantments",
                NbtSummaryValue::Compound(vec![entry(
                    "minecraft:vanishing_curse",
                    NbtSummaryValue::Int(1),
                )]),
            ),
        ]),
    ));
    let reloaded_sword = sword.clone();
    let allay_inventory = NbtSummaryValue::List(vec![NbtSummaryValue::Compound(sword.clone())]);
    let reloaded_allay_inventory =
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(reloaded_sword.clone())]);
    load(
        &mut world,
        allay,
        &[entry("Inventory", allay_inventory.clone())],
    );
    let cargo = |sword: Vec<NbtSummaryEntry>| {
        let mut forbidden_cargo = vec![entry("Slot", NbtSummaryValue::Byte(7))];
        forbidden_cargo.extend(sword);
        NbtSummaryValue::List(vec![
            slotted(0, "minecraft:stone", 2),
            NbtSummaryValue::Compound(forbidden_cargo),
            slotted(14, "minecraft:cake", 1),
        ])
    };
    let donkey_items = cargo(sword);
    let reloaded_donkey_items = cargo(reloaded_sword);
    load(
        &mut world,
        donkey,
        &[
            entry("ChestedHorse", NbtSummaryValue::Byte(1)),
            entry("Items", donkey_items.clone()),
        ],
    );
    let llama_items = NbtSummaryValue::List(vec![slotted(5, "minecraft:apple", 3)]);
    load(
        &mut world,
        llama,
        &[
            entry("ChestedHorse", NbtSummaryValue::Byte(1)),
            entry("Strength", NbtSummaryValue::Int(2)),
            entry("Items", llama_items.clone()),
        ],
    );

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let written = saved_entities(&request);
    assert!(written.iter().any(|record| {
        record
            .iter()
            .any(|e| e.name == "Inventory" && e.value == allay_inventory)
    }));
    assert!(written.iter().any(|record| {
        record
            .iter()
            .any(|e| e.name == "Items" && e.value == donkey_items)
    }));
    assert!(written.iter().any(|record| {
        record
            .iter()
            .any(|e| e.name == "Items" && e.value == llama_items)
    }));

    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(datapack_config(), &candidate);
    assert_eq!(
        field(&record(&restored, allay_uuid), "Inventory"),
        &reloaded_allay_inventory
    );
    assert_eq!(
        field(&record(&restored, donkey_uuid), "Items"),
        &reloaded_donkey_items
    );
    assert_eq!(field(&record(&restored, llama_uuid), "Items"), &llama_items);

    for (uuid, expected) in [
        (allay_uuid, vec![("minecraft:diamond_sword", 1, Some(7))]),
        (
            donkey_uuid,
            vec![
                ("minecraft:stone", 2, None),
                ("minecraft:cake", 1, None),
                ("minecraft:chest", 1, None),
            ],
        ),
        (
            llama_uuid,
            vec![("minecraft:apple", 3, None), ("minecraft:chest", 1, None)],
        ),
    ] {
        let id = restored
            .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, uuid)
            .unwrap();
        let before = restored
            .stored_items_for_test(Java26BuiltinDimension::Overworld)
            .len();
        kill(&mut restored, id);
        let items = restored.stored_items_for_test(Java26BuiltinDimension::Overworld);
        let actual: Vec<_> = items[before..]
            .iter()
            .map(|item| (item.2.as_str(), item.3, item.5))
            .collect();
        assert!(
            actual.ends_with(&expected),
            "cargo follows ordinary loot: {actual:?}"
        );
    }
}
