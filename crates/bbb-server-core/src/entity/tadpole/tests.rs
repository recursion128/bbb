use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID;
use bbb_protocol::entity_synched_data::VANILLA_TADPOLE_AGE_LOCKED_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TADPOLE_ID;
use bbb_protocol::packets::EntityDataValueKind;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::{read_save_data, synched_values, write_save_data, TadpoleRetainedState};

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

#[test]
fn tadpole_retained_defaults_round_trip() {
    let state = read_save_data(&[]);
    assert_eq!(state.age(), 0);
    assert!(!state.age_locked());
    let mut entries = Vec::new();
    write_save_data(&mut entries, state);
    assert_eq!(
        entries,
        vec![
            entry("Age", NbtSummaryValue::Int(0)),
            entry("AgeLocked", NbtSummaryValue::Byte(0)),
        ]
    );
    assert_eq!(read_save_data(&entries), state);
}

#[test]
fn tadpole_age_locked_tracking_uses_generated_accessor() {
    let state = TadpoleRetainedState::from_saved(73, true);
    let [value] = synched_values(state);
    assert_eq!(value.value.data_id, VANILLA_TADPOLE_AGE_LOCKED_DATA_ID);
    assert_eq!(
        value.value.serializer_id,
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID
    );
    assert_eq!(value.value.value, EntityDataValueKind::Boolean(true));
    assert!(!value.is_default);

    let [default] = synched_values(
        crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_TADPOLE_ID)
            .unwrap()
            .tadpole_retained()
            .unwrap(),
    );
    assert!(default.is_default);
}

#[test]
fn a_tadpole_ages_each_tick_and_asks_for_its_frog() {
    use crate::entity::tests::native_level::NativeLevel;
    use crate::entity::{tick_one_entity, EntityCommand, EntityData, EntityIdCounter, EntityStore};

    let mut store = EntityStore::new(3, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_TADPOLE_ID,
            bbb_protocol::packets::Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    let EntityData::GeneralLiving(data) = &mut store.get_mut(id).unwrap().data else {
        unreachable!()
    };
    *data.tadpole_retained_mut().unwrap() = TadpoleRetainedState::from_saved(23_998, false);
    let mut native = NativeLevel::new();
    let grow = |store: &mut EntityStore| {
        store
            .drain_commands()
            .iter()
            .any(|command| *command == EntityCommand::TadpoleGrowUp { entity_id: id })
    };

    native.with_level(|level| tick_one_entity(&mut store, level, id));
    assert!(!grow(&mut store));
    native.with_level(|level| tick_one_entity(&mut store, level, id));
    assert!(grow(&mut store));
}
