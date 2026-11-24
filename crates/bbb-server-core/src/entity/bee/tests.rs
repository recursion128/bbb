use crate::random::GameRandom;
use bbb_protocol::entity_synched_data::{
    VANILLA_BEE_ANGER_END_TIME_DATA_ID, VANILLA_BEE_FLAGS_DATA_ID, VANILLA_BEE_FLAG_HAS_NECTAR,
    VANILLA_BEE_FLAG_HAS_STUNG, VANILLA_BEE_FLAG_ROLL,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};
use uuid::Uuid;

use super::{synched_values, BeeRetainedState, BeeRuntime, BeeSavedFields};
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
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

fn set_field(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    if let Some(field) = entries.iter_mut().find(|entry| entry.name == name) {
        field.value = value;
    } else {
        entries.push(entry(name, value));
    }
}

fn uuid_value(uuid: Uuid) -> NbtSummaryValue {
    let bits = uuid.as_u128();
    NbtSummaryValue::IntArray(vec![
        (bits >> 96) as u32 as i32,
        (bits >> 64) as u32 as i32,
        (bits >> 32) as u32 as i32,
        bits as u32 as i32,
    ])
}

fn chunk(entries: Vec<NbtSummaryEntry>) -> Java26EntityChunkData {
    Java26EntityChunkData::parse(
        NamedCompound::new(
            "",
            vec![
                entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                entry("Position", NbtSummaryValue::IntArray(vec![0, 0])),
                entry(
                    "Entities",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entries)]),
                ),
            ],
        ),
        0,
        0,
    )
    .unwrap()
}

