use std::collections::BTreeMap;

use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::*;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::EntityIdCounter;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::entity::{tick_entities, RemovalReason};
use crate::level_ticks::ChunkKey;

fn state_id(name: &str) -> i32 {
    vanilla_block_state_id_by_name_and_properties(name, &BTreeMap::new()).unwrap()
}

fn state_id_with(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties = properties
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect();
    vanilla_block_state_id_by_name_and_properties(name, &properties).unwrap()
}

fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

fn kebab() -> PaintingVariant {
    PaintingVariant::vanilla("minecraft:kebab")
}

fn map_stack(map_id: i32) -> RuntimeItemStack {
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string("minecraft:filled_map"),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(1),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "minecraft:map_id".into(),
                    value: NbtSummaryValue::Int(map_id),
                }]),
            },
        ]),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .unwrap()
}

fn apply_test_interact(
    entity: &mut Entity,
    held: Option<RuntimeItemStack>,
    admission: FramedMapAdmission,
) -> HangInteract {
    let plan = prepare_interact(entity, held, admission);
    let expects_apply = matches!(plan, HangInteract::Add { .. } | HangInteract::Rotate { .. });
    let applied = apply_interact(entity, &plan, |_, _| {});
    assert_eq!(applied, expects_apply);
    plan
}

fn apply_test_non_map_interact(
    entity: &mut Entity,
    held: Option<RuntimeItemStack>,
) -> HangInteract {
    apply_test_interact(entity, held, FramedMapAdmission::NotFramedMap)
}

#[test]
fn a_one_by_one_painting_centres_on_the_attached_wall() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let attached = pos(0, 64, 0);
    let id = spawn_painting(&mut store, attached, Direction::South, kebab()).unwrap();
    let entity = store.get(id).unwrap();
    let box_ = entity.bounding_box();
    assert!((box_.max_z - box_.min_z - DEPTH).abs() < 1.0e-9);
    assert!((box_.max_x - box_.min_x - 1.0).abs() < 1.0e-9);
    assert!((box_.max_y - box_.min_y - 1.0).abs() < 1.0e-9);
    match entity.data() {
        EntityData::Painting(painting) => {
            assert_eq!(painting.pose.attached, attached);
            assert_eq!(painting.pose.direction, Direction::South);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_painting_survives_on_solid_stone_and_dies_in_mid_air() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let attached = pos(0, 64, 0);
    let id = spawn_painting(&mut store, attached, Direction::South, kebab()).unwrap();
    let entity = store.get(id).unwrap();
    let mut supported = NativeLevel::new();
    supported.set(pos(0, 64, -1), state_id("minecraft:stone"));
    assert!(supported.with_level(|level| survives(entity, &store, level)));
    let mut air = NativeLevel::new();
    assert!(!air.with_level(|level| survives(entity, &store, level)));
}

#[test]
fn unbacked_painting_breaks_with_sound_and_item_after_real_ticks() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let attached = pos(0, 64, 0);
    let id = spawn_painting(&mut store, attached, Direction::South, kebab()).unwrap();
    store.drain_lifecycle_events();
    let mut level = NativeLevel::new();
    for _ in 0..SURVIVAL_CHECK_INTERVAL {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert!(store.get(id).is_none());
    let lifecycle = store.drain_lifecycle_events();
    assert!(lifecycle.iter().any(|event| matches!(event, crate::entity::store::EntityLifecycleEvent::Removed { entity, .. } if entity.id() == id && entity.removal_reason() == Some(RemovalReason::Discarded))));
    assert!(lifecycle.iter().any(|event| matches!(
        event,
        crate::entity::store::EntityLifecycleEvent::Sound { .. }
    )));
    assert!(lifecycle.iter().any(|event| matches!(
        event,
        crate::entity::store::EntityLifecycleEvent::Added { .. }
    )));
    let drops: Vec<_> = store
        .ids()
        .into_iter()
        .filter_map(|drop_id| {
            let entity = store.get(drop_id)?;
            match entity.data() {
                EntityData::Item(item) => Some(item.item().clone()),
                _ => None,
            }
        })
        .collect();
    assert_eq!(drops.len(), 1);
}

#[test]
fn item_frame_first_hurt_drops_the_item_and_the_second_drops_the_frame() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let attached = pos(0, 64, 0);
    let id = spawn_item_frame(&mut store, attached, Direction::North, false).unwrap();
    {
        let entity = store.get_mut(id).unwrap();
        assert!(matches!(
            apply_test_non_map_interact(entity, plain_stack("minecraft:diamond", 1)),
            HangInteract::Add { .. }
        ));
    }
    let source = HangHurtSource {
        is_explosion: false,
        is_fire: false,
        is_fall: false,
        fall_damage_immune: false,
        from_mob: false,
        mob_griefing: true,
        caused_by_entity: false,
        creative_player: false,
        bypasses_invulnerability: false,
    };
    let mut level = NativeLevel::new();
    {
        let mut entity = store.detach(id).unwrap();
        let effects = hurt_server_with_post_actions(&mut entity, &mut store, true, source).unwrap();
        assert!(effects.outcome);
        assert!(entity.is_alive());
        assert!(!entity.hurt_marked());
        match entity.data() {
            EntityData::ItemFrame(frame) => assert!(frame.item.is_none()),
            other => panic!("{other:?}"),
        }
        store.attach(entity);
        level.with_level(|level| {
            apply_hanging_post_actions_in_level(effects.actions, &mut store, level, None)
        });
    }
    {
        let mut entity = store.detach(id).unwrap();
        let effects = hurt_server_with_post_actions(&mut entity, &mut store, true, source).unwrap();
        assert!(effects.outcome);
        assert_eq!(entity.removal_reason(), Some(RemovalReason::Killed));
        store.attach(entity);
        level.with_level(|level| {
            apply_hanging_post_actions_in_level(effects.actions, &mut store, level, None)
        });
    }
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::BLOCK_CHANGE));
}

