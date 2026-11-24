use super::*;

#[test]
fn apply_play_packet_applies_world_owned_packets_and_returns_none() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);

    let leftover = store.apply_play_packet(
        PlayClientbound::SetTime(PlayTime {
            game_time: 24000,
            clock_updates: Vec::new(),
        }),
        &mut random,
        &mut NoPlayApplyEffects,
    );

    assert!(leftover.is_none());
    assert_eq!(store.world_time().map(|time| time.game_time), Some(24000));
}

#[test]
fn apply_play_packet_returns_connection_owned_packets_unapplied() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);

    let leftover = store.apply_play_packet(
        PlayClientbound::KeepAlive { id: 7 },
        &mut random,
        &mut NoPlayApplyEffects,
    );

    assert!(matches!(
        leftover,
        Some(PlayClientbound::KeepAlive { id: 7 })
    ));
}

#[test]
fn apply_play_packet_applies_and_returns_player_position() {
    let mut store = WorldStore::new();
    let mut random = LevelEventSoundRandomState::with_seed(0);
    let update = bbb_protocol::packets::PlayerPositionUpdate {
        id: 3,
        position: Vec3d {
            x: 8.5,
            y: 65.0,
            z: -4.5,
        },
        delta_movement: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 90.0,
        x_rot: 0.0,
        relatives_mask: 0,
    };

    let leftover = store.apply_play_packet(
        PlayClientbound::PlayerPosition(update),
        &mut random,
        &mut NoPlayApplyEffects,
    );

    assert!(matches!(leftover, Some(PlayClientbound::PlayerPosition(_))));
    let pose = store.local_player_pose().expect("player pose");
    assert_eq!(pose.position.x, 8.5);
    assert_eq!(pose.position.z, -4.5);
}
