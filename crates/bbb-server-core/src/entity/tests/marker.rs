use super::native_level::NativeLevel;
use super::*;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MARKER_ID;
use bbb_protocol::NbtSummaryEntry;

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::marker::{self, MarkerData};

fn damage_type(resource_id: &str) -> &'static bbb_server_datapack::damage::DamageTypeFacts {
    bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == resource_id)
        .unwrap_or_else(|| panic!("{resource_id} is a built-in damage type"))
}

fn custom_data() -> Vec<NbtSummaryEntry> {
    vec![
        NbtSummaryEntry {
            name: "answer".into(),
            value: NbtSummaryValue::Int(42),
        },
        NbtSummaryEntry {
            name: "label".into(),
            value: NbtSummaryValue::string("spawn"),
        },
    ]
}

fn marker_data(store: &EntityStore, id: i32) -> MarkerData {
    let entity = store.get(id).expect("entity is live");
    let EntityData::Marker(data) = entity.data() else {
        panic!("marker")
    };
    data.clone()
}

fn marker_at(position: Vec3d, data: MarkerData) -> (EntityStore, i32) {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = crate::entity::spawn_marker(&mut store, position, data).expect("a fresh UUID");
    (store, id)
}

#[test]
fn a_spawned_marker_carries_only_its_custom_data_and_the_catalog_size() {
    let position = Vec3d {
        x: 3.5,
        y: 70.0,
        z: -1.5,
    };
    let (store, id) = marker_at(position, MarkerData::with_data(custom_data()));
    let entity = store.get(id).unwrap();
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_MARKER_ID);
    assert_eq!(entity.position(), position);
    assert_eq!(entity.bounding_box(), Aabb::of_entity(position, 0.0, 0.0));
    assert_eq!(entity.gravity(), 0.0);
    assert_eq!(
        marker::marker(entity).unwrap().data(),
        custom_data().as_slice()
    );
    const _: () = assert!(!marker::IS_PICKABLE);
    const _: () = assert!(!marker::CAN_BE_HIT_BY_PROJECTILE);

    let rows = marker::synched_values(entity);
    assert_eq!(
        rows.len(),
        crate::entity::living::entity_synched_values(entity).len()
    );
    assert!(rows.iter().all(|row| row.is_default));
    assert!(crate::entity::tracking::non_default_synched_values(entity).is_empty());
    assert!(!entity.synched_data_is_dirty());
}

#[test]
fn a_marker_survives_below_the_world_without_moving() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    let deep = Vec3d {
        x: 0.5,
        y: f64::from(level.with_level(|level| level.min_y())) - 100.0,
        z: 0.5,
    };
    let marker_id = crate::entity::spawn_marker(&mut store, deep, MarkerData::new()).unwrap();
    let item_id =
        spawn_item_entity(&mut store, deep, stack_of("minecraft:dirt", 1)).expect("a fresh UUID");
    for _ in 0..3 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert!(store.get(item_id).is_none());
    let entity = store.get(marker_id).unwrap();
    assert!(entity.is_alive());
    assert_eq!(entity.position(), deep);
    assert_eq!(entity.delta_movement(), Vec3d::default());
}

#[test]
fn a_marker_is_never_paired_with_a_tracking_session() {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "observer", 1);
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    level
        .world
        .set_player_position_for_test(ConnectionId(1), position);
    let (id, uuid) = level.world.spawn_stored_entity_for_test(
        super::native_level::DIMENSION,
        VANILLA_ENTITY_TYPE_MARKER_ID,
        position,
        EntityData::Marker(MarkerData::new()),
    );
    let cow = level
        .world
        .spawn_stored_entity_for_test(
            super::native_level::DIMENSION,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            position,
            EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
                )
                .unwrap(),
            ),
        )
        .0;
    let output = level.world.tick(receiver.drain());
    assert!(output.events.iter().any(|event| matches!(event, crate::world::WorldEvent::Send { connection_id:ConnectionId(1), packet:ClientboundPlayPacket::AddEntity(entity) } if entity.id==cow)));
    assert!(output.events.iter().all(|event| !matches!(event,crate::world::WorldEvent::Send { packet:ClientboundPlayPacket::AddEntity(entity),.. } if entity.id==id)));
    let killed = crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        &format!("kill {uuid}"),
    );
    assert!(level
        .world
        .stored_entity_for_test(super::native_level::DIMENSION, id)
        .is_none());
    assert!(!killed.events.iter().any(|event| matches!(event, crate::world::WorldEvent::Send { connection_id:ConnectionId(1), packet:ClientboundPlayPacket::RemoveEntities(packet) } if packet.entity_ids.contains(&id))));
}

#[test]
fn a_marker_cannot_be_hurt_by_anything() {
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let (mut store, id) = marker_at(position, MarkerData::new());
    store.drain_lifecycle_events();
    let mut sink = LevelEventSink::default();
    for (resource_id, instabuild) in [
        ("minecraft:player_attack", false),
        ("minecraft:out_of_world", true),
        ("minecraft:generic_kill", true),
    ] {
        let facts = damage_type(resource_id);
        let outcome = store
            .hurt_server(
                id,
                crate::entity::StoredEntityDamageSource {
                    damage_type: facts,
                    direct_entity_id: Some(77),
                    causing_entity: Some(crate::entity::StoredDamageCausingEntity::Player {
                        uuid: uuid::Uuid::nil(),
                        entity_id: 77,
                        instabuild,
                    }),
                    attacker_views: Default::default(),
                },
                1.0e9,
                Default::default(),
                crate::world::registry_runtime::item_stack_resolver_for_test(),
                &mut sink,
            )
            .unwrap()
            .unwrap();
        assert!(!outcome.applied && !outcome.succeeded, "{resource_id}");
    }
    let entity = store.get_mut(id).unwrap();
    assert!(entity.is_alive());
    assert!(!entity.take_hurt_marked());
    assert!(store.drain_lifecycle_events().is_empty());
}

#[test]
fn the_custom_data_compound_round_trips_and_is_omitted_when_empty() {
    let position = Vec3d {
        x: 1.5,
        y: 65.0,
        z: 2.5,
    };
    let (store, id) = marker_at(position, MarkerData::with_data(custom_data()));
    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(document.entities().next().unwrap().id(), "minecraft:marker");
    let record = document.entities().next().unwrap().raw();
    let data = record
        .iter()
        .find(|entry| entry.name == "data")
        .expect("a marker with custom data writes it");
    assert_eq!(data.value, NbtSummaryValue::Compound(custom_data()));

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let after = reloaded.get(ids[0]).unwrap();
    assert_eq!(after.uuid(), store.get(id).unwrap().uuid());
    assert_eq!(after.position(), position);
    assert_eq!(marker_data(&reloaded, ids[0]), marker_data(&store, id));
    assert!(after.needs_sync());
    assert!(!after.synched_data_is_dirty());

    let (empty_store, _) = marker_at(position, MarkerData::new());
    let document = write_entity_chunk(&empty_store, (0, 0)).expect("the save form is well formed");
    assert!(document
        .entities()
        .next()
        .unwrap()
        .raw()
        .iter()
        .all(|entry| entry.name != "data"));
    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    assert_eq!(marker_data(&reloaded, ids[0]), MarkerData::new());
}
