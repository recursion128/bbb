use super::*;
use bbb_protocol::packets::{
    BlockPos as ProtocolBlockPos, PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_X,
    PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y_ROT,
};
use bbb_world::BlockPos;

#[test]
fn player_position_updates_absolute_and_relative_pose() {
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    apply_player_position_update(
        &mut counters,
        &mut world,
        player_position_update(1, [10.0, 64.0, -5.0], [0.125, 0.0, 0.0], 90.0, 15.0, 0),
    );
    let pose = player_pose_from_local_player_pose(world.local_player_pose().unwrap());
    assert_eq!(pose.position, vec3(10.0, 64.0, -5.0));
    assert_eq!(pose.delta_movement, vec3(0.125, 0.0, 0.0));
    assert_eq!(pose.y_rot, 90.0);
    assert_eq!(pose.x_rot, 15.0);
    assert_eq!(pose.last_teleport_id, 1);
    assert_eq!(counters.player_position_packets, 1);

    apply_player_position_update(
        &mut counters,
        &mut world,
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
    let pose = player_pose_from_local_player_pose(world.local_player_pose().unwrap());
    assert_eq!(pose.position, vec3(11.5, -2.0, 7.0));
    assert_eq!(pose.delta_movement, vec3(0.375, 0.5, 0.75));
    assert_eq!(pose.y_rot, 110.0);
    assert_eq!(pose.x_rot, -90.0);
    assert_eq!(pose.last_teleport_id, 2);
    assert_eq!(counters.player_position_packets, 2);
    assert_eq!(world.local_player_pose().unwrap().last_teleport_id, 2);
}

#[test]
fn player_rotation_updates_pose_orientation() {
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_local_player_pose(local_player_pose_from_player_pose(PlayerPose {
        position: vec3(10.0, 64.0, -5.0),
        delta_movement: vec3(0.125, 0.0, 0.0),
        y_rot: 90.0,
        x_rot: 15.0,
        last_teleport_id: 7,
    }));

    apply_player_rotation_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::PlayerRotationUpdate {
            y_rot: 20.0,
            relative_y: true,
            x_rot: -120.0,
            relative_x: false,
        },
    );

    let pose = player_pose_from_local_player_pose(world.local_player_pose().unwrap());
    assert_eq!(pose.position, vec3(10.0, 64.0, -5.0));
    assert_eq!(pose.delta_movement, vec3(0.125, 0.0, 0.0));
    assert_eq!(pose.y_rot, 110.0);
    assert_eq!(pose.x_rot, -90.0);
    assert_eq!(pose.last_teleport_id, 7);
    assert_eq!(counters.player_rotation_packets, 1);
    assert_eq!(world.local_player_pose().unwrap().y_rot, 110.0);
}

