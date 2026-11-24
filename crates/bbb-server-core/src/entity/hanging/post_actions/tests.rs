use super::*;
use crate::entity::{EntityIdCounter, EntityLifecycleEvent, RemovalReason};

fn source() -> HangHurtSource {
    HangHurtSource {
        is_explosion: false,
        is_fire: false,
        is_fall: false,
        fall_damage_immune: false,
        from_mob: false,
        mob_griefing: true,
        caused_by_entity: true,
        creative_player: false,
        bypasses_invulnerability: false,
    }
}

fn frame(chance: f32, has_item: bool, glow: bool) -> (EntityStore, Box<Entity>) {
    let mut store = EntityStore::new(817, EntityIdCounter::new());
    let id = spawn_item_frame(
        &mut store,
        BlockPos { x: 1, y: 64, z: 2 },
        Direction::North,
        glow,
    )
    .unwrap();
    let mut entity = store.detach(id).unwrap();
    let frame = frame_mut(&mut entity).unwrap();
    frame.item = has_item.then(|| plain_stack("minecraft:diamond", 1).unwrap());
    frame.drop_chance = chance;
    store.drain_lifecycle_events();
    (store, entity)
}

#[test]
fn hanging_post_actions_drop_chance_gates_the_framed_item() {
    for (chance, dropped) in [(0.0, 0), (1.0, 1)] {
        let (mut store, mut entity) = frame(chance, true, false);
        let effects =
            hurt_server_with_post_actions(&mut entity, &mut store, true, source()).unwrap();
        assert!(effects.outcome);
        let mut events = LevelEventSink::default();
        apply_hanging_post_actions(effects.actions, &mut store, &mut events, None);
        let lifecycle = store.drain_lifecycle_events();
        let drops = lifecycle
            .iter()
            .filter(|event| matches!(event, EntityLifecycleEvent::Added { .. }))
            .count();
        assert_eq!(drops, dropped, "chance {chance}");
        assert_eq!(events.drain_game_events().len(), 1);
    }
}

fn map_item() -> RuntimeItemStack {
    use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Value};
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &Value::Compound(vec![
            Entry {
                name: "id".into(),
                value: Value::String("minecraft:filled_map".into()),
            },
            Entry {
                name: "count".into(),
                value: Value::Int(1),
            },
            Entry {
                name: "components".into(),
                value: Value::Compound(vec![Entry {
                    name: "minecraft:map_id".into(),
                    value: Value::Int(17),
                }]),
            },
        ]),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .unwrap()
}

#[test]
fn hanging_post_actions_silent_frame_keeps_drop_map_and_event() {
    for glow in [false, true] {
        for explosion in [false, true] {
            let (mut store, mut entity) = frame(1.0, true, glow);
            frame_mut(&mut entity).unwrap().item = Some(map_item());
            entity.set_silent(true);
            let id = entity.id();
            let effects = hurt_server_with_post_actions(
                &mut entity,
                &mut store,
                true,
                HangHurtSource {
                    is_explosion: explosion,
                    ..source()
                },
            )
            .unwrap();
            assert_eq!(effects.emits_entity_die, explosion);
            assert_eq!(
                entity.removal_reason(),
                explosion.then_some(RemovalReason::Killed)
            );
            store.attach(entity);
            let mut events = LevelEventSink::default();
            apply_hanging_post_actions(effects.actions, &mut store, &mut events, Some(99));
            let lifecycle = store.drain_lifecycle_events();
            assert!(!lifecycle
                .iter()
                .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
            if explosion {
                assert_eq!(
                    lifecycle
                        .iter()
                        .filter(|event| matches!(event, EntityLifecycleEvent::Removed { entity, .. }
                            if entity.id() == id && entity.removal_reason() == Some(RemovalReason::Killed)))
                        .count(),
                    1
                );
            }
            assert_eq!(
                lifecycle
                    .iter()
                    .filter(|event| matches!(event, EntityLifecycleEvent::Added { .. }))
                    .count(),
                if explosion { 2 } else { 1 }
            );
            assert_eq!(
                store.drain_framed_map_removals(),
                vec![crate::entity::FramedMapRemoval {
                    map_id: 17,
                    pos: BlockPos { x: 1, y: 64, z: 2 },
                    entity_id: id
                }]
            );
            assert_eq!(events.drain_game_events().len(), 1);
        }
    }
}

#[test]
fn hanging_post_actions_silent_painting_and_knot_keep_existing_drop_gates() {
    for painting in [false, true] {
        for entity_drops in [false, true] {
            for silent in [false, true] {
                let mut store = EntityStore::new(991, EntityIdCounter::new());
                let attached = BlockPos { x: 0, y: 64, z: 0 };
                let id = if painting {
                    spawn_painting(
                        &mut store,
                        attached,
                        Direction::North,
                        crate::entity::hanging::PaintingVariant::vanilla("minecraft:kebab"),
                    )
                    .unwrap()
                } else {
                    spawn_leash_knot(&mut store, attached).unwrap()
                };
                store.drain_lifecycle_events();
                let mut entity = store.detach(id).unwrap();
                entity.set_silent(silent);
                let effects =
                    hurt_server_with_post_actions(&mut entity, &mut store, entity_drops, source())
                        .unwrap();
                assert!(effects.outcome && effects.emits_entity_die);
                store.attach(entity);
                apply_hanging_post_actions(
                    effects.actions,
                    &mut store,
                    &mut LevelEventSink::default(),
                    None,
                );
                let lifecycle = store.drain_lifecycle_events();
                let sounds = lifecycle
                    .iter()
                    .filter(|event| matches!(event, EntityLifecycleEvent::Sound { .. }))
                    .count();
                assert_eq!(sounds, usize::from(!silent && (!painting || entity_drops)));
                let drops = lifecycle
                    .iter()
                    .filter(|event| matches!(event, EntityLifecycleEvent::Added { .. }))
                    .count();
                assert_eq!(drops, usize::from(painting && entity_drops));
            }
        }
    }
}
