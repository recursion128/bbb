use super::*;
use bbb_protocol::NbtSummaryValue;
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_worldgen::noise::proto_chunk_entity::{
    mark_structure_finalize, ProtoChunkEntityRecord, StructureMobKind, StructureMobRecord,
    TemplateMobRecord,
};

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

fn compound<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a [NbtSummaryEntry]> {
    match field(entries, name)? {
        NbtSummaryValue::Compound(entries) => Some(entries),
        _ => None,
    }
}

fn mainhand(record: &[NbtSummaryEntry]) -> Option<&NbtSummaryValue> {
    field(compound(compound(record, "equipment")?, "mainhand")?, "id")
}

fn template(nbt: Vec<NbtSummaryEntry>, uuid: [i32; 4]) -> Vec<NbtSummaryEntry> {
    let record = TemplateMobRecord::from_template_nbt(
        &NbtSummaryValue::Compound(nbt),
        [8.5, 100.0, 8.5],
        [0.0, 0.0],
        uuid,
    )
    .unwrap()
    .unwrap();
    ProtoChunkEntityRecord::TemplateMob(record).entries()
}

fn id(resource_id: &str) -> NbtSummaryEntry {
    entry("id", NbtSummaryValue::string(resource_id))
}

#[test]
fn marked_structure_mobs_are_finalized_once_and_lose_the_marker() {
    let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
    let key: Java26LevelKey = Java26BuiltinDimension::Overworld.into();
    let dimension = world.require_level_id(&key);
    let chunks = &mut world.level_mut_by_id(&dimension).chunks;
    chunks.try_ensure_served_full_column(0, 0).unwrap();
    let villager = vec![
        id("minecraft:villager"),
        entry(
            "VillagerData",
            NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:plains")),
                entry("profession", NbtSummaryValue::string("minecraft:none")),
                entry("level", NbtSummaryValue::Int(1)),
            ]),
        ),
    ];
    let vindicator = StructureMobRecord::new(
        StructureMobKind::Vindicator,
        [8.5, 100.0, 8.5],
        [0, 0, 0, 2],
    )
    .unwrap();
    let mut records = vec![
        template(vec![id("minecraft:piglin_brute")], [0, 0, 0, 1]),
        ProtoChunkEntityRecord::StructureMob(vindicator).entries(),
        template(villager, [0, 0, 0, 3]),
        template(vec![id("minecraft:piglin_brute")], [0, 0, 0, 4]),
    ];
    for record in &mut records[..3] {
        mark_structure_finalize(record);
    }
    chunks.worldgen_entity_records_mut((0, 0)).unwrap()[0].extend(records);

    world.finalize_structure_mobs();
    let finalized = world
        .level_by_id(&dimension)
        .chunks
        .proto_entity_records(0, 0)
        .to_vec();
    assert_eq!(finalized.len(), 4);
    assert!(!finalized
        .iter()
        .any(|record| has_structure_finalize_marker(record)));
    let by_uuid = |last: i32| {
        finalized
            .iter()
            .find(|record| {
                field(record, "UUID") == Some(&NbtSummaryValue::IntArray(vec![0, 0, 0, last]))
            })
            .unwrap()
    };
    let golden_axe = NbtSummaryValue::string("minecraft:golden_axe");
    assert_eq!(mainhand(by_uuid(1)), Some(&golden_axe));
    assert_eq!(
        mainhand(by_uuid(2)),
        Some(&NbtSummaryValue::string("minecraft:iron_axe"))
    );
    assert_eq!(
        field(by_uuid(3), "AssignProfessionWhenSpawned"),
        Some(&NbtSummaryValue::Byte(1))
    );
    assert_eq!(mainhand(by_uuid(4)), None);

    // A second pass has nothing left to finalize.
    world.finalize_structure_mobs();
    assert_eq!(
        world
            .level_by_id(&dimension)
            .chunks
            .proto_entity_records(0, 0),
        finalized.as_slice()
    );
}
