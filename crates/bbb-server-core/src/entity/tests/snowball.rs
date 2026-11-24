use super::native_level::NativeLevel;
use super::*;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNOWBALL_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};

use crate::entity::item::plain_stack;
use crate::entity::portal::PortalKind;
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent, EntityStore};
use crate::entity::{
    spawn_snowball, tick_entities, tick_one_entity, EntityData, EntityTickOutcome, SnowballData,
};

fn snowball_stack() -> RuntimeItemStack {
    plain_stack("minecraft:snowball", 1).expect("snowball is a built-in stack")
}

fn flying_snowball() -> (EntityStore, NativeLevel, i32) {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let level = NativeLevel::new();
    let id = spawn_snowball(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        ProjectileShot::new(Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0,
        }),
        snowball_stack(),
        None,
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();
    (store, level, id)
}

fn snowball_data(store: &EntityStore, id: i32) -> SnowballData {
    match store.get(id).expect("entity is live").data() {
        EntityData::Snowball(data) => data.clone(),
        other => panic!("expected a snowball, found {other:?}"),
    }
}

#[test]
fn a_spawned_snowball_carries_motion_and_item_defaults() {
    let (store, _, id) = flying_snowball();
    let entity = store.get(id).unwrap();
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_SNOWBALL_ID);
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0
        }
    );
    assert_eq!(snowball_data(&store, id).item(), &snowball_stack());
    assert!(!snowball_data(&store, id).projectile().has_been_shot());
}

fn land_snowball(store: &mut EntityStore, level: &mut NativeLevel, id: i32) {
    for _ in 0..200 {
        level.with_level(|level| tick_entities(store, level, true));
        if store.get(id).is_none() {
            return;
        }
    }
    panic!("snowball never hit the ground");
}

#[test]
fn a_snowball_falls_and_discards_on_block_hit() {
    let (mut store, mut level, id) = flying_snowball();
    land_snowball(&mut store, &mut level, id);
    assert!(store.get(id).is_none());
    let removed = store
        .drain_lifecycle_events()
        .into_iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Removed { .. }));
    assert!(removed);
}

#[test]
fn throwable_sweep_contacts_an_intermediate_portal() {
    let mut store = EntityStore::new(12, EntityIdCounter::new());
    let start = Vec3d {
        x: 0.5,
        y: 68.0,
        z: 0.5,
    };
    let id = spawn_snowball(
        &mut store,
        start,
        ProjectileShot::new(Vec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        }),
        snowball_stack(),
        None,
    )
    .unwrap();
    let entry = BlockPos { x: 1, y: 68, z: 0 };
    let mut level = NativeLevel::new();
    level.set(entry, state_id("minecraft:nether_portal"));

    let EntityTickOutcome::Portal(pending) =
        level.with_level(|level| tick_one_entity(&mut store, level, id))
    else {
        panic!("the snowball contacts the portal");
    };

    assert_eq!(pending.requester_entity_id(), id);
    assert_eq!(pending.request().kind, PortalKind::Nether);
    assert_eq!(pending.request().entry, entry);
    assert!(store.get(id).unwrap().position().x > 2.0);
}

#[test]
fn a_snowball_round_trips_through_the_entity_chunk_save_form() {
    let (mut store, mut level, id) = flying_snowball();
    {
        let entity = store.get_mut(id).unwrap();
        entity.set_rot(30.0, -10.0);
        entity.set_delta_movement(Vec3d {
            x: 0.15,
            y: -0.25,
            z: 0.05,
        });
    }
    for _ in 0..2 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    let before = store.get(id).unwrap().clone();
    let before_data = snowball_data(&store, id);

    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:snowball"
    );

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let after = reloaded.get(ids[0]).unwrap();
    assert_eq!(after.uuid(), before.uuid());
    assert_eq!(after.position(), before.position());
    assert_eq!(after.delta_movement(), before.delta_movement());
    assert!(after.needs_sync());

    let after_data = snowball_data(&reloaded, ids[0]);
    assert_eq!(after_data.item(), before_data.item());
    assert_eq!(
        after_data.projectile().owner(),
        before_data.projectile().owner()
    );
    assert_eq!(
        after_data.projectile().left_owner(),
        before_data.projectile().left_owner()
    );
    assert_eq!(
        after_data.projectile().has_been_shot(),
        before_data.projectile().has_been_shot()
    );
}

#[test]
fn a_snowball_breaks_on_the_world_border() {
    let (mut store, mut level, id) = flying_snowball();
    level.set_border(bbb_server_types::interaction::WorldBorder::from_settings(
        0.0, 0.0, 0.2, 5.0, 5, 15, 4.0, 0, 4.0, 0,
    ));
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    for _ in 0..4 {
        level.with_level(|level| tick_one_entity(&mut store, level, id));
    }
    assert!(store
        .get(id)
        .is_none_or(|entity| entity.removal_reason().is_some()));
}
