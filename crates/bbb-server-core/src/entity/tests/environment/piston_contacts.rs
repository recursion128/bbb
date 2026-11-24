use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use bbb_protocol::packets::Direction;

#[test]
fn piston_living_contact_hurts_without_running_the_freezing_tick_suffix() {
    for (block, expected) in [("minecraft:cactus", 1.0), ("minecraft:lava", 4.0)] {
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        let id = living_at(&mut store, "minecraft:pig", FEET);
        let mut entity = store.detach(id).unwrap();
        let health = entity.living().unwrap().health();
        entity.set_ticks_frozen(140);
        let mut level = NativeLevel::new();
        level
            .world
            .set_block(DIMENSION, pos(1, 64, 0), state_id(block), 18);
        assert!(
            level.with_level(|ctx| crate::entity::movement::move_entity_by_piston(
                &mut entity,
                &mut store,
                ctx,
                Vec3d {
                    x: 0.4,
                    y: 0.0,
                    z: 0.0
                },
                Direction::East,
            )),
            "{block}"
        );
        assert_eq!(
            entity.living().unwrap().health(),
            health - expected,
            "{block}"
        );
        if block == "minecraft:cactus" {
            assert_eq!(entity.ticks_frozen(), 140);
        }
    }
}

#[test]
fn piston_pushes_an_invulnerable_body_into_lava_without_hurting_it() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = living_at(&mut store, "minecraft:pig", FEET);
    store.get_mut(id).unwrap().set_invulnerable(true);
    let mut level = NativeLevel::new();
    level
        .world
        .set_block(DIMENSION, pos(1, 64, 0), state_id("minecraft:lava"), 18);
    let mut body = store.detach(id).unwrap();
    assert!(
        level.with_level(|ctx| crate::entity::movement::move_entity_by_piston(
            &mut body,
            &mut store,
            ctx,
            Vec3d {
                x: 0.4,
                y: 0.0,
                z: 0.0
            },
            Direction::East
        ))
    );
    store.attach(body);
    assert!(store.get(id).unwrap().position().x > FEET.x);
    assert_eq!(store.get(id).unwrap().living().unwrap().health(), 10.0);
}

#[test]
fn all_contact_entries_extinguish_fire() {
    for entry in 0..3 {
        let mut store = EntityStore::new(44, EntityIdCounter::new());
        let id = item_at(&mut store, FEET);
        let mut entity = store.detach(id).unwrap();
        entity.set_silent(true);
        entity.set_remaining_fire_ticks(100);
        entity.take_effect_movements();
        let mut level = rainy_level();
        store.drain_lifecycle_events();
        let resolved = match entry {
            0 => level.with_level(|ctx| {
                crate::entity::apply_nonliving_inside_block_effects(
                    &mut entity,
                    ctx,
                    Some(&mut store),
                )
            }),
            1 => level.with_level(|ctx| {
                crate::entity::apply_nonliving_inside_block_effects_for_last_movements(
                    &mut entity,
                    ctx,
                    Some(&mut store),
                )
            }),
            _ => level.with_level(|ctx| {
                crate::entity::apply_nonliving_inside_block_effects_between(
                    &mut entity,
                    ctx,
                    Some(&mut store),
                    FEET,
                    FEET,
                )
            }),
        };
        assert!(resolved);
        assert_eq!(entity.remaining_fire_ticks(), 0);
        let events = store.drain_lifecycle_events();
        assert!(events
            .iter()
            .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
    }
}

#[test]
fn piston_batch_keeps_nonliving_portal_contact() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = item_at(&mut store, FEET);
    let mut level = NativeLevel::new();
    level.world.set_block(
        DIMENSION,
        pos(1, 64, 0),
        state_id("minecraft:nether_portal"),
        18,
    );
    let mut body = store.detach(id).unwrap();
    assert!(
        level.with_level(|ctx| crate::entity::movement::move_entity_by_piston(
            &mut body,
            &mut store,
            ctx,
            Vec3d {
                x: 0.4,
                y: 0.0,
                z: 0.0
            },
            Direction::East
        ))
    );
    store.attach(body);
    assert!(store.get(id).unwrap().portal_process().is_some());
}