#[test]
fn removing_a_framed_item_retains_rotation_and_syncs_only_the_item() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = spawn_item_frame(&mut store, pos(0, 64, 0), Direction::North, false).unwrap();
    let mut entity = store.detach(id).unwrap();
    assert!(matches!(
        apply_test_non_map_interact(&mut entity, plain_stack("minecraft:diamond", 1)),
        HangInteract::Add { .. }
    ));
    assert!(matches!(
        apply_test_non_map_interact(&mut entity, None),
        HangInteract::Rotate { .. }
    ));
    assert!(matches!(
        apply_test_non_map_interact(&mut entity, None),
        HangInteract::Rotate { .. }
    ));
    entity.take_synched_dirty();
    let effects = hurt_server(
        &mut entity,
        true,
        HangHurtSource {
            is_explosion: false,
            is_fire: false,
            is_fall: false,
            fall_damage_immune: false,
            from_mob: false,
            mob_griefing: true,
            caused_by_entity: false,
            creative_player: false,
            bypasses_invulnerability: false,
        },
    );
    assert!(effects.outcome);
    let EntityData::ItemFrame(frame) = entity.data() else {
        panic!("target was not an item frame")
    };
    assert!(frame.item().is_none());
    assert_eq!(frame.rotation(), 2);
    assert_eq!(
        entity.take_synched_dirty(),
        [VANILLA_ITEM_FRAME_ITEM_DATA_ID]
    );
}

#[test]
fn framed_map_attack_clear_recalculates_box_and_queues_one_removal() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = spawn_item_frame(&mut store, pos(0, 64, 0), Direction::North, false).unwrap();
    let mut entity = store.detach(id).unwrap();
    assert!(matches!(
        apply_test_interact(
            &mut entity,
            Some(map_stack(7)),
            FramedMapAdmission::SavedDataMissing,
        ),
        HangInteract::Add { .. }
    ));
    let map_width = entity.bounding_box().max_x - entity.bounding_box().min_x;
    entity.take_synched_dirty();
    let effects = hurt_server_with_post_actions(
        &mut entity,
        &mut store,
        true,
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
        },
    )
    .unwrap();
    assert!(effects.outcome);
    let empty_width = entity.bounding_box().max_x - entity.bounding_box().min_x;
    assert!((map_width - 1.0).abs() < 1.0e-9);
    assert!((empty_width - FRAME_SIZE).abs() < 1.0e-9);
    assert_eq!(
        entity.take_synched_dirty(),
        [VANILLA_ITEM_FRAME_ITEM_DATA_ID]
    );
    store.attach(entity);
    let mut level = NativeLevel::new();
    level.with_level(|level| {
        apply_hanging_post_actions_in_level(effects.actions, &mut store, level, None)
    });
    assert_eq!(
        store.drain_framed_map_removals(),
        vec![crate::entity::FramedMapRemoval {
            map_id: 7,
            pos: pos(0, 64, 0),
            entity_id: id,
        }]
    );
    assert!(store.drain_framed_map_removals().is_empty());
}

