use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAGS_DATA_ID, VANILLA_HANGING_ENTITY_DIRECTION_DATA_ID,
    VANILLA_ITEM_FRAME_ITEM_DATA_ID, VANILLA_ITEM_FRAME_ROTATION_DATA_ID,
};
use bbb_protocol::entity_types::{
    vanilla_entity_type_client_tracking_range, vanilla_entity_type_update_interval,
    VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_BAT_ID, VANILLA_ENTITY_TYPE_CAT_ID,
    VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COD_ID, VANILLA_ENTITY_TYPE_COW_ID,
    VANILLA_ENTITY_TYPE_CREEPER_ID, VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, VANILLA_ENTITY_TYPE_FIREBALL_ID,
    VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
    VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_MARKER_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    VANILLA_ENTITY_TYPE_OCELOT_ID, VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_TADPOLE_ID,
    VANILLA_ENTITY_TYPE_VEX_ID, VANILLA_ENTITY_TYPE_WITHER_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::packets::{BlockPos, Direction, SoundEntityEvent, SoundEventHolder, SoundSource};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

use super::*;
use crate::entity::falling_block::FallingBlockData;
use crate::entity::item::{plain_stack, spawn_item_entity};
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::EntityIdCounter;

mod door_progress;
mod player;

/// `EntityType.ITEM.clientTrackingRange() * 16` (`ChunkMap.java:1142`) off the
/// generated 26.1 entity catalog, which is what the tracker itself reads. The
/// number is never spelled here: a test that repeats the table it is checking
/// only proves the two copies agree.
fn item_tracking_range_blocks() -> i32 {
    vanilla_entity_type_client_tracking_range(VANILLA_ENTITY_TYPE_ITEM_ID)
        .expect("the item type is in the generated catalog")
        * CHUNK_BLOCKS
}

/// `EntityType.ITEM.updateInterval()` (`ChunkMap.java:1144`), from the same
/// catalog.
fn item_update_interval() -> i32 {
    vanilla_entity_type_update_interval(VANILLA_ENTITY_TYPE_ITEM_ID)
        .expect("the item type is in the generated catalog")
}

pub(crate) fn origin() -> Vec3d {
    Vec3d {
        x: 0.0,
        y: 64.0,
        z: 0.0,
    }
}

fn player_sync_frame(entity_id: i32) -> PlayerSyncFrame {
    PlayerSyncFrame {
        shoulder_parrots: [None; 2],
        shoulders_dirty: false,
        entity_id,
        position: origin(),
        delta_movement: Vec3d::default(),
        known_movement: Vec3d::default(),
        needs_sync: false,
        y_rot: 0.0,
        x_rot: 0.0,
        y_head_rot: 0.0,
        on_ground: true,
        riding: false,
        shared_flags: 0,
        shared_flags_dirty: false,
        air_supply: crate::entity::living::MAX_AIR_SUPPLY,
        air_supply_dirty: false,
        silent: false,
        silent_dirty: false,
        pose_id: bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID,
        pose_dirty: false,
        sleeping_pos: None,
        sleeping_pos_dirty: false,
        ticks_frozen: 0,
        ticks_frozen_dirty: false,
        living_entity_flags: 0,
        living_entity_flags_dirty: false,
        arrow_count: 0,
        arrow_count_dirty: false,
        stinger_count: 0,
        stinger_count_dirty: false,
        // The define-time defaults, so the rows a test sets stand alone.
        health: 1.0,
        absorption: 0.0,
        effect_particles: Vec::new(),
        effect_particles_dirty: false,
        effect_ambience: false,
        effect_ambience_dirty: false,
        main_hand: bbb_protocol::packets::ClientMainHand::Right,
        main_hand_dirty: false,
        model_customization: 0,
        model_customization_dirty: false,
        dirty_attributes: Vec::new(),
        hurt_marked: false,
    }
}

#[test]
fn ghast_autonomous_runtime_tracking_charging_default_and_dirty_are_exact() {
    let mut store = EntityStore::new(0x6A57, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_GHAST_ID, origin())
        .unwrap();
    assert!(non_default_synched_values(store.get(id).unwrap())
        .iter()
        .all(|value| value.data_id
            != bbb_protocol::entity_synched_data::VANILLA_GHAST_CHARGING_DATA_ID));

    let entity = store.get_mut(id).unwrap();
    assert!(crate::entity::ghast::set_charging(entity, true));
    assert!(entity
        .take_synched_dirty()
        .contains(&bbb_protocol::entity_synched_data::VANILLA_GHAST_CHARGING_DATA_ID));
    assert!(!crate::entity::ghast::set_charging(entity, true));
    assert!(entity.take_synched_dirty().is_empty());
    assert!(non_default_synched_values(entity).iter().any(|value| {
        value.data_id == bbb_protocol::entity_synched_data::VANILLA_GHAST_CHARGING_DATA_ID
            && value.value == EntityDataValueKind::Boolean(true)
    }));
}

/// One resting item at `position`, with the constructor's random shove taken
/// back off so a movement packet only appears when the test asks for one.
pub(crate) fn store_with_one_item(position: Vec3d) -> (EntityStore, i32) {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let stack = plain_stack("minecraft:stone", 1).expect("stone is an item");
    let id = spawn_item_entity(&mut store, position, stack).expect("the store accepts an item");
    store
        .get_mut(id)
        .expect("the item was just spawned")
        .set_delta_movement(Vec3d::default());
    (store, id)
}

/// A session standing at `x` on the z axis of the level, watching from far
/// enough away that the entity's own range is always the binding half of
/// `min(effectiveRange, viewDistance * 16)` (`ChunkMap.java:1389`).
fn viewer(connection: u64, x: f64) -> TrackingViewer {
    TrackingViewer {
        connection_id: ConnectionId(connection),
        position: Vec3d { x, y: 64.0, z: 0.0 },
        // `ChunkMap.setServerViewDistance`'s own upper clamp
        // (`ChunkMap.java:810`).
        view_distance: 32,
        uuid: uuid::Uuid::from_u128(
            0xA000_0000_0000_0000_0000_0000_0000_0000 + u128::from(connection),
        ),
        entity_id: -(connection as i32) - 1,
    }
}

pub(crate) fn packets_for(out: &[TrackingPacket], connection: u64) -> Vec<ClientboundPlayPacket> {
    recipient_packets(out)
        .filter(|(id, _)| **id == ConnectionId(connection))
        .map(|(_, packet)| packet.clone())
        .collect()
}

#[test]
fn entity_bound_sound_uses_emit_position_for_level_broadcast() {
    let sound = SoundEntityEvent {
        sound: SoundEventHolder::Reference { registry_id: 7 },
        source: SoundSource::Blocks,
        entity_id: 42,
        volume: 1.0,
        pitch: 1.0,
        seed: 9,
    };
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &[EntityLifecycleEvent::SoundFromEntity {
            sound: sound.clone(),
            position_at_emit: origin(),
        }],
        &[viewer(1, 15.99), viewer(2, 16.0)],
        |_, _, _| true,
        &mut out,
    );
    assert_eq!(
        out,
        vec![TrackingPacket {
            recipients: vec![ConnectionId(1)],
            packet: ClientboundPlayPacket::SoundEntity(sound)
        }]
    );
}

/// `TrackedEntity.updatePlayer` (`ChunkMap.java:1385-1408`): the horizontal
/// distance decides, and a session outside the entity's own range is never
/// paired at all.
#[test]
fn pairing_shares_one_ordered_packet_sequence_for_visible_sessions() {
    let (mut store, id) = store_with_one_item(origin());
    let range = f64::from(item_tracking_range_blocks());
    let viewers = vec![
        viewer(1, range - 1.0),
        viewer(2, range + 1.0),
        viewer(3, 0.0),
    ];
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut out,
    );

    assert_eq!(
        tracker.viewers_of(id),
        vec![ConnectionId(1), ConnectionId(3)]
    );
    assert_eq!(out.len(), 4);
    assert!(out
        .iter()
        .all(|targeted| targeted.recipients == [ConnectionId(1), ConnectionId(3)]));
    assert_eq!(packets_for(&out, 1), packets_for(&out, 3));
    let paired = packets_for(&out, 1);
    // `ServerEntity.sendPairingData` (`ServerEntity.java:269-314`): the add
    // packet and the synched-data snapshot behind it.
    assert!(matches!(
        paired.first(),
        Some(ClientboundPlayPacket::BundleDelimiter)
    ));
    assert!(matches!(
        paired.get(1),
        Some(ClientboundPlayPacket::AddEntity(add))
            if add.id == id && add.entity_type_id == VANILLA_ENTITY_TYPE_ITEM_ID
    ));
    assert!(matches!(
        paired.get(2),
        Some(ClientboundPlayPacket::SetEntityData(data))
            if data.id == id
                && data.values.len() == 1
                && data.values[0].data_id == VANILLA_ITEM_ENTITY_ITEM_DATA_ID
    ));
    assert!(matches!(
        paired.get(3),
        Some(ClientboundPlayPacket::BundleDelimiter)
    ));
    assert_eq!(paired.len(), 4);
    assert!(packets_for(&out, 2).is_empty());
}

#[test]
fn passenger_changes_reach_current_and_later_viewers() {
    let (mut store, vehicle_id) = store_with_one_item(origin());
    let mut tracker = EntityTracker::new();
    let first_viewer = viewer(1, 1.0);
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &[first_viewer],
        |_, _, _| true,
        &mut out,
    );

    out.clear();
    let passenger_ids = vec![41, 42];
    tracker.apply_lifecycle_events(
        &[EntityLifecycleEvent::PassengersChanged {
            vehicle_id,
            passenger_ids: passenger_ids.clone(),
        }],
        &[first_viewer],
        |_, _, _| true,
        &mut out,
    );
    assert!(matches!(
        packets_for(&out, 1).as_slice(),
        [ClientboundPlayPacket::SetPassengers(update)]
            if update.vehicle_id == vehicle_id && update.passenger_ids == passenger_ids
    ));

    out.clear();
    tracker.tick(
        &mut store,
        &[first_viewer, viewer(2, 1.0)],
        |_, _, _| true,
        |_, _| true,
        &mut out,
    );
    assert!(packets_for(&out, 2).iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::SetPassengers(update)
            if update.vehicle_id == vehicle_id && update.passenger_ids == passenger_ids
    )));
}

#[test]
fn external_player_vehicle_set_passengers_includes_network_id() {
    let mut store = EntityStore::new(0x4558_5452, EntityIdCounter::new());
    let boat =
        crate::entity::spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, origin(), 0.0)
            .unwrap();
    let stored = crate::entity::spawn_armor_stand(&mut store, origin()).unwrap();
    assert!(store.link_existing_passenger(boat, stored));
    let mut tracker = EntityTracker::new();
    let watching = viewer(1, 0.0);
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &[watching],
        |_, _, _| true,
        &mut out,
    );

    out.clear();
    let player_id = 50_001;
    assert!(store.attach_test_player(
        player_id,
        boat,
        crate::entity::ExternalPlayerCanRide::Allowed,
    ));
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &[watching],
        |_, _, _| true,
        &mut out,
    );
    assert!(matches!(
        packets_for(&out, 1).as_slice(),
        [ClientboundPlayPacket::SetPassengers(update)]
            if update.vehicle_id == boat && update.passenger_ids == [player_id, stored]
    ));
}

