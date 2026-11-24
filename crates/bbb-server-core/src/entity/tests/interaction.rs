use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

// ---- interaction ----------------------------------------------------------

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_INTERACTION_HEIGHT_DATA_ID, VANILLA_INTERACTION_RESPONSE_DATA_ID,
    VANILLA_INTERACTION_WIDTH_DATA_ID,
};
use bbb_protocol::entity_types::{
    vanilla_entity_type_client_tracking_range, VANILLA_ENTITY_TYPE_INTERACTION_ID,
};
use bbb_protocol::packets::{EntityDataValueKind, SetEntityData};
use bbb_protocol::NbtSummaryEntry;

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::interaction::{self, InteractionData, InteractionUse, PlayerAction};
use crate::entity::tracking::{EntityTracker, ServerEntity, TrackingViewer};

fn origin() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn interaction_at(position: Vec3d) -> (EntityStore, i32) {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = crate::entity::spawn_interaction(&mut store, position).expect("a fresh UUID");
    (store, id)
}

fn interaction_data(store: &EntityStore, id: i32) -> InteractionData {
    interaction::interaction(store.get(id).expect("entity is live"))
        .expect("interaction")
        .clone()
}

fn field<'a>(record: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    record
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

#[test]
fn a_spawned_interaction_is_a_unit_box_with_the_three_default_rows() {
    // `Interaction(EntityType, Level)` (`Interaction.java:40-43`) over the
    // `Entity` constructor's `setPos` (`Entity.java:321`), whose
    // `makeBoundingBox` override (`Interaction.java:176-178`) sizes the box
    // from the `1.0F` defaults rather than the catalog's `0 x 0`.
    let (mut store, id) = interaction_at(origin());
    let entity = store.get(id).unwrap();
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_INTERACTION_ID);
    assert_eq!(entity.bounding_box(), Aabb::of_entity(origin(), 1.0, 1.0));
    assert_eq!(entity.gravity(), 0.0);
    const _: () = assert!(interaction::IS_PICKABLE);
    const _: () = assert!(!interaction::CAN_BE_HIT_BY_PROJECTILE);
    let data = interaction::interaction(entity).unwrap();
    assert_eq!(data, &InteractionData::new());
    assert_eq!(data.dimensions(), [1.0, 1.0]);
    assert_eq!(data.attack(), None);
    assert_eq!(data.interaction(), None);

    // `defineSynchedData` (`Interaction.java:46-50`): the three rows directly
    // after `Entity`'s, as the generated catalog numbers them.
    let rows = interaction::synched_values(entity);
    let entity_rows = crate::entity::living::entity_synched_values(entity).len();
    assert_eq!(rows.len(), entity_rows + 3);
    let [width, height, response] = &rows[entity_rows..] else {
        panic!("three interaction rows")
    };
    assert_eq!(width.value.data_id, VANILLA_INTERACTION_WIDTH_DATA_ID);
    assert_eq!(
        width.value.serializer_id,
        VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID
    );
    assert_eq!(width.value.value, EntityDataValueKind::Float(1.0));
    assert_eq!(height.value.data_id, VANILLA_INTERACTION_HEIGHT_DATA_ID);
    assert_eq!(
        height.value.serializer_id,
        VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID
    );
    assert_eq!(height.value.value, EntityDataValueKind::Float(1.0));
    assert_eq!(response.value.data_id, VANILLA_INTERACTION_RESPONSE_DATA_ID);
    assert_eq!(
        response.value.serializer_id,
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID
    );
    assert_eq!(response.value.value, EntityDataValueKind::Boolean(false));
    assert_eq!(
        [
            VANILLA_INTERACTION_WIDTH_DATA_ID,
            VANILLA_INTERACTION_HEIGHT_DATA_ID,
            VANILLA_INTERACTION_RESPONSE_DATA_ID
        ],
        [
            entity_rows as u8,
            entity_rows as u8 + 1,
            entity_rows as u8 + 2
        ]
    );
    assert!(rows.iter().all(|row| row.is_default));
    assert!(crate::entity::tracking::non_default_synched_values(entity).is_empty());

    let events = store.drain_lifecycle_events();
    let [EntityLifecycleEvent::Added { entity: added, .. }] = events.as_slice() else {
        panic!("a spawned interaction publishes one add")
    };
    assert_eq!(added.bounding_box(), Aabb::of_entity(origin(), 1.0, 1.0));
}

