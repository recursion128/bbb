use super::*;
use bbb_protocol::packets::{
    BlockPos as ProtocolBlockPos, PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_X,
    PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y_ROT,
};

#[test]
fn player_position_updates_absolute_and_relative_pose() {
    let mut counters = NetCounters::default();
    apply_player_position_update(
        &mut counters,
        player_position_update(1, [10.0, 64.0, -5.0], [0.125, 0.0, 0.0], 90.0, 15.0, 0),
    );
    let pose = counters.player_pose.unwrap();
    assert_eq!(pose.position, vec3(10.0, 64.0, -5.0));
    assert_eq!(pose.delta_movement, vec3(0.125, 0.0, 0.0));
    assert_eq!(pose.y_rot, 90.0);
    assert_eq!(pose.x_rot, 15.0);
    assert_eq!(pose.last_teleport_id, 1);
    assert_eq!(counters.player_position_packets, 1);

    apply_player_position_update(
        &mut counters,
        player_position_update(
            2,
            [1.5, -2.0, 7.0],
            [0.25, 0.5, 0.75],
            20.0,
            -120.0,
            PLAYER_RELATIVE_X
                | PLAYER_RELATIVE_Y_ROT
                | PLAYER_RELATIVE_X_ROT
                | PLAYER_RELATIVE_DELTA_X,
        ),
    );
    let pose = counters.player_pose.unwrap();
    assert_eq!(pose.position, vec3(11.5, -2.0, 7.0));
    assert_eq!(pose.delta_movement, vec3(0.375, 0.5, 0.75));
    assert_eq!(pose.y_rot, 110.0);
    assert_eq!(pose.x_rot, -90.0);
    assert_eq!(pose.last_teleport_id, 2);
    assert_eq!(counters.player_position_packets, 2);
}

#[test]
fn player_rotation_updates_pose_orientation() {
    let mut counters = NetCounters {
        player_pose: Some(PlayerPose {
            position: vec3(10.0, 64.0, -5.0),
            delta_movement: vec3(0.125, 0.0, 0.0),
            y_rot: 90.0,
            x_rot: 15.0,
            last_teleport_id: 7,
        }),
        ..NetCounters::default()
    };

    apply_player_rotation_update(
        &mut counters,
        bbb_protocol::packets::PlayerRotationUpdate {
            y_rot: 20.0,
            relative_y: true,
            x_rot: -120.0,
            relative_x: false,
        },
    );

    let pose = counters.player_pose.unwrap();
    assert_eq!(pose.position, vec3(10.0, 64.0, -5.0));
    assert_eq!(pose.delta_movement, vec3(0.125, 0.0, 0.0));
    assert_eq!(pose.y_rot, 110.0);
    assert_eq!(pose.x_rot, -90.0);
    assert_eq!(pose.last_teleport_id, 7);
    assert_eq!(counters.player_rotation_packets, 1);
}

#[test]
fn player_look_at_updates_snapshot_and_pose_orientation() {
    let mut counters = NetCounters {
        player_pose: Some(PlayerPose {
            position: vec3(0.0, 64.0, 0.0),
            delta_movement: vec3(0.0, 0.0, 0.0),
            y_rot: 90.0,
            x_rot: 30.0,
            last_teleport_id: 7,
        }),
        ..NetCounters::default()
    };

    apply_player_look_at_update(
        &mut counters,
        bbb_protocol::packets::PlayerLookAt {
            from_anchor: bbb_protocol::packets::EntityAnchor::Eyes,
            position: bbb_protocol::packets::Vec3d {
                x: 0.0,
                y: 65.62,
                z: 10.0,
            },
            target: None,
        },
    );

    let pose = counters.player_pose.unwrap();
    assert_eq!(pose.position, vec3(0.0, 64.0, 0.0));
    assert_eq!(pose.delta_movement, vec3(0.0, 0.0, 0.0));
    assert!((pose.y_rot - 0.0).abs() < 0.001);
    assert!((pose.x_rot - 0.0).abs() < 0.001);
    assert_eq!(pose.last_teleport_id, 7);
    assert_eq!(counters.player_look_at_packets, 1);
    assert_eq!(
        counters.last_player_look_at,
        Some(PlayerLookAtState {
            from_anchor: "eyes".to_string(),
            position: vec3(0.0, 65.62, 10.0),
            target_entity_id: None,
            to_anchor: None,
        })
    );
}

#[test]
fn player_health_updates_snapshot_counters() {
    let mut counters = NetCounters::default();

    apply_player_health_update(
        &mut counters,
        bbb_protocol::packets::PlayerHealth {
            health: 7.5,
            food: 16,
            saturation: 2.0,
        },
    );

    assert_eq!(
        counters.player_health,
        Some(PlayerHealth {
            health: 7.5,
            food: 16,
            saturation: 2.0,
        })
    );
    assert_eq!(counters.player_health_packets, 1);
}

