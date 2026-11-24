use super::native_level::{NativeLevel, DIMENSION};
use super::*;

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_END_CRYSTAL_BEAM_TARGET_DATA_ID, VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID,
};
use bbb_protocol::entity_types::{
    vanilla_entity_type_client_tracking_range, VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
};
use bbb_protocol::packets::{EntityDataValueKind, SetEntityData};
use bbb_protocol::NbtSummaryEntry;

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::end_crystal::{self, EndCrystalData};
use crate::entity::tracking::{
    non_default_synched_values, EntityTracker, ServerEntity, TrackingViewer,
};
use crate::entity::{
    spawn_end_crystal, StoredEntityDamageRules, StoredEntityDamageSource, StoredEntityHurtOutcome,
};

const PLAYER_UUID: u128 = 0x0102_0304_0506_0708_090a_0b0c_0d0e_0f10;

fn position() -> Vec3d {
    Vec3d {
        x: 2.5,
        y: 70.0,
        z: -3.5,
    }
}

fn end_crystal_data(store: &EntityStore, id: i32) -> EndCrystalData {
    let EntityData::EndCrystal(data) = store.get(id).expect("the crystal is live").data() else {
        panic!("expected an end crystal")
    };
    data.clone()
}

fn damage_type(resource_id: &str) -> &'static bbb_server_datapack::damage::DamageTypeFacts {
    bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == resource_id)
        .unwrap_or_else(|| panic!("{resource_id} is a built-in damage type"))
}

fn source(
    damage_type: &'static bbb_server_datapack::damage::DamageTypeFacts,
) -> StoredEntityDamageSource<'static> {
    StoredEntityDamageSource {
        damage_type,
        direct_entity_id: Some(77),
        causing_entity: Some(crate::entity::StoredDamageCausingEntity::Player {
            uuid: uuid::Uuid::from_u128(PLAYER_UUID),
            entity_id: 77,
            instabuild: false,
        }),
        attacker_views: Default::default(),
    }
}

fn field<'a>(record: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    record
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

#[test]
fn spawn_sets_a_bounded_time_and_publishes_the_catalog_entity() {
    let mut store = EntityStore::new(0x51_a3_c0_de, EntityIdCounter::new());
    let id = spawn_end_crystal(&mut store, position(), true).expect("a fresh UUID");
    let expected_time = end_crystal_data(&store, id).time();
    assert!((0..100_000).contains(&expected_time));

    let entity = store.get(id).unwrap();
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_END_CRYSTAL_ID);
    assert_eq!(entity.position(), position());
    assert_eq!(entity.bounding_box(), Aabb::of_entity(position(), 2.0, 2.0));
    assert_eq!(entity.gravity(), end_crystal::DEFAULT_GRAVITY);
    assert_eq!(
        end_crystal_data(&store, id),
        EndCrystalData::new(expected_time, true)
    );
    assert!(non_default_synched_values(entity).is_empty());
}

#[test]
fn beam_and_bottom_metadata_mark_only_real_changes_and_reach_tracking() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = spawn_end_crystal(&mut store, position(), false).expect("a fresh UUID");
    let viewer = TrackingViewer {
        connection_id: ConnectionId(1),
        position: position(),
        view_distance: 32,
        uuid: uuid::Uuid::from_u128(1),
        entity_id: -1,
    };
    assert_eq!(
        vanilla_entity_type_client_tracking_range(VANILLA_ENTITY_TYPE_END_CRYSTAL_ID),
        Some(16)
    );
    let mut tracker = EntityTracker::new();
    let mut paired = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &[viewer],
        |_, _, _| true,
        &mut paired,
    );
    assert_eq!(tracker.viewers_of(id), vec![ConnectionId(1)]);
    let paired = crate::entity::tracking::recipient_packets(&paired)
        .map(|(recipient, packet)| (*recipient, packet))
        .collect::<Vec<_>>();
    assert!(matches!(
        paired.as_slice(),
        [
            (ConnectionId(1), ClientboundPlayPacket::BundleDelimiter),
            (ConnectionId(1), ClientboundPlayPacket::AddEntity(add)),
            (ConnectionId(1), ClientboundPlayPacket::SetEntityData(SetEntityData { id: sent_id, values })),
            (ConnectionId(1), ClientboundPlayPacket::BundleDelimiter),
        ] if add.id == id
            && add.entity_type_id == VANILLA_ENTITY_TYPE_END_CRYSTAL_ID
            && *sent_id == id
            && matches!(values.as_slice(), [value]
                if value.data_id == VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID
                    && value.serializer_id == VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID
                    && value.value == EntityDataValueKind::Boolean(false))
    ));

    let entity = store.get_mut(id).unwrap();
    assert!(!entity.synched_data_is_dirty());
    end_crystal::set_beam_target(entity, None);
    end_crystal::set_show_bottom(entity, false);
    assert!(!entity.synched_data_is_dirty());

    let mut server_entity = ServerEntity::new(entity, i32::MAX, false);
    server_entity.send_changes(entity);
    let target = BlockPos { x: 4, y: 80, z: -6 };
    end_crystal::set_beam_target(entity, Some(target));
    end_crystal::set_beam_target(entity, Some(target));
    end_crystal::set_show_bottom(entity, true);
    end_crystal::set_show_bottom(entity, true);
    assert_eq!(
        entity.take_synched_dirty(),
        vec![
            VANILLA_END_CRYSTAL_BEAM_TARGET_DATA_ID,
            VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID
        ]
    );
    end_crystal::set_beam_target(entity, None);
    end_crystal::set_show_bottom(entity, false);
    let packets = server_entity.send_changes(entity);
    let [ClientboundPlayPacket::SetEntityData(SetEntityData {
        id: sent_id,
        values,
    })] = packets.as_slice()
    else {
        panic!("dirty crystal metadata publishes one packet: {packets:?}")
    };
    assert_eq!(*sent_id, id);
    assert_eq!(values.len(), 2);
    assert!(matches!(
        values.as_slice(),
        [beam, bottom]
            if beam.data_id == VANILLA_END_CRYSTAL_BEAM_TARGET_DATA_ID
                && beam.serializer_id == VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID
                && beam.value == EntityDataValueKind::OptionalBlockPos(None)
                && bottom.data_id == VANILLA_END_CRYSTAL_SHOW_BOTTOM_DATA_ID
                && bottom.serializer_id == VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID
                && bottom.value == EntityDataValueKind::Boolean(false)
    ));
    assert_eq!(non_default_synched_values(entity).len(), 1);
}