#[test]
fn the_size_rows_resize_the_box_and_mark_only_a_changed_row_dirty() {
    // `setWidth`/`setHeight` (`Interaction.java:142-152`) through
    // `SynchedEntityData.set` (`SynchedEntityData.java:60-68`) and
    // `onSyncedDataUpdated` (`Interaction.java:72-77`).
    let (mut store, id) = interaction_at(origin());
    let entity = store.get_mut(id).unwrap();
    interaction::set_width(entity, 2.5);
    assert_eq!(
        entity.take_synched_dirty(),
        vec![VANILLA_INTERACTION_WIDTH_DATA_ID]
    );
    assert_eq!(entity.bounding_box(), Aabb::of_entity(origin(), 2.5, 1.0));
    interaction::set_height(entity, 0.25);
    assert_eq!(
        entity.take_synched_dirty(),
        vec![VANILLA_INTERACTION_HEIGHT_DATA_ID]
    );
    assert_eq!(entity.bounding_box(), Aabb::of_entity(origin(), 2.5, 0.25));
    assert_eq!(
        interaction::interaction(entity).unwrap().dimensions(),
        [2.5, 0.25]
    );

    // `Float.equals` is a bit comparison: the same value is not a change,
    // `-0.0F` against `0.0F` is.
    interaction::set_width(entity, 2.5);
    interaction::set_height(entity, 0.25);
    assert!(!entity.synched_data_is_dirty());
    interaction::set_width(entity, 0.0);
    interaction::set_width(entity, -0.0);
    assert_eq!(
        entity.take_synched_dirty(),
        vec![VANILLA_INTERACTION_WIDTH_DATA_ID]
    );
    assert_eq!(
        interaction::interaction(entity).unwrap().width().to_bits(),
        (-0.0_f32).to_bits()
    );

    interaction::set_response(entity, true);
    assert_eq!(
        entity.take_synched_dirty(),
        vec![VANILLA_INTERACTION_RESPONSE_DATA_ID]
    );
    interaction::set_response(entity, true);
    assert!(!entity.synched_data_is_dirty());
    assert!(interaction::interaction(entity).unwrap().response());
}

#[test]
fn an_attack_is_recorded_against_the_game_time_and_skipped_unless_response_is_set() {
    // `Interaction.skipAttackInteraction` (`Interaction.java:100-111`).
    let (mut store, id) = interaction_at(origin());
    let alice = Uuid::from_u128(0xa11ce);
    let bob = Uuid::from_u128(0xb0b);
    let entity = store.get_mut(id).unwrap();
    assert!(interaction::skip_attack_interaction(entity, alice, 1_200));
    assert_eq!(
        interaction::interaction(entity).unwrap().attack(),
        Some(PlayerAction {
            player: alice,
            timestamp: 1_200,
        })
    );
    assert_eq!(
        interaction::interaction(entity).unwrap().interaction(),
        None
    );
    // Recording an attack is not a synched change (`:105`, a plain field).
    assert!(!entity.synched_data_is_dirty());

    interaction::set_response(entity, true);
    assert!(!interaction::skip_attack_interaction(entity, bob, 1_201));
    // The later attacker replaces the earlier one (`:105`).
    assert_eq!(
        interaction::interaction(entity).unwrap().attack(),
        Some(PlayerAction {
            player: bob,
            timestamp: 1_201,
        })
    );
}

#[test]
fn a_use_is_recorded_against_the_game_time_and_consumed() {
    // `Interaction.interact` (`Interaction.java:119-126`) on the server.
    let (mut store, id) = interaction_at(origin());
    let alice = Uuid::from_u128(0xa11ce);
    let entity = store.get_mut(id).unwrap();
    assert_eq!(
        interaction::interact(entity, alice, 77),
        InteractionUse::Consume
    );
    assert_eq!(
        interaction::interaction(entity).unwrap().interaction(),
        Some(PlayerAction {
            player: alice,
            timestamp: 77,
        })
    );
    assert_eq!(interaction::interaction(entity).unwrap().attack(), None);
}

