use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::Direction;

const MOVING: BlockPos = BlockPos { x: 9, y: 63, z: 8 };
const START: Vec3d = Vec3d {
    x: 8.5,
    y: 64.6,
    z: 8.5,
};
const DOWN: Vec3d = Vec3d {
    x: 0.0,
    y: -0.5,
    z: 0.0,
};

fn cow(no_ai: bool) -> (EntityStore, i32) {
    let mut store = EntityStore::new(17, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            vanilla_entity_type_id_for_resource_id("minecraft:cow").unwrap(),
            START,
        )
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        data.set_no_ai(no_ai);
    }
    entity.set_delta_movement(DOWN);
    entity.portal_cooldown = 3;
    entity
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::MainHand, Some(stack_of("minecraft:dirt", 1)));
    store.drain_lifecycle_events();
    (store, id)
}

#[test]
fn moving_piston_cursor_actual_tick_clips_fence_fragment_across_two_boundaries_and_supports() {
    let (mut store, id) = cow(false);
    let mut native = NativeLevel::new();
    native.set(MOVING, state_id("minecraft:moving_piston"));
    let piston = native
        .world
        .level_block_entity_for_test(DIMENSION, MOVING)
        .unwrap();
    assert!(piston.configure_piston(
        state_id("minecraft:oak_fence"),
        Direction::East,
        true,
        false
    ));
    piston.tick_piston_without_entities(0).unwrap();
    native.with_level(|level| {
        crate::entity::tick_one_entity(&mut store, level, id);
    });
    let entity = store.get(id).unwrap();
    assert_eq!(entity.position().y, 64.5);
    assert!(entity.on_ground());
    assert!(entity.is_supported_by(MOVING));
}
