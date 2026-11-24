use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::{
    vanilla_entity_type_id_for_resource_id, VANILLA_ENTITY_TYPE_ITEM_ID,
    VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;

use crate::entity::chest_boat::{self, CONTAINER_SIZE};

use crate::entity::{spawn_chest_boat, ChestBoatData};

const CHEST_BOAT_TYPES: [&str; 10] = [
    "minecraft:acacia_chest_boat",
    "minecraft:bamboo_chest_raft",
    "minecraft:birch_chest_boat",
    "minecraft:cherry_chest_boat",
    "minecraft:dark_oak_chest_boat",
    "minecraft:jungle_chest_boat",
    "minecraft:mangrove_chest_boat",
    "minecraft:oak_chest_boat",
    "minecraft:pale_oak_chest_boat",
    "minecraft:spruce_chest_boat",
];

fn spawn_oak(store: &mut EntityStore) -> i32 {
    spawn_chest_boat(
        store,
        VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        90.0,
    )
    .expect("oak chest boat constructor")
}

fn data(store: &EntityStore, id: i32) -> &ChestBoatData {
    chest_boat::chest_boat(store.get(id).expect("entity is live").data())
        .expect("entity is a chest boat")
}

#[test]
fn every_chest_boat_type_has_exact_identity_dimensions_and_defaults() {
    for resource_id in CHEST_BOAT_TYPES {
        let type_id = vanilla_entity_type_id_for_resource_id(resource_id).expect("built-in type");
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = spawn_chest_boat(
            &mut store,
            type_id,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            123.5,
        )
        .expect("chest boat constructor");
        let entity = store.get(id).expect("entity is live");
        assert_eq!(entity.entity_type_id(), type_id);
        assert_eq!(entity.data().entity_resource_id(), resource_id);
        assert_eq!((entity.bb_width(), entity.bb_height()), (1.375, 0.5625));
        assert_eq!((entity.y_rot(), entity.x_rot()), (123.5, 0.0));
        assert_eq!(entity.gravity(), crate::entity::boat::DEFAULT_GRAVITY);
        assert!(entity.is_pickable());
        let data = chest_boat::chest_boat(entity.data()).expect("chest boat data");
        assert!((0..CONTAINER_SIZE).all(|slot| data.item(slot).is_none()));
        assert!(crate::entity::boat::synched_values(entity, data.boat())
            .iter()
            .all(|value| value.is_default));
    }
}

#[test]
fn slot_access_is_fixed_to_twenty_seven_and_normalizes_empty_stacks() {
    let mut data = ChestBoatData::new();
    let stone = plain_stack("minecraft:stone", 3).expect("stone stack");
    data.set_item(26, Some(stone.clone())).unwrap();
    assert_eq!(data.item(26).as_deref(), Some(&stone));
    assert!(data.set_item(27, Some(stone)).is_err());
    data.set_item(26, crate::entity::item::empty_stack())
        .unwrap();
    assert!(data.item(26).is_none());
    assert!(data.remove_item(27).is_err());
}

#[test]
fn chest_boats_use_the_existing_passengerless_boat_tick() {
    for resource_id in CHEST_BOAT_TYPES {
        let type_id = vanilla_entity_type_id_for_resource_id(resource_id).unwrap();
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = spawn_chest_boat(
            &mut store,
            type_id,
            Vec3d {
                x: 0.5,
                y: 70.0,
                z: 0.5,
            },
            0.0,
        )
        .unwrap();
        let mut level = NativeLevel::new();
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        let entity = store.get(id).expect("entity is live");
        assert_eq!(
            data(&store, id).boat().status(),
            Some(crate::entity::BoatStatus::InAir)
        );
        assert_eq!(
            entity.delta_movement().y,
            -crate::entity::boat::DEFAULT_GRAVITY
        );
        assert_eq!(
            entity.position().y,
            70.0 - crate::entity::boat::DEFAULT_GRAVITY
        );
    }
}

#[test]
fn inventory_identity_and_components_round_trip_through_entity_regions() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = spawn_oak(&mut store);
    let stack = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string("minecraft:diamond"),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(7),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "minecraft:custom_data".into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "answer".into(),
                        value: NbtSummaryValue::Int(42),
                    }]),
                }]),
            },
        ]),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .expect("valid component stack");
    chest_boat::chest_boat_mut(&mut store.get_mut(id).unwrap().data)
        .unwrap()
        .set_item(26, Some(stack.clone()))
        .unwrap();

    let document = write_entity_chunk(&store, (0, 0)).expect("canonical chest boat save");
    let items = document
        .entities()
        .next()
        .unwrap()
        .raw()
        .iter()
        .find(|entry| entry.name == "Items")
        .expect("inventory branch is always written");
    assert!(matches!(&items.value, NbtSummaryValue::List(values) if values.len() == 1));
    let mut restored = EntityStore::new(1, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut restored, &document).expect("inventory branch restores");
    assert_eq!(data(&restored, ids[0]).item(26).as_deref(), Some(&stack));
    assert_eq!(
        restored.get(ids[0]).unwrap().uuid(),
        store.get(id).unwrap().uuid()
    );
}