#[test]
fn framed_map_creative_clear_queues_removal_without_a_drop() {
    let mut store = EntityStore::new(0x4b1, EntityIdCounter::new());
    let id = spawn_item_frame(&mut store, pos(5, 64, 1), Direction::South, false).unwrap();
    let mut entity = store.detach(id).unwrap();
    assert!(matches!(
        apply_test_interact(
            &mut entity,
            Some(map_stack(23)),
            FramedMapAdmission::SavedDataMissing,
        ),
        HangInteract::Add { .. }
    ));
    let effects = hurt_server_with_post_actions(
        &mut entity,
        &mut store,
        true,
        HangHurtSource {
            is_explosion: false,
            is_fire: false,
            is_fall: false,
            fall_damage_immune: false,
            from_mob: false,
            mob_griefing: true,
            caused_by_entity: true,
            creative_player: true,
            bypasses_invulnerability: false,
        },
    )
    .unwrap();
    assert!(entity.is_alive());
    store.attach(entity);
    let mut level = NativeLevel::new();
    level.with_level(|level| {
        apply_hanging_post_actions_in_level(effects.actions, &mut store, level, None)
    });
    assert_eq!(store.ids(), vec![id]);
    assert_eq!(
        store.drain_framed_map_removals(),
        vec![crate::entity::FramedMapRemoval {
            map_id: 23,
            pos: pos(5, 64, 1),
            entity_id: id,
        }]
    );
}

#[test]
fn framed_map_entity_drops_false_only_environment_clears_the_marker() {
    let source = |caused_by_entity| HangHurtSource {
        is_explosion: false,
        is_fire: false,
        is_fall: false,
        fall_damage_immune: false,
        from_mob: false,
        mob_griefing: true,
        caused_by_entity,
        creative_player: false,
        bypasses_invulnerability: false,
    };
    let mut level = NativeLevel::new();

    let mut attacked = EntityStore::new(0x4b2, EntityIdCounter::new());
    let attacked_id =
        spawn_item_frame(&mut attacked, pos(6, 64, 1), Direction::South, false).unwrap();
    let mut entity = attacked.detach(attacked_id).unwrap();
    assert!(matches!(
        apply_test_interact(
            &mut entity,
            Some(map_stack(24)),
            FramedMapAdmission::SavedDataMissing,
        ),
        HangInteract::Add { .. }
    ));
    let effects =
        hurt_server_with_post_actions(&mut entity, &mut attacked, false, source(true)).unwrap();
    attacked.attach(entity);
    level.with_level(|level| {
        apply_hanging_post_actions_in_level(effects.actions, &mut attacked, level, None)
    });
    assert_eq!(attacked.ids(), vec![attacked_id]);
    assert!(matches!(
        attacked.get(attacked_id).unwrap().data(),
        EntityData::ItemFrame(frame) if frame.item().is_none()
    ));
    assert!(attacked.drain_framed_map_removals().is_empty());

    let mut environmental = EntityStore::new(0x4b3, EntityIdCounter::new());
    let environmental_id =
        spawn_item_frame(&mut environmental, pos(7, 64, 1), Direction::South, false).unwrap();
    let mut entity = environmental.detach(environmental_id).unwrap();
    assert!(matches!(
        apply_test_interact(
            &mut entity,
            Some(map_stack(25)),
            FramedMapAdmission::SavedDataMissing,
        ),
        HangInteract::Add { .. }
    ));
    let effects =
        hurt_server_with_post_actions(&mut entity, &mut environmental, false, source(false))
            .unwrap();
    environmental.attach(entity);
    level.with_level(|level| {
        apply_hanging_post_actions_in_level(effects.actions, &mut environmental, level, None)
    });
    assert_eq!(environmental.ids(), vec![environmental_id]);
    assert_eq!(
        environmental.drain_framed_map_removals(),
        vec![crate::entity::FramedMapRemoval {
            map_id: 25,
            pos: pos(7, 64, 1),
            entity_id: environmental_id,
        }]
    );
}

