use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

use crate::fluid::with_property;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::level_event::VanillaGameEvent;

use crate::entity::minecart::{self, DAMAGE_TO_DESTROY};
use crate::entity::minecart_container::{
    self, MinecartContainerData, MinecartContainerKind, HOPPER_CONTAINER_SIZE,
};

const CONTAINER_TYPES: [(i32, &str, MinecartContainerKind); 2] = [
    (
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        "minecraft:chest_minecart",
        MinecartContainerKind::Chest,
    ),
    (
        VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
        "minecraft:hopper_minecart",
        MinecartContainerKind::Hopper,
    ),
];

fn container_at(kind: MinecartContainerKind) -> (EntityStore, i32) {
    let entity_type_id = match kind {
        MinecartContainerKind::Chest => VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        MinecartContainerKind::Hopper => VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
    };
    let mut store = EntityStore::new(0x43_4f_4e_54, EntityIdCounter::new());
    let id = crate::entity::spawn_minecart_container(
        &mut store,
        entity_type_id,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        37.5,
    )
    .expect("a built-in minecart container");
    (store, id)
}

fn data(store: &EntityStore, id: i32) -> &MinecartContainerData {
    minecart_container::minecart_container(store.get(id).expect("entity is live").data())
        .expect("entity is a minecart container")
}

fn data_mut(store: &mut EntityStore, id: i32) -> &mut MinecartContainerData {
    minecart_container::minecart_container_mut(&mut store.get_mut(id).expect("entity is live").data)
        .expect("entity is a minecart container")
}

#[test]
fn both_container_types_construct_with_exact_identity_dimensions_and_defaults() {
    for (entity_type_id, resource_id, kind) in CONTAINER_TYPES {
        let (store, id) = container_at(kind);
        let entity = store.get(id).expect("entity is live");
        assert_eq!(entity.entity_type_id(), entity_type_id);
        assert_eq!(entity.data().entity_resource_id(), resource_id);
        assert_eq!([entity.bb_width(), entity.bb_height()], [0.98, 0.7]);
        assert_eq!(entity.y_rot(), 37.5);
        assert!(entity.is_pickable());
        let data = data(&store, id);
        assert_eq!(data.kind(), kind);
        assert_eq!(data.container_size(), kind.container_size());
        assert!(data.is_enabled());
        assert_eq!(
            data.minecart().display_offset(),
            kind.default_display_offset()
        );
        assert_eq!(
            data.minecart().default_display_offset(),
            kind.default_display_offset()
        );
        assert!(crate::entity::tracking::non_default_synched_values(entity).is_empty());
    }

    let (chest_store, chest_id) = container_at(MinecartContainerKind::Chest);
    let chest =
        vanilla_block_state_identity(data(&chest_store, chest_id).default_display_block_state_id())
            .expect("built-in chest state");
    assert_eq!(chest.name, "minecraft:chest");
    assert_eq!(
        chest.properties.get("facing").map(String::as_str),
        Some("north")
    );

    let (hopper_store, hopper_id) = container_at(MinecartContainerKind::Hopper);
    let hopper = vanilla_block_state_identity(
        data(&hopper_store, hopper_id).default_display_block_state_id(),
    )
    .expect("built-in hopper state");
    assert_eq!(hopper.name, "minecraft:hopper");
}

#[test]
fn inventory_signal_and_water_scale_natural_slowdown() {
    let movement = Vec3d {
        x: 1.0,
        y: 0.25,
        z: -1.0,
    };
    let empty = MinecartContainerData::new_chest();
    let dry = empty.apply_natural_slowdown(movement, false);
    assert_eq!(dry.x, f64::from(0.995_f32));
    assert_eq!(dry.y, 0.0);
    assert_eq!(dry.z, -f64::from(0.995_f32));
    let wet = empty.apply_natural_slowdown(movement, true);
    assert_eq!(wet.x, f64::from(0.995_f32 * 0.95_f32));

    let mut partial = MinecartContainerData::new_chest();
    partial
        .set_item(0, Some(stack_of("minecraft:stone", 64)))
        .unwrap();
    assert_eq!(
        partial.apply_natural_slowdown(movement, false).x,
        f64::from(0.994_f32)
    );

    let mut full = MinecartContainerData::new_hopper();
    for slot in 0..HOPPER_CONTAINER_SIZE {
        full.set_item(slot, Some(stack_of("minecraft:stone", 64)))
            .unwrap();
    }
    assert_eq!(
        full.apply_natural_slowdown(movement, false).x,
        f64::from(0.98_f32)
    );
}

#[test]
fn rail_tick_dispatches_to_container_natural_slowdown() {
    let mut level = NativeLevel::new();
    level.set(pos(0, 64, 0), state_id("minecraft:rail"));
    let (mut store, id) = container_at(MinecartContainerKind::Chest);
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 0.0,
        y: 0.0,
        z: 0.2,
    });
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let entity = store.get(id).unwrap();
    assert_eq!(entity.delta_movement().z, 0.2 * f64::from(0.995_f32));
    assert!(data(&store, id).minecart().on_rails());
}