#[test]
fn player_experience_updates_snapshot_counters() {
    let mut counters = NetCounters::default();

    apply_player_experience_update(
        &mut counters,
        bbb_protocol::packets::PlayerExperience {
            progress: 0.75,
            level: 8,
            total: 123,
        },
    );

    assert_eq!(
        counters.player_experience,
        Some(PlayerExperience {
            progress: 0.75,
            level: 8,
            total: 123,
        })
    );
    assert_eq!(counters.player_experience_packets, 1);
}

#[test]
fn held_slot_updates_snapshot_counters() {
    let mut counters = NetCounters::default();

    apply_held_slot_update(
        &mut counters,
        bbb_protocol::packets::SetHeldSlot { slot: 5 },
    );

    assert_eq!(counters.selected_hotbar_slot, 5);
    assert_eq!(counters.held_slot_packets, 1);

    apply_held_slot_update(
        &mut counters,
        bbb_protocol::packets::SetHeldSlot { slot: 99 },
    );

    assert_eq!(counters.selected_hotbar_slot, 5);
    assert_eq!(counters.held_slot_packets, 2);
}

#[test]
fn player_abilities_spawn_distance_and_chat_update_snapshot_counters() {
    let mut counters = NetCounters::default();

    apply_player_abilities_update(
        &mut counters,
        bbb_protocol::packets::PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        },
    );
    apply_default_spawn_update(
        &mut counters,
        bbb_protocol::packets::SetDefaultSpawnPosition {
            dimension: "minecraft:overworld".to_string(),
            pos: ProtocolBlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            yaw: 90.0,
            pitch: -10.0,
        },
    );
    apply_simulation_distance_update(
        &mut counters,
        bbb_protocol::packets::SetSimulationDistance { distance: 12 },
    );
    apply_system_chat_update(
        &mut counters,
        bbb_protocol::packets::SystemChat {
            content: "Server restarting".to_string(),
            overlay: true,
        },
    );

    assert_eq!(
        counters.player_abilities,
        Some(PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        })
    );
    assert_eq!(
        counters.default_spawn,
        Some(DefaultSpawn {
            dimension: "minecraft:overworld".to_string(),
            pos: BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            yaw: 90.0,
            pitch: -10.0,
        })
    );
    assert_eq!(counters.simulation_distance, Some(12));
    assert_eq!(
        counters.last_system_chat,
        Some(SystemChatLine {
            content: "Server restarting".to_string(),
            overlay: true,
        })
    );
    assert_eq!(counters.player_abilities_packets, 1);
    assert_eq!(counters.default_spawn_position_packets, 1);
    assert_eq!(counters.simulation_distance_packets, 1);
    assert_eq!(counters.system_chat_packets, 1);
}

#[test]
fn hud_text_and_ticking_updates_snapshot_counters() {
    let mut counters = NetCounters::default();

    apply_titles_animation_update(
        &mut counters,
        bbb_protocol::packets::SetTitlesAnimation {
            fade_in: 5,
            stay: -1,
            fade_out: 15,
        },
    );
    assert_eq!(counters.title.fade_in, 5);
    assert_eq!(counters.title.stay, 70);
    assert_eq!(counters.title.fade_out, 15);
    assert_eq!(counters.title.title_time, 0);

    apply_title_text_update(
        &mut counters,
        bbb_protocol::packets::SetTitleText {
            content: "Quest complete".to_string(),
        },
    );
    apply_subtitle_text_update(
        &mut counters,
        bbb_protocol::packets::SetSubtitleText {
            content: "Return to camp".to_string(),
        },
    );
    apply_action_bar_update(
        &mut counters,
        bbb_protocol::packets::SetActionBarText {
            content: "+12 XP".to_string(),
        },
    );
    apply_titles_animation_update(
        &mut counters,
        bbb_protocol::packets::SetTitlesAnimation {
            fade_in: -1,
            stay: 40,
            fade_out: -1,
        },
    );
    apply_ticking_state_update(
        &mut counters,
        bbb_protocol::packets::TickingState {
            tick_rate: 0.25,
            frozen: true,
        },
    );
    apply_ticking_step_update(
        &mut counters,
        bbb_protocol::packets::TickingStep { tick_steps: 7 },
    );

    assert_eq!(counters.title.title.as_deref(), Some("Quest complete"));
    assert_eq!(counters.title.subtitle.as_deref(), Some("Return to camp"));
    assert_eq!(counters.title.fade_in, 5);
    assert_eq!(counters.title.stay, 40);
    assert_eq!(counters.title.fade_out, 15);
    assert_eq!(counters.title.title_time, 60);
    assert_eq!(
        counters.last_action_bar,
        Some(ActionBarText {
            content: "+12 XP".to_string(),
            display_ticks: 60,
        })
    );
    assert_eq!(counters.ticking.tick_rate, 1.0);
    assert!(counters.ticking.frozen);
    assert_eq!(counters.ticking.frozen_ticks_to_run, 7);
    assert_eq!(counters.titles_animation_packets, 2);
    assert_eq!(counters.title_text_packets, 1);
    assert_eq!(counters.subtitle_text_packets, 1);
    assert_eq!(counters.action_bar_packets, 1);
    assert_eq!(counters.ticking_state_packets, 1);
    assert_eq!(counters.ticking_step_packets, 1);
}