#[test]
fn framed_map_kill_and_unload_preserve_marker_removal_results() {
    let mut store = EntityStore::new(0x4ad, EntityIdCounter::new());
    let first = spawn_item_frame(&mut store, pos(1, 64, 0), Direction::North, false).unwrap();
    let second = spawn_item_frame(&mut store, pos(2, 64, 0), Direction::North, true).unwrap();
    for (id, map_id) in [(first, 31), (second, 32)] {
        let entity = store.get_mut(id).unwrap();
        assert!(matches!(
            apply_test_interact(
                entity,
                Some(map_stack(map_id)),
                FramedMapAdmission::SavedDataMissing,
            ),
            HangInteract::Add { .. }
        ));
    }
    store.remove(first, RemovalReason::Discarded).unwrap();
    let mut removed = store.detach(second).unwrap();
    removed.set_removed(RemovalReason::Killed);
    store.attach(removed);
    let mut removals = store.drain_framed_map_removals();
    removals.sort_by_key(|removal| removal.map_id);
    assert_eq!(
        removals,
        vec![
            crate::entity::FramedMapRemoval {
                map_id: 31,
                pos: pos(1, 64, 0),
                entity_id: first,
            },
            crate::entity::FramedMapRemoval {
                map_id: 32,
                pos: pos(2, 64, 0),
                entity_id: second,
            },
        ]
    );

    let unloaded = spawn_item_frame(&mut store, pos(4, 64, 0), Direction::North, false).unwrap();
    assert!(matches!(
        apply_test_interact(
            store.get_mut(unloaded).unwrap(),
            Some(map_stack(33)),
            FramedMapAdmission::SavedDataMissing,
        ),
        HangInteract::Add { .. }
    ));
    store
        .remove(unloaded, RemovalReason::UnloadedToChunk)
        .unwrap();
    assert!(store.drain_framed_map_removals().is_empty());

    let transferred = spawn_item_frame(&mut store, pos(5, 64, 0), Direction::North, false).unwrap();
    assert!(matches!(
        apply_test_interact(
            store.get_mut(transferred).unwrap(),
            Some(map_stack(34)),
            FramedMapAdmission::SavedDataMissing,
        ),
        HangInteract::Add { .. }
    ));
    let mut removed = store.detach(transferred).unwrap();
    removed.set_removed(RemovalReason::UnloadedWithPlayer);
    store.attach(removed);
    assert!(store.drain_framed_map_removals().is_empty());

    let non_map = spawn_item_frame(&mut store, pos(3, 64, 0), Direction::North, false).unwrap();
    let entity = store.get_mut(non_map).unwrap();
    assert!(matches!(
        apply_test_non_map_interact(entity, plain_stack("minecraft:diamond", 1)),
        HangInteract::Add { .. }
    ));
    store.remove(non_map, RemovalReason::Discarded).unwrap();
    assert!(store.drain_framed_map_removals().is_empty());
}

#[test]
fn framed_map_explosion_and_support_loss_each_queue_one_removal() {
    let mut exploded = EntityStore::new(0x4ae, EntityIdCounter::new());
    let exploded_id =
        spawn_item_frame(&mut exploded, pos(0, 64, 0), Direction::North, false).unwrap();
    let mut entity = exploded.detach(exploded_id).unwrap();
    assert!(matches!(
        apply_test_interact(
            &mut entity,
            Some(map_stack(41)),
            FramedMapAdmission::SavedDataMissing,
        ),
        HangInteract::Add { .. }
    ));
    let effects = hurt_server_with_post_actions(
        &mut entity,
        &mut exploded,
        false,
        HangHurtSource {
            is_explosion: true,
            is_fire: false,
            is_fall: false,
            fall_damage_immune: false,
            from_mob: false,
            mob_griefing: true,
            caused_by_entity: true,
            creative_player: true,
            bypasses_invulnerability: false,
        },
    )
    .unwrap();
    exploded.attach(entity);
    let mut level = NativeLevel::new();
    level.with_level(|level| {
        apply_hanging_post_actions_in_level(effects.actions, &mut exploded, level, None)
    });
    assert_eq!(
        exploded.drain_framed_map_removals(),
        vec![crate::entity::FramedMapRemoval {
            map_id: 41,
            pos: pos(0, 64, 0),
            entity_id: exploded_id,
        }]
    );

    let mut support_loss = EntityStore::new(0x4af, EntityIdCounter::new());
    let support_loss_id =
        spawn_item_frame(&mut support_loss, pos(0, 64, 0), Direction::North, false).unwrap();
    assert!(matches!(
        apply_test_interact(
            support_loss.get_mut(support_loss_id).unwrap(),
            Some(map_stack(42)),
            FramedMapAdmission::SavedDataMissing,
        ),
        HangInteract::Add { .. }
    ));
    let mut no_support = NativeLevel::new();
    for _ in 0..=SURVIVAL_CHECK_INTERVAL {
        no_support.with_level(|level| tick_entities(&mut support_loss, level, true));
    }
    assert_eq!(
        support_loss.drain_framed_map_removals(),
        vec![crate::entity::FramedMapRemoval {
            map_id: 42,
            pos: pos(0, 64, 0),
            entity_id: support_loss_id,
        }]
    );
}