fn default_record() -> Vec<NbtSummaryEntry> {
    let mut store = EntityStore::new(0x42_45_45, EntityIdCounter::new());
    store
        .spawn_general_living(VANILLA_ENTITY_TYPE_BEE_ID, position())
        .expect("Bee is GeneralLiving");
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

#[test]
fn bee_spawn_save_and_tracking_defaults() {
    let record = default_record();
    for name in ["HasNectar", "HasStung"] {
        assert_eq!(field(&record, name), Some(&NbtSummaryValue::Byte(0)));
    }
    for name in [
        "TicksSincePollination",
        "CannotEnterHiveTicks",
        "CropsGrownSincePollination",
    ] {
        assert_eq!(field(&record, name), Some(&NbtSummaryValue::Int(0)));
    }
    assert_eq!(
        field(&record, "anger_end_time"),
        Some(&NbtSummaryValue::Long(-1))
    );
    assert_eq!(field(&record, "hive_pos"), None);
    assert_eq!(field(&record, "flower_pos"), None);
    assert_eq!(field(&record, "angry_at"), None);

    let values = synched_values(&BeeRetainedState::default());
    assert_eq!(values[0].value.data_id, VANILLA_BEE_FLAGS_DATA_ID);
    assert_eq!(values[1].value.data_id, VANILLA_BEE_ANGER_END_TIME_DATA_ID);
    assert!(values.iter().all(|value| value.is_default));
}

#[test]
fn bee_saved_fields_restore() {
    let target = Uuid::from_u128(0xfeed_beef);
    let mut record = default_record();
    set_field(
        &mut record,
        "hive_pos",
        NbtSummaryValue::IntArray(vec![1, 2, 3]),
    );
    set_field(
        &mut record,
        "flower_pos",
        NbtSummaryValue::IntArray(vec![-4, 5, -6]),
    );
    set_field(&mut record, "HasNectar", NbtSummaryValue::Byte(1));
    set_field(&mut record, "HasStung", NbtSummaryValue::Byte(1));
    set_field(
        &mut record,
        "TicksSincePollination",
        NbtSummaryValue::Int(7),
    );
    set_field(&mut record, "CannotEnterHiveTicks", NbtSummaryValue::Int(8));
    set_field(
        &mut record,
        "CropsGrownSincePollination",
        NbtSummaryValue::Int(9),
    );
    set_field(&mut record, "anger_end_time", NbtSummaryValue::Long(100));
    set_field(&mut record, "angry_at", uuid_value(target));

    let mut restored = EntityStore::new(1, EntityIdCounter::new());
    let id = read_entity_chunk(&mut restored, &chunk(record.clone())).unwrap()[0];
    let EntityData::GeneralLiving(data) = restored.get(id).unwrap().data() else {
        unreachable!()
    };
    let state = data.bee_retained().unwrap();
    assert_eq!(state.hive_pos, Some(BlockPos { x: 1, y: 2, z: 3 }));
    assert_eq!(state.flower_pos, Some(BlockPos { x: -4, y: 5, z: -6 }));
    assert!(state.has_flag(VANILLA_BEE_FLAG_HAS_NECTAR));
    assert!(state.has_flag(VANILLA_BEE_FLAG_HAS_STUNG));
    assert_eq!(state.ticks_without_nectar, 7);
    assert_eq!(state.stay_out_of_hive_countdown, 8);
    assert_eq!(state.crops_grown_since_pollination, 9);
    assert_eq!(state.anger_end_time, 100);
    assert_eq!(state.anger_target, Some(target));
}

#[test]
fn bee_custom_ai_counters_and_roll() {
    let random = GameRandom::with_seed(42);
    let mut runtime = BeeRuntime::new(&random);
    assert!((20..=60).contains(&runtime.counters().3));

    for _ in 0..20 {
        assert!(
            !runtime
                .custom_server_ai_step(&random, false, Some(true))
                .drown
        );
    }
    assert!(
        runtime
            .custom_server_ai_step(&random, false, Some(true))
            .drown
    );
    assert_eq!(runtime.counters().1, 21);
    runtime.custom_server_ai_step(&random, false, None);
    assert_eq!(runtime.counters().1, 21);
    runtime.custom_server_ai_step(&random, false, Some(false));
    assert_eq!(runtime.counters().1, 0);

    let mut state = BeeRetainedState::from_saved(BeeSavedFields {
        hive_pos: None,
        flower_pos: None,
        has_nectar: false,
        has_stung: false,
        ticks_without_nectar: 4,
        stay_out_of_hive_countdown: 2,
        crops_grown_since_pollination: 0,
        anger_end_time: 10,
        anger_target: Some(Uuid::from_u128(1)),
    });
    state.set_test_flags(false, false, true);
    state.custom_server_ai_step();
    state.custom_server_ai_step();
    assert_eq!(state.ticks_without_nectar, 6);
    assert_eq!(state.anger_end_time, 10);
    assert_eq!(state.anger_target, Some(Uuid::from_u128(1)));
    assert!(state.after_ai_step(false));
    assert_eq!(state.stay_out_of_hive_countdown, 1);
    assert!(!state.after_ai_step(false));
    assert_eq!(state.stay_out_of_hive_countdown, 0);
    assert!(!state.has_flag(VANILLA_BEE_FLAG_ROLL));

    runtime.after_tick(true);
    assert_eq!(runtime.roll_amount, 0.2);
    runtime.after_tick(false);
    assert_eq!(runtime.roll_amount_o, 0.2);
    assert_eq!(runtime.roll_amount, 0.0);
}

#[test]
fn cold_bee_anger_end_time_round_trips() {
    for (saved, legacy, expected) in [
        (Some(17), None, 17),
        (None, None, -1),
        (None, Some(20), 1020),
        (Some(17), Some(20), 17),
    ] {
        let mut record = default_record();
        record.retain(|entry| entry.name != "anger_end_time");
        if let Some(end_time) = saved {
            record.push(entry("anger_end_time", NbtSummaryValue::Long(end_time)));
        }
        if let Some(remaining) = legacy {
            record.push(entry("AngerTime", NbtSummaryValue::Int(remaining)));
        }
        let mut store = EntityStore::new(7, EntityIdCounter::new());
        store.set_game_time(1000);
        let id = read_entity_chunk(&mut store, &chunk(record)).unwrap()[0];
        let anger = crate::entity::ai::mob::target::anger::read(store.get(id).unwrap()).unwrap();
        assert_eq!(anger.end_time, expected);
        let saved = write_entity_chunk(&store, (0, 0)).unwrap();
        assert_eq!(
            field(saved.entities().next().unwrap().raw(), "anger_end_time"),
            Some(&NbtSummaryValue::Long(expected))
        );
    }
}

#[test]
fn common_anger_writes_only_the_typed_bee_owner_and_publishes_changed_end_time() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_BEE_ID, position())
        .unwrap();
    let body = store.get_mut(id).unwrap();
    body.take_synched_dirty();
    let anger = crate::entity::ai::mob::target::anger::NeutralAnger {
        end_time: 99,
        target: Some(Uuid::from_u128(8)),
    };
    crate::entity::ai::mob::target::anger::write(body, anger);
    assert_eq!(
        crate::entity::ai::mob::target::anger::read(body),
        Some(anger)
    );
    assert_eq!(
        body.take_synched_dirty(),
        [VANILLA_BEE_ANGER_END_TIME_DATA_ID]
    );
    crate::entity::ai::mob::target::anger::write(body, anger);
    assert!(body.take_synched_dirty().is_empty());
    let EntityData::GeneralLiving(data) = body.data() else {
        unreachable!()
    };
    assert!(data
        .opaque_fields()
        .iter()
        .all(|entry| entry.name != "anger_end_time" && entry.name != "angry_at"));
}
