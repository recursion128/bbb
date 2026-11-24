use super::*;

use bbb_server_datapack::dynamic_registry::RawRegistryElement;
use bbb_server_persistence::{
    Java26WorldClockEntry, Java26WorldClockState, Java26WorldClocks, RegionCompression,
};

fn clocks(entries: &[(&str, Java26WorldClockState)]) -> Java26WorldClocks {
    Java26WorldClocks::new(
        entries
            .iter()
            .map(|(resource_id, state)| Java26WorldClockEntry {
                resource_id: (*resource_id).to_string(),
                state: *state,
            })
            .collect(),
    )
}

fn config_with_custom_clock() -> ServerWorldConfig {
    let mut config = datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let registry = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:world_clock")
        .unwrap();
    let json = serde_json::json!({});
    registry.elements.insert(
        0,
        RawRegistryElement {
            id: "example:aaa".to_string(),
            source_pack: "file/world_clock".to_string(),
            source_json: serde_json::to_vec(&json).unwrap().into(),
            source: Some(
                bbb_registry_codec::decode_direct_registry_json(
                    &serde_json::to_vec(&(json)).unwrap(),
                )
                .unwrap(),
            ),
            codec_value: None,
        },
    );
    config
}

#[test]
fn custom_holder_keeps_the_live_registry_and_wire_order() {
    let mut world = ServerWorld::new(config_with_custom_clock()).unwrap();
    assert_eq!(world.world_clock_states_for_test().len(), 3);
    for (index, total_ticks) in [101, 202, 303].into_iter().enumerate() {
        world.set_world_clock_state_for_test(
            index,
            Java26WorldClockState {
                total_ticks,
                ..Java26WorldClockState::default()
            },
        );
    }
    assert_eq!(
        world
            .world_clock_snapshot_for_test()
            .entries()
            .iter()
            .map(|entry| (entry.resource_id.as_str(), entry.state.total_ticks))
            .collect::<Vec<_>>(),
        [
            ("example:aaa", 101),
            ("minecraft:overworld", 202),
            ("minecraft:the_end", 303),
        ]
    );
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(7),
            profile: profile("clock", 7),
        },
    )
    .unwrap();
    let joined = world.tick(receiver.drain());
    let update = joined
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(7),
                packet: ClientboundPlayPacket::SetTime(update),
            } if !update.clock_updates.is_empty() => Some(update),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        update
            .clock_updates
            .iter()
            .map(|clock| (clock.clock_id, clock.total_ticks))
            .collect::<Vec<_>>(),
        [(0, 101), (1, 202), (2, 303)]
    );
}

#[test]
fn fractional_paused_and_global_advance_time_state_tick_independently() {
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    world.set_world_clock_state_for_test(
        0,
        Java26WorldClockState {
            total_ticks: 10,
            partial_tick: 0.25,
            rate: 0.5,
            paused: false,
        },
    );
    world.set_world_clock_state_for_test(
        1,
        Java26WorldClockState {
            total_ticks: 20,
            partial_tick: 0.75,
            rate: 2.0,
            paused: true,
        },
    );

    world.tick(Vec::new());
    assert_eq!(
        world.world_clock_states_for_test(),
        vec![
            Java26WorldClockState {
                total_ticks: 10,
                partial_tick: 0.75,
                rate: 0.5,
                paused: false,
            },
            Java26WorldClockState {
                total_ticks: 20,
                partial_tick: 0.75,
                rate: 2.0,
                paused: true,
            }
        ]
    );
    world.tick(Vec::new());
    assert_eq!(world.world_clock_states_for_test()[0].total_ticks, 11);
    assert_eq!(world.world_clock_states_for_test()[0].partial_tick, 0.25);

    world.set_advance_time_for_test(false);
    let before = world.world_clock_states_for_test();
    world.tick(Vec::new());
    assert_eq!(world.world_clock_states_for_test(), before);
    assert!(world
        .world_clock_updates_for_test()
        .iter()
        .all(|update| update.rate == 0.0));

    world.set_advance_time_for_test(true);
    let updates = world.world_clock_updates_for_test();
    assert_eq!(updates[0].rate, 0.5);
    assert_eq!(updates[1].rate, 0.0);
    world.tick(Vec::new());
    assert_ne!(world.world_clock_states_for_test(), before);

    world.tick_rate_mut().set_frozen(true);
    let frozen = world.world_clock_states_for_test();
    world.tick(Vec::new());
    assert_eq!(world.world_clock_states_for_test(), frozen);
    assert_eq!(world.world_clock_updates_for_test()[0].rate, 0.5);
}

#[test]
fn restore_join_and_save_snapshot_keep_every_clock_field() {
    let overworld = Java26WorldClockState {
        total_ticks: -17,
        partial_tick: 0.75,
        rate: 0.5,
        paused: false,
    };
    let end = Java26WorldClockState {
        total_ticks: i64::MAX,
        partial_tick: 0.25,
        rate: 3.0,
        paused: true,
    };
    let saved = clocks(&[
        ("minecraft:overworld", overworld),
        ("minecraft:the_end", end),
    ]);
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate.world_clocks = saved.clone();
    let mut world = ServerWorld::restore(datapack_config(), &candidate).unwrap();
    assert_eq!(world.world_clock_states_for_test(), vec![overworld, end]);
    assert_eq!(
        world
            .save_request(1, 1, RegionCompression::Zlib)
            .unwrap()
            .world_clocks,
        saved
    );

    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(8),
            profile: profile("restored-clock", 8),
        },
    )
    .unwrap();
    let joined = world.tick(receiver.drain());
    let update = joined
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(8),
                packet: ClientboundPlayPacket::SetTime(update),
            } if !update.clock_updates.is_empty() => Some(update),
            _ => None,
        })
        .unwrap();
    assert_eq!(update.clock_updates[0].total_ticks, overworld.total_ticks);
    assert_eq!(update.clock_updates[0].partial_tick, overworld.partial_tick);
    assert_eq!(update.clock_updates[0].rate, overworld.rate);
    assert_eq!(update.clock_updates[1].total_ticks, end.total_ticks);
    assert_eq!(update.clock_updates[1].partial_tick, end.partial_tick);
    assert_eq!(update.clock_updates[1].rate, 0.0);
}
