use super::*;

use crate::entity::{EntityData, GeneralLivingData};
use bbb_protocol::entity_synched_data::VANILLA_HAPPY_GHAST_STAYS_STILL_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID;
use bbb_protocol::packets::EntityDataValueKind;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
pub(crate) const POSITION: Vec3d = Vec3d {
    x: 0.5,
    y: 300.0,
    z: 0.5,
};
const TOP: Vec3d = Vec3d {
    x: 0.5,
    y: 304.0,
    z: 0.5,
};
const FAR: Vec3d = Vec3d {
    x: 12.5,
    y: 300.0,
    z: 12.5,
};

pub(crate) fn fixture() -> (crate::mailbox::WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) =
        crate::mailbox::world_command_mailbox(crate::mailbox::WorldMailboxLimits::default())
            .unwrap();
    let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.set_player_position_for_test(ConnectionId(1), FAR);
    world.set_player_position_for_test(ConnectionId(2), FAR);
    (receiver, world)
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn record(world: &ServerWorld, dimension: Java26BuiltinDimension, id: i32) -> Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(entries) = world
        .stored_entity_command_data_for_test(dimension, id)
        .unwrap()
    else {
        panic!()
    };
    entries
}

fn set_field(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    entries.retain(|row| row.name != name);
    entries.push(entry(name, value));
}

pub(crate) fn set_timeout(
    world: &mut ServerWorld,
    dimension: Java26BuiltinDimension,
    id: i32,
    timeout: i32,
) {
    let mut entries = record(world, dimension, id);
    set_field(&mut entries, "still_timeout", NbtSummaryValue::Int(timeout));
    world.load_stored_entity_record_for_test(dimension, id, &entries);
}

pub(crate) fn timeout(world: &ServerWorld, dimension: Java26BuiltinDimension, id: i32) -> i32 {
    match record(world, dimension, id)
        .iter()
        .find(|row| row.name == "still_timeout")
        .map(|row| &row.value)
    {
        Some(NbtSummaryValue::Int(value)) => *value,
        other => panic!("still_timeout is an int: {other:?}"),
    }
}

pub(crate) fn spawn_happy_ghast(
    world: &mut ServerWorld,
    dimension: Java26BuiltinDimension,
    position: Vec3d,
    still_timeout: i32,
) -> i32 {
    let id = world
        .spawn_stored_entity_for_test(
            dimension,
            VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
            position,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID).unwrap(),
            ),
        )
        .0;
    let mut entries = record(world, dimension, id);
    set_field(&mut entries, "NoAI", NbtSummaryValue::Byte(1));
    set_field(&mut entries, "NoGravity", NbtSummaryValue::Byte(1));
    set_field(
        &mut entries,
        "still_timeout",
        NbtSummaryValue::Int(still_timeout),
    );
    world.load_stored_entity_record_for_test(dimension, id, &entries);
    id
}

fn mount(world: &mut ServerWorld, connection_id: ConnectionId, happy: i32) {
    world.start_external_player_ride(connection_id, DIMENSION, happy, 0.0, 0.0, &mut Vec::new());
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(connection_id)
            .2,
        Some(happy)
    );
}

fn metadata_flag(event: &WorldEvent, id: i32) -> Option<bool> {
    let Some(ClientboundPlayPacket::SetEntityData(packet)) = event.sent_packet() else {
        return None;
    };
    if packet.id != id {
        return None;
    }
    packet.values.iter().find_map(|row| {
        if row.data_id == VANILLA_HAPPY_GHAST_STAYS_STILL_DATA_ID {
            if let EntityDataValueKind::Boolean(value) = row.value {
                return Some(value);
            }
        }
        None
    })
}

#[test]
fn happy_ghast_still_no_ai_real_tick_has_grace_decay_and_last_precise_frame() {
    let (receiver, mut world) = fixture();
    let happy = spawn_happy_ghast(&mut world, DIMENSION, POSITION, 1);
    let mut publication = Vec::new();
    world.apply_level_entity_lifecycle_for_test(DIMENSION, &mut publication);
    while world
        .stored_entity_for_test(DIMENSION, happy)
        .unwrap()
        .tick_count()
        < 60
    {
        world.tick(receiver.drain());
        assert_eq!(timeout(&world, DIMENSION, happy), 1);
    }
    world.tick(receiver.drain());
    assert_eq!(timeout(&world, DIMENSION, happy), 0);
    let last = world.tick(receiver.drain());
    assert!(last
        .events
        .iter()
        .any(|event| metadata_flag(event, happy) == Some(false)));
    assert!(last.events.iter().any(|event| matches!(
        event.sent_packet(),
        Some(ClientboundPlayPacket::EntityPositionSync(packet))
            if packet.id == happy && packet.position == POSITION
    )));
    set_timeout(&mut world, DIMENSION, happy, -7);
    world.tick(receiver.drain());
    assert_eq!(timeout(&world, DIMENSION, happy), -7);
}

#[test]
fn happy_ghast_still_real_tick_scan_filters_spectators_and_any_happy_root() {
    let (receiver, mut world) = fixture();
    let happy = spawn_happy_ghast(&mut world, DIMENSION, POSITION, 0);
    world.set_player_position_for_test(ConnectionId(2), TOP);
    world.set_player_game_mode(ConnectionId(2), GameType::Spectator, &mut Vec::new());
    world.tick(receiver.drain());
    assert_eq!(timeout(&world, DIMENSION, happy), 0);
    world.set_player_game_mode(ConnectionId(2), GameType::Creative, &mut Vec::new());
    let scan = world.tick(receiver.drain());
    assert_eq!(timeout(&world, DIMENSION, happy), 10);
    let mut tracked_events = scan.events;
    tracked_events.extend(world.tick(receiver.drain()).events);
    let other_happy = spawn_happy_ghast(&mut world, DIMENSION, TOP, 0);
    mount(&mut world, ConnectionId(2), other_happy);
    world.set_player_position_for_test(ConnectionId(2), TOP);
    set_timeout(&mut world, DIMENSION, happy, 0);
    world.tick(receiver.drain());
    assert_eq!(timeout(&world, DIMENSION, happy), 0);
}