#[test]
fn external_player_vehicle_tracking_range_widens_and_terminates_at_player() {
    let mut store = EntityStore::new(0x4558_5457, EntityIdCounter::new());
    let boat =
        crate::entity::spawn_boat(&mut store, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, origin(), 0.0)
            .unwrap();
    assert!(store.attach_test_player(50_002, boat, crate::entity::ExternalPlayerCanRide::Allowed,));
    let distance = 300.0;
    assert!(
        f64::from(
            vanilla_entity_type_client_tracking_range(VANILLA_ENTITY_TYPE_OAK_BOAT_ID).unwrap()
                * CHUNK_BLOCKS
        ) < distance
    );
    assert!(
        f64::from(
            vanilla_entity_type_client_tracking_range(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
            )
            .unwrap()
                * CHUNK_BLOCKS
        ) > distance
    );

    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &[],
        |_, _, _| true,
        &mut out,
    );
    tracker.tick(
        &mut store,
        &[viewer(1, distance)],
        |_, _, _| true,
        |_, _| true,
        &mut out,
    );
    assert!(tracker.viewers_of(boat).contains(&ConnectionId(1)));
}

#[test]
fn external_player_dynamic_geometry_keeps_structural_tracking_range() {
    let mut store = EntityStore::new(0x4558_5458, EntityIdCounter::new());
    let chicken = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CHICKEN_ID, origin())
        .unwrap();
    assert!(store.attach_test_player(
        50_003,
        chicken,
        crate::entity::ExternalPlayerCanRide::Allowed,
    ));
    store
        .get_mut(chicken)
        .unwrap()
        .living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
        .unwrap()
        .set_base_value(1.5);
    assert_eq!(store.external_player_vehicle(50_003), Some(chicken));

    let distance = 300.0;
    assert!(
        f64::from(
            vanilla_entity_type_client_tracking_range(VANILLA_ENTITY_TYPE_CHICKEN_ID).unwrap()
                * CHUNK_BLOCKS
        ) < distance
    );
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &[],
        |_, _, _| true,
        &mut out,
    );
    tracker.tick(
        &mut store,
        &[viewer(1, distance)],
        |_, _, _| true,
        |_, _| true,
        &mut out,
    );
    assert!(tracker.viewers_of(chicken).contains(&ConnectionId(1)));
}

#[test]
fn mounted_updates_suppress_position_and_dismount_resyncs_absolutely() {
    let mut store = EntityStore::new(0x5249_4445, EntityIdCounter::new());
    let vehicle = crate::entity::spawn_minecart(&mut store, origin(), 0.0).unwrap();
    let passenger = crate::entity::spawn_armor_stand(&mut store, origin()).unwrap();
    assert!(store.link_existing_passenger(vehicle, passenger));
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &[viewer(1, 0.0)],
        |_, _, _| true,
        &mut out,
    );
    out.clear();
    {
        let entity = store.get_mut(passenger).unwrap();
        entity.set_pos(Vec3d {
            x: 7.0,
            y: 70.0,
            z: 9.0,
        });
        entity.set_rot(90.0, 15.0);
        crate::entity::armor_stand::set_small(entity, true);
        entity.set_y_head_rot(45.0);
    }

    tracker.tick(
        &mut store,
        &[viewer(1, 0.0)],
        |_, _, _| true,
        |_, _| true,
        &mut out,
    );
    let mounted = packets_for(&out, 1);
    assert!(mounted.iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::MoveEntity(update)
            if update.id == passenger && !update.has_position
    )));
    assert!(mounted.iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::SetEntityData(update) if update.id == passenger
    )));
    assert!(mounted.iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::RotateHead(update) if update.id == passenger
    )));
    assert!(!mounted.iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::EntityPositionSync(update) if update.id == passenger
    )));

    store.stop_invalid_riding(passenger);
    out.clear();
    for _ in 0..3 {
        tracker.tick(
            &mut store,
            &[viewer(1, 0.0)],
            |_, _, _| true,
            |_, _| true,
            &mut out,
        );
    }
    assert!(packets_for(&out, 1).iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::EntityPositionSync(update) if update.id == passenger
    )));
}

#[test]
fn indirect_passenger_range_widens_the_root_tracker() {
    let mut store = EntityStore::new(0x5241_4e47, EntityIdCounter::new());
    let vehicle = crate::entity::spawn_minecart(&mut store, origin(), 0.0).unwrap();
    let passenger = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
            origin(),
        )
        .unwrap();
    assert!(store.link_existing_passenger(vehicle, passenger));
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &[],
        |_, _, _| true,
        &mut out,
    );

    tracker.tick(
        &mut store,
        &[viewer(1, 150.0)],
        |_, _, _| true,
        |_, _| true,
        &mut out,
    );

    assert!(tracker.viewers_of(vehicle).contains(&ConnectionId(1)));
}

#[test]
fn an_item_saved_with_retained_components_publishes_them_in_its_pairing_metadata() {
    let stack = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string("minecraft:stone"),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(1),
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
    .expect("the saved stack has the persistent ItemStack shape");
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = spawn_item_entity(&mut store, origin(), stack).expect("the store accepts an item");
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &[viewer(1, 1.0)],
        |_, _, _| true,
        &mut out,
    );

    let values = packets_for(&out, 1)
        .into_iter()
        .find_map(|packet| match packet {
            ClientboundPlayPacket::SetEntityData(update) if update.id == id => Some(update.values),
            _ => None,
        })
        .expect("the retained stack is sent as item metadata");
    let EntityDataValueKind::ItemStack(stack) = &values[0].value else {
        panic!("the item metadata carries a stack: {values:?}");
    };
    assert_eq!(
        stack.component_patch.custom_data,
        Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "answer".into(),
            value: NbtSummaryValue::Int(42),
        }]))
    );
}

/// The other arm of `updatePlayer` (`:1404-1406`) through
/// `ServerEntity.removePairing` (`ServerEntity.java:257-260`), reached from the
/// `movedPlayers` sweep `ChunkMap.tick` runs (`ChunkMap.java:1207-1211`).
#[test]
fn a_session_that_walks_out_of_the_item_range_is_told_to_forget_it() {
    let (mut store, id) = store_with_one_item(origin());
    let range = f64::from(item_tracking_range_blocks());
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    let inside = vec![viewer(1, range - 1.0)];
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &inside,
        |_, _, _| true,
        &mut out,
    );
    assert_eq!(tracker.viewers_of(id), vec![ConnectionId(1)]);

    // The first walk settles the session's remembered section without moving
    // it, so the pass after it is the one the move is measured against.
    out.clear();
    tracker.tick(
        &mut store,
        &inside,
        |_, _, _| true,
        |_, _| true,
        &mut Vec::new(),
    );
    let outside = vec![viewer(1, range + 1.0)];
    tracker.tick(&mut store, &outside, |_, _, _| true, |_, _| true, &mut out);

    assert!(tracker.viewers_of(id).is_empty());
    assert!(
        packets_for(&out, 1).iter().any(
            |packet| matches!(packet, ClientboundPlayPacket::RemoveEntities(remove)
                if remove.entity_ids == vec![id])
        ),
        "the session that left the radius is told to drop the entity: {out:?}"
    );
}

/// A jump past the view distance leaves the item outside every column the
/// session searches, so only what it already sees can unpair it.
#[test]
fn a_session_that_jumps_far_away_forgets_what_it_saw() {
    let (mut store, id) = store_with_one_item(origin());
    let mut tracker = EntityTracker::new();
    let near = vec![viewer(1, 1.0)];
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &near,
        |_, _, _| true,
        &mut Vec::new(),
    );
    tracker.tick(
        &mut store,
        &near,
        |_, _, _| true,
        |_, _| true,
        &mut Vec::new(),
    );
    assert_eq!(tracker.viewers_of(id), vec![ConnectionId(1)]);

    let mut out = Vec::new();
    let far = vec![viewer(1, 100_000.0)];
    tracker.tick(&mut store, &far, |_, _, _| true, |_, _| true, &mut out);

    assert!(tracker.viewers_of(id).is_empty());
    assert!(packets_for(&out, 1).iter().any(
        |packet| matches!(packet, ClientboundPlayPacket::RemoveEntities(remove)
            if remove.entity_ids == vec![id])
    ));
}

/// A session that never entered the radius hears nothing at all, however long
/// the tracker runs -- the interval gate publishes to `seenBy`, which is empty.
#[test]
fn a_session_outside_the_item_range_hears_nothing_while_the_item_moves() {
    let (mut store, id) = store_with_one_item(origin());
    let range = f64::from(item_tracking_range_blocks());
    let viewers = vec![viewer(1, range + 1.0)];
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut out,
    );
    for _ in 0..(item_update_interval() * 3) {
        let entity = store.get_mut(id).expect("the item is still in the store");
        let position = entity.position();
        entity.set_pos(Vec3d {
            x: position.x + 0.25,
            ..position
        });
        tracker.tick(&mut store, &viewers, |_, _, _| true, |_, _| true, &mut out);
    }
    assert!(
        out.is_empty(),
        "nothing reaches an unpaired session: {out:?}"
    );
}

/// `ChunkMap.isChunkTracked` (`ChunkMap.java:227-229`) is the second half of
/// the visibility test: a session that has not been sent the entity's column
/// is not paired even standing on top of it.
#[test]
fn a_session_still_waiting_for_the_column_is_not_paired_with_what_is_in_it() {
    let (mut store, id) = store_with_one_item(origin());
    let viewers = vec![viewer(1, 1.0)];
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| false,
        &mut out,
    );
    assert!(tracker.viewers_of(id).is_empty());
    assert!(out.is_empty());
}

/// `ServerEntity.sendChanges`' interval gate (`ServerEntity.java:120`): the
/// whole body runs on one pass in `updateInterval`, so an item that moves every
/// tick is still only published every twentieth.
#[test]
fn the_item_update_interval_gates_every_pass_of_send_changes() {
    let (mut store, id) = store_with_one_item(origin());
    let interval = item_update_interval();
    let mut server_entity = ServerEntity::new(
        store.get(id).expect("the item is in the store"),
        interval,
        true,
    );
    let mut passes_that_published = Vec::new();
    for pass in 0..(interval * 3) {
        let entity = store.get_mut(id).expect("the item is still in the store");
        let position = entity.position();
        entity.set_pos(Vec3d {
            x: position.x + 0.25,
            ..position
        });
        if !server_entity
            .send_changes(store.get_mut(id).expect("the item is still in the store"))
            .is_empty()
        {
            passes_that_published.push(pass);
        }
    }
    assert_eq!(
        passes_that_published,
        vec![0, interval, interval * 2],
        "only a pass on the interval publishes"
    );
}

/// `Entity.needsSync` (`Entity.java:274`) opens the gate whatever the interval
/// says (`ServerEntity.java:120`), and `sendChanges` clears it again (`:216`).
#[test]
fn a_needs_sync_entity_publishes_off_the_interval_and_then_stops() {
    let (mut store, id) = store_with_one_item(origin());
    let interval = item_update_interval();
    let mut server_entity = ServerEntity::new(
        store.get(id).expect("the item is in the store"),
        interval,
        true,
    );
    // Pass zero is on the interval, so step past it before raising the flag.
    server_entity.send_changes(store.get_mut(id).expect("the item is in the store"));
    let entity = store.get_mut(id).expect("the item is in the store");
    let position = entity.position();
    entity.set_pos(Vec3d {
        x: position.x + 0.25,
        ..position
    });
    entity.set_needs_sync(true);
    assert!(!server_entity
        .send_changes(store.get_mut(id).expect("the item is in the store"))
        .is_empty());
    assert!(!store
        .get(id)
        .expect("the item is in the store")
        .needs_sync());

    let entity = store.get_mut(id).expect("the item is in the store");
    let position = entity.position();
    entity.set_pos(Vec3d {
        x: position.x + 0.25,
        ..position
    });
    assert!(
        server_entity
            .send_changes(store.get_mut(id).expect("the item is in the store"))
            .is_empty(),
        "the flag is cleared, so the interval owns the next pass again"
    );
}

