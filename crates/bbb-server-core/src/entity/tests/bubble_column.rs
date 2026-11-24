use super::native_level::NativeLevel;
use super::*;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID;
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_BUBBLE_ID;

use crate::entity::item::plain_stack;
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::{boat, spawn_boat, spawn_snowball, tick_entities};

const TOP: i32 = 104;

/// A soul sand (`drag = false`) or magma (`drag = true`) column from y=100 to
/// `TOP`, open air above.
fn column(native: &mut NativeLevel, drag: bool) {
    let floor = if drag {
        "minecraft:magma_block"
    } else {
        "minecraft:soul_sand"
    };
    native.set(pos(0, 99, 0), state_id(floor));
    let drag = if drag { "true" } else { "false" };
    for y in 100..=TOP {
        native.set(
            pos(0, y, 0),
            state_with("minecraft:bubble_column", &[("drag", drag)]),
        );
    }
}

fn bubble_particles(store: &mut EntityStore) -> usize {
    store
        .drain_lifecycle_events()
        .iter()
        .filter(|event| {
            matches!(
                event,
                EntityLifecycleEvent::Particles { particles }
                    if particles.particle.particle_type_id == VANILLA_PARTICLE_TYPE_BUBBLE_ID
            )
        })
        .count()
}

#[test]
fn an_item_rides_an_upward_bubble_column_and_keeps_ageing() {
    let mut native = NativeLevel::new();
    column(&mut native, false);
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = spawn_item_entity(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 100.1,
            z: 0.5,
        },
        stack_of("minecraft:stone", 1),
    )
    .unwrap();
    let age = item_state(&store, id).age();
    store.drain_lifecycle_events();
    let mut highest: f64 = 0.0;
    let mut particles = 0;
    for _ in 0..20 {
        native.with_level(|level| tick_entities(&mut store, level, true));
        highest = highest.max(store.get(id).unwrap().position().y);
        particles += bubble_particles(&mut store);
    }
    assert!(highest > f64::from(TOP) + 0.5, "item reached {highest}");
    assert_eq!(item_state(&store, id).age(), age + 20);
    assert!(particles > 0);
}

#[test]
fn a_boat_over_a_downward_bubble_column_is_dragged_down() {
    let mut native = NativeLevel::new();
    column(&mut native, true);
    let mut store = EntityStore::new(0x0b_0a_71, EntityIdCounter::new());
    let id = spawn_boat(
        &mut store,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        Vec3d {
            x: 0.5,
            y: f64::from(TOP) + 0.6,
            z: 0.5,
        },
        90.0,
    )
    .unwrap();
    native.with_level(|level| tick_entities(&mut store, level, true));
    let data = *boat::boat(store.get(id).unwrap().data()).unwrap();
    assert_eq!(data.bubble_time(), boat::BUBBLE_TIME - 1);
    let mut lowest = f64::MAX;
    for _ in 0..boat::BUBBLE_TIME {
        native.with_level(|level| tick_entities(&mut store, level, true));
        lowest = lowest.min(store.get(id).unwrap().delta_movement().y);
    }
    assert!(lowest < -0.6, "the strongest pull was {lowest}");
}

#[test]
fn a_projectile_above_a_bubble_column_is_pushed_without_aborting_its_tick() {
    let pushed = |with_column: bool| {
        let mut native = NativeLevel::new();
        if with_column {
            column(&mut native, false);
        }
        let mut store = EntityStore::new(11, EntityIdCounter::new());
        let id = spawn_snowball(
            &mut store,
            Vec3d {
                x: 0.5,
                y: f64::from(TOP) + 0.3,
                z: 0.5,
            },
            ProjectileShot::new(Vec3d::default()),
            plain_stack("minecraft:snowball", 1).unwrap(),
            None,
        )
        .unwrap();
        store.drain_lifecycle_events();
        native.with_level(|level| tick_entities(&mut store, level, true));
        (
            store.get(id).unwrap().delta_movement().y,
            bubble_particles(&mut store),
        )
    };
    let (dry, dry_particles) = pushed(false);
    let (wet, wet_particles) = pushed(true);
    assert!((wet - dry - 0.1).abs() < 1e-9, "{dry} -> {wet}");
    assert_eq!((dry_particles, wet_particles), (0, 2));
}
