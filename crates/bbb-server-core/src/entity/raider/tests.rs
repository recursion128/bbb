use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_PILLAGER_ID,
    VANILLA_ENTITY_TYPE_RAVAGER_ID, VANILLA_ENTITY_TYPE_VINDICATOR_ID,
    VANILLA_ENTITY_TYPE_WITCH_ID,
};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

use super::{owns_save_field, read_save_data, write_save_data, RaiderRaid, RaiderRetainedState};
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::{EntityData, EntityIdCounter, EntityStore};

mod johnny;

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

fn chunk_document(entries: Vec<NbtSummaryEntry>) -> NamedCompound {
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
    )
}

fn chunk(entries: Vec<NbtSummaryEntry>) -> Java26EntityChunkData {
    Java26EntityChunkData::parse(chunk_document(entries), 0, 0).unwrap()
}

fn default_witch_record() -> Vec<NbtSummaryEntry> {
    let mut store = EntityStore::new(0x52_41_49_44, EntityIdCounter::new());
    store
        .spawn_general_living(VANILLA_ENTITY_TYPE_WITCH_ID, position())
        .expect("Witch is GeneralLiving");
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

#[test]
fn raider_retained_family_defaults_and_round_trip_are_exact() {
    let supported: Vec<_> = VANILLA_ENTITY_TYPES
        .iter()
        .filter(|facts| RaiderRetainedState::new(facts.id).is_some())
        .map(|facts| facts.resource_id)
        .collect();
    assert_eq!(
        supported,
        [
            "minecraft:evoker",
            "minecraft:illusioner",
            "minecraft:pillager",
            "minecraft:ravager",
            "minecraft:vindicator",
            "minecraft:witch",
        ]
    );
    let default = read_save_data(VANILLA_ENTITY_TYPE_WITCH_ID, &[]);
    assert_eq!(
        default,
        RaiderRetainedState::from_saved(None, false, false, 0, false, RaiderRaid::None)
    );

    let source = [
        entry(
            "patrol_target",
            NbtSummaryValue::IntArray(vec![i32::MIN, -2, i32::MAX]),
        ),
        entry("PatrolLeader", NbtSummaryValue::Byte(1)),
        entry("Patrolling", NbtSummaryValue::Byte(0)),
        entry("Wave", NbtSummaryValue::Int(i32::MIN)),
        entry("CanJoinRaid", NbtSummaryValue::Byte(1)),
    ];
    let state = read_save_data(VANILLA_ENTITY_TYPE_RAVAGER_ID, &source);
    assert_eq!(
        state,
        RaiderRetainedState {
            ravager: Some(crate::entity::ravager::RavagerTicks::DEFAULT),
            ..RaiderRetainedState::from_saved(
                Some(BlockPos {
                    x: i32::MIN,
                    y: -2,
                    z: i32::MAX,
                }),
                true,
                false,
                i32::MIN,
                true,
                RaiderRaid::None,
            )
        }
    );
    let mut written = Vec::new();
    write_save_data(&mut written, &state);
    assert_eq!(
        field(&written, "patrol_target"),
        Some(&NbtSummaryValue::IntArray(vec![i32::MIN, -2, i32::MAX]))
    );
    assert_eq!(
        field(&written, "PatrolLeader"),
        Some(&NbtSummaryValue::Byte(1))
    );
    assert_eq!(
        field(&written, "Patrolling"),
        Some(&NbtSummaryValue::Byte(0))
    );
    assert_eq!(
        field(&written, "Wave"),
        Some(&NbtSummaryValue::Int(i32::MIN))
    );
    assert_eq!(
        field(&written, "CanJoinRaid"),
        Some(&NbtSummaryValue::Byte(1))
    );
}

#[test]
fn witch_raider_save_restore_defaults_and_resolved_raid_id_are_exact() {
    let record = default_witch_record();
    assert_eq!(field(&record, "patrol_target"), None);
    assert_eq!(
        field(&record, "PatrolLeader"),
        Some(&NbtSummaryValue::Byte(0))
    );
    assert_eq!(
        field(&record, "Patrolling"),
        Some(&NbtSummaryValue::Byte(0))
    );
    assert_eq!(field(&record, "Wave"), Some(&NbtSummaryValue::Int(0)));
    assert_eq!(
        field(&record, "CanJoinRaid"),
        Some(&NbtSummaryValue::Byte(0))
    );

    let mut sparse = record.clone();
    sparse.retain(|entry| !owns_save_field(&entry.name.to_jdk_string()));
    let mut sparse_store = EntityStore::new(1, EntityIdCounter::new());
    let sparse_id = read_entity_chunk(&mut sparse_store, &chunk(sparse)).unwrap()[0];
    let EntityData::GeneralLiving(sparse_data) = sparse_store.get(sparse_id).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(
        sparse_data.raider_retained(),
        Some(&RaiderRetainedState::from_saved(
            None,
            false,
            false,
            0,
            false,
            RaiderRaid::None,
        ))
    );
    let mut custom = record;
    set_field(
        &mut custom,
        "patrol_target",
        NbtSummaryValue::IntArray(vec![1, -2, 3]),
    );
    set_field(&mut custom, "PatrolLeader", NbtSummaryValue::Byte(1));
    set_field(&mut custom, "Patrolling", NbtSummaryValue::Byte(1));
    set_field(&mut custom, "Wave", NbtSummaryValue::Int(7));
    set_field(&mut custom, "CanJoinRaid", NbtSummaryValue::Byte(1));
    set_field(&mut custom, "RaidId", NbtSummaryValue::Int(31));
    let custom_record = custom.clone();
    let mut restored = EntityStore::new(2, EntityIdCounter::new());
    let id = read_entity_chunk(&mut restored, &chunk(custom)).unwrap()[0];
    let EntityData::GeneralLiving(data) = restored.get(id).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(
        data.raider_retained(),
        Some(&RaiderRetainedState::from_saved(
            Some(BlockPos { x: 1, y: -2, z: 3 }),
            true,
            true,
            7,
            true,
            RaiderRaid::Unresolved(31),
        ))
    );

    // `Raider.addAdditionalSaveData` (`Raider.java:185-187`) writes the id off
    // the resolved raid, so an id no `Raids` claimed is not written back.
    let saved = write_entity_chunk(&restored, (0, 0)).unwrap();
    let raw = saved.entities().next().unwrap().raw();
    assert_eq!(field(raw, "RaidId"), None);

    let mut refused = EntityStore::new(2, EntityIdCounter::new());
    let refused_id = read_entity_chunk(&mut refused, &chunk(custom_record)).unwrap()[0];
    assert_eq!(
        super::resolve_raider_raid(refused.get_mut(refused_id).unwrap(), |_| false),
        None
    );
    assert_eq!(
        raid_of(&refused, refused_id),
        RaiderRaid::None,
        "an id the manager rejects leaves no current raid"
    );

    assert_eq!(
        super::resolve_raider_raid(restored.get_mut(id).unwrap(), |raid_id| raid_id == 31),
        Some(super::RaiderRaidJoin {
            raid_id: 31,
            wave: 7,
            patrol_leader: true,
        })
    );
    assert_eq!(raid_of(&restored, id), RaiderRaid::Current(31));
    let saved = write_entity_chunk(&restored, (0, 0)).unwrap();
    let raw = saved.entities().next().unwrap().raw();
    assert_eq!(field(raw, "RaidId"), Some(&NbtSummaryValue::Int(31)));

    assert!(super::clear_raider_raid(restored.get_mut(id).unwrap()));
    assert_eq!(raid_of(&restored, id), RaiderRaid::None);
    assert_eq!(
        field(
            write_entity_chunk(&restored, (0, 0))
                .unwrap()
                .entities()
                .next()
                .unwrap()
                .raw(),
            "RaidId"
        ),
        None
    );
}

fn raid_of(store: &EntityStore, id: i32) -> RaiderRaid {
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        unreachable!()
    };
    data.raider_retained().unwrap().raid()
}

