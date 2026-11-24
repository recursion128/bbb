use super::*;
use crate::entity::hanging::{frame_mut, spawn_item_frame, spawn_leash_knot, spawn_painting};
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::entity::{EntityIdCounter, EntityLifecycleEvent};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::packets::{BlockPos, Direction};

fn native(drops: bool) -> NativeLevel {
    let mut native = NativeLevel::new();
    if !drops {
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::console_command(
            &sender,
            &receiver,
            &mut native.world,
            1,
            "gamerule minecraft:entity_drops false",
        );
    }
    native
}

fn body(kind: usize) -> (EntityStore, Box<Entity>) {
    let mut store = EntityStore::new(301, EntityIdCounter::new());
    let pos = BlockPos { x: 1, y: 64, z: 2 };
    let id = match kind {
        0 => spawn_painting(
            &mut store,
            pos,
            Direction::North,
            crate::entity::hanging::PaintingVariant::vanilla("minecraft:kebab"),
        )
        .unwrap(),
        1 | 2 => spawn_item_frame(&mut store, pos, Direction::North, kind == 2).unwrap(),
        3 => spawn_leash_knot(&mut store, pos).unwrap(),
        _ => unreachable!(),
    };
    store.drain_lifecycle_events();
    let mut entity = store.detach(id).unwrap();
    if let Some(frame) = frame_mut(&mut entity) {
        frame.item = crate::entity::item::plain_stack("minecraft:diamond", 1);
    }
    entity.set_delta_movement(Vec3d {
        x: 0.2,
        y: -0.3,
        z: 0.4,
    });
    entity.set_fall_distance(9.0);
    (store, entity)
}

fn assert_motion_unchanged(entity: &Entity, before: (Vec3d, Vec3d, f64)) {
    assert_eq!(entity.position(), before.0);
    assert_eq!(entity.delta_movement(), before.1);
    assert_eq!(entity.fall_distance(), before.2);
}

/// `kind`: 0 painting, 1 item frame, 2 glow item frame, 3 leash knot.
fn expected_drops(kind: usize, drops: bool) -> usize {
    match (drops, kind) {
        (false, _) | (_, 3) => 0,
        (_, 0) => 1,
        _ => 2,
    }
}

fn expected_events(kind: usize, id: i32) -> Vec<(VanillaGameEvent, Option<i32>)> {
    let mut events = vec![(VanillaGameEvent::ENTITY_DIE, Some(id))];
    if kind == 1 || kind == 2 {
        events.push((VanillaGameEvent::BLOCK_CHANGE, None));
    }
    events
}

fn assert_events(native: &NativeLevel, kind: usize, id: i32) {
    let events = native.world.positioned_game_events_for_test(DIMENSION);
    for (expected, source) in expected_events(kind, id) {
        assert!(events
            .iter()
            .any(|event| event.recorded.game_event == expected
                && event.recorded.source_entity_id == source));
    }
}

fn count(lifecycle: &[EntityLifecycleEvent], added: bool) -> usize {
    lifecycle
        .iter()
        .filter(|event| {
            if added {
                matches!(event, EntityLifecycleEvent::Added { .. })
            } else {
                matches!(event, EntityLifecycleEvent::Sound { .. })
            }
        })
        .count()
}

const CASES: [(usize, bool, bool); 4] = [
    (0, true, false),
    (1, true, false),
    (2, false, true),
    (3, true, false),
];

#[test]
fn hanging_piston_breaks_hanging_entities_without_generic_movement_or_immunity() {
    for (kind, drops, silent) in CASES {
        let (mut store, mut entity) = body(kind);
        entity.set_invulnerable(true);
        entity.set_silent(silent);
        let before = (
            entity.position(),
            entity.delta_movement(),
            entity.fall_distance(),
        );
        let mut level = native(drops);
        assert!(
            level.with_level(|level| crate::entity::movement::move_entity_by_piston(
                &mut entity,
                &mut store,
                level,
                Vec3d {
                    x: 1e-8,
                    y: 0.0,
                    z: 0.0
                },
                Direction::East
            ))
        );
        assert_motion_unchanged(&entity, before);
        assert_eq!(entity.removal_reason(), Some(RemovalReason::Killed));
        assert_events(&level, kind, entity.id());
        let lifecycle = store.drain_lifecycle_events();
        assert_eq!(count(&lifecycle, true), expected_drops(kind, drops));
        assert_eq!(
            count(&lifecycle, false),
            usize::from(!silent && (kind != 0 || drops))
        );
    }
}

#[test]
fn hanging_piston_zero_delta_and_fixed_frame_do_not_break() {
    let (mut store, mut entity) = body(1);
    let before = (
        entity.position(),
        entity.delta_movement(),
        entity.fall_distance(),
    );
    let mut level = native(true);
    assert!(
        level.with_level(|level| crate::entity::movement::move_entity_by_piston(
            &mut entity,
            &mut store,
            level,
            Vec3d::default(),
            Direction::East
        ))
    );
    assert_motion_unchanged(&entity, before);
    assert!(entity.removal_reason().is_none());
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .is_empty());
    assert!(store.drain_lifecycle_events().is_empty());

    let (mut store, mut entity) = body(1);
    frame_mut(&mut entity).unwrap().fixed = true;
    let before = (
        entity.position(),
        entity.delta_movement(),
        entity.fall_distance(),
    );
    assert!(
        level.with_level(|level| crate::entity::movement::move_entity_by_piston(
            &mut entity,
            &mut store,
            level,
            Vec3d {
                x: 0.5,
                y: 0.0,
                z: 0.0
            },
            Direction::East
        ))
    );
    assert_motion_unchanged(&entity, before);
    assert!(entity.removal_reason().is_none());
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .is_empty());
    assert!(store.drain_lifecycle_events().is_empty());
}

mod push_tests;