#[test]
fn creative_frame_break_keeps_sound_and_suppresses_all_drops() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = spawn_item_frame(&mut store, pos(0, 64, 0), Direction::North, false).unwrap();
    let mut entity = store.detach(id).unwrap();
    assert!(matches!(
        apply_test_non_map_interact(&mut entity, plain_stack("minecraft:diamond", 1)),
        HangInteract::Add { .. }
    ));
    store.drain_lifecycle_events();
    let effects = hurt_server_with_post_actions(
        &mut entity,
        &mut store,
        true,
        HangHurtSource {
            is_explosion: true,
            is_fire: false,
            is_fall: false,
            fall_damage_immune: false,
            from_mob: false,
            mob_griefing: true,
            caused_by_entity: true,
            creative_player: true,
            bypasses_invulnerability: false,
        },
    )
    .unwrap();
    assert!(effects.outcome);
    store.attach(entity);
    let mut level = NativeLevel::new();
    level.with_level(|level| {
        apply_hanging_post_actions_in_level(effects.actions, &mut store, level, Some(9))
    });
    let lifecycle = store.drain_lifecycle_events();
    assert!(lifecycle
        .iter()
        .any(|event| matches!(event, crate::entity::EntityLifecycleEvent::Removed { .. })));
    assert!(lifecycle
        .iter()
        .any(|event| matches!(event, crate::entity::EntityLifecycleEvent::Sound { .. })));
    assert!(!lifecycle
        .iter()
        .any(|event| matches!(event, crate::entity::EntityLifecycleEvent::Added { .. })));
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::BLOCK_CHANGE));
}

#[test]
fn nonfixed_frame_drops_its_item_before_the_mob_griefing_gate() {
    let mut store = EntityStore::new(3, EntityIdCounter::new());
    let id = spawn_item_frame(&mut store, pos(0, 64, 0), Direction::North, false).unwrap();
    let mut entity = store.detach(id).unwrap();
    assert!(matches!(
        apply_test_non_map_interact(&mut entity, plain_stack("minecraft:diamond", 1)),
        HangInteract::Add { .. }
    ));
    let source = HangHurtSource {
        is_explosion: false,
        is_fire: false,
        is_fall: false,
        fall_damage_immune: false,
        from_mob: true,
        mob_griefing: false,
        caused_by_entity: true,
        creative_player: false,
        bypasses_invulnerability: false,
    };

    let effects = hurt_server(&mut entity, true, source);
    assert!(effects.outcome);
    assert!(entity.is_alive());
    assert!(!entity.hurt_marked());
    assert!(frame_mut(&mut entity).unwrap().item.is_none());
}

#[test]
fn fixed_frame_reaches_the_block_attached_mob_griefing_gate() {
    let mut store = EntityStore::new(5, EntityIdCounter::new());
    let id = spawn_item_frame(&mut store, pos(0, 64, 0), Direction::North, false).unwrap();
    let mut entity = store.detach(id).unwrap();
    frame_mut(&mut entity).unwrap().fixed = true;
    let source = HangHurtSource {
        is_explosion: false,
        is_fire: false,
        is_fall: false,
        fall_damage_immune: false,
        from_mob: true,
        mob_griefing: false,
        caused_by_entity: true,
        creative_player: true,
        bypasses_invulnerability: false,
    };

    assert!(!hurt_server(&mut entity, true, source).outcome);
    assert!(entity.is_alive());
    assert!(!entity.hurt_marked());
}

