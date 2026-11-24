//! Custom boss-event state across world restore, sessions, and save projection.

use super::*;

use bbb_protocol::packets::{BossBarColor, BossBarOverlay, BossEvent, BossEventOperation};
use bbb_server_persistence::{Java26CustomBossEventPacked, Java26CustomBossEventsPacked};

const ID: &str = "minecraft:test";

fn restored_boss_events(player: Uuid) -> Java26CustomBossEventsPacked {
    Java26CustomBossEventsPacked(BTreeMap::from([(
        ID.to_string(),
        Java26CustomBossEventPacked {
            name: bbb_protocol::component_styled_literal_nbt(
                "Saved boss",
                &bbb_protocol::ComponentStyle {
                    bold: Some(true),
                    italic: Some(true),
                    ..Default::default()
                },
            )
            .unwrap(),
            visible: true,
            value: 25,
            max: 40,
            color: "purple".to_string(),
            overlay: "notched_12".to_string(),
            darken_screen: true,
            play_boss_music: true,
            create_world_fog: true,
            players: vec![player, player],
        },
    )]))
}

fn boss_event(tick: &TickResult, connection_id: u64) -> &BossEvent {
    tick.events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: actual,
                packet: ClientboundPlayPacket::BossEvent(event),
            } if *actual == ConnectionId(connection_id) => Some(event),
            _ => None,
        })
        .expect("the custom boss event reaches its member")
}

#[test]
fn restored_bossbar_skips_bad_names_and_keeps_membership_styles_and_save_state() {
    let alice = Uuid::from_u128(1);
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate.custom_boss_events = restored_boss_events(alice);
    let saved_name = candidate.custom_boss_events.0[ID].name.clone();
    candidate.custom_boss_events.0.insert(
        "minecraft:bad".into(),
        Java26CustomBossEventPacked {
            name: bbb_protocol::NbtSummaryValue::Int(7),
            ..Default::default()
        },
    );
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate)
        .expect("the boss event restores");

    let snapshot = world.command_boss_bars_snapshot_for_test();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].name.component(), &saved_name);
    assert_eq!(snapshot[0].players, [alice]);
    assert!(snapshot[0].online_players.is_empty());

    let (sender, receiver) = mailbox();
    let (joined, _) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "alice", 1);
    let first_runtime_id = boss_event(&joined, 1).id;
    assert!(matches!(
        &boss_event(&joined, 1).operation,
        BossEventOperation::Add {
            progress,
            color: BossBarColor::Purple,
            overlay: BossBarOverlay::Notched12,
            flags,
            ..
        } if *progress == 0.625
            && flags.darken_screen
            && flags.play_music
            && flags.create_world_fog
    ));
    assert_eq!(
        world.command_boss_bars_snapshot_for_test()[0].online_players,
        [NameAndId {
            id: alice,
            name: "alice".into(),
        }]
    );

    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    let state = &world.command_boss_bars_snapshot_for_test()[0];
    assert_eq!(state.players, [alice]);
    assert!(state.online_players.is_empty());

    let (rejoined, _) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "alice", 1);
    assert_eq!(boss_event(&rejoined, 1).id, first_runtime_id);

    let request = world
        .save_request(1, 1, bbb_server_persistence::RegionCompression::Zlib)
        .expect("the boss event reaches the save request");
    assert_eq!(request.custom_boss_events.0.len(), 1);
    let saved = &request.custom_boss_events.0[ID];
    assert_eq!(saved.name, saved_name);
    assert_eq!(saved.players, [alice]);
    assert_eq!(saved.value, 25);
    assert_eq!(saved.max, 40);

    let mut reloaded_candidate = restore_candidate(0, None, Vec::new());
    reloaded_candidate.custom_boss_events = request.custom_boss_events;
    let mut reloaded = ServerWorld::restore(ServerWorldConfig::default(), &reloaded_candidate)
        .expect("the saved boss event reloads");
    let (sender, receiver) = mailbox();
    let (joined, _) = join_loaded_ticks(&sender, &receiver, &mut reloaded, 2, "alice", 1);
    assert_ne!(boss_event(&joined, 2).id, first_runtime_id);
}