#[test]
fn an_interaction_never_ticks_and_never_moves() {
    // `Interaction.tick()` (`Interaction.java:129-130`) is empty, so neither
    // `baseTick` nor any movement runs: it hangs in the air and survives
    // below the world.
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    let deep = Vec3d {
        x: 0.5,
        y: -164.0,
        z: 0.5,
    };
    let id = crate::entity::spawn_interaction(&mut store, deep).unwrap();
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    });
    for _ in 0..4 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    let entity = store.get(id).unwrap();
    assert!(entity.is_alive());
    assert_eq!(entity.position(), deep);
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: 0.0,
            y: -1.0,
            z: 0.0
        }
    );
}

#[test]
fn an_interaction_cannot_be_hurt_by_anything() {
    let mut level = NativeLevel::new();
    let (id, _) = level.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_INTERACTION_ID,
        origin(),
        EntityData::Interaction(InteractionData::new()),
    );
    for resource_id in ["minecraft:player_attack", "minecraft:out_of_world"] {
        let outcome = level
            .world
            .hurt_stored_entity_by_command_for_test(DIMENSION, id, resource_id, 1.0e9, None)
            .unwrap();
        assert!(!outcome.applied && !outcome.succeeded, "{resource_id}");
        assert!(!level
            .world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .hurt_marked());
    }
    assert!(level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .is_alive());
}

#[test]
fn an_interaction_is_tracked_within_its_range_and_publishes_size_changes() {
    // `EntityType.INTERACTION` has a tracking range, unlike the marker, so
    // `ChunkMap.addEntity` pairs a session inside it; the pairing has no
    // `SetEntityData` because every row is at its default
    // (`ServerEntity.java:269-314`), and a later `setWidth` reaches the session
    // through `sendDirtyEntityData` (`:336-341`) as that one row.
    let (mut store, id) = interaction_at(origin());
    let range = vanilla_entity_type_client_tracking_range(VANILLA_ENTITY_TYPE_INTERACTION_ID)
        .expect("the interaction type is in the generated catalog")
        * 16;
    assert!(range > 0);
    let viewers = vec![
        TrackingViewer {
            connection_id: ConnectionId(1),
            position: Vec3d {
                x: f64::from(range) - 1.0,
                y: 64.0,
                z: 0.5,
            },
            view_distance: 32,
            uuid: uuid::Uuid::from_u128(1),
            entity_id: -1,
        },
        TrackingViewer {
            connection_id: ConnectionId(2),
            position: Vec3d {
                x: f64::from(range) + 1.0,
                y: 64.0,
                z: 0.5,
            },
            view_distance: 32,
            uuid: uuid::Uuid::from_u128(2),
            entity_id: -2,
        },
    ];
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut out,
    );
    assert_eq!(tracker.viewers_of(id), vec![ConnectionId(1)]);
    let paired: Vec<&ClientboundPlayPacket> = crate::entity::tracking::recipient_packets(&out)
        .filter(|(connection, _)| **connection == ConnectionId(1))
        .map(|(_, packet)| packet)
        .collect();
    assert!(matches!(
        paired.as_slice(),
        [
            ClientboundPlayPacket::BundleDelimiter,
            ClientboundPlayPacket::AddEntity(add),
            ClientboundPlayPacket::BundleDelimiter,
        ] if add.id == id && add.entity_type_id == VANILLA_ENTITY_TYPE_INTERACTION_ID
    ));
    assert!(crate::entity::tracking::recipient_packets(&out)
        .all(|(connection, _)| *connection != ConnectionId(2)));

    let entity = store.get_mut(id).unwrap();
    let mut server_entity = ServerEntity::new(entity, 3, true);
    assert!(server_entity
        .send_changes(entity)
        .iter()
        .all(|packet| !matches!(packet, ClientboundPlayPacket::SetEntityData(_))));
    interaction::set_width(entity, 3.0);
    let packets = server_entity.send_changes(entity);
    let [ClientboundPlayPacket::SetEntityData(SetEntityData {
        id: sent_id,
        values,
    })] = packets.as_slice()
    else {
        panic!("one dirty row publishes one data packet: {packets:?}")
    };
    assert_eq!(*sent_id, id);
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].data_id, VANILLA_INTERACTION_WIDTH_DATA_ID);
    assert_eq!(values[0].value, EntityDataValueKind::Float(3.0));
    assert!(!entity.synched_data_is_dirty());
}