/// `ServerEntity.FORCED_TELEPORT_PERIOD` (`ServerEntity.java:55`, `:150`):
/// `teleportDelay` rises on every pass that runs and the pass that takes it
/// past four hundred resyncs absolutely, even for an entity that never moved.
#[test]
fn four_hundred_passes_without_an_absolute_sync_force_one() {
    let (mut store, id) = store_with_one_item(origin());
    // One tick per pass so the pass index is the delay, which is the boundary
    // under test.
    let mut server_entity =
        ServerEntity::new(store.get(id).expect("the item is in the store"), 1, true);
    let mut first_sync = None;
    for pass in 0..=(FORCED_TELEPORT_PERIOD + 1) {
        let packets =
            server_entity.send_changes(store.get_mut(id).expect("the item is in the store"));
        if packets
            .iter()
            .any(|packet| matches!(packet, ClientboundPlayPacket::EntityPositionSync(_)))
        {
            first_sync = Some(pass);
            break;
        }
    }
    // The delay is incremented before the test, so the pass that first sees
    // `teleportDelay > 400` is pass four hundred.
    assert_eq!(first_sync, Some(FORCED_TELEPORT_PERIOD));
}

/// `ServerEntity.FORCED_POS_UPDATE_PERIOD` (`ServerEntity.java:54`, `:141`): a
/// resting entity is still told about once a minute, and the relative packet it
/// gets carries three zero deltas rather than an absolute position.
#[test]
fn a_resting_entity_is_republished_every_sixty_passes() {
    let (mut store, id) = store_with_one_item(origin());
    let mut server_entity =
        ServerEntity::new(store.get(id).expect("the item is in the store"), 1, true);
    let mut published = Vec::new();
    for pass in 0..(FORCED_POS_UPDATE_PERIOD * 2) {
        let packets =
            server_entity.send_changes(store.get_mut(id).expect("the item is in the store"));
        if !packets.is_empty() {
            published.push((pass, packets));
        }
    }
    let passes: Vec<i32> = published.iter().map(|(pass, _)| *pass).collect();
    assert_eq!(passes, vec![0, FORCED_POS_UPDATE_PERIOD]);
    for (_, packets) in &published {
        assert!(matches!(
            packets.as_slice(),
            [ClientboundPlayPacket::MoveEntity(update)]
                if update.has_position
                    && (update.delta_x, update.delta_y, update.delta_z) == (0, 0, 0)
                    && update.y_rot.is_none()
        ));
    }
}

/// `ServerEntity.sendChanges`' velocity arm (`ServerEntity.java:172-191`),
/// which runs for these kinds because `EntityType.trackDeltas()` is true for
/// both (`EntityType.java:1556-1567`).
#[test]
fn a_change_of_velocity_publishes_a_motion_packet() {
    let (mut store, id) = store_with_one_item(origin());
    let mut server_entity =
        ServerEntity::new(store.get(id).expect("the item is in the store"), 1, true);
    server_entity.send_changes(store.get_mut(id).expect("the item is in the store"));
    let moved = Vec3d {
        x: 0.0,
        y: -0.5,
        z: 0.0,
    };
    store
        .get_mut(id)
        .expect("the item is in the store")
        .set_delta_movement(moved);
    let packets = server_entity.send_changes(store.get_mut(id).expect("the item is in the store"));
    assert!(
        packets.iter().any(
            |packet| matches!(packet, ClientboundPlayPacket::SetEntityMotion(motion)
                if motion.id == id && motion.delta_movement == moved)
        ),
        "a velocity worth predicting from is published: {packets:?}"
    );
    assert_eq!(server_entity.last_sent_movement(), moved);
}

#[test]
fn hurting_projectile_motion_and_power_are_one_inner_bundle() {
    let mut entity = Entity::new(
        41,
        uuid::Uuid::from_u128(41),
        VANILLA_ENTITY_TYPE_FIREBALL_ID,
        origin(),
        EntityData::LargeFireball(crate::entity::LargeFireballData::new(
            crate::entity::HurtingProjectileState::new(None, 0.125),
            plain_stack("minecraft:fire_charge", 1).unwrap(),
            1,
        )),
    );
    let mut server_entity = ServerEntity::new(&entity, 1, true);
    let _ = server_entity.send_changes(&mut entity);
    entity.set_delta_movement(Vec3d {
        x: 0.25,
        y: 0.0,
        z: 0.0,
    });
    let packets = server_entity.send_changes(&mut entity);
    let bundle = packets
        .windows(4)
        .find(|window| {
            matches!(window[0], ClientboundPlayPacket::BundleDelimiter)
                && matches!(window[1], ClientboundPlayPacket::SetEntityMotion(_))
                && matches!(window[2], ClientboundPlayPacket::ProjectilePower(_))
                && matches!(window[3], ClientboundPlayPacket::BundleDelimiter)
        })
        .expect("motion and power must remain adjacent inside delimiters");
    assert!(matches!(
        bundle[2],
        ClientboundPlayPacket::ProjectilePower(ProjectilePower {
            entity_id: 41,
            acceleration_power: 0.125,
        })
    ));
}

#[test]
fn projectile_add_packet_resolves_a_stored_owner_uuid() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let owner_uuid = uuid::Uuid::from_u128(70);
    let owner_id = store
        .restore(
            owner_uuid,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            origin(),
            EntityData::Item(crate::entity::ItemEntityData::new(
                plain_stack("minecraft:stone", 1).unwrap(),
                0.0,
            )),
        )
        .unwrap();
    let fireball_id = store
        .restore(
            uuid::Uuid::from_u128(71),
            VANILLA_ENTITY_TYPE_FIREBALL_ID,
            origin(),
            EntityData::LargeFireball(crate::entity::LargeFireballData::new(
                crate::entity::HurtingProjectileState::new(Some(owner_uuid), 0.1),
                plain_stack("minecraft:fire_charge", 1).unwrap(),
                1,
            )),
        )
        .unwrap();
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &[viewer(1, 1.0)],
        |_, _, _| true,
        &mut out,
    );
    assert!(recipient_packets(&out).any(|(_, packet)| matches!(
        packet,
        ClientboundPlayPacket::AddEntity(add)
            if add.id == fireball_id && add.data == owner_id
    )));
}

/// `ServerEntity.sendChanges` consumes `hurtMarked` after the normal interval
/// gate (`ServerEntity.java:219-223`), so damage publishes the current motion
/// even on a pass the entity type would otherwise skip.
#[test]
fn hurt_marked_resends_motion_once_outside_the_update_interval() {
    let (mut store, id) = store_with_one_item(origin());
    let entity = store.get_mut(id).expect("the item is in the store");
    let moved = Vec3d {
        x: 0.25,
        y: 0.5,
        z: -0.25,
    };
    entity.set_delta_movement(moved);
    let mut server_entity = ServerEntity::new(entity, item_update_interval(), true);
    // Tick zero is itself an interval pass (and a forced position pass), so
    // advance past it before raising the damage-only flag.
    let _ = server_entity.send_changes(entity);
    entity.mark_hurt();

    // Construction captured the new motion, so only hurtMarked can cause a
    // packet; the ordinary difference is exactly zero.
    let packets = server_entity.send_changes(entity);
    assert!(matches!(
        packets.as_slice(),
        [ClientboundPlayPacket::SetEntityMotion(motion)]
            if motion.id == id && motion.delta_movement == moved
    ));
    assert!(!entity.hurt_marked());
    assert!(server_entity.send_changes(entity).is_empty());
}

/// A damage-and-discard item is removed from the tracker before its next
/// `sendChanges`; its retained hurt flag must not leak a motion packet after
/// `RemoveEntities`.
#[test]
fn a_removed_hurt_entity_emits_no_stray_motion_packet() {
    let (mut store, id) = store_with_one_item(origin());
    let viewers = vec![viewer(1, 1.0)];
    let mut tracker = EntityTracker::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut Vec::new(),
    );
    store
        .get_mut(id)
        .expect("the item remains live")
        .mark_hurt();
    store.remove(id, crate::entity::RemovalReason::Discarded);

    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut out,
    );
    assert!(matches!(
        packets_for(&out, 1).as_slice(),
        [ClientboundPlayPacket::RemoveEntities(remove)]
            if remove.entity_ids == vec![id]
    ));
}

/// `ChunkMap.removeEntity` (`ChunkMap.java:1165-1178`) reaching
/// `broadcastRemoved` (`:1370-1374`): the entity leaving the store un-pairs it
/// from everybody still watching.
#[test]
fn an_entity_that_leaves_the_store_is_dropped_by_every_session_watching_it() {
    let (mut store, id) = store_with_one_item(origin());
    let viewers = vec![viewer(1, 1.0), viewer(2, 2.0)];
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut out,
    );
    assert_eq!(
        tracker.viewers_of(id),
        vec![ConnectionId(1), ConnectionId(2)]
    );

    out.clear();
    store.remove(id, crate::entity::RemovalReason::Discarded);
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut out,
    );
    assert!(tracker.is_empty());
    for connection in [1, 2] {
        assert!(
            packets_for(&out, connection).iter().any(
                |packet| matches!(packet, ClientboundPlayPacket::RemoveEntities(remove)
                    if remove.entity_ids == vec![id])
            ),
            "session {connection} is told the entity is gone: {out:?}"
        );
    }
}

#[test]
fn visibility_tracker_hide_show_pairs_the_same_entity_identity() {
    let (mut store, id) = store_with_one_item(origin());
    let before = store.get(id).unwrap().clone();
    let viewers = [viewer(1, 1.0)];
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut out,
    );
    out.clear();
    store.apply_chunk_visibility((0, 0), crate::entity::EntityVisibility::Hidden);
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut out,
    );
    assert!(tracker.is_empty());
    assert_eq!(store.get(id), Some(&before));
    assert!(recipient_packets(&out).any(|(_, packet)| matches!(packet, ClientboundPlayPacket::RemoveEntities(remove) if remove.entity_ids == [id])));
    out.clear();
    store.apply_chunk_visibility((0, 0), crate::entity::EntityVisibility::Tracked);
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut out,
    );
    assert_eq!(tracker.viewers_of(id), [viewers[0].connection_id]);
    assert!(!store.is_ticking(id));
    assert!(recipient_packets(&out).any(|(_, packet)| matches!(packet, ClientboundPlayPacket::AddEntity(add) if add.id == id && add.uuid == before.uuid())));
}

/// `ChunkMap.tick`'s entity-ticking gate (`ChunkMap.java:1202`): an entity in a
/// column outside the simulation distance is not published, however far it
/// moved, until its own section changes.
#[test]
fn an_entity_outside_the_entity_ticking_range_is_not_published() {
    let (mut store, id) = store_with_one_item(origin());
    let viewers = vec![viewer(1, 1.0)];
    let mut tracker = EntityTracker::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut Vec::new(),
    );
    tracker.tick(
        &mut store,
        &viewers,
        |_, _, _| true,
        |_, _| true,
        &mut Vec::new(),
    );

    let mut out = Vec::new();
    for _ in 0..item_update_interval() {
        let entity = store.get_mut(id).expect("the item is in the store");
        let position = entity.position();
        entity.set_pos(Vec3d {
            x: position.x + 0.25,
            ..position
        });
        tracker.tick(&mut store, &viewers, |_, _, _| true, |_, _| false, &mut out);
    }
    assert!(
        out.is_empty(),
        "a frozen-out column publishes nothing: {out:?}"
    );
}

/// `Entity.ENTITY_COUNTER` (`Entity.java:189`, `:210`) is one counter for the
/// process, so two levels sharing it never mint the same network id -- which is
/// what keeps a tracker's `RemoveEntities` from naming somebody else's entity.
#[test]
fn two_stores_sharing_the_process_counter_never_mint_the_same_id() {
    let ids = EntityIdCounter::new();
    let mut overworld = EntityStore::new(1, ids.clone());
    let mut nether = EntityStore::new(2, ids);
    let stack = plain_stack("minecraft:stone", 1).expect("stone is an item");
    let first = spawn_item_entity(&mut overworld, origin(), stack.clone()).expect("spawned");
    let second = spawn_item_entity(&mut nether, origin(), stack.clone()).expect("spawned");
    let third = spawn_item_entity(&mut overworld, origin(), stack).expect("spawned");
    assert_eq!([first, second, third], [1, 2, 3]);
}

