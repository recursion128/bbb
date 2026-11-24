use bbb_protocol::entity_synched_data::{
    VANILLA_FOX_FLAGS_DATA_ID, VANILLA_FOX_TRUSTED_DATA_ID_0, VANILLA_FOX_TRUSTED_DATA_ID_1,
    VANILLA_FOX_TYPE_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID;
use bbb_protocol::packets::{BlockPos, EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};
use uuid::Uuid;

use crate::entity::environment::{fluid_contact, FluidContact, FluidTag};
use crate::entity::tests::native_level::{state, NativeLevel};

use super::{after_super, before_ai_step, read_save_data, synched_values, FoxVariant};
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
    let mut store = EntityStore::new(0x46_4f_58, EntityIdCounter::new());
    store
        .spawn_general_living(VANILLA_ENTITY_TYPE_FOX_ID, position())
        .expect("Fox is GeneralLiving");
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

#[test]
fn fox_spawn_save_and_tracking_defaults() {
    let record = default_record();
    assert_eq!(
        field(&record, "CanPickUpLoot"),
        Some(&NbtSummaryValue::Byte(1))
    );
    assert_eq!(
        field(&record, "Trusted"),
        Some(&NbtSummaryValue::List(Vec::new()))
    );
    assert_eq!(
        field(&record, "Type"),
        Some(&NbtSummaryValue::string("red"))
    );
    for name in ["Sleeping", "Sitting", "Crouching"] {
        assert_eq!(field(&record, name), Some(&NbtSummaryValue::Byte(0)));
    }

    let state = read_save_data(&[]);
    let values = synched_values(&state);
    assert_eq!(values[0].value.data_id, VANILLA_FOX_TYPE_DATA_ID);
    assert_eq!(values[1].value.data_id, VANILLA_FOX_FLAGS_DATA_ID);
    assert_eq!(values[2].value.data_id, VANILLA_FOX_TRUSTED_DATA_ID_0);
    assert_eq!(values[3].value.data_id, VANILLA_FOX_TRUSTED_DATA_ID_1);
    assert!(values.iter().all(|value| value.is_default));
}

#[test]
fn fox_saved_fields_restore_and_default() {
    let first = Uuid::from_u128(1);
    let second = Uuid::from_u128(2);
    let mut record = default_record();
    set_field(
        &mut record,
        "Trusted",
        NbtSummaryValue::List(vec![uuid_value(first), uuid_value(second)]),
    );
    set_field(&mut record, "Sleeping", NbtSummaryValue::Byte(1));
    set_field(&mut record, "Type", NbtSummaryValue::string("snow"));
    set_field(&mut record, "Sitting", NbtSummaryValue::Byte(1));
    set_field(&mut record, "Crouching", NbtSummaryValue::Byte(1));
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = read_entity_chunk(&mut store, &chunk(record.clone())).unwrap()[0];
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        unreachable!()
    };
    let state = data.fox_retained().unwrap();
    assert_eq!(state.variant, FoxVariant::Snow);
    assert_eq!(state.trusted, [Some(first), Some(second)]);
    assert!(state.has_flag(super::FLAG_SLEEPING));
    assert!(state.has_flag(super::FLAG_SITTING));
    assert!(state.has_flag(super::FLAG_CROUCHING));

    let values = synched_values(state);
    assert_eq!(values[0].value.value, EntityDataValueKind::Int(1));
    assert_eq!(
        values[2].value.value,
        EntityDataValueKind::OptionalLivingEntityReference(Some(first))
    );
    assert_eq!(
        values[3].value.value,
        EntityDataValueKind::OptionalLivingEntityReference(Some(second))
    );

    let mut sparse = record.clone();
    sparse.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "Trusted" | "Sleeping" | "Type" | "Sitting" | "Crouching"
        )
    });
    let mut defaults = EntityStore::new(2, EntityIdCounter::new());
    let id = read_entity_chunk(&mut defaults, &chunk(sparse)).unwrap()[0];
    let EntityData::GeneralLiving(data) = defaults.get(id).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(
        data.fox_retained().unwrap(),
        &super::FoxRetainedState::default()
    );
}