#[test]
fn item_frame_interact_pass_and_map_admission_gates() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let attached = pos(0, 64, 0);
    let id = spawn_item_frame(&mut store, attached, Direction::East, false).unwrap();
    let mut entity = store.detach(id).unwrap();
    assert_eq!(
        prepare_interact(&entity, None, FramedMapAdmission::NotFramedMap),
        HangInteract::Pass
    );

    frame_mut(&mut entity).unwrap().fixed = true;
    assert_eq!(
        prepare_interact(
            &entity,
            plain_stack("minecraft:apple", 1),
            FramedMapAdmission::NotFramedMap,
        ),
        HangInteract::Pass
    );
    frame_mut(&mut entity).unwrap().fixed = false;

    assert!(matches!(
        prepare_interact(
            &entity,
            Some(map_stack(7)),
            FramedMapAdmission::SavedDataMissing,
        ),
        HangInteract::Add { .. }
    ));
    assert!(matches!(
        prepare_interact(
            &entity,
            Some(map_stack(7)),
            FramedMapAdmission::Present {
                tracked_decoration_count: 256,
            },
        ),
        HangInteract::Add { .. }
    ));
    assert_eq!(
        prepare_interact(
            &entity,
            Some(map_stack(7)),
            FramedMapAdmission::Present {
                tracked_decoration_count: 257,
            },
        ),
        HangInteract::Fail
    );
    assert!(matches!(
        apply_test_non_map_interact(&mut entity, plain_stack("minecraft:apple", 4)),
        HangInteract::Add { .. }
    ));
    assert!(matches!(
        prepare_interact(&entity, None, FramedMapAdmission::Unavailable),
        HangInteract::Rotate { .. }
    ));
}

#[test]
fn item_frame_interact_add_and_rotate_sound_and_metadata() {
    for (glow, add_sound, rotate_sound) in [
        (
            false,
            ITEM_FRAME_ADD_ITEM_SOUND,
            ITEM_FRAME_ROTATE_ITEM_SOUND,
        ),
        (
            true,
            GLOW_ITEM_FRAME_ADD_ITEM_SOUND,
            GLOW_ITEM_FRAME_ROTATE_ITEM_SOUND,
        ),
    ] {
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        let attached = pos(0, 64, 0);
        let id = spawn_item_frame(&mut store, attached, Direction::East, glow).unwrap();
        let entity = store.get_mut(id).unwrap();
        entity.take_synched_dirty();

        let add = prepare_interact(
            entity,
            plain_stack("minecraft:apple", 4),
            FramedMapAdmission::NotFramedMap,
        );
        let HangInteract::Add {
            item,
            sound_resource_id,
            attached: planned_attached,
            ..
        } = &add
        else {
            panic!("nonempty hand prepares an add: {add:?}")
        };
        assert_eq!(*sound_resource_id, add_sound);
        assert_eq!(*planned_attached, attached);
        assert_eq!(item.count(), 1);
        let mut add_sound_played = None;
        assert!(apply_interact(entity, &add, |_, sound| {
            add_sound_played = Some(sound);
        }));
        let sound = add_sound_played.unwrap();
        assert_eq!(sound.resource_id, add_sound);
        assert_eq!(sound.source, SoundSource::Neutral);
        assert_eq!(sound.volume, 1.0);
        assert_eq!(sound.pitch, 1.0);
        assert_eq!(
            entity.take_synched_dirty(),
            [VANILLA_ITEM_FRAME_ITEM_DATA_ID]
        );

        let rotate = prepare_interact(entity, None, FramedMapAdmission::Unavailable);
        let HangInteract::Rotate {
            previous_rotation,
            rotation,
            sound_resource_id,
            attached: planned_attached,
            ..
        } = &rotate
        else {
            panic!("occupied frame prepares a rotation: {rotate:?}")
        };
        assert_eq!((*previous_rotation, *rotation), (0, 1));
        assert_eq!(*sound_resource_id, rotate_sound);
        assert_eq!(*planned_attached, attached);
        let mut rotate_sound_played = None;
        assert!(apply_interact(entity, &rotate, |_, sound| {
            rotate_sound_played = Some(sound);
        }));
        assert_eq!(rotate_sound_played.unwrap().resource_id, rotate_sound);
        assert_eq!(
            entity.take_synched_dirty(),
            [VANILLA_ITEM_FRAME_ROTATION_DATA_ID]
        );
        assert_eq!(frame_mut(entity).unwrap().rotation, 1);
    }
}

