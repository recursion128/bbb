//! `TagValueInput` reads each field on its own: an unreadable one takes its
//! default and the rest of the entity, its passengers and its brain still load.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_MANNEQUIN_ID, VANILLA_ENTITY_TYPE_RABBIT_ID,
    VANILLA_ENTITY_TYPE_VILLAGER_ID, VANILLA_ENTITY_TYPE_WARDEN_ID,
};
use bbb_protocol::packets::Vec3d;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::{EntityIdCounter, EntityStore};

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn put(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    entries.retain(|entry| entry.name != name);
    entries.push(entry(name, value));
}

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn store() -> EntityStore {
    EntityStore::new(0x5a_17, EntityIdCounter::new())
}

fn default_record(entity_type_id: i32) -> Vec<NbtSummaryEntry> {
    let mut store = store();
    store
        .spawn_general_living(
            entity_type_id,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
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

/// Loads `record` and writes it back out.
fn reloaded(record: Vec<NbtSummaryEntry>) -> (EntityStore, Vec<i32>, Vec<NbtSummaryEntry>) {
    let mut store = store();
    let ids = read_entity_chunk(&mut store, &chunk(record)).unwrap();
    let saved = write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec();
    (store, ids, saved)
}

#[test]
fn an_unreadable_variant_field_keeps_the_entity_and_its_passenger() {
    let mut warden = default_record(VANILLA_ENTITY_TYPE_WARDEN_ID);
    put(&mut warden, "anger", NbtSummaryValue::Int(3));
    let Some(NbtSummaryValue::Compound(mut listener)) = field(&warden, "listener").cloned() else {
        panic!("the warden writes its listener");
    };
    put(&mut listener, "event_delay", NbtSummaryValue::Int(7));
    let listener = NbtSummaryValue::Compound(listener);
    put(&mut warden, "listener", listener.clone());
    put(&mut warden, "CustomName", NbtSummaryValue::string("kept"));
    put(
        &mut warden,
        "Passengers",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(default_record(
            VANILLA_ENTITY_TYPE_RABBIT_ID,
        ))]),
    );
    let mut store = store();
    let ids = read_entity_chunk(&mut store, &chunk(warden)).unwrap();
    assert_eq!(ids.len(), 1, "the warden loads");
    let warden = store
        .get(ids[0])
        .expect("the vehicle loads")
        .custom_name()
        .map(|name| name.text().to_string());
    assert_eq!(warden.as_deref(), Some("kept"));
    assert_eq!(store.passengers(ids[0]).len(), 1, "its rider loads");
    let saved = write_entity_chunk(&store, (0, 0)).unwrap();
    assert_eq!(
        field(saved.entities().next().unwrap().raw(), "listener"),
        Some(&listener)
    );
}

#[test]
fn an_unreadable_memory_leaves_the_other_memories() {
    let mut villager = default_record(VANILLA_ENTITY_TYPE_VILLAGER_ID);
    let home = NbtSummaryValue::Compound(vec![entry(
        "value",
        NbtSummaryValue::Compound(vec![
            entry("dimension", NbtSummaryValue::string("minecraft:overworld")),
            entry("pos", NbtSummaryValue::IntArray(vec![1, 64, 2])),
        ]),
    )]);
    put(
        &mut villager,
        "Brain",
        NbtSummaryValue::Compound(vec![entry(
            "memories",
            NbtSummaryValue::Compound(vec![
                entry("minecraft:home", home.clone()),
                entry(
                    "minecraft:job_site",
                    NbtSummaryValue::Compound(vec![entry("value", NbtSummaryValue::Int(7))]),
                ),
            ]),
        )]),
    );
    let (_, _, saved) = reloaded(villager);
    let Some(NbtSummaryValue::Compound(brain)) = field(&saved, "Brain") else {
        panic!("the villager writes its brain");
    };
    let Some(NbtSummaryValue::Compound(memories)) = field(brain, "memories") else {
        panic!("the brain writes its memories");
    };
    assert_eq!(field(memories, "minecraft:home"), Some(&home));
    assert_eq!(field(memories, "minecraft:job_site"), None);
}

/// `Mannequin.java:130,148`.
#[test]
fn a_mannequin_pose_loads_onto_the_body_and_saves_back() {
    let mut mannequin = default_record(VANILLA_ENTITY_TYPE_MANNEQUIN_ID);
    put(&mut mannequin, "pose", NbtSummaryValue::string("crouching"));
    let (store, ids, saved) = reloaded(mannequin);
    assert_eq!(
        store.get(ids[0]).unwrap().pose_id(),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID
    );
    assert_eq!(
        field(&saved, "pose"),
        Some(&NbtSummaryValue::string("crouching"))
    );

    let mut invalid = default_record(VANILLA_ENTITY_TYPE_MANNEQUIN_ID);
    put(&mut invalid, "pose", NbtSummaryValue::string("roaring"));
    let (store, ids, saved) = reloaded(invalid);
    assert_eq!(
        store.get(ids[0]).unwrap().pose_id(),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID
    );
    assert_eq!(
        field(&saved, "pose"),
        Some(&NbtSummaryValue::string("standing"))
    );
}
