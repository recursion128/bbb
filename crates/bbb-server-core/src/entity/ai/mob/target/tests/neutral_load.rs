use super::*;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::tests::native_level::NativeLevel;
use crate::entity::{EntityIdCounter, EntityStore};
use bbb_protocol::entity_synched_data::{
    VANILLA_BEE_ANGER_END_TIME_DATA_ID, VANILLA_WOLF_ANGER_END_TIME_DATA_ID,
};
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_COW_ID};
use bbb_protocol::packets::EntityDataValueKind;
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

const KINDS: [i32; 6] = [
    VANILLA_ENTITY_TYPE_BEE_ID,
    VANILLA_ENTITY_TYPE_WOLF_ID,
    VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
];
fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}
fn put(fields: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    fields.retain(|e| e.name != name);
    fields.push(entry(name, value));
}
fn field<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    fields.iter().find(|e| e.name == name).map(|e| &e.value)
}
fn record(kind: i32) -> Vec<NbtSummaryEntry> {
    let mut store = EntityStore::new(19, EntityIdCounter::new());
    store.spawn_general_living(kind, at(0.5)).unwrap();
    let mut fields = write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec();
    fields.retain(|e| {
        !matches!(
            e.name.to_jdk_string().as_str(),
            "anger_end_time" | "AngerTime" | "angry_at"
        )
    });
    fields
}
fn chunk(
    fields: Vec<NbtSummaryEntry>,
) -> Result<Java26EntityChunkData, bbb_server_types::save::SaveError> {
    Java26EntityChunkData::parse(
        NamedCompound::new(
            "",
            vec![
                entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                entry("Position", NbtSummaryValue::IntArray(vec![0, 0])),
                entry(
                    "Entities",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(fields)]),
                ),
            ],
        ),
        0,
        0,
    )
}
fn assert_state(store: &EntityStore, id: i32, end_time: i64, target: Option<Uuid>) {
    let entity = store.get(id).unwrap();
    assert_eq!(anger::read(entity), Some(NeutralAnger { end_time, target }));
    if let EntityData::GeneralLiving(data) = entity.data() {
        if let Some(bee) = data.bee_retained() {
            assert_eq!(bee.persistent_anger(), (end_time, target));
        }
    }
    let data_id = match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_WOLF_ID => Some(VANILLA_WOLF_ANGER_END_TIME_DATA_ID),
        VANILLA_ENTITY_TYPE_BEE_ID => Some(VANILLA_BEE_ANGER_END_TIME_DATA_ID),
        _ => None,
    };
    if let Some(data_id) = data_id {
        let rows = crate::entity::tracking::synched_values(entity);
        assert_eq!(
            rows.iter()
                .find(|r| r.value.data_id == data_id)
                .unwrap()
                .value
                .value,
            EntityDataValueKind::Long(end_time)
        );
    }
    let NbtSummaryValue::Compound(fields) = store.command_data(id).unwrap() else {
        panic!()
    };
    assert_canonical(&fields, end_time, target);
    let saved = write_entity_chunk(store, (0, 0)).unwrap();
    let record = saved
        .entities()
        .find(|r| r.uuid() == entity.uuid())
        .unwrap();
    assert_canonical(record.raw(), end_time, target);
}
fn assert_canonical(fields: &[NbtSummaryEntry], end: i64, target: Option<Uuid>) {
    assert_eq!(
        field(fields, "anger_end_time"),
        Some(&NbtSummaryValue::Long(end))
    );
    assert!(field(fields, "AngerTime").is_none());
    assert_eq!(
        field(fields, "angry_at").cloned(),
        target.map(crate::entity::save::uuid_int_array)
    );
}
fn exercise(kind: i32, fields: Vec<NbtSummaryEntry>, time: i64, end: i64, target: Option<Uuid>) {
    let mut cold = EntityStore::new(23, EntityIdCounter::new());
    cold.set_game_time(time);
    let id = read_entity_chunk(&mut cold, &chunk(fields.clone()).unwrap()).unwrap()[0];
    assert_state(&cold, id, end, target);
    let mut command = EntityStore::new(23, EntityIdCounter::new());
    command.set_game_time(time);
    let id = command.spawn_general_living(kind, at(0.5)).unwrap();
    // Exercise both absent and previously canonical angry_at command shapes.
    for previous in [None, Some(Uuid::from_u128(99))] {
        anger::write(
            command.get_mut(id).unwrap(),
            NeutralAnger {
                end_time: 999,
                target: previous,
            },
        );
        command.load_command_data(id, &fields).unwrap();
        assert_state(&command, id, end, target);
    }
}
#[test]
fn six_neutral_readers_cold_and_command_read_the_anger_deadline() {
    for kind in KINDS {
        let mut fields = record(kind);
        put(&mut fields, "anger_end_time", NbtSummaryValue::Short(700));
        put(&mut fields, "AngerTime", NbtSummaryValue::Int(20));
        exercise(kind, fields, 100, 700, None);
        exercise(kind, record(kind), 100, -1, None);
    }
}

#[test]
fn a_cold_enderman_resolves_its_saved_anger_target_on_the_real_level() {
    let mut store = EntityStore::new(31, EntityIdCounter::new());
    let cow = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, at(2.5))
        .unwrap();
    let target = TargetRef {
        entity_id: cow,
        uuid: store.get(cow).unwrap().uuid(),
    };
    let mut fields = record(VANILLA_ENTITY_TYPE_ENDERMAN_ID);
    put(&mut fields, "anger_end_time", NbtSummaryValue::Long(500));
    put(
        &mut fields,
        "angry_at",
        crate::entity::save::uuid_int_array(target.uuid),
    );
    put(&mut fields, "NoAI", NbtSummaryValue::Byte(1));
    let id = read_entity_chunk(&mut store, &chunk(fields).unwrap()).unwrap()[0];
    let mut native = NativeLevel::new();
    native.world.set_game_time_for_test(100);
    native.with_level(|level| crate::entity::passenger::tick_root(&mut store, level, id));
    assert_eq!(store.mob_ai(id).unwrap().target_ref(), Some(target));
}