#[test]
fn activator_rail_disables_and_reenables_hopper_minecart() {
    let activator = with_property(state_id("minecraft:activator_rail"), "powered", "true")
        .expect("powered activator state");
    let mut level = NativeLevel::new();
    level.set(pos(0, 63, 0), state_id("minecraft:redstone_block"));
    level.set(pos(0, 64, 0), activator);
    let (mut store, id) = container_at(MinecartContainerKind::Hopper);
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert!(!data(&store, id).is_enabled());
    assert_eq!(data(&store, id).minecart().hurt_time(), 0);

    level.set(pos(0, 63, 0), state_id("minecraft:stone"));
    level.world.set_block(
        DIMENSION,
        pos(0, 64, 0),
        state_id("minecraft:activator_rail"),
        18,
    );
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert!(data(&store, id).is_enabled());
}

#[test]
fn inventory_enabled_identity_and_display_state_round_trip_canonically() {
    for (_, resource_id, kind) in CONTAINER_TYPES {
        let (mut store, id) = container_at(kind);
        let uuid = store.get(id).unwrap().uuid();
        data_mut(&mut store, id)
            .set_item(0, Some(stack_of("minecraft:diamond", 3)))
            .unwrap();
        if kind == MinecartContainerKind::Hopper {
            data_mut(&mut store, id).set_enabled(false);
        }
        minecart::set_custom_display_block(
            store.get_mut(id).unwrap(),
            Some(state_id("minecraft:stone")),
        );
        assert_eq!(
            store.get_mut(id).unwrap().take_synched_dirty(),
            vec![
                bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID
            ]
        );
        assert_eq!(
            crate::entity::tracking::non_default_synched_values(store.get(id).unwrap()).len(),
            1
        );
        let document = write_entity_chunk(&store, (0, 0)).expect("container save is canonical");
        let record = document.entities().next().unwrap();
        assert_eq!(record.id(), resource_id);
        assert!(record.raw().iter().any(|entry| entry.name == "Items"));
        assert_eq!(
            record
                .raw()
                .iter()
                .find(|entry| entry.name == "Enabled")
                .map(|entry| &entry.value),
            if kind == MinecartContainerKind::Hopper {
                Some(&NbtSummaryValue::Byte(0))
            } else {
                None
            }
        );
        assert!(record
            .raw()
            .iter()
            .all(|entry| entry.name != "DisplayOffset"));

        let mut restored = EntityStore::new(1, EntityIdCounter::new());
        let ids = read_entity_chunk(&mut restored, &document).expect("container save restores");
        let restored_entity = restored.get(ids[0]).unwrap();
        assert_eq!(restored_entity.uuid(), uuid);
        let restored_data = data(&restored, ids[0]);
        assert_eq!(
            restored_data.item(0).as_deref(),
            Some(&stack_of("minecraft:diamond", 3))
        );
        assert_eq!(
            restored_data.is_enabled(),
            kind != MinecartContainerKind::Hopper
        );
        assert_eq!(
            restored_data.minecart().custom_display_block(),
            Some(state_id("minecraft:stone"))
        );
        assert_eq!(
            restored_data.minecart().display_offset(),
            kind.default_display_offset()
        );
    }
}

#[test]
fn hopper_restore_defaults_a_missing_enabled_field_to_true() {
    let (store, _) = container_at(MinecartContainerKind::Hopper);
    let document = write_entity_chunk(&store, (0, 0)).unwrap();
    let mut raw = document.into_raw();
    let entities = raw
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "Entities")
        .expect("entity list");
    let NbtSummaryValue::List(records) = &mut entities.value else {
        panic!("Entities is a list")
    };
    let NbtSummaryValue::Compound(record) = &mut records[0] else {
        panic!("entity is a compound")
    };
    record.retain(|entry| entry.name != "Enabled");

    let document = Java26EntityChunkData::parse(raw, 0, 0).expect("common fields remain valid");
    let mut restored = EntityStore::new(2, EntityIdCounter::new());
    let ids =
        read_entity_chunk(&mut restored, &document).expect("missing Enabled uses its default");
    assert!(data(&restored, ids[0]).is_enabled());
}

