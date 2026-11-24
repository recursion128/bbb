use super::*;

fn at(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

/// `GameEvent.BLOCK_CHANGE` and `GameEvent.BLOCK_DESTROY` resolve against the
/// generated `minecraft:game_event` catalog, so neither the name nor the
/// registry id is written down twice.
#[test]
fn the_named_game_events_resolve_against_the_generated_registry() {
    assert_eq!(
        VanillaGameEvent::BLOCK_ACTIVATE.resource_id(),
        "minecraft:block_activate"
    );
    assert_eq!(
        VanillaGameEvent::BLOCK_CHANGE.resource_id(),
        "minecraft:block_change"
    );
    assert_eq!(
        VanillaGameEvent::BLOCK_CLOSE.resource_id(),
        "minecraft:block_close"
    );
    assert_eq!(
        VanillaGameEvent::BLOCK_DESTROY.resource_id(),
        "minecraft:block_destroy"
    );
    assert_eq!(
        VanillaGameEvent::BLOCK_DEACTIVATE.resource_id(),
        "minecraft:block_deactivate"
    );
    assert_eq!(
        VanillaGameEvent::BLOCK_OPEN.resource_id(),
        "minecraft:block_open"
    );
    assert_eq!(
        VanillaGameEvent::BLOCK_PLACE.resource_id(),
        "minecraft:block_place"
    );
    assert_eq!(
        VanillaGameEvent::ENTITY_DAMAGE.resource_id(),
        "minecraft:entity_damage"
    );
    assert_eq!(
        VanillaGameEvent::BLOCK_ATTACH.resource_id(),
        "minecraft:block_attach"
    );
    assert_eq!(
        VanillaGameEvent::ENTITY_INTERACT.resource_id(),
        "minecraft:entity_interact"
    );
    assert_eq!(VanillaGameEvent::BLOCK_ATTACH.holder_id(), 1);
    assert_eq!(VanillaGameEvent::ENTITY_INTERACT.holder_id(), 17);
    assert_eq!(
        VanillaGameEvent::ENTITY_ACTION.resource_id(),
        "minecraft:entity_action"
    );
    assert_eq!(VanillaGameEvent::EAT.resource_id(), "minecraft:eat");
    assert_eq!(VanillaGameEvent::DRINK.resource_id(), "minecraft:drink");
    // Registration order is the registry id, and `block_destroy` is declared
    // three rows after `block_change` (`GameEvent.java:18`, `:21`).
    assert_eq!(
        VanillaGameEvent::BLOCK_DESTROY.holder_id(),
        VanillaGameEvent::BLOCK_CHANGE.holder_id() + 3
    );
    assert_eq!(VanillaGameEvent::BLOCK_CLOSE.holder_id(), 3);
    assert_eq!(VanillaGameEvent::BLOCK_OPEN.holder_id(), 7);
    assert_eq!(VanillaGameEvent::BLOCK_PLACE.holder_id(), 8);
    assert_eq!(VanillaGameEvent::SHEAR.resource_id(), "minecraft:shear");
    assert_eq!(VanillaGameEvent::SHEAR.holder_id(), 38);
    assert_eq!(
        VanillaGameEvent::FLUID_PICKUP.resource_id(),
        "minecraft:fluid_pickup"
    );
    assert_eq!(VanillaGameEvent::FLUID_PICKUP.holder_id(), 24);
    assert_eq!(
        VanillaGameEvent::FLUID_PLACE.resource_id(),
        "minecraft:fluid_place"
    );
    assert_eq!(VanillaGameEvent::FLUID_PLACE.holder_id(), 25);
}

/// The two rows the sculk chain names: the tendril click a sensor makes
/// (`SculkSensorBlock.java:217`) and the shriek a shrieker answers it with
/// (`SculkShriekerBlockEntity.java:124`). Both resolve through the same
/// generated catalog as the rows above.
#[test]
fn the_sculk_chain_game_events_resolve_against_the_generated_registry() {
    assert_eq!(
        VanillaGameEvent::SCULK_SENSOR_TENDRILS_CLICKING.resource_id(),
        "minecraft:sculk_sensor_tendrils_clicking"
    );
    assert_eq!(
        VanillaGameEvent::SCULK_SENSOR_TENDRILS_CLICKING.holder_id(),
        37
    );
    assert_eq!(VanillaGameEvent::SHRIEK.resource_id(), "minecraft:shriek");
    assert_eq!(VanillaGameEvent::SHRIEK.holder_id(), 39);
    // `shear` is registered between them (`GameEvent.java:53`, `:54`, `:55`),
    // and the row above already pins that one at 38.
    assert_eq!(
        VanillaGameEvent::SHRIEK.holder_id(),
        VanillaGameEvent::SCULK_SENSOR_TENDRILS_CLICKING.holder_id() + 2
    );
}

/// `GameEvent.ENTITY_DIE` (`GameEvent.java:31`), the row `LivingEntity.die`
/// raises (`LivingEntity.java:1447`), resolves through the same generated
/// catalog as the rows above.
#[test]
fn entity_die_resolves_against_the_generated_registry() {
    assert_eq!(
        VanillaGameEvent::ENTITY_DIE.resource_id(),
        "minecraft:entity_die"
    );
    assert_eq!(VanillaGameEvent::ENTITY_DIE.holder_id(), 15);
}

#[test]
fn queued_block_sounds_preserve_explicit_volume_and_pitch() {
    let mut sink = LevelEventSink::default();
    let pos = at(1, 2, 3);
    sink.play_block_sound_with_volume_pitch(pos, "minecraft:block.comparator.click", 0.3, 0.55);
    let sounds = sink.drain_block_sounds().collect::<Vec<_>>();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].pos, pos);
    assert_eq!(sounds[0].resource_id, "minecraft:block.comparator.click");
    assert_eq!(sounds[0].volume, 0.3);
    assert_eq!(sounds[0].pitch, 0.55);
}

