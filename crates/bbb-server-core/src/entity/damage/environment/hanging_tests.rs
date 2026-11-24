use super::*;
use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::entity::{EntityIdCounter, EntityLifecycleEvent};
use bbb_protocol::packets::BlockPos;
use bbb_protocol::packets::Direction;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
fn fixture(occupied: bool, fixed: bool, glow: bool) -> (Box<Entity>, EntityStore, NativeLevel) {
    let mut store = EntityStore::new(29, EntityIdCounter::new());
    let id = crate::entity::hanging::spawn_item_frame(
        &mut store,
        BlockPos { x: 0, y: 64, z: 0 },
        Direction::North,
        glow,
    )
    .unwrap();
    let mut entity = store.detach(id).unwrap();
    match &mut entity.data {
        EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame) => {
            *frame = frame.clone().with_saved_fields(
                occupied.then(|| crate::entity::item::plain_stack("minecraft:diamond", 1).unwrap()),
                0,
                1.0,
                fixed,
                false,
            );
        }
        _ => unreachable!(),
    }
    store.drain_lifecycle_events();
    let mut native = NativeLevel::new();
    native.set(BlockPos { x: 0, y: 64, z: 1 }, state("minecraft:stone"));
    (entity, store, native)
}

#[test]
fn occupied_frame_pops_before_empty_frame_removal_for_both_frame_types() {
    for glow in [false, true] {
        let (mut entity, mut store, mut level) = fixture(true, false, glow);
        assert_eq!(
            level.with_level(|level| hurt_detached_from_environment(
                &mut entity,
                &mut store,
                level,
                "minecraft:cactus",
                1.0
            )),
            Some(true)
        );
        assert!(entity.is_alive());
        assert!(level
            .world
            .positioned_game_events_for_test(DIMENSION)
            .iter()
            .any(
                |event| event.recorded.game_event == VanillaGameEvent::BLOCK_CHANGE
                    && event.recorded.source_entity_id.is_none()
            ));
        assert!(!store
            .drain_lifecycle_events()
            .iter()
            .any(|event| matches!(event, EntityLifecycleEvent::Removed { .. })));
        let before_remove = level.world.positioned_game_events_for_test(DIMENSION).len();
        assert_eq!(
            level.with_level(|level| hurt_detached_from_environment(
                &mut entity,
                &mut store,
                level,
                "minecraft:cactus",
                1.0
            )),
            Some(true)
        );
        assert!(entity.removal_reason().is_some());
        let removal_events = level.world.positioned_game_events_for_test(DIMENSION);
        let removal_events = &removal_events[before_remove..];
        assert!(removal_events.iter().any(|event| event.recorded.game_event
            == VanillaGameEvent::ENTITY_DIE
            && event.recorded.source_entity_id == Some(entity.id())));
        assert!(removal_events.iter().any(|event| event.recorded.game_event
            == VanillaGameEvent::BLOCK_CHANGE
            && event.recorded.source_entity_id.is_none()));
        let events = store.drain_lifecycle_events();
        assert!(events
            .iter()
            .any(|event| matches!(event, EntityLifecycleEvent::Removed { .. })));
        assert!(events
            .iter()
            .any(|event| matches!(event, EntityLifecycleEvent::Added { .. })));
        store.attach(entity);
        assert!(!store
            .drain_lifecycle_events()
            .iter()
            .any(|event| matches!(event, EntityLifecycleEvent::Removed { .. })));
    }
}

#[test]
fn fixed_frame_does_not_mutate_or_publish() {
    let (mut entity, mut store, mut level) = fixture(true, true, false);
    assert_eq!(
        level.with_level(|level| hurt_detached_from_environment(
            &mut entity,
            &mut store,
            level,
            "minecraft:lava",
            4.0
        )),
        Some(false)
    );
    assert!(entity.is_alive());
    let EntityData::ItemFrame(frame) = entity.data() else {
        unreachable!()
    };
    assert!(frame.item().is_some());
    assert!(store.drain_lifecycle_events().is_empty());
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .is_empty());
}