/// `FallingBlockEntity.getAddEntityPacket` (`FallingBlockEntity.java:339-341`)
/// is the one override among these kinds: its `data` int is the block state the
/// client draws, which is the whole of what makes a falling block visible.
#[test]
fn a_falling_block_is_added_with_the_block_state_it_carries() {
    let sand = vanilla_block_default_state_id(
        vanilla_block_type_for_resource_id("minecraft:sand")
            .expect("sand is a built-in block")
            .id,
    )
    .expect("sand has a default state");
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let start = BlockPos { x: 0, y: 70, z: 0 };
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 0.5,
            y: 70.0,
            z: 0.5,
        },
        EntityData::FallingBlock(FallingBlockData::new(sand, start)),
    );
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &[viewer(1, 1.0)],
        |_, _, _| true,
        &mut out,
    );

    let paired = packets_for(&out, 1);
    assert!(
        matches!(
            paired.as_slice(),
            [
                ClientboundPlayPacket::BundleDelimiter,
                ClientboundPlayPacket::AddEntity(add),
                ClientboundPlayPacket::SetEntityData(data),
                ClientboundPlayPacket::BundleDelimiter,
            ]
                if add.id == id
                    && add.entity_type_id == VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID
                    && add.data == sand
                    && data.id == id
                    && data.values == [EntityDataValue {
                        data_id: 8,
                        serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_POS_ID,
                        value: EntityDataValueKind::BlockPos(start),
                    }]
        ),
        "a falling block is added with its state and DATA_START_POS: {paired:?}"
    );
}

fn store_with_one_armor_stand() -> (EntityStore, i32) {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = crate::entity::armor_stand::spawn(&mut store, origin()).expect("a fresh UUID");
    (store, id)
}

#[test]
fn general_zombie_pairs_with_add_and_living_metadata_then_publishes_dirty_health() {
    let mut store = EntityStore::new(31, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, origin())
        .expect("zombie has generated default attributes");
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &[viewer(1, 1.0)],
        |_, _, _| true,
        &mut out,
    );
    let paired = packets_for(&out, 1);
    assert!(paired.iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::AddEntity(add)
            if add.id == id
                && add.entity_type_id == VANILLA_ENTITY_TYPE_ZOMBIE_ID
                && add.data == 0
    )));
    assert!(paired.iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::SetEntityData(data)
            if data.id == id
                && data.values.iter().any(|row| {
                    row.data_id
                        == bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_HEALTH_DATA_ID
                })
    )));

    crate::entity::living::set_health(store.get_mut(id).unwrap(), 7.0);
    out.clear();
    tracker.tick(
        &mut store,
        &[viewer(1, 1.0)],
        |_, _, _| true,
        |_, _| true,
        &mut out,
    );
    assert!(packets_for(&out, 1).iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::SetEntityData(data)
            if data.values.iter().any(|row| {
                row.data_id
                    == bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_HEALTH_DATA_ID
                    && row.value == EntityDataValueKind::Float(7.0)
            })
    )));
}

#[test]
fn mob_ai_runtime_tracking_projects_mob_flags_but_not_mannequin_metadata() {
    let mut zombie = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap();
    zombie.set_no_ai(true);
    zombie.set_left_handed(true);
    let entity = Entity::new(
        71,
        uuid::Uuid::from_u128(71),
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        origin(),
        EntityData::GeneralLiving(zombie),
    );
    let values = non_default_synched_values(&entity);
    assert!(values.iter().any(|value| {
        value.data_id == bbb_protocol::entity_synched_data::VANILLA_MOB_FLAGS_DATA_ID
            && value.serializer_id
                == bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID
            && value.value == EntityDataValueKind::Byte(3)
    }));

    let mannequin_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID;
    let mannequin = Entity::new(
        72,
        uuid::Uuid::from_u128(72),
        mannequin_type,
        origin(),
        EntityData::GeneralLiving(crate::entity::GeneralLivingData::new(mannequin_type).unwrap()),
    );
    assert!(non_default_synched_values(&mannequin).iter().all(|value| {
        value.data_id != bbb_protocol::entity_synched_data::VANILLA_MOB_FLAGS_DATA_ID
    }));
}

#[test]
fn ageable_mob_runtime_tracking_projects_baby_and_age_locked_rows() {
    let mut data = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap();
    *data.ageable_retained_mut().unwrap() =
        crate::entity::general_living::AgeableMobRetainedState::from_saved(-1, 0, true);
    let cow = Entity::new(
        73,
        uuid::Uuid::from_u128(73),
        VANILLA_ENTITY_TYPE_COW_ID,
        origin(),
        EntityData::GeneralLiving(data),
    );
    let values = non_default_synched_values(&cow);
    for data_id in [
        bbb_protocol::entity_synched_data::VANILLA_AGEABLE_MOB_BABY_DATA_ID,
        bbb_protocol::entity_synched_data::VANILLA_AGEABLE_MOB_AGE_LOCKED_DATA_ID,
    ] {
        assert!(values.iter().any(|value| {
            value.data_id == data_id
                && value.serializer_id
                    == bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID
                && value.value == EntityDataValueKind::Boolean(true)
        }));
    }

    let fresh = Entity::new(
        74,
        uuid::Uuid::from_u128(74),
        VANILLA_ENTITY_TYPE_COW_ID,
        origin(),
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap(),
        ),
    );
    for data_id in [
        bbb_protocol::entity_synched_data::VANILLA_AGEABLE_MOB_BABY_DATA_ID,
        bbb_protocol::entity_synched_data::VANILLA_AGEABLE_MOB_AGE_LOCKED_DATA_ID,
    ] {
        let row = synched_values(&fresh)
            .into_iter()
            .find(|row| row.value.data_id == data_id)
            .unwrap();
        assert!(row.is_default);
        assert_eq!(row.value.value, EntityDataValueKind::Boolean(false));
    }
}

#[test]
fn abstract_horse_runtime_tracking_projects_exact_flag_byte() {
    let mut data = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_HORSE_ID).unwrap();
    *data.abstract_horse_retained_mut().unwrap() =
        crate::entity::general_living::AbstractHorseRetainedState::from_saved(
            true, true, 0, true, None,
        );
    let horse = Entity::new(
        75,
        uuid::Uuid::from_u128(75),
        VANILLA_ENTITY_TYPE_HORSE_ID,
        origin(),
        EntityData::GeneralLiving(data),
    );
    let values = non_default_synched_values(&horse);
    assert!(values.iter().any(|value| {
        value.data_id == bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID
            && value.serializer_id
                == bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID
            && value.value
                == EntityDataValueKind::Byte(
                    bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_HORSE_FLAG_TAME
                        | bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_HORSE_FLAG_BRED
                        | bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_HORSE_FLAG_EATING,
                )
    }));

    let fresh = Entity::new(
        76,
        uuid::Uuid::from_u128(76),
        VANILLA_ENTITY_TYPE_HORSE_ID,
        origin(),
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_HORSE_ID).unwrap(),
        ),
    );
    let row = synched_values(&fresh)
        .into_iter()
        .find(|row| {
            row.value.data_id
                == bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID
        })
        .unwrap();
    assert!(row.is_default);
    assert_eq!(row.value.value, EntityDataValueKind::Byte(0));
}

#[test]
fn bat_autonomous_runtime_tracking_projects_exact_flag_byte_and_default() {
    let mut data = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_BAT_ID).unwrap();
    *data.bat_retained_mut().unwrap() =
        crate::entity::general_living::BatRetainedState::from_saved(0x55);
    let bat = Entity::new(
        77,
        uuid::Uuid::from_u128(77),
        VANILLA_ENTITY_TYPE_BAT_ID,
        origin(),
        EntityData::GeneralLiving(data),
    );
    let values = non_default_synched_values(&bat);
    assert!(values.iter().any(|value| {
        value.data_id == bbb_protocol::entity_synched_data::VANILLA_BAT_FLAGS_DATA_ID
            && value.serializer_id
                == bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID
            && value.value == EntityDataValueKind::Byte(0x55)
    }));

    let mut data = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_BAT_ID).unwrap();
    *data.bat_retained_mut().unwrap() =
        crate::entity::general_living::BatRetainedState::from_saved(0);
    let fresh = Entity::new(
        78,
        uuid::Uuid::from_u128(78),
        VANILLA_ENTITY_TYPE_BAT_ID,
        origin(),
        EntityData::GeneralLiving(data),
    );
    let row = synched_values(&fresh)
        .into_iter()
        .find(|row| {
            row.value.data_id == bbb_protocol::entity_synched_data::VANILLA_BAT_FLAGS_DATA_ID
        })
        .unwrap();
    assert!(row.is_default);
    assert_eq!(row.value.value, EntityDataValueKind::Byte(0));
}

#[test]
fn vex_autonomous_runtime_tracking_projects_exact_flag_byte_and_default() {
    let mut data = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_VEX_ID).unwrap();
    data.vex_retained_mut().unwrap().set_flags(0x55);
    let vex = Entity::new(
        79,
        uuid::Uuid::from_u128(79),
        VANILLA_ENTITY_TYPE_VEX_ID,
        origin(),
        EntityData::GeneralLiving(data),
    );
    let values = non_default_synched_values(&vex);
    assert!(values.iter().any(|value| {
        value.data_id == bbb_protocol::entity_synched_data::VANILLA_VEX_FLAGS_DATA_ID
            && value.serializer_id
                == bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID
            && value.value == EntityDataValueKind::Byte(0x55)
    }));

    let fresh = Entity::new(
        80,
        uuid::Uuid::from_u128(80),
        VANILLA_ENTITY_TYPE_VEX_ID,
        origin(),
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_VEX_ID).unwrap(),
        ),
    );
    let row = synched_values(&fresh)
        .into_iter()
        .find(|row| {
            row.value.data_id == bbb_protocol::entity_synched_data::VANILLA_VEX_FLAGS_DATA_ID
        })
        .unwrap();
    assert!(row.is_default);
    assert_eq!(row.value.value, EntityDataValueKind::Byte(0));
}