#[test]
fn hanging_save_reload_round_trips_painting_and_item_frame() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let painting = spawn_painting(&mut store, pos(1, 64, 2), Direction::West, kebab()).unwrap();
    let frame = spawn_item_frame(&mut store, pos(2, 64, 2), Direction::Up, false).unwrap();
    {
        let entity = store.get_mut(frame).unwrap();
        apply_test_non_map_interact(entity, plain_stack("minecraft:stick", 1));
    }
    let structure_frame = spawn_structure_item_frame(
        &mut store,
        pos(3, 64, 2),
        Direction::East,
        plain_stack("minecraft:elytra", 1).unwrap(),
    )
    .unwrap();
    let entity = store.get_mut(structure_frame).unwrap();
    let rotation = transform_template_pose(
        entity,
        1,
        bbb_protocol::block_state_types::VanillaBlockStateMirror::FrontBack,
    )
    .unwrap();
    set_attached_pos(
        entity,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 2.5,
        },
    );
    entity.set_rot(rotation[0], rotation[1]);
    assert_eq!(pose_of(entity.data()).direction, Direction::South);
    assert_eq!(entity.y_rot(), 270.0);
    let chunk: ChunkKey = (0, 0);
    let document = write_entity_chunk(&store, chunk).expect("writer-canonical hanging save");
    let mut restored = EntityStore::new(9, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut restored, &document).unwrap();
    assert_eq!(ids.len(), 3);
    let _ = (painting, frame);
    let kinds: Vec<_> = restored
        .ids()
        .into_iter()
        .map(|id| restored.get(id).unwrap().data().entity_resource_id())
        .collect();
    assert!(kinds.contains(&"minecraft:painting"));
    assert!(kinds.contains(&"minecraft:item_frame"));
}

