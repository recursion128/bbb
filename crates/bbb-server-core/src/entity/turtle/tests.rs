use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::entity_synched_data::{
    VANILLA_TURTLE_HAS_EGG_DATA_ID, VANILLA_TURTLE_LAYING_EGG_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID;
use bbb_protocol::packets::{BlockPos, EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

use super::{after_ai_step, read_save_data, synched_values, travel};
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::{EntityData, EntityIdCounter, EntityStore};

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn state_id(name: &str) -> i32 {
    let block = vanilla_block_type_for_resource_id(name).expect("built-in block");
    vanilla_block_default_state_id(block.id).expect("built-in block default state")
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
    let mut store = EntityStore::new(0x54_55_52_54, EntityIdCounter::new());
    store
        .spawn_general_living(VANILLA_ENTITY_TYPE_TURTLE_ID, position())
        .unwrap();
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

#[test]
fn turtle_retained_runtime_census_save_and_tracking_are_exact() {
    let record = default_record();
    assert_eq!(
        field(&record, "home_pos"),
        Some(&NbtSummaryValue::IntArray(vec![0, 0, 0]))
    );
    assert_eq!(field(&record, "has_egg"), Some(&NbtSummaryValue::Byte(0)));
    let state = read_save_data(&[], [3.8, 70.0, -1.2]);
    assert_eq!(state.home_pos, BlockPos { x: 3, y: 70, z: -2 });
    let values = synched_values(&state);
    assert_eq!(values[0].value.data_id, VANILLA_TURTLE_HAS_EGG_DATA_ID);
    assert_eq!(values[1].value.data_id, VANILLA_TURTLE_LAYING_EGG_DATA_ID);
    assert!(values.iter().all(|value| value.is_default));
}

#[test]
fn turtle_retained_runtime_restore_values_and_defaults() {
    let mut record = default_record();
    set_field(
        &mut record,
        "home_pos",
        NbtSummaryValue::IntArray(vec![4, 70, -3]),
    );
    set_field(&mut record, "has_egg", NbtSummaryValue::Byte(1));
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = read_entity_chunk(&mut store, &chunk(record.clone())).unwrap()[0];
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        unreachable!()
    };
    let state = data.turtle_retained().unwrap();
    assert_eq!(state.home_pos, BlockPos { x: 4, y: 70, z: -3 });
    assert!(state.has_egg);
    assert_eq!(
        synched_values(state)[0].value.value,
        EntityDataValueKind::Boolean(true)
    );

    let mut sparse = record.clone();
    sparse.retain(|entry| !matches!(entry.name.to_jdk_string().as_str(), "home_pos" | "has_egg"));
    let mut defaults = EntityStore::new(2, EntityIdCounter::new());
    let id = read_entity_chunk(&mut defaults, &chunk(sparse)).unwrap()[0];
    let EntityData::GeneralLiving(data) = defaults.get(id).unwrap().data() else {
        unreachable!()
    };
    assert_eq!(
        data.turtle_retained().unwrap().home_pos,
        BlockPos { x: 0, y: 64, z: 0 }
    );
}

#[test]
fn turtle_retained_runtime_targetless_water_travel() {
    let mut store = EntityStore::new(4, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_TURTLE_ID, position())
        .unwrap();
    let mut entity = store.detach(id).unwrap();
    entity.set_delta_movement(Vec3d {
        x: 0.2,
        y: 0.1,
        z: -0.2,
    });
    let mut water = NativeLevel::new();
    for x in -1..=1 {
        for z in -1..=1 {
            water.set(BlockPos { x, y: 64, z }, state_id("minecraft:water"));
        }
    }
    assert_eq!(
        water.with_level(|level| travel(
            &mut entity,
            &mut store,
            level,
            Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0
            }
        )),
        Ok(true)
    );
    assert_vec_close(
        entity.position(),
        Vec3d {
            x: 0.7,
            y: 64.1,
            z: 0.3,
        },
    );
    assert_vec_close(
        entity.delta_movement(),
        Vec3d {
            x: 0.18,
            y: 0.085,
            z: -0.18,
        },
    );

    let mut dry = NativeLevel::new();
    assert_eq!(
        dry.with_level(|level| travel(
            &mut entity,
            &mut store,
            level,
            Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0
            }
        )),
        Ok(false)
    );

    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.turtle_retained_mut()
        .unwrap()
        .set_homeward(BlockPos { x: 0, y: 64, z: 0 });
    entity.set_delta_movement(Vec3d {
        x: 0.0,
        y: 0.1,
        z: 0.0,
    });
    assert_eq!(
        water.with_level(|level| travel(
            &mut entity,
            &mut store,
            level,
            Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0
            }
        )),
        Ok(true)
    );
    assert!((entity.delta_movement().y - 0.09).abs() < 1.0e-12);
}

