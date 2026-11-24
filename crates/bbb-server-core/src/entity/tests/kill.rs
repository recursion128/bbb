use super::native_level::{NativeLevel, DIMENSION};
use super::*;

use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_ITEM_ID};

use crate::entity::armor_stand::ArmorStandData;
use crate::entity::kill;
use crate::level_event::VanillaGameEvent;

const POSITION: Vec3d = Vec3d {
    x: 0.5,
    y: 70.0,
    z: 0.5,
};
const BLOCK: BlockPos = BlockPos { x: 0, y: 70, z: 0 };

fn store_with_item() -> (EntityStore, NativeLevel, i32) {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let level = NativeLevel::new();
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_ITEM_ID,
        POSITION,
        EntityData::Item(ItemEntityData::new(stack_of("minecraft:stone", 1), 0.0)),
    );
    (store, level, id)
}

#[test]
fn kill_removes_the_entity_then_raises_entity_die_from_it() {
    let (mut store, mut level, id) = store_with_item();
    let _ = store.drain_lifecycle_events();

    assert!(level.with_level(|level| kill::kill(&mut store, level, id)));
    assert_eq!(store.get(id), None, "the removal took the entity out");
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DIE
                && event.recorded.pos == BLOCK
                && event.recorded.source_entity_id == Some(id)
        ));

    assert!(store.drain_lifecycle_events().iter().any(|event| matches!(event, EntityLifecycleEvent::Removed { entity, .. } if entity.removal_reason() == Some(RemovalReason::Killed))));
}

#[test]
fn kill_of_an_armor_stand_is_the_base_body_and_raises_entity_die() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        POSITION,
        EntityData::ArmorStand(ArmorStandData::new().expect("the catalog holds an armor stand")),
    );
    let _ = store.drain_lifecycle_events();

    assert!(level.with_level(|level| kill::kill(&mut store, level, id)));
    assert_eq!(store.get(id), None);
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DIE
                && event.recorded.pos == BLOCK
                && event.recorded.source_entity_id == Some(id)
        ));
}
