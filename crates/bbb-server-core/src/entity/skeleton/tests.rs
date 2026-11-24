use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};

use bbb_protocol::entity_synched_data::VANILLA_SKELETON_STRAY_CONVERSION_DATA_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_SKELETON_ID, VANILLA_ENTITY_TYPE_STRAY_ID};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{BlockPos, EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};
use uuid::Uuid;

use super::{before_super, read_save_data, synched_values};
use crate::entity::item::plain_stack;
use crate::entity::{tick_one_entity, EntityData, EntityIdCounter, EntityStore, GeneralLivingData};

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

#[test]
fn skeleton_freeze_conversion_retained_save_and_tracking_are_exact() {
    assert!(GeneralLivingData::new(VANILLA_ENTITY_TYPE_SKELETON_ID)
        .unwrap()
        .skeleton_retained()
        .is_some());
    assert!(GeneralLivingData::new(VANILLA_ENTITY_TYPE_STRAY_ID)
        .unwrap()
        .skeleton_retained()
        .is_none());
    let default = read_save_data(&[]);
    assert_eq!(default.conversion_time(), -1);
    let converting = read_save_data(&[entry("StrayConversionTime", NbtSummaryValue::Int(42))]);
    assert_eq!(converting.conversion_time(), 42);
    assert!(matches!(
        synched_values(converting)[0].value.value,
        EntityDataValueKind::Boolean(true)
    ));
    assert_eq!(
        synched_values(converting)[0].value.data_id,
        VANILLA_SKELETON_STRAY_CONVERSION_DATA_ID
    );
}

#[test]
fn skeleton_freeze_conversion_contact_threshold_and_reset() {
    let mut native = NativeLevel::new();
    native.set(
        BlockPos { x: 0, y: 64, z: 0 },
        state("minecraft:powder_snow"),
    );
    native.set(
        BlockPos { x: 0, y: 65, z: 0 },
        state("minecraft:powder_snow"),
    );
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let mut entity = crate::entity::Entity::new(
        1,
        Uuid::nil(),
        VANILLA_ENTITY_TYPE_SKELETON_ID,
        position(),
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_SKELETON_ID).unwrap()),
    );
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.skeleton_retained_mut()
        .unwrap()
        .set_powder_time_for_test(139);
    native.with_level(|level| before_super(&mut entity, &mut store, level));
    assert_eq!(
        match entity.data() {
            EntityData::GeneralLiving(data) => data.skeleton_retained().unwrap().conversion_time(),
            _ => unreachable!(),
        },
        300
    );
    entity.take_synched_dirty();
    native.set(BlockPos { x: 0, y: 64, z: 0 }, state("minecraft:air"));
    native.set(BlockPos { x: 0, y: 65, z: 0 }, state("minecraft:air"));
    native.with_level(|level| before_super(&mut entity, &mut store, level));
    assert_eq!(
        match entity.data() {
            EntityData::GeneralLiving(data) => data.skeleton_retained().unwrap().conversion_time(),
            _ => unreachable!(),
        },
        -1
    );
    assert_eq!(
        entity.take_synched_dirty(),
        vec![VANILLA_SKELETON_STRAY_CONVERSION_DATA_ID]
    );
}

#[test]
fn skeleton_freeze_conversion_preserves_equipment_and_phenotype() {
    let mut store = EntityStore::new(2, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_SKELETON_ID, position())
        .unwrap();
    {
        let source = store.get_mut(id).unwrap();
        source.set_delta_movement(Vec3d {
            x: 0.25,
            y: 0.5,
            z: -0.75,
        });
        source.fall_distance = 3.5;
        source.on_ground = true;
        source.set_rot(45.0, -10.0);
        source.set_custom_name(Some(RichTextComponent::plain("frozen")));
        source.set_custom_name_visible(true);
        let EntityData::GeneralLiving(data) = &mut source.data else {
            unreachable!()
        };
        data.skeleton_retained_mut().unwrap().start_for_test(0);
        data.mob.as_mut().unwrap().set_can_pick_up_loot(true);
        data.mob.as_mut().unwrap().set_persistence_required(true);
        let _ = data
            .mob
            .as_mut()
            .unwrap()
            .set_drop_chance(EquipmentSlot::MainHand, 0.75);
        let living = data.living_mut();
        living.set_item_slot(EquipmentSlot::MainHand, plain_stack("minecraft:bow", 1));
        living.hurt_time = 7;
        living.y_body_rot = 30.0;
    }
    store.drain_lifecycle_events();
    let mut native = NativeLevel::new();
    native.set(
        BlockPos { x: 0, y: 64, z: 0 },
        state("minecraft:powder_snow"),
    );
    native.set(
        BlockPos { x: 0, y: 65, z: 0 },
        state("minecraft:powder_snow"),
    );
    native.with_level(|level| tick_one_entity(&mut store, level, id));

    assert!(native
        .world
        .drain_level_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.event_type == 1048 && event.pos == BlockPos { x: 0, y: 64, z: 0 }));
    assert!(store.get(id).is_none());
    let child = store
        .ids()
        .into_iter()
        .map(|id| store.get(id).unwrap())
        .find(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_STRAY_ID)
        .unwrap();
    assert_eq!(child.entity_type_id(), VANILLA_ENTITY_TYPE_STRAY_ID);
    assert_eq!(child.position(), position());
    assert_eq!(
        child.delta_movement(),
        Vec3d {
            x: 0.25,
            y: 0.5,
            z: -0.75
        }
    );
    assert_eq!(child.fall_distance(), 3.5);
    assert!(child.on_ground());
    assert_eq!((child.y_rot(), child.x_rot()), (45.0, -10.0));
    assert_eq!(
        child.custom_name(),
        Some(&RichTextComponent::plain("frozen"))
    );
    assert!(child.custom_name_visible());
    let EntityData::GeneralLiving(data) = child.data() else {
        unreachable!()
    };
    assert!(data.mob_retained().unwrap().can_pick_up_loot());
    assert!(data.mob_retained().unwrap().persistence_required());
    assert_eq!(data.living().hurt_time, 7);
    assert_eq!(data.living().y_body_rot, 30.0);
    assert!(data
        .living()
        .item_by_slot(EquipmentSlot::MainHand)
        .is_some());
    assert_eq!(
        data.mob_retained()
            .unwrap()
            .drop_chance(EquipmentSlot::MainHand),
        0.75
    );
}
