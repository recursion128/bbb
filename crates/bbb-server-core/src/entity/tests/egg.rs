use super::native_level::NativeLevel;
use super::*;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EGG_ID;
use bbb_protocol::packets::Vec3d;

use crate::entity::item::plain_stack;
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent, EntityStore};
use crate::entity::{spawn_egg, tick_entities, EggData, EntityData};

fn egg_stack() -> RuntimeItemStack {
    plain_stack("minecraft:egg", 1).expect("egg is a built-in stack")
}

fn flying_egg() -> (EntityStore, NativeLevel, i32) {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let level = NativeLevel::new();
    let id = spawn_egg(
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
        egg_stack(),
        None,
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();
    (store, level, id)
}

fn egg_data(store: &EntityStore, id: i32) -> EggData {
    match store.get(id).expect("entity is live").data() {
        EntityData::Egg(data) => data.clone(),
        other => panic!("expected an egg, found {other:?}"),
    }
}

#[test]
fn a_spawned_egg_carries_motion_and_item_defaults() {
    let (store, _, id) = flying_egg();
    let entity = store.get(id).unwrap();
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_EGG_ID);
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0
        }
    );
    assert_eq!(egg_data(&store, id).item(), &egg_stack());
    assert!(!egg_data(&store, id).projectile().has_been_shot());
}

fn land_egg(store: &mut EntityStore, level: &mut NativeLevel, id: i32) {
    for _ in 0..200 {
        level.with_level(|level| tick_entities(store, level, true));
        if store.get(id).is_none() {
            return;
        }
    }
    panic!("egg never hit the ground");
}

#[test]
fn an_egg_falls_and_discards_on_block_hit() {
    let (mut store, mut level, id) = flying_egg();
    land_egg(&mut store, &mut level, id);
    assert!(store.get(id).is_none());
    let removed = store
        .drain_lifecycle_events()
        .into_iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Removed { .. }));
    assert!(removed);
}

#[test]
fn an_egg_round_trips_through_the_entity_chunk_save_form() {
    let (mut store, mut level, id) = flying_egg();
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
    let before_data = egg_data(&store, id);

    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(document.entities().next().unwrap().id(), "minecraft:egg");

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let after = reloaded.get(ids[0]).unwrap();
    assert_eq!(after.uuid(), before.uuid());
    assert_eq!(after.position(), before.position());
    assert_eq!(after.delta_movement(), before.delta_movement());
    assert!(after.needs_sync());

    let after_data = egg_data(&reloaded, ids[0]);
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