#[test]
fn turtle_retained_runtime_laying_event_on_sand() {
    let mut store = EntityStore::new(5, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_TURTLE_ID, position())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    let state = data.turtle_retained_mut().unwrap();
    assert!(state.set_laying_egg(true));
    state.lay_egg_counter = 5;
    let sand = state_id("minecraft:sand");
    let mut native = NativeLevel::new();
    native.set(BlockPos { x: 0, y: 63, z: 0 }, sand);
    native.with_level(|level| after_ai_step(entity, level));
    assert!(native
        .world
        .drain_level_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.event_type == 2001
            && event.pos == BlockPos { x: 0, y: 64, z: 0 }
            && event.data == sand));
    assert!(native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_ACTION
                && event.recorded.source_entity_id == Some(id)
                && event.recorded.pos == BlockPos { x: 0, y: 64, z: 0 }
        ));
}

#[test]
fn adult_turtles_walk_home_without_eggs_while_babies_stay() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        for x in -1..=15 {
            for z in -1..=6 {
                level.set_block(
                    BlockPos { x, y: 63, z },
                    state_id("minecraft:stone"),
                    2,
                    crate::neighbor_updater::UPDATE_LIMIT,
                );
            }
        }
    });
    let mut store = EntityStore::new(0x54_55_52_54, EntityIdCounter::new());
    let mut ids = Vec::new();
    for (z, age) in [(0.5_f64, 0), (4.5_f64, -24_000)] {
        let mut record = default_record();
        set_field(
            &mut record,
            "home_pos",
            NbtSummaryValue::IntArray(vec![100, 64, z as i32]),
        );
        set_field(&mut record, "Age", NbtSummaryValue::Int(age));
        set_field(
            &mut record,
            "Pos",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Double(0.5_f64.to_bits()),
                NbtSummaryValue::Double(64.0_f64.to_bits()),
                NbtSummaryValue::Double(z.to_bits()),
            ]),
        );
        set_field(
            &mut record,
            "UUID",
            NbtSummaryValue::IntArray(vec![1, 2, 3, age]),
        );
        let id = read_entity_chunk(&mut store, &chunk(record)).unwrap()[0];
        store.get_mut(id).unwrap().on_ground = true;
        ids.push(id);
    }
    for _ in 0..4000 {
        native
            .world
            .set_game_time_for_test(native.world.game_time() + 1);
        native.with_level(|level| crate::entity::tick_entities(&mut store, level, true));
        if store.get(ids[0]).unwrap().position().x > 4.0 {
            break;
        }
    }
    assert!(store.get(ids[0]).unwrap().position().x > 4.0);
    assert!((store.get(ids[1]).unwrap().position().x - 0.5).abs() < 0.1);
}

fn assert_vec_close(actual: Vec3d, expected: Vec3d) {
    assert!((actual.x - expected.x).abs() < 1.0e-12);
    assert!((actual.y - expected.y).abs() < 1.0e-12);
    assert!((actual.z - expected.z).abs() < 1.0e-12);
}
