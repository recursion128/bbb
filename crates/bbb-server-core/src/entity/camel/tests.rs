use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID;
use bbb_protocol::entity_poses::{VANILLA_ENTITY_POSE_SITTING_ID, VANILLA_ENTITY_POSE_STANDING_ID};
use bbb_protocol::entity_synched_data::{
    VANILLA_CAMEL_DASH_DATA_ID, VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_HORSE_ID,
    VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
};
use bbb_protocol::packets::{EntityDataValueKind, Vec3d};
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

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn spawn_record(entity_type_id: i32) -> Vec<NbtSummaryEntry> {
    let mut store = EntityStore::new(0x43_41_4d_4c, EntityIdCounter::new());
    store
        .spawn_general_living(entity_type_id, position())
        .expect("camel family is GeneralLiving");
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

#[test]
fn camel_last_pose_tick_spawn_defaults() {
    for entity_type_id in [
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    ] {
        let record = spawn_record(entity_type_id);
        assert_eq!(
            field(&record, "LastPoseTick"),
            Some(&NbtSummaryValue::Long(0))
        );

        let mut store = EntityStore::new(2, EntityIdCounter::new());
        let id = store
            .spawn_general_living(entity_type_id, position())
            .unwrap();
        let ids = non_default_synched_values(store.get(id).unwrap())
            .iter()
            .map(|value| value.data_id)
            .collect::<Vec<_>>();
        assert!(!ids.contains(&VANILLA_CAMEL_DASH_DATA_ID));
        assert!(!ids.contains(&VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID));
        assert_eq!(
            store.get(id).unwrap().pose_id(),
            VANILLA_ENTITY_POSE_STANDING_ID
        );
    }

    let horse = spawn_record(VANILLA_ENTITY_TYPE_HORSE_ID);
    assert!(field(&horse, "LastPoseTick").is_none());
    let skeleton_horse = spawn_record(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID);
    assert!(field(&skeleton_horse, "LastPoseTick").is_none());
}

#[test]
fn camel_last_pose_tick_runtime_retains_non_default_save_and_tracking() {
    for entity_type_id in [
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    ] {
        let mut store = EntityStore::new(6, EntityIdCounter::new());
        let id = store
            .spawn_general_living(entity_type_id, position())
            .unwrap();
        assert!(set_pose_tick(store.get_mut(id).unwrap(), -100));
        assert_eq!(
            store.get(id).unwrap().pose_id(),
            VANILLA_ENTITY_POSE_STANDING_ID
        );
        let tracked = non_default_synched_values(store.get(id).unwrap());
        let tick = tracked
            .iter()
            .find(|value| value.data_id == VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID)
            .expect("last pose change tick is dirty");
        assert_eq!(tick.serializer_id, VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID);
        assert_eq!(tick.value, EntityDataValueKind::Long(-100));
        assert!(tracked
            .iter()
            .all(|value| value.data_id != VANILLA_CAMEL_DASH_DATA_ID));

        let saved = write_entity_chunk(&store, (0, 0)).unwrap();
        let record = saved.entities().next().unwrap().raw().to_vec();
        assert_eq!(
            field(&record, "LastPoseTick"),
            Some(&NbtSummaryValue::Long(-100))
        );

        let mut restored = EntityStore::new(7, EntityIdCounter::new());
        let restored_id = read_entity_chunk(&mut restored, &saved).unwrap()[0];
        let EntityData::GeneralLiving(living) = restored.get(restored_id).unwrap().data() else {
            unreachable!()
        };
        assert_eq!(living.camel_state(), Some(CamelState::from_saved(-100)));
        assert_eq!(
            restored.get(restored_id).unwrap().pose_id(),
            VANILLA_ENTITY_POSE_SITTING_ID
        );
        let restored_tick = non_default_synched_values(restored.get(restored_id).unwrap())
            .into_iter()
            .find(|value| value.data_id == VANILLA_CAMEL_LAST_POSE_CHANGE_TICK_DATA_ID)
            .expect("restored last pose change tick is published");
        assert_eq!(restored_tick.value, EntityDataValueKind::Long(-100));
    }

    let mut store = EntityStore::new(8, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CAMEL_ID, position())
        .unwrap();
    assert!(set_pose_tick(store.get_mut(id).unwrap(), 140));
    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    assert_eq!(
        field(saved.entities().next().unwrap().raw(), "LastPoseTick"),
        Some(&NbtSummaryValue::Long(140))
    );
    let mut restored = EntityStore::new(9, EntityIdCounter::new());
    let restored_id = read_entity_chunk(&mut restored, &saved).unwrap()[0];
    assert_eq!(
        restored.get(restored_id).unwrap().pose_id(),
        VANILLA_ENTITY_POSE_STANDING_ID
    );
    let EntityData::GeneralLiving(living) = restored.get(restored_id).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(living.camel_state(), Some(CamelState::from_saved(140)));
}