/// `PlayerList.broadcast` compares squared distances with a strict `<`
/// (`PlayerList.java:612-614`), so the boundary is exclusive.
#[test]
fn a_listener_exactly_at_the_range_is_not_reached() {
    let pos = at(0, 64, 0);
    assert!(level_event_reaches(
        pos,
        LEVEL_EVENT_BROADCAST_RANGE,
        63.999,
        64.0,
        0.0
    ));
    assert!(!level_event_reaches(
        pos,
        LEVEL_EVENT_BROADCAST_RANGE,
        64.0,
        64.0,
        0.0
    ));
    assert!(!level_event_reaches(
        pos,
        LEVEL_EVENT_BROADCAST_RANGE,
        64.001,
        64.0,
        0.0
    ));
}

/// The distance is measured from the raw block coordinates, not from the block
/// centre (`PlayerList.java:609-611`).
#[test]
fn the_broadcast_distance_is_measured_from_the_raw_block_coordinates() {
    // Half a block past the range on the +x axis. Measured from the corner the
    // listener is out of range; measured from the centre it would be inside.
    let pos = at(0, 0, 0);
    assert!(!level_event_reaches(
        pos,
        LEVEL_EVENT_BROADCAST_RANGE,
        64.4,
        0.0,
        0.0
    ));
}

#[test]
fn the_sink_keeps_level_events_in_order_and_drains_them_once() {
    let mut sink = LevelEventSink::default();
    assert!(!sink.has_pending_level_events());
    sink.level_event(LEVEL_EVENT_PARTICLES_DESTROY_BLOCK, at(1, 2, 3), 7);
    sink.level_event(LEVEL_EVENT_LAVA_FIZZ, at(4, 5, 6), 0);
    assert!(sink.has_pending_level_events());
    let drained: Vec<_> = sink.drain_level_broadcasts().collect();
    assert_eq!(
        drained,
        vec![
            PendingLevelBroadcast::LevelEvent(PendingLevelEvent {
                event_type: LEVEL_EVENT_PARTICLES_DESTROY_BLOCK,
                pos: at(1, 2, 3),
                data: 7,
            }),
            PendingLevelBroadcast::LevelEvent(PendingLevelEvent {
                event_type: LEVEL_EVENT_LAVA_FIZZ,
                pos: at(4, 5, 6),
                data: 0,
            }),
        ]
    );
    assert!(!sink.has_pending_level_events());
}

/// The game-event landing point: the count is unbounded and the log is not, so
/// a body that raises one is observable even after the bound has scrolled.
#[test]
fn the_sink_counts_every_game_event_and_keeps_the_last_ones() {
    let mut sink = LevelEventSink::default();
    for index in 0..(MAX_RECORDED_GAME_EVENTS as i32 + 5) {
        sink.game_event(
            VanillaGameEvent::BLOCK_DESTROY,
            at(index, 0, 0),
            Some(index),
        );
    }
    assert_eq!(sink.game_event_count(), MAX_RECORDED_GAME_EVENTS as u64 + 5);
    let kept: Vec<_> = sink.recent_game_events().collect();
    assert_eq!(kept.len(), MAX_RECORDED_GAME_EVENTS);
    assert_eq!(kept[0].pos, at(5, 0, 0));
    assert_eq!(
        kept[MAX_RECORDED_GAME_EVENTS - 1].pos,
        at(MAX_RECORDED_GAME_EVENTS as i32 + 4, 0, 0)
    );
    assert_eq!(kept[0].game_event, VanillaGameEvent::BLOCK_DESTROY);
    assert_eq!(kept[0].source_entity_id, None);
    assert_eq!(kept[0].affected_state, Some(5));
}

