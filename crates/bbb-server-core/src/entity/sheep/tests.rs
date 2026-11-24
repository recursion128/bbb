use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::level_event::{VanillaGameEvent, LEVEL_EVENT_PARTICLES_DESTROY_BLOCK};
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{
    VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_GRASS_BLOCK_ID, VANILLA_BLOCK_SHORT_GRASS_ID,
};
use bbb_protocol::entity_synched_data::VANILLA_SHEEP_WOOL_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHEEP_ID;
use bbb_protocol::packets::{BlockPos, EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::{apply_grazing, read_save_data, synched_values};
use crate::entity::ai::mob::sheep::GrazingAction;
use crate::entity::general_living::AgeableMobRetainedState;
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

#[test]
fn sheep_grazing_runtime_retained_save_and_tracking_are_exact() {
    let state = read_save_data(&[
        entry("Sheared", NbtSummaryValue::Byte(1)),
        entry("Color", NbtSummaryValue::Byte(14)),
    ]);
    assert!(state.sheared());
    assert_eq!(state.color(), 14);
    let row = synched_values(state)[0].clone();
    assert_eq!(row.value.data_id, VANILLA_SHEEP_WOOL_DATA_ID);
    assert_eq!(row.value.value, EntityDataValueKind::Byte(30));
    assert!(!row.is_default);

    let unknown = read_save_data(&[entry("Color", NbtSummaryValue::Byte(-1))]);
    assert_eq!(unknown.color(), 0);
}

#[test]
fn sheep_grazing_runtime_event_destroy_ate_and_growth_are_exact() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_SHEEP_ID, position())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    assert!(data.sheep_retained_mut().unwrap().set_sheared(true));
    *data.ageable_retained_mut().unwrap() = AgeableMobRetainedState::from_saved(-2400, 0, false);
    entity
        .living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .unwrap()
        .set_base_value(0.0);
    let _ = entity.take_synched_dirty();
    let _ = store.drain_lifecycle_events();

    let pos = BlockPos { x: 0, y: 64, z: 0 };
    let short_grass = vanilla_block_default_state_id(VANILLA_BLOCK_SHORT_GRASS_ID).unwrap();
    let mut native = NativeLevel::new();
    native.set(BlockPos { y: 63, ..pos }, state("minecraft:dirt"));
    native.set(pos, short_grass);
    for _ in 0..800 {
        native.with_level(|level| crate::entity::tick_entities(&mut store, level, true));
        let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
            unreachable!()
        };
        if !data.sheep_retained().unwrap().sheared() {
            break;
        }
    }

    let entity = store.get_mut(id).unwrap();
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!()
    };
    assert!(!data.sheep_retained().unwrap().sheared());
    assert!((-1200..0).contains(&data.ageable_retained().unwrap().age()));
    assert!(entity
        .take_synched_dirty()
        .contains(&VANILLA_SHEEP_WOOL_DATA_ID));
    assert_eq!(native.world.block_state(DIMENSION, pos), Some(air()));
    assert!(native
        .world
        .drain_level_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.event_type == LEVEL_EVENT_PARTICLES_DESTROY_BLOCK
                && event.pos == pos
                && event.data == short_grass
        ));
    let events = native.world.positioned_game_events_for_test(DIMENSION);
    assert!(events.iter().any(|event| event.recorded.game_event
        == VanillaGameEvent::BLOCK_DESTROY
        && event.recorded.pos == pos
        && event.recorded.source_entity_id.is_none()));
    assert!(events
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::EAT
            && event.recorded.source_entity_id == Some(id)
            && event.recorded.pos == pos));
    assert!(store.drain_lifecycle_events().iter().any(|event| matches!(
        event,
        crate::entity::store::EntityLifecycleEvent::Broadcast {
            entity_id,
            packet: crate::connection::ClientboundPlayPacket::EntityEvent(event),
        } if *entity_id == id && event.event_id == 10
    )));
}

#[test]
fn sheep_grazing_runtime_griefing_gate() {
    let mut store = EntityStore::new(2, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_SHEEP_ID, position())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.sheep_retained_mut().unwrap().set_sheared(true);
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    let below = BlockPos { y: 63, ..pos };
    let mut native = NativeLevel::new();
    native.set(below, state("minecraft:grass_block"));
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut native.world,
        1,
        "gamerule minecraft:mob_griefing false",
    );
    let mut entity = store.detach(id).unwrap();
    native.with_level(|level| apply_grazing(&mut entity, &mut store, level, GrazingAction::Eat));
    store.attach(entity);
    assert_eq!(
        native.world.block_state(DIMENSION, below),
        Some(state("minecraft:grass_block"))
    );
    assert!(native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::EAT
            && event.recorded.source_entity_id == Some(id)
            && event.recorded.pos == pos));
    assert!(!match store.get(id).unwrap().data() {
        EntityData::GeneralLiving(data) => data.sheep_retained().unwrap().sheared(),
        _ => unreachable!(),
    });

    crate::tests::console_command(
        &sender,
        &receiver,
        &mut native.world,
        2,
        "gamerule minecraft:mob_griefing true",
    );
    store
        .get_mut(id)
        .and_then(|entity| match &mut entity.data {
            EntityData::GeneralLiving(data) => data.sheep_retained_mut(),
            _ => None,
        })
        .unwrap()
        .set_sheared(true);
    let mut entity = store.detach(id).unwrap();
    native.with_level(|level| apply_grazing(&mut entity, &mut store, level, GrazingAction::Eat));
    store.attach(entity);
    let dirt =
        vanilla_block_default_state_id(bbb_protocol::block_types::VANILLA_BLOCK_DIRT_ID).unwrap();
    let grass = vanilla_block_default_state_id(VANILLA_BLOCK_GRASS_BLOCK_ID).unwrap();
    assert_eq!(native.world.block_state(DIMENSION, below), Some(dirt));
    assert!(native
        .world
        .drain_level_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.event_type == LEVEL_EVENT_PARTICLES_DESTROY_BLOCK
                && event.pos == below
                && event.data == grass
        ));
    assert!(native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::EAT
            && event.recorded.source_entity_id == Some(id)
            && event.recorded.pos == pos));
}

fn air() -> i32 {
    vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).unwrap()
}

#[path = "tests/growth.rs"]
mod growth;