#[test]
fn player_look_at_updates_world_pose_orientation() {
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.set_local_player_pose(local_player_pose_from_player_pose(PlayerPose {
        position: vec3(0.0, 64.0, 0.0),
        delta_movement: vec3(0.0, 0.0, 0.0),
        y_rot: 90.0,
        x_rot: 30.0,
        last_teleport_id: 7,
    }));

    apply_player_look_at_update(
        &mut counters,
        &mut world,
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

    let pose = player_pose_from_local_player_pose(world.local_player_pose().unwrap());
    assert_eq!(pose.position, vec3(0.0, 64.0, 0.0));
    assert_eq!(pose.delta_movement, vec3(0.0, 0.0, 0.0));
    assert!((pose.y_rot - 0.0).abs() < 0.001);
    assert!((pose.x_rot - 0.0).abs() < 0.001);
    assert_eq!(pose.last_teleport_id, 7);
    assert_eq!(counters.player_look_at_packets, 1);
    assert_eq!(
        world.local_player().last_look_at,
        Some(bbb_world::LocalPlayerLookAtState {
            from_anchor: bbb_protocol::packets::EntityAnchor::Eyes,
            position: bbb_protocol::packets::Vec3d {
                x: 0.0,
                y: 65.62,
                z: 10.0,
            },
            target_entity_id: None,
            to_anchor: None,
        })
    );
}

#[test]
fn player_health_updates_snapshot_counters() {
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    world.apply_player_health(bbb_protocol::packets::PlayerHealth {
        health: 7.5,
        food: 16,
        saturation: 2.0,
    });
    sync_local_player_counters(&mut counters, &world);

    assert_eq!(world.local_player().health.unwrap().health, 7.5);
    assert_eq!(world.local_player().health.unwrap().food, 16);
    assert_eq!(counters.player_health_packets, 1);
}

#[test]
fn player_experience_updates_snapshot_counters() {
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    world.apply_player_experience(bbb_protocol::packets::PlayerExperience {
        progress: 0.75,
        level: 8,
        total: 123,
    });
    sync_local_player_counters(&mut counters, &world);

    assert_eq!(world.local_player().experience.unwrap().level, 8);
    assert_eq!(world.local_player().experience.unwrap().total, 123);
    assert_eq!(counters.player_experience_packets, 1);
}

#[test]
fn held_slot_updates_snapshot_counters() {
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    assert!(world.apply_held_slot(bbb_protocol::packets::SetHeldSlot { slot: 5 }));
    sync_local_player_counters(&mut counters, &world);

    assert_eq!(counters.selected_hotbar_slot, 5);
    assert_eq!(counters.held_slot_packets, 1);
    assert_eq!(counters.held_slot_updates_applied, 1);
    assert_eq!(counters.held_slot_updates_ignored, 0);

    assert!(!world.apply_held_slot(bbb_protocol::packets::SetHeldSlot { slot: 99 }));
    sync_local_player_counters(&mut counters, &world);

    assert_eq!(counters.selected_hotbar_slot, 5);
    assert_eq!(counters.held_slot_packets, 2);
    assert_eq!(counters.held_slot_updates_applied, 1);
    assert_eq!(counters.held_slot_updates_ignored, 1);
}

#[test]
fn local_hotbar_selection_syncs_snapshot_counters_without_held_packet() {
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    assert!(world.set_local_selected_hotbar_slot(7));
    sync_local_player_counters(&mut counters, &world);

    assert_eq!(counters.selected_hotbar_slot, 7);
    assert_eq!(counters.held_slot_packets, 0);
    assert_eq!(counters.held_slot_updates_applied, 0);
    assert_eq!(counters.held_slot_updates_ignored, 0);
}

#[test]
fn player_abilities_spawn_distance_and_chat_update_snapshot_counters() {
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    world.apply_default_spawn_position(bbb_protocol::packets::SetDefaultSpawnPosition {
        dimension: "minecraft:overworld".to_string(),
        pos: ProtocolBlockPos {
            x: -5,
            y: 70,
            z: 12,
        },
        yaw: 90.0,
        pitch: -10.0,
    });
    world.apply_simulation_distance(bbb_protocol::packets::SetSimulationDistance { distance: 12 });
    sync_local_player_counters(&mut counters, &world);
    apply_system_chat_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::SystemChat {
            content: "Server restarting".to_string(),
            overlay: true,
        },
    );

    let local = world.local_player();
    assert_eq!(local.abilities.unwrap().can_fly, true);
    assert_eq!(
        local.default_spawn.as_ref().map(|spawn| spawn.pos),
        Some(BlockPos {
            x: -5,
            y: 70,
            z: 12,
        })
    );
    assert_eq!(
        local.default_spawn.as_ref().map(|spawn| spawn.yaw),
        Some(90.0)
    );
    assert_eq!(local.simulation_distance, Some(12));
    let system_chat = world.system_chat().unwrap();
    assert_eq!(system_chat.content, "Server restarting");
    assert!(system_chat.overlay);
    assert_eq!(counters.player_abilities_packets, 1);
    assert_eq!(counters.default_spawn_position_packets, 1);
    assert_eq!(counters.simulation_distance_packets, 1);
    assert_eq!(counters.system_chat_packets, 1);
}