#[test]
/// `ContainerEntity.readChestVehicleSaveData` (`ContainerEntity.java:69-77`):
/// a pending table is kept and the saved inventory is not read.
fn a_loot_table_save_restores_the_pending_table_without_items() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    spawn_oak(&mut store);
    let document = write_entity_chunk(&store, (0, 0)).expect("canonical inventory save");
    let mut raw = document.into_raw();
    let entities = raw
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "Entities")
        .expect("entity list");
    let NbtSummaryValue::List(records) = &mut entities.value else {
        unreachable!()
    };
    let NbtSummaryValue::Compound(fields) = &mut records[0] else {
        unreachable!()
    };
    fields.retain(|entry| entry.name != "Items");
    fields.push(NbtSummaryEntry {
        name: "LootTable".into(),
        value: NbtSummaryValue::string("minecraft:chests/simple_dungeon"),
    });
    fields.push(NbtSummaryEntry {
        name: "LootTableSeed".into(),
        value: NbtSummaryValue::Long(7),
    });
    let document = Java26EntityChunkData::parse(raw, 0, 0).expect("valid loot-table save");
    let mut target = EntityStore::new(1, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut target, &document).expect("valid loot-table save");
    assert_eq!(ids.len(), 1);
    let restored = data(&target, ids[0]);
    assert_eq!(
        restored.loot_table_id().as_deref(),
        Some("minecraft:chests/simple_dungeon")
    );
    assert_eq!(restored.loot_table_seed(), 7);
    assert!((0..CONTAINER_SIZE).all(|slot| restored.item(slot).is_none()));
}

fn stock(store: &mut EntityStore, id: i32, count: i32) {
    let stack = plain_stack("minecraft:stone", count).expect("stone stack");
    chest_boat::chest_boat_mut(&mut store.get_mut(id).unwrap().data)
        .unwrap()
        .set_item(0, Some(stack))
        .unwrap();
}

fn native_chest_boat(count: i32) -> (NativeLevel, i32) {
    let mut level = NativeLevel::new();
    let mut data = ChestBoatData::new();
    data.set_item(0, plain_stack("minecraft:stone", count))
        .unwrap();
    let (id, _) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::ChestBoat(data),
    );
    (level, id)
}

#[test]
fn destruction_drops_all_cargo_and_the_vehicle_item() {
    let (mut level, id) = native_chest_boat(64);
    let target_uuid = level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .uuid();
    let (cause, cause_uuid) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
        Vec3d {
            x: 8.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::Minecart(crate::entity::MinecartData::new()),
    );
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        &format!("damage {target_uuid} 5 minecraft:generic by {cause_uuid}"),
    );
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DAMAGE
                && event.recorded.source_entity_id == Some(cause)
                && event.recorded.pos == pos(0, 64, 0)
        ));

    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    let items = level.world.stored_items_for_test(DIMENSION);
    assert_eq!(
        items
            .iter()
            .filter(|item| item.2 == "minecraft:stone")
            .map(|item| item.3)
            .sum::<i32>(),
        64
    );
    assert!(items
        .iter()
        .any(|item| item.2 == "minecraft:oak_chest_boat" && item.3 == 1));
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DIE
                && event.recorded.source_entity_id == Some(id)
                && event.recorded.pos == pos(0, 64, 0)
        ));
}

#[test]
fn entity_drops_only_suppresses_the_vehicle_item() {
    let (mut level, id) = native_chest_boat(31);
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "gamerule minecraft:entity_drops false",
    );
    level
        .world
        .hurt_stored_entity_by_command_for_test(DIMENSION, id, "minecraft:generic", 5.0, None)
        .unwrap();
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    let items = level.world.stored_items_for_test(DIMENSION);
    assert_eq!(
        items
            .iter()
            .filter(|item| item.2 == "minecraft:stone")
            .map(|item| item.3)
            .sum::<i32>(),
        31
    );
    assert!(!items
        .iter()
        .any(|item| item.2 == "minecraft:oak_chest_boat"));
}

#[test]
fn creative_discard_still_drops_cargo() {
    let (mut level, id) = native_chest_boat(1);
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "alice", 1);
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "gamemode creative alice",
    );
    let uuid = level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .uuid();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        2,
        &format!("damage {uuid} 5 minecraft:generic by alice"),
    );
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    let items = level.world.stored_items_for_test(DIMENSION);
    assert!(items
        .iter()
        .any(|item| item.2 == "minecraft:stone" && item.3 == 1));
    assert!(!items
        .iter()
        .any(|item| item.2 == "minecraft:oak_chest_boat"));
}

#[test]
fn non_destroying_removal_never_drops_cargo() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = spawn_oak(&mut store);
    stock(&mut store, id, 64);
    store.drain_lifecycle_events();
    let removed = store
        .remove(id, RemovalReason::UnloadedToChunk)
        .expect("live chest boat");
    assert_eq!(
        chest_boat::chest_boat(removed.data())
            .unwrap()
            .item(0)
            .map(|stack| stack_count(&stack)),
        Some(64)
    );
    assert!(store.is_empty());
}

#[test]
fn chest_boat_explosion_ignore_is_the_vehicle_predicate() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = spawn_oak(&mut store);
    assert!(crate::entity::boat::is_boat(store.get(id).unwrap().data()));
    assert!(crate::entity::boat::ignore_explosion(true, false));
    assert!(!crate::entity::boat::ignore_explosion(true, true));
    assert!(!crate::entity::boat::ignore_explosion(false, false));
    assert!(spawn_chest_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d::default(),
        0.0
    )
    .is_none());
}
