use super::native_level::NativeLevel;
use super::*;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDER_PEARL_ID;
use bbb_protocol::packets::Vec3d;

use crate::entity::item::plain_stack;
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::{EntityIdCounter, EntityStore};
use crate::entity::{spawn_ender_pearl, tick_entities, EnderPearlData, EntityData};

fn ender_pearl_stack() -> RuntimeItemStack {
    plain_stack("minecraft:ender_pearl", 1).expect("ender pearl is a built-in stack")
}

fn flying_ender_pearl() -> (EntityStore, NativeLevel, i32) {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let level = NativeLevel::new();
    let id = spawn_ender_pearl(
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
        ender_pearl_stack(),
        None,
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();
    (store, level, id)
}

fn ender_pearl_data(store: &EntityStore, id: i32) -> EnderPearlData {
    match store.get(id).expect("entity is live").data() {
        EntityData::EnderPearl(data) => data.clone(),
        other => panic!("expected an ender pearl, found {other:?}"),
    }
}

#[test]
fn a_spawned_ender_pearl_carries_motion_and_item_defaults() {
    let (store, _, id) = flying_ender_pearl();
    let entity = store.get(id).unwrap();
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_ENDER_PEARL_ID);
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0
        }
    );
    assert_eq!(ender_pearl_data(&store, id).item(), &ender_pearl_stack());
    assert!(!ender_pearl_data(&store, id).projectile().has_been_shot());
}

fn land_ender_pearl(store: &mut EntityStore, level: &mut NativeLevel, id: i32) {
    for _ in 0..200 {
        level.with_level(|level| tick_entities(store, level, true));
        if store.drain_commands().into_iter().any(|command| matches!(command, crate::entity::EntityCommand::EnderPearlHit { entity_id, target: None } if entity_id == id)) {
            return;
        }
    }
    panic!("ender pearl never hit the ground");
}

#[test]
fn an_ender_pearl_falls_and_hands_block_hit_to_the_world() {
    let (mut store, mut level, id) = flying_ender_pearl();
    land_ender_pearl(&mut store, &mut level, id);
    let pearl = store
        .get(id)
        .expect("impact keeps its direct damage source until the world runs");
    assert!(pearl.position().y < 64.1);
}

#[test]
fn an_ender_pearl_round_trips_through_the_entity_chunk_save_form() {
    let (mut store, mut level, id) = flying_ender_pearl();
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
    let before_data = ender_pearl_data(&store, id);

    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:ender_pearl"
    );

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let after = reloaded.get(ids[0]).unwrap();
    assert_eq!(after.uuid(), before.uuid());
    assert_eq!(after.position(), before.position());
    assert_eq!(after.delta_movement(), before.delta_movement());
    assert!(after.needs_sync());

    let after_data = ender_pearl_data(&reloaded, ids[0]);
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