#[test]
fn leash_knot_uses_attached_pose_and_tracking_position() {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let attached = pos(3, 64, -2);
    let id = spawn_leash_knot(&mut store, attached).unwrap();
    let entity = store.get(id).unwrap();
    assert_eq!(entity.position(), leash_knot_position(attached));
    assert!(entity.is_pickable());

    let packets =
        crate::entity::tracking::ServerEntity::new(entity, 1, true).send_pairing_data(entity, &[]);
    let crate::connection::ClientboundPlayPacket::AddEntity(packet) = &packets[0] else {
        panic!("first pairing packet was not AddEntity");
    };
    assert_eq!(
        packet.position,
        Vec3d {
            x: 3.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(packet.data, 0);
}

#[test]
fn leash_knot_survives_fence_and_breaks_on_gate_after_real_ticks() {
    let attached = pos(0, 64, 0);
    let mut supported_store = EntityStore::new(13, EntityIdCounter::new());
    let supported_id = spawn_leash_knot(&mut supported_store, attached).unwrap();
    let mut supported = NativeLevel::new();
    supported.set(
        attached,
        state_id_with(
            "minecraft:nether_brick_fence",
            &[
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("waterlogged", "false"),
                ("west", "false"),
            ],
        ),
    );
    assert!(supported.with_level(|level| survives(
        supported_store.get(supported_id).unwrap(),
        &supported_store,
        level
    )));

    let mut invalid_store = EntityStore::new(17, EntityIdCounter::new());
    let invalid_id = spawn_leash_knot(&mut invalid_store, attached).unwrap();
    let mut invalid_support = NativeLevel::new();
    invalid_support.set(
        attached,
        state_id_with(
            "minecraft:oak_fence_gate",
            &[
                ("facing", "north"),
                ("in_wall", "false"),
                ("open", "false"),
                ("powered", "false"),
            ],
        ),
    );
    for _ in 0..SURVIVAL_CHECK_INTERVAL {
        invalid_support.with_level(|level| tick_entities(&mut invalid_store, level, true));
    }
    invalid_support.with_level(|level| tick_entities(&mut invalid_store, level, true));
    assert!(invalid_store.get(invalid_id).is_none());
    assert!(invalid_store
        .drain_lifecycle_events()
        .into_iter()
        .any(|event| matches!(
            event,
            crate::entity::store::EntityLifecycleEvent::Sound { .. }
        )));
}

#[test]
fn leash_knot_damage_obeys_mob_griefing_and_plays_break_sound() {
    let mut store = EntityStore::new(19, EntityIdCounter::new());
    let id = spawn_leash_knot(&mut store, pos(0, 64, 0)).unwrap();
    let mut entity = store.detach(id).unwrap();
    let mob_source = HangHurtSource {
        is_explosion: false,
        is_fire: false,
        is_fall: false,
        fall_damage_immune: false,
        from_mob: true,
        mob_griefing: false,
        caused_by_entity: true,
        creative_player: false,
        bypasses_invulnerability: false,
    };
    assert!(
        !hurt_server_with_post_actions(&mut entity, &mut store, true, mob_source)
            .unwrap()
            .outcome
    );
    assert!(entity.is_alive());
    let effects = hurt_server_with_post_actions(
        &mut entity,
        &mut store,
        true,
        HangHurtSource {
            from_mob: false,
            ..mob_source
        },
    )
    .unwrap();
    assert!(effects.outcome);
    assert_eq!(entity.removal_reason(), Some(RemovalReason::Killed));
    store.attach(entity);
    let mut level = NativeLevel::new();
    level.with_level(|level| {
        apply_hanging_post_actions_in_level(effects.actions, &mut store, level, None)
    });
    assert!(store
        .drain_lifecycle_events()
        .into_iter()
        .any(|event| matches!(
            event,
            crate::entity::store::EntityLifecycleEvent::Sound { .. }
        )));
}

#[test]
fn frame_hurt_pops_contents_and_explosion_removes_the_frame() {
    let source = HangHurtSource {
        is_explosion: false,
        is_fire: false,
        is_fall: false,
        fall_damage_immune: false,
        from_mob: false,
        mob_griefing: true,
        caused_by_entity: true,
        creative_player: false,
        bypasses_invulnerability: false,
    };
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = spawn_item_frame(&mut store, pos(0, 64, 0), Direction::North, false).unwrap();
    let mut entity = store.detach(id).unwrap();
    assert!(matches!(
        apply_test_non_map_interact(&mut entity, plain_stack("minecraft:apple", 1)),
        HangInteract::Add { .. }
    ));
    store.attach(entity);

    let mut entity = store.detach(id).unwrap();
    let dropped = hurt_server(&mut entity, true, source);
    assert!(dropped.outcome);
    assert!(entity.is_alive());
    let destroyed = hurt_server(
        &mut entity,
        true,
        HangHurtSource {
            is_explosion: true,
            ..source
        },
    );
    assert!(destroyed.outcome);
    assert_eq!(entity.removal_reason(), Some(RemovalReason::Killed));
}

#[test]
fn leash_knot_no_save_is_skipped_by_entity_chunk_writer() {
    let mut store = EntityStore::new(23, EntityIdCounter::new());
    let attached = pos(4, 70, 5);
    spawn_leash_knot(&mut store, attached).unwrap();
    let document = write_entity_chunk(&store, (0, 0)).unwrap();
    assert!(document.is_empty());
}

#[test]
fn silent_frame_support_loss_drops_item_and_frame() {
    let mut store = EntityStore::new(923, EntityIdCounter::new());
    let id = spawn_item_frame(&mut store, pos(0, 64, 0), Direction::North, false).unwrap();
    store.drain_lifecycle_events();
    let mut entity = store.detach(id).unwrap();
    entity.set_silent(true);
    let frame = frame_mut(&mut entity).unwrap();
    frame.item = plain_stack("minecraft:diamond", 1);
    frame.drop_chance = 1.0;
    let mut level = NativeLevel::new();
    let mut removed_actions = None;
    for _ in 0..=SURVIVAL_CHECK_INTERVAL {
        let actions = level
            .with_level(|level| tick(&mut entity, &mut store, level))
            .unwrap();
        if entity.removal_reason().is_some() {
            removed_actions = Some(actions);
            break;
        }
    }
    let actions = removed_actions.expect("unbacked frame is removed by real ticks");
    assert!(!actions.is_empty());
    assert_eq!(entity.removal_reason(), Some(RemovalReason::Discarded));
    store.attach(entity);
    level.with_level(|level| apply_hanging_post_actions_in_level(actions, &mut store, level, None));
    let events = store.drain_lifecycle_events();
    assert!(events
        .iter()
        .any(|event| matches!(event, crate::entity::EntityLifecycleEvent::Removed { .. })));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, crate::entity::EntityLifecycleEvent::Added { .. }))
            .count(),
        2
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, crate::entity::EntityLifecycleEvent::Sound { .. })));
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::BLOCK_CHANGE));
}