#[test]
fn loot_table_restore_keeps_the_pending_table_and_seed_without_unpacking() {
    let (store, _) = container_at(MinecartContainerKind::Chest);
    let document = write_entity_chunk(&store, (0, 0)).unwrap();
    let mut raw = document.into_raw();
    let entities = raw
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "Entities")
        .expect("entity list");
    let NbtSummaryValue::List(records) = &mut entities.value else {
        panic!("Entities is a list")
    };
    let NbtSummaryValue::Compound(record) = &mut records[0] else {
        panic!("entity is a compound")
    };
    record.retain(|entry| entry.name != "Items");
    record.push(NbtSummaryEntry {
        name: "LootTable".into(),
        value: NbtSummaryValue::string("minecraft:chests/simple_dungeon"),
    });
    record.push(NbtSummaryEntry {
        name: "LootTableSeed".into(),
        value: NbtSummaryValue::Long(37),
    });
    let document = Java26EntityChunkData::parse(raw, 0, 0).expect("loot branch is valid");
    let mut restored = EntityStore::new(9, EntityIdCounter::new());
    let ids =
        read_entity_chunk(&mut restored, &document).expect("a pending key does not unpack loot");
    assert_eq!(ids.len(), 1);
    let container = data(&restored, ids[0]);
    assert_eq!(
        container.loot_table_id().as_deref(),
        Some("minecraft:chests/simple_dungeon")
    );
    assert_eq!(container.loot_table_seed(), 37);
    assert!((0..container.container_size()).all(|slot| container.item(slot).is_none()));
    let saved =
        write_entity_chunk(&restored, (0, 0)).expect("a pending table saves without unpacking");
    let record = saved.entities().next().unwrap().raw();
    assert_eq!(
        record
            .iter()
            .find(|entry| entry.name == "LootTable")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::string("minecraft:chests/simple_dungeon"))
    );
    assert_eq!(
        record
            .iter()
            .find(|entry| entry.name == "LootTableSeed")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::Long(37))
    );
    assert!(record.iter().all(|entry| entry.name != "Items"));
}

fn native_container(kind: MinecartContainerKind) -> (NativeLevel, i32) {
    let mut level = NativeLevel::new();
    let (type_id, mut data) = match kind {
        MinecartContainerKind::Chest => (
            VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
            MinecartContainerData::new_chest(),
        ),
        MinecartContainerKind::Hopper => (
            VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
            MinecartContainerData::new_hopper(),
        ),
    };
    data.set_item(0, Some(stack_of("minecraft:diamond", 2)))
        .unwrap();
    data.set_item(1, Some(stack_of("minecraft:emerald", 2)))
        .unwrap();
    let data = match kind {
        MinecartContainerKind::Chest => EntityData::ChestMinecart(data),
        MinecartContainerKind::Hopper => EntityData::HopperMinecart(data),
    };
    let (id, _) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        type_id,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        data,
    );
    (level, id)
}

#[test]
fn destroyed_container_minecart_drops_its_cargo_and_vehicle_item() {
    for (_, resource_id, kind) in CONTAINER_TYPES {
        let (mut level, id) = native_container(kind);
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
            &format!(
                "damage {target_uuid} {} minecraft:generic by {cause_uuid}",
                DAMAGE_TO_DESTROY / 10.0 + 0.1
            ),
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
        for (item, count) in [
            ("minecraft:diamond", 2),
            ("minecraft:emerald", 2),
            (resource_id, 1),
        ] {
            assert_eq!(
                items
                    .iter()
                    .filter(|row| row.2 == item)
                    .map(|row| row.3)
                    .sum::<i32>(),
                count
            );
        }
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
}

#[test]
fn cargo_ignores_entity_drops_and_creative_discard_but_not_unload() {
    for creative in [false, true] {
        let (mut level, id) = native_container(if creative {
            MinecartContainerKind::Hopper
        } else {
            MinecartContainerKind::Chest
        });
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "alice", 1);
        let uuid = level
            .world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .uuid();
        if creative {
            crate::tests::console_command(
                &sender,
                &receiver,
                &mut level.world,
                1,
                "gamemode creative alice",
            );
            crate::tests::console_command(
                &sender,
                &receiver,
                &mut level.world,
                2,
                &format!("damage {uuid} 5 minecraft:generic by alice"),
            );
        } else {
            crate::tests::console_command(
                &sender,
                &receiver,
                &mut level.world,
                1,
                "gamerule minecraft:entity_drops false",
            );
            level
                .world
                .hurt_stored_entity_by_command_for_test(
                    DIMENSION,
                    id,
                    "minecraft:generic",
                    5.0,
                    None,
                )
                .unwrap();
        }
        assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
        let items = level.world.stored_items_for_test(DIMENSION);
        for item in ["minecraft:diamond", "minecraft:emerald"] {
            assert_eq!(
                items
                    .iter()
                    .filter(|row| row.2 == item)
                    .map(|row| row.3)
                    .sum::<i32>(),
                2
            );
        }
        assert!(
            !items
                .iter()
                .any(|row| row.2 == "minecraft:chest_minecart"
                    || row.2 == "minecraft:hopper_minecart")
        );
    }
    let (mut unloaded, id) = container_at(MinecartContainerKind::Chest);
    data_mut(&mut unloaded, id)
        .set_item(0, Some(stack_of("minecraft:lapis_lazuli", 1)))
        .unwrap();
    let removed = unloaded.remove(id, RemovalReason::UnloadedToChunk).unwrap();
    assert_eq!(
        minecart_container::minecart_container(removed.data())
            .unwrap()
            .item(0)
            .as_deref(),
        Some(&stack_of("minecraft:lapis_lazuli", 1))
    );
    assert!(unloaded.is_empty());
}