/// `Raider.hasRaid()` (`Raider.java:156-158`) is
/// `getCurrentRaid() != null || level.getRaidAt(blockPosition()) != null`, and
/// the second disjunct is the level's answer this crate is handed.
///
/// A resolved current raid short-circuits it; a raider with none takes the
/// level's answer, and a level that owns no raid map (or a saved id the level
/// has not resolved) leaves the term open for the loot engine to count.
#[test]
fn has_raid_reads_the_levels_near_raid_answer_per_retained_state() {
    let mut store = EntityStore::new(0x52_41_49_44, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PILLAGER_ID, position())
        .expect("Pillager is GeneralLiving");
    for answer in [Some(false), Some(true), None] {
        assert_eq!(
            super::has_raid(store.get(id).unwrap(), answer),
            answer,
            "a raider with no current raid takes the level's answer {answer:?}"
        );
    }

    // The first disjunct returns before `level.getRaidAt` is consulted.
    let entity = store.get_mut(id).unwrap();
    assert!(crate::entity::join_raid(entity, 4, 0));
    assert_eq!(
        super::has_raid(store.get(id).unwrap(), Some(false)),
        Some(true)
    );

    // `RaiderRaid::Unresolved` is a saved id the level has not looked up yet,
    // so neither disjunct is known: the level's map alone cannot decide it.
    let unresolved = read_save_data(
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
        &[entry("RaidId", NbtSummaryValue::Int(9))],
    );
    let entity = store.get_mut(id).unwrap();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        panic!("a pillager is GeneralLiving");
    };
    *data.raider_retained_mut().unwrap() = unresolved;
    assert_eq!(super::has_raid(store.get(id).unwrap(), Some(false)), None);

    // A type that is not a raider never answers: the `instanceof Raider` gate.
    let cow = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, position())
        .expect("Cow is GeneralLiving");
    assert_eq!(super::has_raid(store.get(cow).unwrap(), Some(false)), None);
}