#[test]
fn creeper_runtime_tracking_projects_three_owned_rows_in_order() {
    let mut data = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_CREEPER_ID).unwrap();
    *data.creeper_retained_mut().unwrap() =
        crate::entity::general_living::CreeperRetainedState::from_saved(true, 30, 3, true);
    let creeper = Entity::new(
        81,
        uuid::Uuid::from_u128(81),
        VANILLA_ENTITY_TYPE_CREEPER_ID,
        origin(),
        EntityData::GeneralLiving(data),
    );
    let rows = synched_values(&creeper);
    let owned = rows
        .iter()
        .filter(|row| {
            matches!(
                row.value.data_id,
                bbb_protocol::entity_synched_data::VANILLA_CREEPER_SWELL_DIR_DATA_ID
                    | bbb_protocol::entity_synched_data::VANILLA_CREEPER_POWERED_DATA_ID
                    | bbb_protocol::entity_synched_data::VANILLA_CREEPER_IGNITED_DATA_ID
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(owned.len(), 3);
    assert_eq!(
        owned
            .iter()
            .map(|row| row.value.data_id)
            .collect::<Vec<_>>(),
        vec![
            bbb_protocol::entity_synched_data::VANILLA_CREEPER_SWELL_DIR_DATA_ID,
            bbb_protocol::entity_synched_data::VANILLA_CREEPER_POWERED_DATA_ID,
            bbb_protocol::entity_synched_data::VANILLA_CREEPER_IGNITED_DATA_ID,
        ]
    );
    assert_eq!(owned[0].value.value, EntityDataValueKind::Int(-1));
    assert_eq!(owned[1].value.value, EntityDataValueKind::Boolean(true));
    assert_eq!(owned[2].value.value, EntityDataValueKind::Boolean(true));
    assert!(owned[0].is_default);
    assert!(!owned[1].is_default);
    assert!(!owned[2].is_default);
}

#[test]
fn strider_environment_runtime_tracking_projects_two_owned_rows_in_order() {
    let mut data = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_STRIDER_ID).unwrap();
    data.strider_state_mut()
        .unwrap()
        .set_suffocating_for_test(true);
    let strider = Entity::new(
        82,
        uuid::Uuid::from_u128(82),
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        origin(),
        EntityData::GeneralLiving(data),
    );
    let rows = synched_values(&strider);
    let owned = rows
        .iter()
        .filter(|row| {
            matches!(
                row.value.data_id,
                bbb_protocol::entity_synched_data::VANILLA_STRIDER_BOOST_TIME_DATA_ID
                    | bbb_protocol::entity_synched_data::VANILLA_STRIDER_SUFFOCATING_DATA_ID
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(owned.len(), 2);
    assert_eq!(
        owned
            .iter()
            .map(|row| row.value.data_id)
            .collect::<Vec<_>>(),
        vec![
            bbb_protocol::entity_synched_data::VANILLA_STRIDER_BOOST_TIME_DATA_ID,
            bbb_protocol::entity_synched_data::VANILLA_STRIDER_SUFFOCATING_DATA_ID,
        ]
    );
    assert_eq!(owned[0].value.value, EntityDataValueKind::Int(0));
    assert_eq!(owned[1].value.value, EntityDataValueKind::Boolean(true));
    assert!(owned[0].is_default);
    assert!(!owned[1].is_default);

    let fresh = Entity::new(
        83,
        uuid::Uuid::from_u128(83),
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        origin(),
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_STRIDER_ID).unwrap(),
        ),
    );
    let defaults = synched_values(&fresh)
        .into_iter()
        .filter(|row| {
            matches!(
                row.value.data_id,
                bbb_protocol::entity_synched_data::VANILLA_STRIDER_BOOST_TIME_DATA_ID
                    | bbb_protocol::entity_synched_data::VANILLA_STRIDER_SUFFOCATING_DATA_ID
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(defaults.len(), 2);
    assert!(defaults.iter().all(|row| row.is_default));
}

#[test]
fn bucketable_despawn_tracking_projects_exact_name_and_class_specific_flag_rows() {
    let component =
        bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".into(),
                value: NbtSummaryValue::string("Tracked fish"),
            },
            NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::string("aqua"),
            },
        ]))
        .unwrap();
    let mut cod_data = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_COD_ID).unwrap();
    assert!(cod_data.set_from_bucket(true));
    let mut cod = Entity::new(
        73,
        uuid::Uuid::from_u128(73),
        VANILLA_ENTITY_TYPE_COD_ID,
        origin(),
        EntityData::GeneralLiving(cod_data),
    );
    cod.set_custom_name(Some(component.clone()));
    cod.set_custom_name_visible(true);
    let values = non_default_synched_values(&cod);
    assert!(values.iter().any(|value| {
        value.data_id == bbb_protocol::entity_synched_data::VANILLA_ENTITY_CUSTOM_NAME_DATA_ID
            && value.value == EntityDataValueKind::OptionalComponent(Some(component.clone()))
    }));
    assert!(values.iter().any(|value| {
        value.data_id
            == bbb_protocol::entity_synched_data::VANILLA_ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID
            && value.value == EntityDataValueKind::Boolean(true)
    }));
    assert!(values.iter().any(|value| {
        value.data_id
            == bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_FISH_FROM_BUCKET_DATA_ID
            && value.serializer_id
                == bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID
            && value.value == EntityDataValueKind::Boolean(true)
    }));

    let mut axolotl_data =
        crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_AXOLOTL_ID).unwrap();
    assert!(axolotl_data.set_from_bucket(true));
    let axolotl = Entity::new(
        74,
        uuid::Uuid::from_u128(74),
        VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        origin(),
        EntityData::GeneralLiving(axolotl_data),
    );
    assert!(non_default_synched_values(&axolotl).iter().any(|value| {
        value.data_id == bbb_protocol::entity_synched_data::VANILLA_AXOLOTL_FROM_BUCKET_DATA_ID
            && value.value == EntityDataValueKind::Boolean(true)
    }));

    let tadpole = Entity::new(
        75,
        uuid::Uuid::from_u128(75),
        VANILLA_ENTITY_TYPE_TADPOLE_ID,
        origin(),
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_TADPOLE_ID).unwrap(),
        ),
    );
    let row = synched_values(&tadpole)
        .into_iter()
        .find(|row| {
            row.value.data_id
                == bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_FISH_FROM_BUCKET_DATA_ID
        })
        .unwrap();
    assert!(row.is_default);
    assert_eq!(row.value.value, EntityDataValueKind::Boolean(false));
}

#[test]
fn ageable_water_runtime_tracking_keeps_subtype_order_and_defaults() {
    let mut dolphin_data = crate::entity::GeneralLivingData::new(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID,
    )
    .unwrap();
    *dolphin_data.ageable_water_retained_mut().unwrap() =
        crate::entity::general_living::AgeableWaterCreatureRetainedState::from_saved_dolphin(
            true, 23,
        );
    let dolphin = Entity::new(
        74,
        uuid::Uuid::from_u128(74),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID,
        origin(),
        EntityData::GeneralLiving(dolphin_data),
    );
    let dolphin_rows = synched_values(&dolphin);
    let subtype = dolphin_rows
        .iter()
        .filter(|row| row.value.data_id >= 18)
        .map(|row| (row.value.data_id, row.value.value.clone(), row.is_default))
        .collect::<Vec<_>>();
    assert_eq!(
        subtype,
        vec![
            (
                bbb_protocol::entity_synched_data::VANILLA_DOLPHIN_GOT_FISH_DATA_ID,
                EntityDataValueKind::Boolean(true),
                false,
            ),
            (
                bbb_protocol::entity_synched_data::VANILLA_DOLPHIN_MOISTNESS_DATA_ID,
                EntityDataValueKind::Int(23),
                false,
            ),
        ]
    );

    let fresh = Entity::new(
        75,
        uuid::Uuid::from_u128(75),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID,
        origin(),
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID,
            )
            .unwrap(),
        ),
    );
    let fresh_rows = synched_values(&fresh);
    assert!(fresh_rows.iter().any(|row| {
        row.value.data_id == bbb_protocol::entity_synched_data::VANILLA_DOLPHIN_GOT_FISH_DATA_ID
            && row.is_default
            && row.value.value == EntityDataValueKind::Boolean(false)
    }));
    assert!(fresh_rows.iter().any(|row| {
        row.value.data_id == bbb_protocol::entity_synched_data::VANILLA_DOLPHIN_MOISTNESS_DATA_ID
            && row.is_default
            && row.value.value == EntityDataValueKind::Int(2400)
    }));

    let mut glow_data = crate::entity::GeneralLivingData::new(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
    )
    .unwrap();
    *glow_data.ageable_water_retained_mut().unwrap() =
        crate::entity::general_living::AgeableWaterCreatureRetainedState::from_saved_glow_squid(41);
    let glow = Entity::new(
        76,
        uuid::Uuid::from_u128(76),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
        origin(),
        EntityData::GeneralLiving(glow_data),
    );
    assert!(non_default_synched_values(&glow).iter().any(|row| {
        row.data_id == bbb_protocol::entity_synched_data::VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID
            && row.value == EntityDataValueKind::Int(41)
    }));

    let squid = Entity::new(
        77,
        uuid::Uuid::from_u128(77),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SQUID_ID,
        origin(),
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SQUID_ID,
            )
            .unwrap(),
        ),
    );
    assert!(synched_values(&squid)
        .iter()
        .all(|row| row.value.data_id < 18));
}

#[test]
fn tamable_despawn_tracking_projects_exact_flags_and_owner_rows() {
    let owner = uuid::Uuid::from_u128(0xCA7);
    let mut data = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_CAT_ID).unwrap();
    *data.tamable_retained_mut().unwrap() =
        crate::entity::general_living::TamableRetainedState::from_saved(Some(owner), true);
    let cat = Entity::new(
        76,
        uuid::Uuid::from_u128(76),
        VANILLA_ENTITY_TYPE_CAT_ID,
        origin(),
        EntityData::GeneralLiving(data),
    );
    let values = non_default_synched_values(&cat);
    assert!(values.iter().any(|value| {
        value.data_id == bbb_protocol::entity_synched_data::VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID
            && value.serializer_id
                == bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID
            && value.value == EntityDataValueKind::Byte(5)
    }));
    assert!(values.iter().any(|value| {
        value.data_id
            == bbb_protocol::entity_synched_data::VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID
            && value.serializer_id
                == bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID
            && value.value == EntityDataValueKind::OptionalLivingEntityReference(Some(owner))
    }));

    let fresh = Entity::new(
        77,
        uuid::Uuid::from_u128(77),
        VANILLA_ENTITY_TYPE_CAT_ID,
        origin(),
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_CAT_ID).unwrap(),
        ),
    );
    let rows = synched_values(&fresh);
    let flags = rows
        .iter()
        .find(|row| {
            row.value.data_id
                == bbb_protocol::entity_synched_data::VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID
        })
        .unwrap();
    assert!(flags.is_default);
    assert_eq!(flags.value.value, EntityDataValueKind::Byte(0));
    let owner_row = rows
        .iter()
        .find(|row| {
            row.value.data_id
                == bbb_protocol::entity_synched_data::VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID
        })
        .unwrap();
    assert!(owner_row.is_default);
    assert_eq!(
        owner_row.value.value,
        EntityDataValueKind::OptionalLivingEntityReference(None)
    );
}

#[test]
fn conditional_animal_despawn_tracking_projects_only_ocelot_trusting() {
    let mut data = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_OCELOT_ID).unwrap();
    data.set_ocelot_trusting(true);
    let ocelot = Entity::new(
        78,
        uuid::Uuid::from_u128(78),
        VANILLA_ENTITY_TYPE_OCELOT_ID,
        origin(),
        EntityData::GeneralLiving(data),
    );
    let values = non_default_synched_values(&ocelot);
    assert!(values.iter().any(|value| {
        value.data_id == bbb_protocol::entity_synched_data::VANILLA_OCELOT_TRUSTING_DATA_ID
            && value.serializer_id
                == bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID
            && value.value == EntityDataValueKind::Boolean(true)
    }));

    let fresh = Entity::new(
        79,
        uuid::Uuid::from_u128(79),
        VANILLA_ENTITY_TYPE_OCELOT_ID,
        origin(),
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_OCELOT_ID).unwrap(),
        ),
    );
    let row = synched_values(&fresh)
        .into_iter()
        .find(|row| {
            row.value.data_id == bbb_protocol::entity_synched_data::VANILLA_OCELOT_TRUSTING_DATA_ID
        })
        .unwrap();
    assert!(row.is_default);
    assert_eq!(row.value.value, EntityDataValueKind::Boolean(false));
}