#[test]
fn metadata_rows_preserve_defaults_and_non_default_values() {
    let mut store = EntityStore::new(8, EntityIdCounter::new());
    let id = spawn_end_crystal(&mut store, position(), true).unwrap();
    let target = BlockPos { x: 8, y: 90, z: 3 };
    let entity = store.get_mut(id).unwrap();
    let entity_rows = crate::entity::living::entity_synched_values(entity).len();
    let rows = end_crystal::synched_values(
        entity,
        match entity.data() {
            EntityData::EndCrystal(data) => data,
            _ => unreachable!(),
        },
    );
    assert!(rows[entity_rows..].iter().all(|row| row.is_default));

    end_crystal::set_beam_target(entity, Some(target));
    end_crystal::set_show_bottom(entity, false);
    let EntityData::EndCrystal(data) = entity.data() else {
        unreachable!()
    };
    let rows = end_crystal::synched_values(entity, data);
    let [beam, bottom] = &rows[entity_rows..] else {
        panic!("two end crystal rows")
    };
    assert_eq!(
        beam.value.value,
        EntityDataValueKind::OptionalBlockPos(Some(target))
    );
    assert_eq!(
        beam.value.serializer_id,
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID
    );
    assert!(!beam.is_default);
    assert_eq!(bottom.value.value, EntityDataValueKind::Boolean(false));
    assert_eq!(
        bottom.value.serializer_id,
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID
    );
    assert!(!bottom.is_default);
}

#[test]
fn an_end_crystal_carries_its_portal_cooldown() {
    let mut store = EntityStore::new(10, EntityIdCounter::new());
    let id = spawn_end_crystal(&mut store, position(), true).unwrap();
    let mut entity = store.detach(id).unwrap();
    entity.portal_cooldown = 1;
    let entry = entity.block_position();
    let mut level = NativeLevel::new();
    level.set(entry, state_id("minecraft:nether_portal"));
    assert!(level.with_level(|level| crate::entity::end_crystal::tick(
        &mut entity,
        &mut store,
        level
    )));

    assert_eq!(entity.portal_cooldown(), 299);
    assert!(entity.portal_process().is_none());
}

#[test]
fn beam_and_bottom_round_trip_through_the_entity_chunk_save_form() {
    for (seed, target, show_bottom) in [
        (10, None, true),
        (11, Some(BlockPos { x: -4, y: 82, z: 9 }), false),
    ] {
        let mut store = EntityStore::new(seed, EntityIdCounter::new());
        let id = spawn_end_crystal(&mut store, position(), show_bottom).unwrap();
        end_crystal::set_beam_target(store.get_mut(id).unwrap(), target);
        let before = store.get(id).unwrap().clone();

        let document = write_entity_chunk(&store, (0, -1)).expect("save succeeds");
        assert_eq!(
            document.entities().next().unwrap().id(),
            "minecraft:end_crystal"
        );
        let record = document.entities().next().unwrap().raw();
        assert_eq!(
            field(record, "beam_target").cloned(),
            target.map(|target| NbtSummaryValue::IntArray(vec![target.x, target.y, target.z]))
        );
        assert_eq!(
            field(record, "ShowBottom"),
            Some(&NbtSummaryValue::Byte(i8::from(show_bottom)))
        );

        let mut reloaded = EntityStore::new(seed + 100, EntityIdCounter::new());
        let ids = read_entity_chunk(&mut reloaded, &document).expect("save reads back");
        let after = reloaded.get(ids[0]).unwrap();
        let after_data = end_crystal_data(&reloaded, ids[0]);
        assert_eq!(after.uuid(), before.uuid());
        assert_eq!(after.position(), before.position());
        assert_eq!(after_data.beam_target(), target);
        assert_eq!(after_data.shows_bottom(), show_bottom);
        assert!((0..100_000).contains(&after_data.time()));
        assert!(after.needs_sync());
    }
}