#[test]
fn every_interaction_field_round_trips_and_the_loaded_size_rebuilds_the_box() {
    // `addAdditionalSaveData` (`Interaction.java:63-69`) and
    // `readAdditionalSaveData` (`:53-60`), with `setBoundingBox(makeBoundingBox())`
    // at the end of the read.
    let position = Vec3d {
        x: 4.5,
        y: 70.0,
        z: 2.5,
    };
    let (mut store, id) = interaction_at(position);
    let alice = Uuid::from_u128(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210);
    let bob = Uuid::from_u128(0xb0b);
    {
        let entity = store.get_mut(id).unwrap();
        interaction::set_width(entity, 2.5);
        interaction::set_height(entity, 0.25);
        interaction::set_response(entity, true);
        interaction::skip_attack_interaction(entity, alice, 1_200);
        interaction::interact(entity, bob, -7);
        entity.take_synched_dirty();
    }
    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:interaction"
    );
    let record = document.entities().next().unwrap().raw();
    assert_eq!(
        field(record, "width"),
        Some(&NbtSummaryValue::Float(2.5_f32.to_bits()))
    );
    assert_eq!(
        field(record, "height"),
        Some(&NbtSummaryValue::Float(0.25_f32.to_bits()))
    );
    assert_eq!(field(record, "response"), Some(&NbtSummaryValue::Byte(1)));
    // `UUIDUtil.CODEC` (`UUIDUtil.java:23-24`) is the four-int array and
    // `Codec.LONG` the long tag.
    assert_eq!(
        field(record, "attack"),
        Some(&NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "player".into(),
                value: NbtSummaryValue::IntArray(vec![
                    0x0123_4567,
                    0x89ab_cdef_u32 as i32,
                    0xfedc_ba98_u32 as i32,
                    0x7654_3210,
                ]),
            },
            NbtSummaryEntry {
                name: "timestamp".into(),
                value: NbtSummaryValue::Long(1_200),
            },
        ]))
    );
    assert!(matches!(
        field(record, "interaction"),
        Some(NbtSummaryValue::Compound(_))
    ));

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let after = reloaded.get_mut(ids[0]).unwrap();
    assert_eq!(after.position(), position);
    assert_eq!(after.bounding_box(), Aabb::of_entity(position, 2.5, 0.25));
    assert!(after.needs_sync());
    // The three `entityData.set` calls of the read each moved a row off its
    // default, so all three are dirty for the first `sendChanges`.
    assert_eq!(
        after.take_synched_dirty(),
        vec![
            VANILLA_INTERACTION_WIDTH_DATA_ID,
            VANILLA_INTERACTION_HEIGHT_DATA_ID,
            VANILLA_INTERACTION_RESPONSE_DATA_ID,
        ]
    );
    assert_eq!(
        interaction_data(&reloaded, ids[0]),
        interaction_data(&store, id)
    );
    assert_eq!(
        interaction_data(&reloaded, ids[0]).attack(),
        Some(PlayerAction {
            player: alice,
            timestamp: 1_200,
        })
    );
    assert_eq!(
        interaction_data(&reloaded, ids[0]).interaction(),
        Some(PlayerAction {
            player: bob,
            timestamp: -7,
        })
    );

    // A fresh interaction writes no action records and loads with nothing
    // dirty and the unit box.
    let (fresh, fresh_id) = interaction_at(position);
    let document = write_entity_chunk(&fresh, (0, 0)).expect("the save form is well formed");
    let record = document.entities().next().unwrap().raw();
    assert_eq!(field(record, "attack"), None);
    assert_eq!(field(record, "interaction"), None);
    assert_eq!(field(record, "response"), Some(&NbtSummaryValue::Byte(0)));
    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let after = reloaded.get(ids[0]).unwrap();
    assert!(!after.synched_data_is_dirty());
    assert_eq!(after.bounding_box(), Aabb::of_entity(position, 1.0, 1.0));
    assert_eq!(
        interaction_data(&reloaded, ids[0]),
        interaction_data(&fresh, fresh_id)
    );
}