#[test]
fn enderman_carried_despawn_tracking_projects_optional_block_state() {
    let stone = vanilla_block_default_state_id(
        vanilla_block_type_for_resource_id("minecraft:stone")
            .expect("stone is a built-in block")
            .id,
    )
    .expect("stone has a default state");
    let mut data = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ENDERMAN_ID).unwrap();
    *data.enderman_retained_mut().unwrap() =
        crate::entity::general_living::EndermanRetainedState::from_saved(Some(stone));
    let enderman = Entity::new(
        80,
        uuid::Uuid::from_u128(80),
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
        origin(),
        EntityData::GeneralLiving(data),
    );
    let values = non_default_synched_values(&enderman);
    assert!(values.iter().any(|value| {
        value.data_id
            == bbb_protocol::entity_synched_data::VANILLA_ENDERMAN_CARRY_STATE_DATA_ID
            && value.serializer_id
                == bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID
            && value.value == EntityDataValueKind::OptionalBlockState(Some(stone))
    }));

    let fresh = Entity::new(
        81,
        uuid::Uuid::from_u128(81),
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
        origin(),
        EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ENDERMAN_ID).unwrap(),
        ),
    );
    let row = synched_values(&fresh)
        .into_iter()
        .find(|row| {
            row.value.data_id
                == bbb_protocol::entity_synched_data::VANILLA_ENDERMAN_CARRY_STATE_DATA_ID
        })
        .unwrap();
    assert!(row.is_default);
    assert_eq!(
        row.value.value,
        EntityDataValueKind::OptionalBlockState(None)
    );
}

#[test]
fn general_delta_tracking_uses_catalog_true_and_false_facts_without_type_exceptions() {
    assert!(
        vanilla_entity_type(VANILLA_ENTITY_TYPE_ZOMBIE_ID)
            .unwrap()
            .track_deltas
    );
    for entity_type_id in [VANILLA_ENTITY_TYPE_BAT_ID, VANILLA_ENTITY_TYPE_WITHER_ID] {
        assert!(!vanilla_entity_type(entity_type_id).unwrap().track_deltas);
    }

    for (entity_type_id, should_track) in [
        (VANILLA_ENTITY_TYPE_ZOMBIE_ID, true),
        (VANILLA_ENTITY_TYPE_BAT_ID, false),
        (VANILLA_ENTITY_TYPE_WITHER_ID, false),
    ] {
        let mut store = EntityStore::new(i64::from(entity_type_id) + 40, EntityIdCounter::new());
        let id = store
            .spawn_general_living(entity_type_id, origin())
            .expect("the catalog type has generated default attributes");
        let mut tracker = EntityTracker::new();
        tracker.apply_lifecycle_events(
            &store.drain_lifecycle_events(),
            &[viewer(1, 1.0)],
            |_, _, _| true,
            &mut Vec::new(),
        );
        store.get_mut(id).unwrap().set_delta_movement(Vec3d {
            x: 0.25,
            y: 0.0,
            z: 0.0,
        });
        let mut out = Vec::new();
        tracker.tick(
            &mut store,
            &[viewer(1, 1.0)],
            |_, _, _| true,
            |_, _| true,
            &mut out,
        );
        assert_eq!(
            packets_for(&out, 1).iter().any(|packet| matches!(
                packet,
                ClientboundPlayPacket::SetEntityMotion(motion) if motion.id == id
            )),
            should_track,
            "entity type {entity_type_id}"
        );
    }
}

fn fireball_owned_by(entity_id: i32, uuid: u128, owner: uuid::Uuid) -> Entity {
    Entity::new(
        entity_id,
        uuid::Uuid::from_u128(uuid),
        VANILLA_ENTITY_TYPE_FIREBALL_ID,
        origin(),
        EntityData::LargeFireball(crate::entity::LargeFireballData::new(
            crate::entity::HurtingProjectileState::new(Some(owner), 0.1),
            plain_stack("minecraft:fire_charge", 1).unwrap(),
            1,
        )),
    )
}

#[test]
fn a_range_zero_marker_is_never_paired_but_still_names_a_projectile_owner() {
    let owner_uuid = uuid::Uuid::from_u128(93);
    let owner_id = 93;
    assert_eq!(
        vanilla_entity_type(VANILLA_ENTITY_TYPE_MARKER_ID)
            .unwrap()
            .client_tracking_range,
        0
    );
    let marker = Entity::new(
        owner_id,
        owner_uuid,
        VANILLA_ENTITY_TYPE_MARKER_ID,
        origin(),
        EntityData::Marker(crate::entity::MarkerData::new()),
    );
    let projectile = fireball_owned_by(94, 94, owner_uuid);
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &[
            EntityLifecycleEvent::TrackingStarted { entity: marker },
            EntityLifecycleEvent::TrackingStarted { entity: projectile },
        ],
        &[viewer(1, 1.0)],
        |_, _, _| true,
        &mut out,
    );
    assert!(tracker.viewers_of(owner_id).is_empty());
    assert!(recipient_packets(&out).any(|(_, packet)| matches!(
        packet,
        ClientboundPlayPacket::AddEntity(add) if add.id == 94 && add.data == owner_id
    )));
}

pub(crate) fn kind(packet: &ClientboundPlayPacket) -> &'static str {
    match packet {
        ClientboundPlayPacket::AddEntity(_) => "add",
        ClientboundPlayPacket::SetEntityData(_) => "data",
        ClientboundPlayPacket::UpdateAttributes(_) => "attributes",
        ClientboundPlayPacket::SetEquipment(_) => "equipment",
        ClientboundPlayPacket::SetPassengers(_) => "passengers",
        ClientboundPlayPacket::RotateHead(_) => "head",
        ClientboundPlayPacket::MoveEntity(_) => "move",
        ClientboundPlayPacket::SetEntityMotion(_) => "motion",
        ClientboundPlayPacket::BundleDelimiter => "delimiter",
        _ => "other",
    }
}

/// `ServerEntity.sendPairingData` (`ServerEntity.java:269-301`): the add
/// packet, the non-default data snapshot, the syncable attribute instances
/// and the non-empty equipment slots, in that order.
#[test]
fn an_armor_stand_pairs_with_data_attributes_and_equipment_in_order() {
    let (mut store, id) = store_with_one_armor_stand();
    {
        let entity = store.get_mut(id).unwrap();
        let living = entity.living_mut().unwrap();
        living.set_item_slot(
            bbb_protocol::equipment_slots::EquipmentSlot::Feet,
            plain_stack("minecraft:iron_boots", 1),
        );
        // `AttributeMap.getInstance` (`AttributeMap.java:79-87`) is what
        // makes an instance exist to be synced.
        living
            .attributes_mut()
            .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            .unwrap();
    }
    let entity = store.get(id).unwrap();
    let server_entity = ServerEntity::new(entity, 3, true);
    let packets = server_entity.send_pairing_data(entity, &[]);
    assert_eq!(
        packets.iter().map(kind).collect::<Vec<_>>(),
        vec!["add", "data", "attributes", "equipment"]
    );
    let ClientboundPlayPacket::AddEntity(add) = &packets[0] else {
        panic!("add")
    };
    assert_eq!(add.y_head_rot, unpack_degrees(pack_degrees(entity.y_rot())));
    let ClientboundPlayPacket::SetEntityData(data) = &packets[1] else {
        panic!("data")
    };
    assert_eq!(data.values.len(), 1);
    assert_eq!(
        data.values[0].data_id,
        bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_HEALTH_DATA_ID
    );
    assert_eq!(data.values[0].value, EntityDataValueKind::Float(20.0));
    let ClientboundPlayPacket::UpdateAttributes(attributes) = &packets[2] else {
        panic!("attributes")
    };
    assert_eq!(attributes.attributes.len(), 1);
    assert_eq!(attributes.attributes[0].base, 20.0);
    let ClientboundPlayPacket::SetEquipment(equipment) = &packets[3] else {
        panic!("equipment")
    };
    assert_eq!(equipment.slots.len(), 1);
    assert_eq!(
        equipment.slots[0].slot,
        bbb_protocol::equipment_slots::EquipmentSlot::Feet
    );
}

/// `ServerEntity.sendChanges` (`ServerEntity.java:120`, `:197`, `:210-214`):
/// dirty synched data opens the gate off the interval, `packDirty` publishes
/// only the dirty rows, a dirty attribute follows, and a head turn of at least
/// one packed degree publishes `ClientboundRotateHeadPacket`.
#[test]
fn dirty_data_attributes_and_head_rotation_publish_through_send_changes() {
    let (mut store, id) = store_with_one_armor_stand();
    let mut server_entity = ServerEntity::new(store.get(id).unwrap(), 3, true);
    // Pass 0 is on the interval and forces the sixty-pass position resend
    // (`:141`); it is not what this test is about.
    server_entity.send_changes(store.get_mut(id).unwrap());
    {
        let entity = store.get_mut(id).unwrap();
        crate::entity::armor_stand::set_small(entity, true);
        entity
            .living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            .unwrap()
            .set_base_value(30.0);
        entity.set_y_head_rot(90.0);
    }
    let packets = server_entity.send_changes(store.get_mut(id).unwrap());
    assert_eq!(
        packets.iter().map(kind).collect::<Vec<_>>(),
        vec!["data", "attributes", "head"]
    );
    let ClientboundPlayPacket::SetEntityData(data) = &packets[0] else {
        panic!("data")
    };
    assert_eq!(data.values.len(), 1);
    assert_eq!(
        data.values[0].data_id,
        bbb_protocol::entity_synched_data::VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID
    );
    let ClientboundPlayPacket::RotateHead(head) = &packets[2] else {
        panic!("head")
    };
    assert_eq!(head.y_head_rot, 90.0);
    // Nothing is dirty any more, and pass 2 is off the interval.
    assert!(server_entity
        .send_changes(store.get_mut(id).unwrap())
        .is_empty());
    // The pairing snapshot now carries the flags row too (`:340`).
    let pairing = server_entity.send_pairing_data(store.get(id).unwrap(), &[]);
    let ClientboundPlayPacket::SetEntityData(data) = &pairing[1] else {
        panic!("data")
    };
    assert_eq!(data.values.len(), 2);
}

#[test]
fn player_server_entity_reuses_motion_position_dirty_and_head_order() {
    let (mut store, id) = store_with_one_armor_stand();
    let mut tracker = ServerEntity::new(store.get(id).unwrap(), 1, true);
    tracker.send_changes(store.get_mut(id).unwrap());
    {
        let entity = store.get_mut(id).unwrap();
        let mut position = entity.position();
        position.x += 0.25;
        entity.set_pos(position);
        entity.set_delta_movement(Vec3d {
            x: 0.5,
            y: 0.0,
            z: 0.0,
        });
        crate::entity::armor_stand::set_small(entity, true);
        entity
            .living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
            .unwrap()
            .set_base_value(1.25);
        entity.set_y_head_rot(90.0);
    }
    let packets = tracker.send_changes(store.get_mut(id).unwrap());
    assert_eq!(
        packets.iter().map(kind).collect::<Vec<_>>(),
        ["motion", "move", "data", "attributes", "head"]
    );
}

fn item_frame_data_packet(packets: &[ClientboundPlayPacket]) -> &SetEntityData {
    packets
        .iter()
        .find_map(|packet| match packet {
            ClientboundPlayPacket::SetEntityData(data) => Some(data),
            _ => None,
        })
        .expect("item-frame metadata packet")
}

fn spawn_test_item_frame(store: &mut EntityStore, glow: bool, direction: Direction) -> i32 {
    crate::entity::hanging::spawn_item_frame(store, BlockPos { x: 0, y: 64, z: 0 }, direction, glow)
        .expect("item frame spawns")
}

