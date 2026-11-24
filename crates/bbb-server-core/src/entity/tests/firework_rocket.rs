use super::native_level::NativeLevel;
use super::*;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::item::plain_stack;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent, EntityStore};
use crate::entity::{
    spawn_firework_rocket, tick_entities, tick_one_entity, EntityData, EntityTickOutcome,
    FireworkRocketData,
};

fn firework_stack() -> RuntimeItemStack {
    plain_stack("minecraft:firework_rocket", 1).expect("firework rocket is a built-in stack")
}

fn exploding_firework_stack() -> RuntimeItemStack {
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string("minecraft:firework_rocket"),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(1),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "minecraft:fireworks".into(),
                    value: NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "flight_duration".into(),
                            value: NbtSummaryValue::Int(1),
                        },
                        NbtSummaryEntry {
                            name: "explosions".into(),
                            value: NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                                NbtSummaryEntry {
                                    name: "shape".into(),
                                    value: NbtSummaryValue::string("small_ball"),
                                },
                            ])]),
                        },
                    ]),
                }]),
            },
        ]),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .expect("exploding firework stack")
}

fn flying_firework() -> (EntityStore, NativeLevel, i32) {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let level = NativeLevel::new();
    let id = spawn_firework_rocket(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        Vec3d {
            x: 0.0,
            y: 0.05,
            z: 0.0,
        },
        firework_stack(),
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();
    (store, level, id)
}

fn firework_data(store: &EntityStore, id: i32) -> FireworkRocketData {
    match store.get(id).expect("entity is live").data() {
        EntityData::FireworkRocket(data) => data.clone(),
        other => panic!("expected a firework rocket, found {other:?}"),
    }
}

#[test]
fn a_spawned_firework_rocket_carries_motion_and_default_lifetime() {
    let (store, _, id) = flying_firework();
    let entity = store.get(id).unwrap();
    assert_eq!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID
    );
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: 0.0,
            y: 0.05,
            z: 0.0
        }
    );
    assert_eq!(firework_data(&store, id).lifetime(), 20);
    assert_eq!(firework_data(&store, id).life(), 0);
}

#[test]
fn a_firework_rocket_discards_after_its_lifetime() {
    let (mut store, mut level, id) = flying_firework();
    for _ in 0..30 {
        level.with_level(|level| tick_entities(&mut store, level, true));
        if store.get(id).is_none() {
            break;
        }
    }
    assert!(store.get(id).is_none());
    let removed = store
        .drain_lifecycle_events()
        .into_iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Removed { .. }));
    assert!(removed);
}

#[test]
fn a_firework_sweep_contacts_a_portal() {
    let start = Vec3d {
        x: 0.5,
        y: 68.0,
        z: 0.5,
    };
    let mut store = EntityStore::new(12, EntityIdCounter::new());
    let id = spawn_firework_rocket(
        &mut store,
        start,
        Vec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        },
        firework_stack(),
    )
    .unwrap();
    let entry = BlockPos { x: 1, y: 68, z: 0 };
    let mut level = NativeLevel::new();
    level.set(entry, state_id("minecraft:nether_portal"));

    let pending = (0..3)
        .find_map(
            |_| match level.with_level(|level| tick_one_entity(&mut store, level, id)) {
                EntityTickOutcome::Portal(pending) => Some(pending),
                _ => None,
            },
        )
        .expect("the firework contacts the portal");
    assert_eq!(pending.request().entry, entry);
    assert!(store.get(id).unwrap().position().x > start.x);
}

#[test]
fn an_exploding_firework_rocket_discards_on_block_hit() {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    level.set(BlockPos { x: 0, y: 66, z: 0 }, state_id("minecraft:stone"));
    let id = spawn_firework_rocket(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0,
        },
        exploding_firework_stack(),
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();

    for _ in 0..200 {
        level.with_level(|level| tick_entities(&mut store, level, true));
        if store.get(id).is_none() {
            break;
        }
    }
    assert!(store.get(id).is_none());
    // Projectile.java:99-103,296-306.
    use crate::level_event::VanillaGameEvent;
    let events: Vec<_> = level
        .world
        .positioned_game_events_for_test(super::native_level::DIMENSION)
        .into_iter()
        .map(|event| (event.recorded.game_event, event.recorded.source_entity_id))
        .collect();
    assert!(events.contains(&(VanillaGameEvent::PROJECTILE_SHOOT, None)));
    assert!(events.contains(&(VanillaGameEvent::PROJECTILE_LAND, Some(id))));
}

#[test]
fn a_firework_rocket_round_trips_through_the_entity_chunk_save_form() {
    let (mut store, mut level, id) = flying_firework();
    {
        let entity = store.get_mut(id).unwrap();
        entity.set_delta_movement(Vec3d {
            x: 0.1,
            y: 0.2,
            z: 0.3,
        });
    }
    for _ in 0..3 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }

    let before_data = firework_data(&store, id);
    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:firework_rocket"
    );

    let mut reloaded = EntityStore::new(11, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("entity chunk read succeeds");
    assert_eq!(ids.len(), 1);
    let after_data = firework_data(&reloaded, ids[0]);
    assert_eq!(after_data.item(), before_data.item());
    assert_eq!(after_data.life(), before_data.life());
    assert_eq!(after_data.lifetime(), before_data.lifetime());
    assert_eq!(after_data.shot_at_angle(), before_data.shot_at_angle());
}

#[test]
fn a_boosting_rocket_explodes_when_its_body_flies_into_a_wall() {
    let boost = |wall: bool| {
        let mut store = EntityStore::new(11, EntityIdCounter::new());
        let mut level = NativeLevel::new();
        if wall {
            level.set(BlockPos { x: 2, y: 68, z: 0 }, state_id("minecraft:stone"));
        }
        let position = Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        };
        let cow = store
            .spawn_general_living(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
                position,
            )
            .unwrap();
        store.get_mut(cow).unwrap().set_delta_movement(Vec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        });
        let owner = store.get(cow).unwrap().uuid();
        let rocket = crate::entity::firework_rocket::spawn_launched_firework(
            &mut store,
            position,
            exploding_firework_stack(),
            owner,
            Some(cow),
        )
        .unwrap();
        level.with_level(|level| tick_one_entity(&mut store, level, rocket));
        level
            .world
            .positioned_game_events_for_test(super::native_level::DIMENSION)
            .into_iter()
            .any(|event| event.recorded.game_event == crate::level_event::VanillaGameEvent::EXPLODE)
    };
    assert!(!boost(false));
    assert!(boost(true));
}

#[test]
fn a_rocket_leaves_its_owner_once_clear_of_it() {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    let position = Vec3d {
        x: 0.5,
        y: 68.0,
        z: 0.5,
    };
    let zombie = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 0.5,
                y: 40.0,
                z: 0.5,
            },
        )
        .unwrap();
    let owner = store.get(zombie).unwrap().uuid();
    let rocket = crate::entity::firework_rocket::spawn_launched_firework(
        &mut store,
        position,
        firework_stack(),
        owner,
        None,
    )
    .unwrap();
    level.with_level(|level| tick_one_entity(&mut store, level, rocket));
    assert!(firework_data(&store, rocket).projectile().left_owner());
}
