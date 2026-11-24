//! Vibration payload and player source values.
use super::{VibrationData, VibrationEntity, VibrationInfo};
use bbb_protocol::packets::Vec3d;
use uuid::Uuid;
fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

#[test]
fn the_listener_record_round_trips_through_nbt() {
    let mut data = VibrationData::default();
    data.selection_strategy_mut().add_candidate(
        VibrationInfo {
            game_event: "minecraft:sculk_sensor_tendrils_clicking",
            distance: 4.25,
            pos: Vec3d {
                x: 1.5,
                y: 64.5,
                z: -2.5,
            },
            source: Some(uuid(7)),
            projectile_owner: Some(uuid(9)),
        },
        41,
    );
    data.set_current_vibration(Some(VibrationInfo {
        game_event: "minecraft:block_place",
        distance: 1.0,
        pos: Vec3d {
            x: 0.5,
            y: 64.5,
            z: 0.5,
        },
        source: None,
        projectile_owner: None,
    }));
    data.set_travel_time_in_ticks(3);
    data.set_reload_vibration_particle(false);

    let saved = data.save();
    let loaded = VibrationData::load(&saved).expect("the record decodes");
    assert_eq!(loaded.current_vibration(), data.current_vibration());
    assert_eq!(loaded.travel_time_in_ticks(), data.travel_time_in_ticks());
    assert_eq!(loaded.selection_strategy(), data.selection_strategy());
}

#[test]
fn each_player_arm_resolves_the_shriek_player() {
    let base = VibrationEntity {
        uuid: uuid(1),
        is_player: false,
        is_projectile: false,
        controlling_passenger_player: None,
        projectile_owner: None,
        item_owner: None,
    };
    let online_player = |candidate| candidate == uuid(3) || candidate == uuid(4);
    assert_eq!(base.shriek_player_with(online_player), None);
    assert_eq!(
        VibrationEntity {
            is_player: true,
            ..base
        }
        .shriek_player_with(online_player),
        Some(uuid(1))
    );
    assert_eq!(
        VibrationEntity {
            controlling_passenger_player: Some(uuid(2)),
            ..base
        }
        .shriek_player_with(online_player),
        Some(uuid(2))
    );
    assert_eq!(
        VibrationEntity {
            is_projectile: true,
            projectile_owner: Some(uuid(3)),
            ..base
        }
        .shriek_player_with(online_player),
        Some(uuid(3))
    );
    assert_eq!(
        VibrationEntity {
            is_projectile: true,
            projectile_owner: Some(uuid(99)),
            ..base
        }
        .shriek_player_with(online_player),
        None
    );
    assert_eq!(
        VibrationEntity {
            item_owner: Some(uuid(4)),
            ..base
        }
        .shriek_player_with(online_player),
        Some(uuid(4))
    );
    assert_eq!(
        VibrationEntity {
            item_owner: Some(uuid(99)),
            ..base
        }
        .shriek_player_with(online_player),
        None
    );
}

#[test]
fn a_user_that_requires_adjacent_chunks_holds_the_arrival() {
    let info = VibrationInfo {
        game_event: "minecraft:step",
        distance: 0.0,
        pos: Vec3d::default(),
        source: None,
        projectile_owner: None,
    };
    let mut data = VibrationData::default();
    data.set_current_vibration(Some(info));
    let blocked = data.tick(10, Some(Vec3d::default()), false);
    assert!(blocked.arrival.is_none());
    assert_eq!(data.current_vibration(), Some(&info));
    let ready = data.tick(11, Some(Vec3d::default()), true);
    assert_eq!(ready.arrival.unwrap().info, info);
    assert!(data.current_vibration().is_none());
}