#[test]
fn fox_targetless_and_no_ai_before_ai_step() {
    let mut store = EntityStore::new(4, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_FOX_ID, position())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.fox_retained_mut()
        .unwrap()
        .set_test_flags(true, true, true, true);
    let living = data.living_mut();
    living.jumping = true;
    living.xxa = 0.75;
    living.zza = -0.5;
    before_ai_step(entity, false);
    let EntityData::GeneralLiving(data) = &entity.data else {
        unreachable!()
    };
    let state = data.fox_retained().unwrap();
    assert!(!state.has_flag(super::FLAG_CROUCHING));
    assert!(!state.has_flag(super::FLAG_INTERESTED));
    assert!(state.has_flag(super::FLAG_SLEEPING));
    assert!(!data.living().jumping);
    assert_eq!((data.living().xxa, data.living().zza), (0.0, 0.0));

    let mut store = EntityStore::new(5, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_FOX_ID, position())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.set_no_ai(true);
    data.fox_retained_mut()
        .unwrap()
        .set_test_flags(true, true, true, true);
    data.living_mut().jumping = true;
    before_ai_step(entity, false);
    let EntityData::GeneralLiving(data) = &entity.data else {
        unreachable!()
    };
    let state = data.fox_retained().unwrap();
    assert!(state.has_flag(super::FLAG_CROUCHING));
    assert!(state.has_flag(super::FLAG_INTERESTED));
    assert!(!data.living().jumping);
}

#[test]
fn fox_water_wakes_and_interpolation() {
    let mut native = NativeLevel::new();
    native.set(BlockPos { x: 0, y: 64, z: 0 }, state("minecraft:water"));
    let mut store = EntityStore::new(6, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_FOX_ID, position())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.fox_retained_mut()
        .unwrap()
        .set_test_flags(true, true, true, true);
    native.with_level(|level| {
        after_super(
            entity,
            fluid_contact(entity, level, FluidTag::Water),
            false,
            false,
        )
    });
    let EntityData::GeneralLiving(data) = &entity.data else {
        unreachable!()
    };
    let state = data.fox_retained().unwrap();
    assert!(!state.has_flag(super::FLAG_SLEEPING));
    assert!(!state.has_flag(super::FLAG_SITTING));
    assert_eq!(state.interested_angle, 0.4);
    assert_eq!(state.crouch_amount, 0.2);

    let mut store = EntityStore::new(8, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_FOX_ID, position())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.set_no_ai(true);
    data.fox_retained_mut()
        .unwrap()
        .set_test_flags(true, true, true, true);
    native.with_level(|level| {
        after_super(
            entity,
            fluid_contact(entity, level, FluidTag::Water),
            false,
            false,
        )
    });
    let EntityData::GeneralLiving(data) = &entity.data else {
        unreachable!()
    };
    let state = data.fox_retained().unwrap();
    assert!(state.has_flag(super::FLAG_SLEEPING));
    assert!(state.has_flag(super::FLAG_SITTING));
    assert_eq!(state.interested_angle, 0.4);
    assert_eq!(state.crouch_amount, 0.2);
}

#[test]
fn a_live_target_keeps_the_crouch_and_wakes_the_fox() {
    // `Fox.aiStep` clears the crouch and interest only without a live target
    // (`Fox.java:223-227`), and `Fox.tick` wakes a fox that has one
    // (`Fox.java:552-554`).
    let mut store = EntityStore::new(9, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_FOX_ID, position())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.fox_retained_mut()
        .unwrap()
        .set_test_flags(true, false, true, true);
    before_ai_step(entity, true);
    after_super(entity, FluidContact::Known(false), true, false);
    let EntityData::GeneralLiving(data) = &entity.data else {
        unreachable!()
    };
    let state = data.fox_retained().unwrap();
    assert!(state.has_flag(super::FLAG_CROUCHING));
    assert!(state.has_flag(super::FLAG_INTERESTED));
    assert!(!state.has_flag(super::FLAG_SLEEPING));
}

mod command_trusted;