fn framed_map_stack(
    item: &str,
    map_id: Option<i32>,
    static_decorations: bool,
    unrelated_components: bool,
) -> bbb_server_types::item_stack::RuntimeItemStack {
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let mut components = Vec::new();
    if let Some(map_id) = map_id {
        components.push(entry("minecraft:map_id", NbtSummaryValue::Int(map_id)));
    }
    if static_decorations {
        components.push(entry(
            "minecraft:map_decorations",
            NbtSummaryValue::Compound(vec![entry(
                "home",
                NbtSummaryValue::Compound(vec![
                    entry("type", NbtSummaryValue::string("minecraft:player")),
                    entry("x", NbtSummaryValue::Double(1.25f64.to_bits())),
                    entry("z", NbtSummaryValue::Double((-3.5f64).to_bits())),
                    entry("rotation", NbtSummaryValue::Float(90.0f32.to_bits())),
                ]),
            )]),
        ));
    }
    if unrelated_components {
        components.push(entry(
            "minecraft:custom_name",
            NbtSummaryValue::string("Renamed map"),
        ));
        components.push(entry("minecraft:damage", NbtSummaryValue::Int(1)));
    }
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string(item)),
            entry("count", NbtSummaryValue::Int(1)),
            entry("components", NbtSummaryValue::Compound(components)),
        ]),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .unwrap()
}

fn spawn_test_item_frame_with_stack(
    store: &mut EntityStore,
    glow: bool,
    direction: Direction,
    stack: bbb_server_types::item_stack::RuntimeItemStack,
) -> i32 {
    let data = ItemFrameData::new(BlockPos { x: 4, y: 70, z: -6 }, direction).with_saved_fields(
        Some(stack),
        0,
        1.0,
        false,
        false,
    );
    let (entity_type_id, data) = if glow {
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
            EntityData::GlowItemFrame(data),
        )
    } else {
        (
            VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            EntityData::ItemFrame(data),
        )
    };
    store.spawn(entity_type_id, origin(), data)
}

#[test]
fn framed_map_snapshot_is_exact_and_ignores_unrelated_components() {
    for glow in [false, true] {
        let mut store = EntityStore::new(0x4a9, EntityIdCounter::new());
        let id = spawn_test_item_frame_with_stack(
            &mut store,
            glow,
            Direction::Up,
            framed_map_stack("minecraft:filled_map", Some(-17), true, false),
        );
        let frame = framed_map_frame(store.get(id).unwrap()).unwrap();
        assert_eq!(frame.map_id, -17);
        assert_eq!(frame.entity_id, id);
        assert_eq!(frame.attached, BlockPos { x: 4, y: 70, z: -6 });
        assert_eq!(frame.direction, Direction::Up);
        assert_eq!(
            frame.static_decorations,
            vec![bbb_protocol::packets::ItemMapDecorationSummary {
                id: "home".into(),
                decoration_type_id: 0,
                decoration_type_resource_id: "minecraft:player".to_owned(),
                x_bits: 1.25f64.to_bits(),
                z_bits: (-3.5f64).to_bits(),
                rotation_bits: 90.0f32.to_bits(),
            }]
        );
    }

    let mut store = EntityStore::new(0x4aa, EntityIdCounter::new());
    for stack in [
        framed_map_stack("minecraft:stone", Some(3), false, false),
        framed_map_stack("minecraft:filled_map", None, false, false),
    ] {
        let id = spawn_test_item_frame_with_stack(&mut store, false, Direction::North, stack);
        assert!(framed_map_frame(store.get(id).unwrap()).is_none());
    }

    let id = spawn_test_item_frame_with_stack(
        &mut store,
        false,
        Direction::North,
        framed_map_stack("minecraft:filled_map", Some(3), true, true),
    );
    let frame = framed_map_frame(store.get(id).unwrap()).unwrap();
    assert_eq!(frame.map_id, 3);
    assert_eq!(frame.static_decorations.len(), 1);
    store
        .remove(id, crate::entity::RemovalReason::Discarded)
        .unwrap();
    assert_eq!(
        store.drain_framed_map_removals(),
        vec![crate::entity::FramedMapRemoval {
            map_id: 3,
            pos: BlockPos { x: 4, y: 70, z: -6 },
            entity_id: id,
        }]
    );
}

#[test]
fn framed_map_tracker_is_due_on_its_own_zero_ten_and_twenty_passes() {
    let mut store = EntityStore::new(0x4ab, EntityIdCounter::new());
    spawn_test_item_frame_with_stack(
        &mut store,
        false,
        Direction::North,
        framed_map_stack("minecraft:filled_map", Some(9), false, false),
    );
    let viewers = [viewer(1, 1.0)];
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut out,
    );
    out.clear();
    let mut due = Vec::new();
    for pass in 0..=20 {
        tracker.tick_with_framed_maps(
            &mut store,
            &viewers,
            |_, _, _| true,
            |_, _| true,
            |frame| {
                due.push((pass, frame.map_id));
                Vec::new()
            },
            &mut out,
        );
    }
    assert_eq!(due, vec![(0, 9), (10, 9), (20, 9)]);
}

#[test]
fn framed_map_direct_packets_ignore_seen_by() {
    let mut store = EntityStore::new(0x4ac, EntityIdCounter::new());
    let id = spawn_test_item_frame_with_stack(
        &mut store,
        false,
        Direction::North,
        framed_map_stack("minecraft:filled_map", Some(21), false, false),
    );
    let viewers = [viewer(1, 1.0), viewer(2, 10_000.0)];
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut out,
    );
    assert_eq!(tracker.viewers_of(id), vec![ConnectionId(1)]);
    store
        .get_mut(id)
        .unwrap()
        .mark_synched_dirty(VANILLA_ITEM_FRAME_ITEM_DATA_ID);
    out.clear();
    tracker.tick_with_framed_maps(
        &mut store,
        &viewers,
        |_, _, _| true,
        |_, _| true,
        |frame| {
            vec![(
                ConnectionId(2),
                ClientboundPlayPacket::MapItemData(bbb_protocol::packets::MapItemData {
                    map_id: frame.map_id,
                    scale: 0,
                    locked: false,
                    decorations: None,
                    color_patch: None,
                }),
            )]
        },
        &mut out,
    );
    assert!(recipient_packets(&out).any(|(connection, packet)| {
        *connection == ConnectionId(2)
            && matches!(packet, ClientboundPlayPacket::MapItemData(data) if data.map_id == 21)
    }));
    assert!(recipient_packets(&out).any(|(connection, packet)| {
        *connection == ConnectionId(1)
            && matches!(packet, ClientboundPlayPacket::SetEntityData(data) if data.id == id)
    }));
    assert!(recipient_packets(&out).all(|(connection, packet)| {
        !(*connection == ConnectionId(1) && matches!(packet, ClientboundPlayPacket::MapItemData(_)))
    }));
}

#[test]
fn item_frame_default_pairing_omits_item_and_rotation() {
    for glow in [false, true] {
        let mut store = EntityStore::new(0x1_7e4, EntityIdCounter::new());
        let id = spawn_test_item_frame(&mut store, glow, Direction::South);
        let entity = store.get(id).unwrap();
        let packets = ServerEntity::new(entity, 1, true).send_pairing_data(entity, &[]);
        assert_eq!(packets.len(), 1, "glow={glow}: {packets:?}");
        assert!(matches!(packets[0], ClientboundPlayPacket::AddEntity(_)));
    }
}

#[test]
fn item_frame_dirty_clear_sends_empty_item_stack() {
    for glow in [false, true] {
        let mut store = EntityStore::new(0x1_7e4, EntityIdCounter::new());
        let id = spawn_test_item_frame(&mut store, glow, Direction::South);
        let entity = store.get_mut(id).unwrap();
        entity.mark_synched_dirty(VANILLA_ITEM_FRAME_ITEM_DATA_ID);
        let mut tracker = ServerEntity::new(entity, 1, true);
        let packets = tracker.send_changes(entity);
        let data = item_frame_data_packet(&packets);
        assert_eq!(data.values.len(), 1, "glow={glow}: {packets:?}");
        assert_eq!(data.values[0].data_id, VANILLA_ITEM_FRAME_ITEM_DATA_ID);
        let EntityDataValueKind::ItemStack(stack) = &data.values[0].value else {
            panic!("glow={glow}: {data:?}")
        };
        assert!(bbb_registry_codec::item_stack_summary_is_empty(stack));
    }
}

#[test]
fn item_frame_dirty_rotation_wrap_sends_zero() {
    for glow in [false, true] {
        let mut store = EntityStore::new(0x1_7e4, EntityIdCounter::new());
        let id = spawn_test_item_frame(&mut store, glow, Direction::South);
        let entity = store.get_mut(id).unwrap();
        entity.mark_synched_dirty(VANILLA_ITEM_FRAME_ROTATION_DATA_ID);
        let mut tracker = ServerEntity::new(entity, 1, true);
        let packets = tracker.send_changes(entity);
        let data = item_frame_data_packet(&packets);
        assert_eq!(data.values.len(), 1, "glow={glow}: {packets:?}");
        assert_eq!(
            data.values[0],
            EntityDataValue {
                data_id: VANILLA_ITEM_FRAME_ROTATION_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(0),
            }
        );
    }
}

fn item_frame_chunk(entries: Vec<NbtSummaryEntry>) -> Java26EntityChunkData {
    Java26EntityChunkData::parse(
        NamedCompound::new(
            "",
            vec![
                NbtSummaryEntry {
                    name: "DataVersion".into(),
                    value: NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION),
                },
                NbtSummaryEntry {
                    name: "Position".into(),
                    value: NbtSummaryValue::IntArray(vec![0, 0]),
                },
                NbtSummaryEntry {
                    name: "Entities".into(),
                    value: NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entries)]),
                },
            ],
        ),
        0,
        0,
    )
    .expect("item-frame chunk fixture")
}

fn set_item_frame_field(entries: &mut [NbtSummaryEntry], name: &str, value: NbtSummaryValue) {
    entries
        .iter_mut()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("item-frame writer emitted {name}"))
        .value = value;
}

#[test]
fn item_frame_load_restores_metadata_and_dirty_order() {
    for glow in [false, true] {
        let mut source = EntityStore::new(0x1_7e4, EntityIdCounter::new());
        spawn_test_item_frame(&mut source, glow, Direction::North);
        let mut record = write_entity_chunk(&source, (0, 0))
            .unwrap()
            .entities()
            .next()
            .unwrap()
            .raw()
            .to_vec();
        record.push(NbtSummaryEntry {
            name: "Item".into(),
            value: plain_stack("minecraft:stone", 4)
                .unwrap()
                .write(&bbb_registry_codec::UnavailableHolderResolver)
                .unwrap()
                .unwrap(),
        });
        set_item_frame_field(&mut record, "ItemRotation", NbtSummaryValue::Byte(1));
        set_item_frame_field(&mut record, "Invisible", NbtSummaryValue::Byte(1));

        let mut restored = EntityStore::new(0x10ad, EntityIdCounter::new());
        let restored_id = read_entity_chunk(&mut restored, &item_frame_chunk(record)).unwrap()[0];
        let entity = restored.get(restored_id).unwrap();
        assert!(entity.is_invisible(), "glow={glow}");
        let mut tracker = ServerEntity::new(entity, 1, true);
        let pairing = tracker.send_pairing_data(entity, &[]);
        assert_eq!(
            item_frame_data_packet(&pairing)
                .values
                .iter()
                .map(|value| value.data_id)
                .collect::<Vec<_>>(),
            [
                VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
                VANILLA_HANGING_ENTITY_DIRECTION_DATA_ID,
                VANILLA_ITEM_FRAME_ITEM_DATA_ID,
                VANILLA_ITEM_FRAME_ROTATION_DATA_ID,
            ],
            "glow={glow}: {pairing:?}"
        );
        let dirty = tracker.send_changes(restored.get_mut(restored_id).unwrap());
        assert_eq!(
            item_frame_data_packet(&dirty)
                .values
                .iter()
                .map(|value| value.data_id)
                .collect::<Vec<_>>(),
            [
                VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
                VANILLA_HANGING_ENTITY_DIRECTION_DATA_ID,
                VANILLA_ITEM_FRAME_ITEM_DATA_ID,
                VANILLA_ITEM_FRAME_ROTATION_DATA_ID,
            ],
            "glow={glow}: {dirty:?}"
        );
    }
}