#[test]
fn non_explosion_damage_kills_explodes_and_unpairs_the_crystal() {
    for damage in ["minecraft:generic", "minecraft:explosion"] {
        let mut level = NativeLevel::new();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "observer", 1);
        level.world.set_player_position_for_test(
            ConnectionId(1),
            Vec3d {
                x: 4.5,
                y: 64.0,
                z: 8.5,
            },
        );
        let (id, uuid) = level.world.spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
            Vec3d {
                x: 4.5,
                y: 64.0,
                z: 4.5,
            },
            EntityData::EndCrystal(EndCrystalData::new(0, true)),
        );
        level.world.tick(receiver.drain());
        sender
            .try_send(crate::world::WorldCommand::Admin {
                envelope: crate::admin::AdminEnvelope {
                    operation_id: 1,
                    request_id: "crystal".into(),
                    principal: "fixture".into(),
                    scope: crate::admin::AdminScope::Command,
                    deadline_tick: u64::MAX,
                    now: 0,
                },
                request: crate::admin::AdminRequest::ExecuteCommand {
                    command: format!("damage {uuid} 1 {damage}"),
                },
            })
            .unwrap();
        let output = level.world.tick(receiver.drain());
        assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
        assert!(output.events.iter().any(|event|matches!(event,crate::world::WorldEvent::Send { packet:ClientboundPlayPacket::RemoveEntities(removed),.. } if removed.entity_ids.contains(&id))));
        let exploded=output.events.iter().any(|event|matches!(event,crate::world::WorldEvent::Send { packet:ClientboundPlayPacket::Explode(explosion),.. } if explosion.radius==6.0));
        assert_eq!(exploded, damage == "minecraft:generic");
    }
}

#[test]
fn explicit_invulnerability_fire_immunity_and_the_fall_tag_refuse_damage() {
    for (seed, resource_id, invulnerable, fall_damage_immune) in [
        (14, "minecraft:generic", true, false),
        (15, "minecraft:on_fire", false, false),
        (16, "minecraft:fall", false, true),
    ] {
        let mut store = EntityStore::new(seed, EntityIdCounter::new());
        let id = spawn_end_crystal(&mut store, position(), true).unwrap();
        store.get_mut(id).unwrap().set_invulnerable(invulnerable);
        store.drain_lifecycle_events();
        let mut sink = LevelEventSink::default();
        let outcome = store
            .hurt_server(
                id,
                source(damage_type(resource_id)),
                100.0,
                StoredEntityDamageRules {
                    fall_damage_immune,
                    ..StoredEntityDamageRules::default()
                },
                crate::world::registry_runtime::item_stack_resolver_for_test(),
                &mut sink,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            outcome,
            StoredEntityHurtOutcome {
                applied: false,
                succeeded: false
            },
            "{resource_id}"
        );
        assert!(store.get(id).is_some(), "{resource_id}");
        assert!(store.drain_explosions().is_empty(), "{resource_id}");
        assert!(store.drain_lifecycle_events().is_empty(), "{resource_id}");
    }
}

#[test]
fn ender_dragon_causing_entity_is_refused_by_actual_hurt() {
    let mut store = EntityStore::new(0xEDC0, EntityIdCounter::new());
    let dragon = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, Vec3d::default())
        .unwrap();
    let crystal = spawn_end_crystal(&mut store, position(), true).unwrap();
    store.drain_lifecycle_events();
    let facts = damage_type("minecraft:generic");
    let outcome = store
        .hurt_server(
            crystal,
            StoredEntityDamageSource {
                damage_type: facts,
                direct_entity_id: None,
                causing_entity: Some(crate::entity::StoredDamageCausingEntity::Mob {
                    entity_id: dragon,
                }),
                attacker_views: Default::default(),
            },
            1.0,
            StoredEntityDamageRules::default(),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
            &mut LevelEventSink::default(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        outcome,
        StoredEntityHurtOutcome {
            applied: false,
            succeeded: false,
        }
    );
    assert!(store.get(crystal).is_some());
    assert!(store.drain_explosions().is_empty());
    assert!(store.drain_lifecycle_events().is_empty());
}