#[test]
fn hud_text_and_ticking_updates_snapshot_counters() {
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    apply_titles_animation_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::SetTitlesAnimation {
            fade_in: 5,
            stay: -1,
            fade_out: 15,
        },
    );
    assert_eq!(world.title().fade_in, 5);
    assert_eq!(world.title().stay, 70);
    assert_eq!(world.title().fade_out, 15);
    assert_eq!(world.title().title_time, 0);

    apply_title_text_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::SetTitleText {
            content: "Quest complete".to_string(),
        },
    );
    apply_subtitle_text_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::SetSubtitleText {
            content: "Return to camp".to_string(),
        },
    );
    apply_action_bar_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::SetActionBarText {
            content: "+12 XP".to_string(),
        },
    );
    apply_titles_animation_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::SetTitlesAnimation {
            fade_in: -1,
            stay: 40,
            fade_out: -1,
        },
    );
    world.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 0.25,
        frozen: true,
    });
    world.apply_ticking_step(bbb_protocol::packets::TickingStep { tick_steps: 7 });
    sync_ticking_counters(&mut counters, &world);

    assert_eq!(world.title().title.as_deref(), Some("Quest complete"));
    assert_eq!(world.title().subtitle.as_deref(), Some("Return to camp"));
    assert_eq!(world.title().fade_in, 5);
    assert_eq!(world.title().stay, 40);
    assert_eq!(world.title().fade_out, 15);
    assert_eq!(world.title().title_time, 60);
    let action_bar = world.action_bar().unwrap();
    assert_eq!(action_bar.content, "+12 XP");
    assert_eq!(action_bar.display_ticks, 60);
    assert_eq!(
        world.ticking(),
        bbb_world::WorldTickingState {
            tick_rate: 1.0,
            frozen: true,
            frozen_ticks_to_run: 7,
        }
    );
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
    let mut world = WorldStore::new();

    apply_titles_animation_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::SetTitlesAnimation {
            fade_in: 5,
            stay: 40,
            fade_out: 15,
        },
    );
    apply_title_text_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::SetTitleText {
            content: "Quest complete".to_string(),
        },
    );
    apply_subtitle_text_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::SetSubtitleText {
            content: "Return to camp".to_string(),
        },
    );

    apply_clear_titles_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::ClearTitles { reset_times: false },
    );
    assert_eq!(world.title().title.as_deref(), None);
    assert_eq!(world.title().subtitle.as_deref(), None);
    assert_eq!(world.title().title_time, 0);
    assert_eq!(world.title().fade_in, 5);
    assert_eq!(world.title().stay, 40);
    assert_eq!(world.title().fade_out, 15);

    apply_titles_animation_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::SetTitlesAnimation {
            fade_in: 6,
            stay: 50,
            fade_out: 16,
        },
    );
    apply_title_text_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::SetTitleText {
            content: "Again".to_string(),
        },
    );
    apply_subtitle_text_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::SetSubtitleText {
            content: "Reset timers".to_string(),
        },
    );

    apply_clear_titles_update(
        &mut counters,
        &mut world,
        bbb_protocol::packets::ClearTitles { reset_times: true },
    );
    assert_eq!(world.title().title.as_deref(), None);
    assert_eq!(world.title().subtitle.as_deref(), None);
    assert_eq!(world.title().fade_in, 10);
    assert_eq!(world.title().stay, 70);
    assert_eq!(world.title().fade_out, 20);
    assert_eq!(world.title().title_time, 0);
    assert_eq!(counters.clear_titles_packets, 2);
    assert_eq!(counters.title_text_packets, 2);
    assert_eq!(counters.subtitle_text_packets, 2);
    assert_eq!(counters.titles_animation_packets, 2);
}

#[test]
fn set_camera_updates_player_camera_and_ignores_unknown_entity() {
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(9));

    assert!(!world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 123 }));
    sync_local_player_counters(&mut counters, &world);
    assert_eq!(
        world.local_player().camera,
        bbb_world::CameraState {
            entity_id: None,
            follows_player: true,
            entity_known: true,
        }
    );
    assert_eq!(counters.set_camera_packets, 1);
    assert_eq!(counters.set_camera_updates_applied, 0);
    assert_eq!(counters.set_camera_updates_ignored, 1);

    assert!(world.apply_set_camera(bbb_protocol::packets::SetCamera { camera_id: 9 }));
    sync_local_player_counters(&mut counters, &world);

    assert_eq!(
        world.local_player().camera,
        bbb_world::CameraState {
            entity_id: Some(9),
            follows_player: true,
            entity_known: true,
        }
    );
    assert_eq!(counters.set_camera_packets, 2);
    assert_eq!(counters.set_camera_updates_applied, 1);
    assert_eq!(counters.set_camera_updates_ignored, 1);
}

fn protocol_play_login(player_id: i32) -> bbb_protocol::packets::PlayLogin {
    bbb_protocol::packets::PlayLogin {
        player_id,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: bbb_protocol::packets::CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 12345,
            game_type: 1,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    }
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