#[test]
fn the_sink_retains_entity_game_event_source_identity() {
    let mut sink = LevelEventSink::default();
    sink.game_event_with_source(VanillaGameEvent::ENTITY_DAMAGE, at(1, 2, 3), Some(42), None);
    let recorded = sink.recent_game_events().next().expect("one event");
    assert_eq!(recorded.game_event, VanillaGameEvent::ENTITY_DAMAGE);
    assert_eq!(recorded.source_entity_id, Some(42));
    assert_eq!(recorded.affected_state, None);
}

#[test]
fn door_animation_keeps_level_event_and_crack_order_for_both_hands() {
    use bbb_protocol::packets::InteractionHand;
    for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
        let mut sink = LevelEventSink::default();
        let pos = at(1, 2, 3);
        sink.level_event(1019, pos, 0);
        sink.entity_animation(7, hand);
        sink.destroy_block_progress(7, pos, 4);
        sink.level_event(1021, pos, 0);
        assert!(sink.has_pending_level_events());
        assert_eq!(
            sink.drain_level_broadcasts().collect::<Vec<_>>(),
            vec![
                PendingLevelBroadcast::LevelEvent(PendingLevelEvent {
                    event_type: 1019,
                    pos,
                    data: 0
                }),
                PendingLevelBroadcast::EntityAnimation { entity_id: 7, hand },
                PendingLevelBroadcast::BlockDestruction {
                    entity_id: 7,
                    pos,
                    stage: 4
                },
                PendingLevelBroadcast::LevelEvent(PendingLevelEvent {
                    event_type: 1021,
                    pos,
                    data: 0
                }),
            ]
        );
        assert!(!sink.has_pending_level_events());
    }
}

/// The dispatcher queue is the sink's own lossless half: a raise is handed out
/// once, in order, and the bounded record is not what it reads.
#[test]
fn the_sink_hands_each_game_event_to_the_dispatcher_once_in_order() {
    let mut sink = LevelEventSink::default();
    sink.game_event(VanillaGameEvent::BLOCK_DESTROY, at(1, 2, 3), None);
    sink.game_event_with_source(VanillaGameEvent::ENTITY_DIE, at(4, 5, 6), Some(42), None);

    assert!(sink.has_pending_game_events());
    assert_eq!(
        sink.drain_game_events(),
        vec![
            RecordedGameEvent {
                game_event: VanillaGameEvent::BLOCK_DESTROY,
                pos: at(1, 2, 3),
                source_entity_id: None,
                affected_state: None,
            },
            RecordedGameEvent {
                game_event: VanillaGameEvent::ENTITY_DIE,
                pos: at(4, 5, 6),
                source_entity_id: Some(42),
                affected_state: None,
            },
        ]
    );
    assert!(!sink.has_pending_game_events());
    assert!(sink.drain_game_events().is_empty());
}

#[test]
fn block_events_center_once_and_vector_events_keep_their_exact_origin() {
    let mut sink = LevelEventSink::default();
    let block = at(0, 0, 0);
    let raw = Vec3d {
        x: 0.9,
        y: 0.5,
        z: 0.5,
    };
    sink.game_event(VanillaGameEvent::BLOCK_CHANGE, block, None);
    sink.game_event_with_source_at(VanillaGameEvent::ENTITY_MOUNT, raw, Some(41), None);

    let recent = sink
        .recent_positioned_game_events()
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(recent.len(), 2);
    assert_eq!(
        recent[0].position,
        Vec3d {
            x: 0.5,
            y: 0.5,
            z: 0.5
        }
    );
    assert_eq!(recent[1].position, raw);
    assert_eq!(recent[1].recorded.pos, block);
    assert_eq!(
        sink.recent_game_events().copied().collect::<Vec<_>>(),
        recent
            .iter()
            .map(|event| event.recorded)
            .collect::<Vec<_>>()
    );
    assert_eq!(sink.game_event_count(), 2);
    assert_eq!(sink.drain_positioned_game_events(), recent);
    assert!(!sink.has_pending_game_events());
    assert!(sink.drain_game_events().is_empty());
}

/// `dropAllDeathLoot` queues the award before the frame's `ENTITY_DIE`
/// listeners run, so the sink keeps the death order until the world flush reads
/// the victim's `wasExperienceConsumed`.
#[test]
fn death_experience_waits_in_raise_order_for_its_flush() {
    let mut sink = LevelEventSink::default();
    let first = bbb_protocol::packets::Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let second = bbb_protocol::packets::Vec3d {
        x: 1.5,
        y: 65.0,
        z: 2.5,
    };
    sink.queue_death_experience(7, first, None);
    sink.queue_death_experience(9, second, Some(3));

    let pending = sink.drain_death_experience();
    assert_eq!(
        pending,
        vec![
            PendingDeathExperience {
                entity_id: 7,
                position: first,
                killer_entity_id: None,
            },
            PendingDeathExperience {
                entity_id: 9,
                position: second,
                killer_entity_id: Some(3),
            },
        ]
    );
    assert!(sink.drain_death_experience().is_empty());
}