/// `AbstractPiglin.DATA_IMMUNE_TO_ZOMBIFICATION` (`AbstractPiglin.java:24`,
/// `:58-61`) follows `IsImmuneToZombification` into the synched rows, so an
/// immune piglin or brute does not shake on the client.
#[test]
fn piglin_zombification_immunity_is_published() {
    use bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID;
    for entity_type_id in [
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_ID,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
    ] {
        let mut store = EntityStore::new(0x919, EntityIdCounter::new());
        let id = store
            .spawn_general_living(entity_type_id, origin())
            .unwrap();
        let immune = |store: &EntityStore| {
            non_default_synched_values(store.get(id).unwrap())
                .iter()
                .any(|value| {
                    value.data_id == VANILLA_ABSTRACT_PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID
                        && value.value == EntityDataValueKind::Boolean(true)
                })
        };
        assert!(!immune(&store));
        store.get_mut(id).unwrap().take_synched_dirty();
        let NbtSummaryValue::Compound(mut data) = store.command_data(id).unwrap() else {
            panic!("entity data is a compound");
        };
        data.retain(|field| field.name != "IsImmuneToZombification");
        data.push(NbtSummaryEntry {
            name: "IsImmuneToZombification".into(),
            value: NbtSummaryValue::Byte(1),
        });
        store.load_command_data(id, &data).unwrap();
        assert!(store
            .get_mut(id)
            .unwrap()
            .take_synched_dirty()
            .contains(&VANILLA_ABSTRACT_PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID));
        assert!(immune(&store));
    }
}

#[test]
fn an_even_sized_painting_is_sent_at_its_attached_block() {
    let mut store = EntityStore::new(0x51, EntityIdCounter::new());
    let attached = BlockPos { x: 3, y: 65, z: -2 };
    let id = crate::entity::hanging::spawn_painting(
        &mut store,
        attached,
        Direction::South,
        crate::entity::hanging::PaintingVariant::vanilla("minecraft:bust"),
    )
    .unwrap();
    let entity = store.get(id).unwrap();
    let corner = Vec3d {
        x: 3.0,
        y: 65.0,
        z: -2.0,
    };
    assert_ne!(entity.position(), corner);
    let server_entity = ServerEntity::new(entity, 1, false);
    assert_eq!(server_entity.position_base(), corner);
    let packets = server_entity.send_pairing_data(entity, &[]);
    let Some(ClientboundPlayPacket::AddEntity(add)) = packets.first() else {
        panic!("pairing starts with the add packet: {packets:?}");
    };
    assert_eq!(add.position, corner);
}

#[test]
fn the_broadcast_range_percentage_scales_the_tracking_range() {
    let (mut store, id) = store_with_one_item(origin());
    let half = f64::from(item_tracking_range_blocks() / 2);
    let mut tracker = EntityTracker::new();
    tracker.set_range_percentage(50, &store);
    let viewers = vec![viewer(1, half - 1.0), viewer(2, -(half + 1.0))];
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut Vec::new(),
    );
    assert_eq!(tracker.viewers_of(id), vec![ConnectionId(1)]);
}

#[test]
fn falling_block_publishes_non_zero_start_pos_and_omits_default_zero() {
    let mut store = EntityStore::new(0x52, EntityIdCounter::new());
    let sand = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_SAND_ID,
    )
    .unwrap();
    let start = BlockPos { x: 4, y: 70, z: -3 };
    let id = crate::entity::falling_block::spawn_fallen_block(&mut store, start, sand);
    let entity = store.get(id).unwrap();
    let values = tracked_data_values(entity);
    assert!(
        values.iter().any(|entry| {
            entry.data_id == 8
                && entry.serializer_id == VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_POS_ID
                && entry.value == EntityDataValueKind::BlockPos(start)
        }),
        "expected DATA_START_POS in {values:?}"
    );

    let origin_id = crate::entity::falling_block::spawn_fallen_block(
        &mut store,
        BlockPos { x: 0, y: 0, z: 0 },
        sand,
    );
    let origin_values = tracked_data_values(store.get(origin_id).unwrap());
    assert!(
        !origin_values.iter().any(|entry| entry.data_id == 8),
        "default BlockPos.ZERO must be omitted: {origin_values:?}"
    );
}

#[test]
fn minecart_track_interpolation_emits_move_minecart_packet_and_falls_back_cleanly() {
    use bbb_protocol::packets::{MinecartStep, MoveMinecartAlongTrack};

    // Standard minecart without track interpolation keeps using MoveEntity.
    let mut store = EntityStore::new(0x101, EntityIdCounter::new());
    let standard_id = crate::entity::minecart::spawn_minecart(&mut store, origin(), 0.0).unwrap();
    let mut standard_se = ServerEntity::new(store.get(standard_id).unwrap(), 3, true);
    store.get_mut(standard_id).unwrap().set_pos(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.0,
    });
    let standard_packets = standard_se.send_changes(store.get_mut(standard_id).unwrap());
    assert!(
        standard_packets
            .iter()
            .any(|packet| matches!(packet, ClientboundPlayPacket::MoveEntity(_))),
        "standard minecart should emit MoveEntity: {standard_packets:?}"
    );
    assert!(
        !standard_packets
            .iter()
            .any(|packet| matches!(packet, ClientboundPlayPacket::MoveMinecartAlongTrack(_))),
        "standard minecart must not emit MoveMinecartAlongTrack: {standard_packets:?}"
    );

    // Experimental minecart with track interpolation accumulates steps across
    // non-due ticks and flushes them in one ClientboundMoveMinecartPacket on the
    // due tick after dirty synched entity data.
    let id = crate::entity::minecart::spawn_minecart(&mut store, origin(), 0.0).unwrap();
    let mut server_entity = ServerEntity::new(store.get(id).unwrap(), 3, true);
    let entity = store.get_mut(id).unwrap();
    entity.enable_minecart_track_interpolation();
    let step_a = MinecartStep {
        position: Vec3d {
            x: 0.25,
            y: 64.0,
            z: 0.0,
        },
        movement: Vec3d {
            x: 0.25,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        weight: 0.25,
    };
    let step_b = MinecartStep {
        position: Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.0,
        },
        movement: Vec3d {
            x: 0.25,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        weight: 0.25,
    };
    // Tick 0 is due (0 % 3 == 0): flush first step + dirty data.
    entity.set_pos(step_a.position);
    entity.set_delta_movement(step_a.movement);
    entity.push_minecart_lerp_step(step_a);
    crate::entity::minecart::set_display_offset(entity, 9);
    let tick0 = server_entity.send_changes(entity);
    assert!(
        matches!(tick0.first(), Some(ClientboundPlayPacket::SetEntityData(_))),
        "dirty entity data must precede MoveMinecartAlongTrack: {tick0:?}"
    );
    assert_eq!(
        tick0.get(1),
        Some(&ClientboundPlayPacket::MoveMinecartAlongTrack(
            MoveMinecartAlongTrack {
                entity_id: id,
                lerp_steps: vec![step_a],
            }
        ))
    );

    // Ticks 1 and 2 are not due: steps accumulate without sending.
    entity.push_minecart_lerp_step(step_a);
    assert!(server_entity.send_changes(entity).is_empty());
    entity.set_pos(step_b.position);
    entity.push_minecart_lerp_step(step_b);
    assert!(server_entity.send_changes(entity).is_empty());

    // Tick 3 is due: both accumulated steps are sent and drained.
    let tick3 = server_entity.send_changes(entity);
    assert_eq!(
        tick3,
        vec![ClientboundPlayPacket::MoveMinecartAlongTrack(
            MoveMinecartAlongTrack {
                entity_id: id,
                lerp_steps: vec![step_a, step_b],
            }
        )]
    );

    // Next due tick with empty lerp_steps and no change emits nothing.
    for _ in 0..2 {
        assert!(server_entity.send_changes(entity).is_empty());
    }
    assert!(server_entity.send_changes(entity).is_empty());

    // Empty lerp_steps with a velocity change emits a single weight-1.0 step.
    for _ in 0..2 {
        assert!(server_entity.send_changes(entity).is_empty());
    }
    let stopped = Vec3d {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    entity.set_delta_movement(stopped);
    let tick9 = server_entity.send_changes(entity);
    assert_eq!(
        tick9,
        vec![ClientboundPlayPacket::MoveMinecartAlongTrack(
            MoveMinecartAlongTrack {
                entity_id: id,
                lerp_steps: vec![MinecartStep {
                    position: step_b.position,
                    movement: stopped,
                    y_rot: 0.0,
                    x_rot: 0.0,
                    weight: 1.0,
                }],
            }
        )]
    );

    // Forced 60-tick position sync emits a single weight-1.0 step even while idle.
    let mut forced = Vec::new();
    for _ in 10..=60 {
        let packets = server_entity.send_changes(entity);
        if !packets.is_empty() {
            forced = packets;
            break;
        }
    }
    assert_eq!(
        forced,
        vec![ClientboundPlayPacket::MoveMinecartAlongTrack(
            MoveMinecartAlongTrack {
                entity_id: id,
                lerp_steps: vec![MinecartStep {
                    position: step_b.position,
                    movement: stopped,
                    y_rot: 0.0,
                    x_rot: 0.0,
                    weight: 1.0,
                }],
            }
        )]
    );
}

#[test]
fn minecart_improvements_feature_flag_emits_move_minecart_in_live_world() {
    use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
    use bbb_server_types::save::Java26BuiltinDimension;

    let rail = vanilla_block_state_id_by_name_and_properties(
        "minecraft:rail",
        &[
            ("shape".to_string(), "east_west".to_string()),
            ("waterlogged".to_string(), "false".to_string()),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap();
    let stone = vanilla_block_default_state_id(
        vanilla_block_type_for_resource_id("minecraft:stone")
            .unwrap()
            .id,
    )
    .unwrap();

    for improvements_enabled in [true, false] {
        let (sender, receiver) = crate::tests::mailbox();
        let mut world =
            crate::world::ServerWorld::new(crate::tests::creative_fixture_config()).unwrap();
        world.set_test_respawn_radius(0);
        world.set_minecart_improvements_for_test(improvements_enabled);
        for x in 0..=8 {
            world.set_block(
                Java26BuiltinDimension::Overworld,
                BlockPos { x, y: 63, z: 0 },
                stone,
                2 | 16,
            );
            world.set_block(
                Java26BuiltinDimension::Overworld,
                BlockPos { x, y: 64, z: 0 },
                rail,
                2 | 16,
            );
        }
        crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let cart_id = world
            .spawn_minecart_for_test(
                Java26BuiltinDimension::Overworld,
                Vec3d {
                    x: 2.5,
                    y: 64.0625,
                    z: 0.5,
                },
                Vec3d {
                    x: 0.35,
                    y: 0.0,
                    z: 0.0,
                },
            )
            .unwrap();
        let mut saw_track_packet = false;
        let mut saw_standard_move = false;
        for _ in 0..6 {
            let tick = world.tick(Vec::new());
            for event in &tick.events {
                let crate::world::WorldEvent::Send { packet, .. } = event else {
                    continue;
                };
                match packet {
                    ClientboundPlayPacket::MoveMinecartAlongTrack(update)
                        if update.entity_id == cart_id =>
                    {
                        assert!(!update.lerp_steps.is_empty());
                        saw_track_packet = true;
                    }
                    ClientboundPlayPacket::MoveEntity(update) if update.id == cart_id => {
                        saw_standard_move = true;
                    }
                    _ => {}
                }
            }
        }
        assert_eq!(
            saw_track_packet, improvements_enabled,
            "improvements_enabled={improvements_enabled}"
        );
        assert_eq!(
            saw_standard_move, !improvements_enabled,
            "improvements_enabled={improvements_enabled}"
        );
    }
}
