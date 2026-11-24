use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_SLEEPING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
};
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_SLIME_ID};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::item_stack::RuntimeItemStack;

fn position(x: f64) -> Vec3d {
    Vec3d { x, y: 80.0, z: 8.5 }
}
fn fixture(entity_type: i32) -> (EntityStore, i32) {
    let mut store = EntityStore::new(103, crate::entity::EntityIdCounter::new());
    let id = store
        .spawn_general_living(entity_type, position(8.5))
        .unwrap();
    (store, id)
}
fn fields(store: &EntityStore, id: i32) -> Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(fields) = store.command_data(id).unwrap() else {
        panic!("compound command data")
    };
    fields
}
fn replace(fields: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    if let Some(field) = fields.iter_mut().find(|field| field.name == name) {
        field.value = value;
    } else {
        fields.push(NbtSummaryEntry {
            name: name.into(),
            value,
        });
    }
}
fn boots(name: &str) -> RuntimeItemStack {
    crate::entity::item::plain_stack(name, 1).unwrap()
}
fn equip(entity: &mut Entity, stack: RuntimeItemStack) {
    entity
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::Feet, Some(stack));
}

#[test]
fn growth_load_slime_applies_loaded_equipment_flags_and_position() {
    for old_no_physics in [false, true] {
        let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_SLIME_ID);
        let old_boots = boots("minecraft:leather_boots");
        let new_boots = boots("minecraft:diamond_boots");
        let entity = store.get_mut(id).unwrap();
        entity.first_tick = false;
        entity.set_no_physics(old_no_physics);
        entity.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, true);
        equip(entity, new_boots.clone());
        let mut input = fields(&store, id);
        equip(store.get_mut(id).unwrap(), old_boots.clone());
        replace(&mut input, "Pos", double_list(position(32.5)));
        replace(&mut input, "Size", NbtSummaryValue::Int(1));
        let mut native = NativeLevel::new();
        assert!(native
            .world
            .block_state(DIMENSION, BlockPos { x: 32, y: 63, z: 8 })
            .is_some());
        native
            .with_level(|level| store.load_command_data_with_level(id, &input, level))
            .unwrap();
        let entity = store.get(id).unwrap();
        assert_eq!(
            entity
                .living()
                .unwrap()
                .equipment()
                .get(EquipmentSlot::Feet),
            Some(&new_boots)
        );
        assert!(entity.get_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN));
        assert_eq!(entity.no_physics(), old_no_physics);
        assert_eq!(entity.position().x, 32.5);
    }
}

#[test]
fn growth_load_armor_marker_switch_publishes_loaded_physics() {
    for old_marker in [false, true] {
        let mut store = EntityStore::new(109, crate::entity::EntityIdCounter::new());
        let id = crate::entity::spawn_armor_stand(&mut store, position(8.5)).unwrap();
        let entity = store.get_mut(id).unwrap();
        if old_marker {
            crate::entity::armor_stand::set_marker(entity, true);
        } else {
            crate::entity::armor_stand::set_small(entity, true);
        }
        entity.set_no_physics(old_marker);
        entity.first_tick = false;
        let mut input = fields(&store, id);
        replace(&mut input, "Small", NbtSummaryValue::Byte(0));
        replace(
            &mut input,
            "Marker",
            NbtSummaryValue::Byte(i8::from(!old_marker)),
        );
        replace(&mut input, "Pos", double_list(position(32.5)));
        let mut native = NativeLevel::new();
        assert!(native
            .world
            .block_state(DIMENSION, BlockPos { x: 32, y: 63, z: 8 })
            .is_some());
        native
            .with_level(|level| store.load_command_data_with_level(id, &input, level))
            .unwrap();
        let entity = store.get(id).unwrap();
        assert_eq!(entity.position(), position(32.5));
        assert_eq!(entity.no_physics(), !old_marker);
        let EntityData::ArmorStand(data) = entity.data() else {
            unreachable!()
        };
        assert_eq!(data.is_marker(), !old_marker);
    }
}

#[test]
fn growth_load_sleeping_scale_age_changes_stay_small_then_wake_current_state() {
    use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_COW_ID);
    assert!(crate::entity::general_living::set_spawner_age(
        store.get_mut(id).unwrap(),
        -24000
    ));
    let entity = store.get_mut(id).unwrap();
    entity.set_sleeping_pos(Some(BlockPos { x: 8, y: 80, z: 8 }));
    entity.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
    entity.first_tick = false;
    entity
        .living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
        .unwrap();
    // Author the new SCALE using the real writer without changing the live pre-load state.
    let previous = entity.clone();
    entity
        .living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
        .unwrap()
        .set_base_value(2.0);
    let mut input = fields(&store, id);
    *store.get_mut(id).unwrap() = previous;
    replace(&mut input, "Age", NbtSummaryValue::Int(0));
    let mut native = NativeLevel::new();
    native
        .with_level(|level| store.load_command_data_with_level(id, &input, level))
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    assert_eq!(
        entity
            .living()
            .unwrap()
            .attribute_value(VANILLA_ATTRIBUTE_SCALE_ID),
        2.0
    );
    assert_eq!(entity.bb_width(), 0.2);
    assert_eq!(entity.bb_height(), 0.2);
    assert_eq!(entity.eye_height(), 0.2);
    entity.set_sleeping_pos(None);
    native
        .with_level(|level| {
            entity.set_pose_id_with_level(VANILLA_ENTITY_POSE_STANDING_ID, level, None)
        })
        .unwrap();
    let [width, height] =
        bbb_protocol::entity_types::vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_COW_ID)
            .unwrap();
    let eye =
        bbb_protocol::entity_types::vanilla_entity_type_eye_height(VANILLA_ENTITY_TYPE_COW_ID)
            .unwrap_or(height * 0.85);
    assert_eq!(entity.bb_width(), width * 2.0);
    assert_eq!(entity.bb_height(), height * 2.0);
    assert_eq!(entity.eye_height(), eye * 2.0);
}

#[test]
fn growth_load_camel_sleeping_to_sitting_stands_up() {
    use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SITTING_ID;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_ID;
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_CAMEL_ID);
    let entity = store.get_mut(id).unwrap();
    entity.first_tick = false;
    entity.set_sleeping_pos(Some(BlockPos { x: 8, y: 80, z: 8 }));
    entity.set_pose_id(VANILLA_ENTITY_POSE_SLEEPING_ID);
    let old_height = entity.bb_height();
    let mut input = fields(&store, id);
    replace(&mut input, "LastPoseTick", NbtSummaryValue::Long(-1));
    let mut native = NativeLevel::new();
    native
        .with_level(|level| store.load_command_data_with_level(id, &input, level))
        .unwrap();
    let entity = store.get(id).unwrap();
    assert_eq!(entity.pose_id(), VANILLA_ENTITY_POSE_SITTING_ID);
    assert_eq!(entity.sleeping_pos(), Some(BlockPos { x: 8, y: 80, z: 8 }));
    assert!(entity.bb_height() > old_height);
    assert_eq!(
        entity.position().y,
        80.6875 + f64::from(old_height) / 2.0 - f64::from(entity.bb_height()) / 2.0
    );
}