#[test]
fn silent_empty_frame_still_drops_and_dies() {
    let (mut entity, mut store, mut level) = fixture(false, false, false);
    entity.set_silent(true);
    assert_eq!(
        level.with_level(|level| hurt_detached_from_environment(
            &mut entity,
            &mut store,
            level,
            "minecraft:lava",
            4.0
        )),
        Some(true)
    );
    let events = store.drain_lifecycle_events();
    assert!(events
        .iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Removed { .. })));
    assert!(events
        .iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Added { .. })));
    assert!(!events
        .iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DIE
                && event.recorded.source_entity_id == Some(entity.id())
        ));
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::BLOCK_CHANGE
                && event.recorded.source_entity_id.is_none()
        ));
}

#[test]
fn fixed_frame_honors_live_bypass_tag_for_environmental_damage() {
    let (mut entity, mut store, mut level) = fixture(false, true, false);
    level.install_tags(crate::registry_tags::RegistryTagSnapshot::new([(
        "minecraft:damage_type".into(),
        "minecraft:bypasses_invulnerability".into(),
        vec!["minecraft:lava".into()],
    )]));
    assert_eq!(
        level.with_level(|level| hurt_detached_from_environment(
            &mut entity,
            &mut store,
            level,
            "minecraft:lava",
            4.0
        )),
        Some(true)
    );
    assert!(entity.removal_reason().is_some());
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DIE
                && event.recorded.source_entity_id == Some(entity.id())
        ));
}

#[test]
fn painting_and_leash_knot_environment_hurt_use_hanging_removal() {
    for knot in [false, true] {
        let mut store = EntityStore::new(29, EntityIdCounter::new());
        let pos = BlockPos { x: 0, y: 64, z: 0 };
        let id = if knot {
            crate::entity::hanging::spawn_leash_knot(&mut store, pos).unwrap()
        } else {
            let variant = crate::entity::hanging::PaintingVariant::vanilla("minecraft:kebab");
            crate::entity::hanging::spawn_painting(&mut store, pos, Direction::North, variant)
                .unwrap()
        };
        let mut entity = store.detach(id).unwrap();
        store.drain_lifecycle_events();
        let mut level = NativeLevel::new();
        assert_eq!(
            level.with_level(|level| hurt_detached_from_environment(
                &mut entity,
                &mut store,
                level,
                "minecraft:cactus",
                1.0
            )),
            Some(true)
        );
        assert!(level
            .world
            .positioned_game_events_for_test(DIMENSION)
            .iter()
            .any(
                |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DIE
                    && event.recorded.source_entity_id == Some(id)
            ));
        assert!(store
            .drain_lifecycle_events()
            .iter()
            .any(|event| matches!(event, EntityLifecycleEvent::Removed { .. })));
        assert!(entity.removal_reason().is_some());
    }
}

#[test]
fn environmental_frame_pop_clears_map_and_metadata_without_removal() {
    let (mut entity, mut store, mut level) = fixture(false, false, false);
    let stack = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::String("minecraft:filled_map".into()),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(1),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "minecraft:map_id".into(),
                    value: NbtSummaryValue::Int(7),
                }]),
            },
        ]),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .unwrap();
    let EntityData::ItemFrame(frame) = &mut entity.data else {
        unreachable!()
    };
    *frame = frame
        .clone()
        .with_saved_fields(Some(stack), 3, 1.0, false, false);
    assert_eq!(
        level.with_level(|level| hurt_detached_from_environment(
            &mut entity,
            &mut store,
            level,
            "minecraft:cactus",
            1.0
        )),
        Some(true)
    );
    let EntityData::ItemFrame(frame) = entity.data() else {
        unreachable!()
    };
    assert!(frame.item().is_none());
    assert_eq!(
        store.drain_framed_map_removals(),
        vec![crate::entity::FramedMapRemoval {
            map_id: 7,
            entity_id: entity.id(),
            pos: BlockPos { x: 0, y: 64, z: 0 },
        }]
    );
    assert!(entity.is_alive());
    assert!(entity.synched_data_is_dirty());
}
