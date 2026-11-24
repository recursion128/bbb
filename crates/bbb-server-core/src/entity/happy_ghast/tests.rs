use super::*;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHICKEN_ID;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{BlockPos, Vec3d};
use uuid::Uuid;

use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent};
use crate::entity::{ExternalPlayerCanRide, RemovalReason};

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn fixture() -> (EntityStore, i32) {
    let mut store = EntityStore::new(17, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, position())
        .unwrap();
    let body = store.get_mut(id).unwrap();
    body.set_no_gravity(true);
    let EntityData::GeneralLiving(data) = &mut body.data else {
        unreachable!()
    };
    data.set_no_ai(true);
    store.drain_lifecycle_events();
    (store, id)
}

fn timeout(store: &mut EntityStore, id: i32, timeout: i32) {
    let mut body = store.detach(id).unwrap();
    publish_still_timeout(&mut body, store, timeout);
    store.attach(body);
    store.drain_lifecycle_events();
}

fn above(entity: &Entity) -> Vec3d {
    Vec3d {
        y: entity.bounding_box().max_y + 0.5,
        ..entity.position()
    }
}

fn add_player(store: &mut EntityStore, id: i32, position: Vec3d, game_type: GameType) {
    let player = crate::world::ServerWorld::detached_player_for_test(id, position, game_type);
    store.insert_player(id, Uuid::from_u128(id as u128), Box::new(player));
}

fn attach_player(store: &mut EntityStore, id: i32, vehicle: i32) -> bool {
    store.attach_test_player(id, vehicle, ExternalPlayerCanRide::Allowed)
}

#[test]
fn happy_ghast_scan_uses_half_open_point_bounds_and_only_excludes_spectators() {
    let (mut store, id) = fixture();
    let body = store.detach(id).unwrap();
    let bb = body.bounding_box();
    let min = Vec3d {
        x: bb.min_x - 1.0,
        y: bb.max_y - f64::from(1.0e-5_f32),
        z: bb.min_z - 1.0,
    };
    let max = Vec3d {
        x: bb.max_x + 1.0,
        y: bb.max_y + (bb.max_y - bb.min_y) / 2.0,
        z: bb.max_z + 1.0,
    };
    let middle = above(&body);
    for (position, expected) in [
        (min, true),
        (middle, true),
        (
            Vec3d {
                x: min.x - 1.0e-8,
                ..middle
            },
            false,
        ),
        (
            Vec3d {
                y: min.y - 1.0e-8,
                ..middle
            },
            false,
        ),
        (
            Vec3d {
                z: min.z - 1.0e-8,
                ..middle
            },
            false,
        ),
        (Vec3d { x: max.x, ..middle }, false),
        (Vec3d { y: max.y, ..middle }, false),
        (Vec3d { z: max.z, ..middle }, false),
        (
            Vec3d {
                x: max.x - 1.0e-8,
                y: max.y - 1.0e-8,
                z: max.z - 1.0e-8,
            },
            true,
        ),
    ] {
        store.remove_player(1000);
        add_player(&mut store, 1000, position, GameType::Survival);
        assert_eq!(
            scan_player_above(&body, &store, None),
            Some(expected),
            "{position:?}"
        );
    }
    store.remove_player(1000);
    add_player(&mut store, 1000, middle, GameType::Spectator);
    assert_eq!(scan_player_above(&body, &store, None), Some(false));
}

#[test]
fn happy_ghast_scan_uses_root_position_and_excludes_happy_roots() {
    let (mut store, id) = fixture();
    let standing = above(store.get(id).unwrap());
    let chicken = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CHICKEN_ID, standing)
        .unwrap();
    add_player(
        &mut store,
        1000,
        Vec3d {
            y: 300.0,
            ..position()
        },
        GameType::Survival,
    );
    assert!(attach_player(&mut store, 1000, chicken));
    assert_eq!(
        scan_player_above(store.get(id).unwrap(), &store, None),
        Some(true)
    );
    assert!(store.link_existing_passenger(id, chicken));
    let body = store.detach(id).unwrap();
    assert_eq!(scan_player_above(&body, &store, None), Some(false));
    store.attach(body);
    store.detach_external_player(1000).unwrap();
    let other = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, standing)
        .unwrap();
    assert!(attach_player(&mut store, 1000, other));
    assert_eq!(
        scan_player_above(store.get(id).unwrap(), &store, None),
        Some(false)
    );
}

fn assert_position_synced(events: &[EntityLifecycleEvent], id: i32) {
    assert!(events.iter().any(|event| matches!(event,
        EntityLifecycleEvent::Broadcast { packet: ClientboundPlayPacket::EntityPositionSync(packet), .. } if packet.id == id
    )));
}

#[test]
fn happy_ghast_passenger_removal_syncs_position() {
    for remove_kind in [0, 3] {
        let (mut store, id) = fixture();
        let chicken = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_CHICKEN_ID, position())
            .unwrap();
        assert!(store.link_existing_passenger(id, chicken));
        assert!(attach_player(&mut store, 1000, id));
        store.drain_lifecycle_events();
        match remove_kind {
            0 => {
                store.detach_external_player(1000).unwrap();
            }
            3 => {
                store.remove(chicken, RemovalReason::Discarded).unwrap();
            }
            _ => unreachable!(),
        }
        assert_position_synced(&store.drain_lifecycle_events(), id);
        timeout(&mut store, id, 3);
        if !store.passengers(id).is_empty() {
            assert!(store.eject_passengers_reverse(id));
        }
    }
}

#[test]
fn happy_ghast_vehicle_removal_clears_passengers_and_syncs_position() {
    for attach_removed in [false, true] {
        let (mut store, id) = fixture();
        assert!(attach_player(&mut store, 1000, id));
        assert!(attach_player(&mut store, 1001, id));
        store.drain_lifecycle_events();
        if attach_removed {
            let mut body = store.detach(id).unwrap();
            body.discard();
            store.attach(body);
        } else {
            store.remove(id, RemovalReason::Discarded).unwrap();
        }
        let events = store.drain_lifecycle_events();
        assert_position_synced(&events, id);
        assert!(store.get(id).is_none());
        assert_eq!(store.vehicle(1000), None);
        assert_eq!(store.vehicle(1001), None);
    }
}

#[test]
fn happy_ghast_hanging_removal_syncs_attached_parent() {
    let (mut store, id) = fixture();
    let frame = crate::entity::hanging::spawn_item_frame(
        &mut store,
        BlockPos { x: 8, y: 65, z: 8 },
        bbb_protocol::packets::Direction::North,
        false,
    )
    .unwrap();
    assert!(store.link_existing_passenger_forced(id, frame));
    store.drain_lifecycle_events();
    let mut body = store.detach(frame).unwrap();
    body.set_removed(RemovalReason::Killed);
    assert_eq!(store.publish_detached_hanging_removal(&body), Some(()));
    assert_position_synced(&store.drain_lifecycle_events(), id);
    store.attach(body);
}

#[test]
fn a_hurt_happy_ghast_regenerates_faster_in_the_clouds() {
    let health_after = |y: f64| {
        let (mut store, id) = fixture();
        let body = store.get_mut(id).unwrap();
        body.set_pos(Vec3d { y, ..position() });
        crate::entity::living::set_health(body, 10.0);
        let mut native = crate::entity::tests::native_level::NativeLevel::new();
        for _ in 0..40 {
            native.with_level(|level| crate::entity::tick_entities(&mut store, level, true));
        }
        store.get(id).unwrap().living().unwrap().health()
    };
    assert_eq!(health_after(64.0), 10.0);
    assert_eq!(health_after(190.0), 12.0);
}
