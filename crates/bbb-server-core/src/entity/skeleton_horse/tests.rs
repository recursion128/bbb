use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
};
use bbb_protocol::packets::Vec3d;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::*;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::tracking::non_default_synched_values;
use crate::entity::{EntityData, EntityIdCounter, EntityStore};

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

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn spawn_record(entity_type_id: i32) -> Vec<NbtSummaryEntry> {
    let mut store = EntityStore::new(0x54_52_41_50, EntityIdCounter::new());
    store
        .spawn_general_living(entity_type_id, position())
        .expect("horse family is GeneralLiving");
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

fn round_trip(entity_type_id: i32) -> Option<SkeletonHorseTrapState> {
    let mut store = EntityStore::new(0x54_52_41_50, EntityIdCounter::new());
    store
        .spawn_general_living(entity_type_id, position())
        .expect("horse family is GeneralLiving");
    let data = write_entity_chunk(&store, (0, 0)).unwrap();
    let mut restored = EntityStore::new(1, EntityIdCounter::new());
    let id = read_entity_chunk(&mut restored, &data).unwrap()[0];
    let EntityData::GeneralLiving(living) = restored.get(id).unwrap().data() else {
        unreachable!()
    };
    living.skeleton_horse_trap()
}

#[test]
fn skeleton_horse_trap_runtime_census_and_spawn_defaults_are_exact() {
    let owned = VANILLA_ENTITY_TYPES
        .iter()
        .filter_map(|facts| SkeletonHorseTrapState::new(facts.id).map(|_| facts.id))
        .collect::<Vec<_>>();
    assert_eq!(owned, [VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID]);

    let skeleton_horse = spawn_record(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID);
    assert_eq!(
        field(&skeleton_horse, "SkeletonTrap"),
        Some(&NbtSummaryValue::Byte(0))
    );
    assert_eq!(
        field(&skeleton_horse, "SkeletonTrapTime"),
        Some(&NbtSummaryValue::Int(0))
    );

    let horse = spawn_record(VANILLA_ENTITY_TYPE_HORSE_ID);
    assert!(field(&horse, "SkeletonTrap").is_none());
    assert!(field(&horse, "SkeletonTrapTime").is_none());

    let zombie_horse = spawn_record(VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID);
    assert!(field(&zombie_horse, "SkeletonTrap").is_none());
    assert!(field(&zombie_horse, "SkeletonTrapTime").is_none());
}

#[test]
fn skeleton_horse_trap_runtime_restore_defaults_and_values() {
    assert_eq!(
        round_trip(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID),
        Some(SkeletonHorseTrapState::from_saved(false, 0))
    );
    assert_eq!(round_trip(VANILLA_ENTITY_TYPE_HORSE_ID), None);
    assert_eq!(round_trip(VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID), None);

    assert_eq!(
        read_save_data(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, &[]),
        Some(SkeletonHorseTrapState::from_saved(false, 0))
    );
    assert_eq!(
        read_save_data(
            VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
            &[
                entry("SkeletonTrap", NbtSummaryValue::Byte(1)),
                entry("SkeletonTrapTime", NbtSummaryValue::Int(i32::MIN)),
            ]
        ),
        Some(SkeletonHorseTrapState::from_saved(true, i32::MIN))
    );
    assert_eq!(
        read_save_data(
            VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
            &[entry("SkeletonTrapTime", NbtSummaryValue::Int(i32::MAX))]
        ),
        Some(SkeletonHorseTrapState::from_saved(false, i32::MAX))
    );
}

#[test]
fn skeleton_horse_trap_runtime_retains_non_default_save_and_is_not_synched() {
    let mut store = EntityStore::new(6, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, position())
        .unwrap();
    let before = non_default_synched_values(store.get(id).unwrap())
        .iter()
        .map(|value| value.data_id)
        .collect::<Vec<_>>();
    assert!(set_state(
        store.get_mut(id).unwrap(),
        SkeletonHorseTrapState::from_saved(true, 18000)
    ));
    let after = non_default_synched_values(store.get(id).unwrap())
        .iter()
        .map(|value| value.data_id)
        .collect::<Vec<_>>();
    assert_eq!(before, after);

    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    let record = saved.entities().next().unwrap().raw().to_vec();
    assert_eq!(
        field(&record, "SkeletonTrap"),
        Some(&NbtSummaryValue::Byte(1))
    );
    assert_eq!(
        field(&record, "SkeletonTrapTime"),
        Some(&NbtSummaryValue::Int(18000))
    );

    let mut restored = EntityStore::new(7, EntityIdCounter::new());
    let restored_id = read_entity_chunk(&mut restored, &saved).unwrap()[0];
    let EntityData::GeneralLiving(living) = restored.get(restored_id).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(
        living.skeleton_horse_trap(),
        Some(SkeletonHorseTrapState::from_saved(true, 18000))
    );
}
