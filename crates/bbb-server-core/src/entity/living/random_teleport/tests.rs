use super::*;
use crate::entity::tests::native_level::{state, NativeLevel};
use crate::entity::EntityData;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_SHULKER_ID,
};
use bbb_server_types::interaction::WorldBorder;

fn fixture() -> (EntityStore, Box<Entity>, NativeLevel) {
    let mut store = EntityStore::new(34, crate::entity::store::EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_ENDERMAN_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    let body = store.detach(id).unwrap();
    store.drain_lifecycle_events();
    (store, body, NativeLevel::new())
}

fn destination() -> Vec3d {
    Vec3d {
        x: 64.5,
        y: 80.75,
        z: 64.5,
    }
}

fn attempt_native(
    body: &mut Entity,
    store: &mut EntityStore,
    native: &mut NativeLevel,
    destination: Vec3d,
    show_particles: bool,
) -> Option<bool> {
    native.with_level(|level| attempt(body, store, level, destination, show_particles))
}

#[test]
fn random_teleport_success_descends_preserves_fractional_y_and_moves_actual_passengers() {
    let (mut store, mut body, mut level) = fixture();
    let child = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, body.position())
        .unwrap();
    let root = body.id();
    store.attach(body);
    assert!(store.link_existing_passenger_forced(root, child));
    body = store.detach(root).unwrap();
    let before = body.delta_movement();
    assert_eq!(
        attempt_native(&mut body, &mut store, &mut level, destination(), true),
        Some(true)
    );
    assert_eq!(
        body.position(),
        Vec3d {
            y: 64.75,
            ..destination()
        }
    );
    assert_eq!(body.old_position(), body.position());
    assert_eq!(body.delta_movement(), before);
    assert_eq!(
        store.get(child).unwrap().old_position(),
        store.get(child).unwrap().position()
    );
    assert!(store.get(child).unwrap().position().x > 64.0);
    assert!(store.drain_lifecycle_events().iter().any(|event| matches!(event, crate::entity::EntityLifecycleEvent::Broadcast {packet: crate::connection::ClientboundPlayPacket::EntityEvent(event), ..} if event.event_id == 46)));
}

#[test]
fn random_teleport_block_collision_snaps_back() {
    let (mut store, mut body, mut level) = fixture();
    let origin = body.position();
    body.old_position = Vec3d::default();
    level.set(
        BlockPos {
            x: 64,
            y: 66,
            z: 64,
        },
        state("minecraft:stone"),
    );
    assert_eq!(
        attempt_native(
            &mut body,
            &mut store,
            &mut level,
            Vec3d {
                y: 64.75,
                ..destination()
            },
            true
        ),
        Some(false)
    );
    assert_eq!(body.position(), origin);
    assert_eq!(body.old_position(), origin);
    assert!(store.drain_lifecycle_events().is_empty());
}

#[test]
fn random_teleport_liquid_collision_and_unloaded_destination_fail() {
    let (mut store, mut body, mut level) = fixture();
    let origin = body.position();
    level.set(
        BlockPos {
            x: 64,
            y: 64,
            z: 64,
        },
        state("minecraft:water"),
    );
    assert_eq!(
        attempt_native(&mut body, &mut store, &mut level, destination(), true),
        Some(false)
    );
    assert_eq!(body.position(), origin);
    level.set(
        BlockPos {
            x: 64,
            y: 64,
            z: 64,
        },
        state("minecraft:air"),
    );
    body.old_position = Vec3d::default();
    assert_eq!(
        attempt_native(
            &mut body,
            &mut store,
            &mut level,
            Vec3d {
                x: 256.5,
                y: 80.75,
                z: 256.5
            },
            true
        ),
        Some(false)
    );
    assert_eq!(body.old_position(), origin);
}

#[test]
fn random_teleport_entity_collision_is_live_and_shared_vehicle_is_excluded() {
    for (kind, collides) in [
        (VANILLA_ENTITY_TYPE_COW_ID, false),
        (VANILLA_ENTITY_TYPE_SHULKER_ID, true),
        (VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, true),
    ] {
        let (mut store, mut body, mut level) = fixture();
        let other = store
            .spawn_general_living(
                kind,
                Vec3d {
                    y: 64.75,
                    ..destination()
                },
            )
            .unwrap();
        if kind == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
            if let EntityData::GeneralLiving(data) = &mut store.get_mut(other).unwrap().data {
                data.happy_ghast_state_mut().unwrap().set_still_timeout(5);
            }
        }
        assert_eq!(
            attempt_native(&mut body, &mut store, &mut level, destination(), false),
            Some(!collides)
        );
    }
    let (mut store, mut body, mut level) = fixture();
    let other = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_SHULKER_ID, body.position())
        .unwrap();
    let root = body.id();
    store.attach(body);
    assert!(store.link_existing_passenger_forced(root, other));
    body = store.detach(root).unwrap();
    assert_eq!(
        attempt_native(&mut body, &mut store, &mut level, destination(), false),
        Some(true)
    );
}

#[test]
fn random_teleport_border_collision_fails_but_far_outside_native_gate_can_pass() {
    let (mut store, mut body, mut level) = fixture();
    level.set_border(WorldBorder::from_settings(
        64.0, 64.0, 0.2, 5.0, 5, 300, 2.0, 0, 2.0, 0,
    ));
    assert_eq!(
        attempt_native(
            &mut body,
            &mut store,
            &mut level,
            Vec3d {
                x: 65.0,
                ..destination()
            },
            false
        ),
        Some(false)
    );
    assert_eq!(
        attempt_native(
            &mut body,
            &mut store,
            &mut level,
            Vec3d {
                x: 80.0,
                ..destination()
            },
            false
        ),
        Some(true)
    );
}

#[test]
fn random_teleport_dead_living_colliders_do_not_block_the_trial() {
    for kind in [
        VANILLA_ENTITY_TYPE_SHULKER_ID,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    ] {
        let (mut store, mut body, mut level) = fixture();
        let other = store
            .spawn_general_living(
                kind,
                Vec3d {
                    y: 64.75,
                    ..destination()
                },
            )
            .unwrap();
        crate::entity::living::set_health(store.get_mut(other).unwrap(), 0.0);
        if let EntityData::GeneralLiving(data) = &mut store.get_mut(other).unwrap().data {
            if let Some(state) = data.happy_ghast_state_mut() {
                state.set_still_timeout(5);
            }
        }
        assert_eq!(
            attempt_native(&mut body, &mut store, &mut level, destination(), false),
            Some(true)
        );
    }
}