#[test]
fn clear_titles_resets_visible_title_and_optionally_times() {
    let mut counters = NetCounters::default();

    apply_titles_animation_update(
        &mut counters,
        bbb_protocol::packets::SetTitlesAnimation {
            fade_in: 5,
            stay: 40,
            fade_out: 15,
        },
    );
    apply_title_text_update(
        &mut counters,
        bbb_protocol::packets::SetTitleText {
            content: "Quest complete".to_string(),
        },
    );
    apply_subtitle_text_update(
        &mut counters,
        bbb_protocol::packets::SetSubtitleText {
            content: "Return to camp".to_string(),
        },
    );

    apply_clear_titles_update(
        &mut counters,
        bbb_protocol::packets::ClearTitles { reset_times: false },
    );
    assert_eq!(counters.title.title, None);
    assert_eq!(counters.title.subtitle, None);
    assert_eq!(counters.title.title_time, 0);
    assert_eq!(counters.title.fade_in, 5);
    assert_eq!(counters.title.stay, 40);
    assert_eq!(counters.title.fade_out, 15);

    apply_titles_animation_update(
        &mut counters,
        bbb_protocol::packets::SetTitlesAnimation {
            fade_in: 6,
            stay: 50,
            fade_out: 16,
        },
    );
    apply_title_text_update(
        &mut counters,
        bbb_protocol::packets::SetTitleText {
            content: "Again".to_string(),
        },
    );
    apply_subtitle_text_update(
        &mut counters,
        bbb_protocol::packets::SetSubtitleText {
            content: "Reset timers".to_string(),
        },
    );

    apply_clear_titles_update(
        &mut counters,
        bbb_protocol::packets::ClearTitles { reset_times: true },
    );
    assert_eq!(counters.title, bbb_control::TitleState::default());
    assert_eq!(counters.clear_titles_packets, 2);
    assert_eq!(counters.title_text_packets, 2);
    assert_eq!(counters.subtitle_text_packets, 2);
    assert_eq!(counters.titles_animation_packets, 2);
}

#[test]
fn set_camera_updates_player_camera_and_ignores_unknown_entity() {
    let mut counters = NetCounters {
        player_entity_id: Some(9),
        camera: CameraState {
            entity_id: Some(42),
            follows_player: false,
            entity_known: true,
        },
        ..NetCounters::default()
    };
    let world = WorldStore::new();

    apply_set_camera_update(
        &mut counters,
        &world,
        bbb_protocol::packets::SetCamera { camera_id: 123 },
    );
    assert_eq!(
        counters.camera,
        CameraState {
            entity_id: Some(42),
            follows_player: false,
            entity_known: true,
        }
    );

    apply_set_camera_update(
        &mut counters,
        &world,
        bbb_protocol::packets::SetCamera { camera_id: 9 },
    );

    assert_eq!(
        counters.camera,
        CameraState {
            entity_id: Some(9),
            follows_player: true,
            entity_known: true,
        }
    );
    assert_eq!(counters.set_camera_packets, 2);
}

fn player_position_update(
    id: i32,
    position: [f64; 3],
    delta_movement: [f64; 3],
    y_rot: f32,
    x_rot: f32,
    relatives_mask: i32,
) -> bbb_protocol::packets::PlayerPositionUpdate {
    bbb_protocol::packets::PlayerPositionUpdate {
        id,
        position: bbb_protocol::packets::Vec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        delta_movement: bbb_protocol::packets::Vec3d {
            x: delta_movement[0],
            y: delta_movement[1],
            z: delta_movement[2],
        },
        y_rot,
        x_rot,
        relatives_mask,
    }
}

fn vec3(x: f64, y: f64, z: f64) -> NetVec3 {
    NetVec3 { x, y, z }
}
