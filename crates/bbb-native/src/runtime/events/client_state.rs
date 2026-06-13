use bbb_control::{
    ActionBarText, CameraState, DefaultSpawn, NetCounters, NetVec3, PlayerAbilities,
    PlayerExperience, PlayerHealth, PlayerPose, SystemChatLine,
};
use bbb_protocol::packets::PlayerPositionState;
use bbb_world::{BlockPos, WorldStore};

pub(super) fn apply_player_abilities_update(
    counters: &mut NetCounters,
    abilities: bbb_protocol::packets::PlayerAbilities,
) {
    counters.player_abilities = Some(PlayerAbilities {
        invulnerable: abilities.invulnerable,
        flying: abilities.flying,
        can_fly: abilities.can_fly,
        instabuild: abilities.instabuild,
        flying_speed: abilities.flying_speed,
        walking_speed: abilities.walking_speed,
    });
    counters.player_abilities_packets += 1;
}

pub(super) fn apply_default_spawn_update(
    counters: &mut NetCounters,
    spawn: bbb_protocol::packets::SetDefaultSpawnPosition,
) {
    counters.default_spawn = Some(DefaultSpawn {
        dimension: spawn.dimension,
        pos: BlockPos {
            x: spawn.pos.x,
            y: spawn.pos.y,
            z: spawn.pos.z,
        },
        yaw: spawn.yaw,
        pitch: spawn.pitch,
    });
    counters.default_spawn_position_packets += 1;
}

pub(super) fn apply_simulation_distance_update(
    counters: &mut NetCounters,
    distance: bbb_protocol::packets::SetSimulationDistance,
) {
    counters.simulation_distance = Some(distance.distance);
    counters.simulation_distance_packets += 1;
}

pub(super) fn apply_system_chat_update(
    counters: &mut NetCounters,
    chat: bbb_protocol::packets::SystemChat,
) {
    counters.last_system_chat = Some(SystemChatLine {
        content: chat.content,
        overlay: chat.overlay,
    });
    counters.system_chat_packets += 1;
}

pub(super) fn apply_action_bar_update(
    counters: &mut NetCounters,
    text: bbb_protocol::packets::SetActionBarText,
) {
    counters.last_action_bar = Some(ActionBarText {
        content: text.content,
        display_ticks: 60,
    });
    counters.action_bar_packets += 1;
}

pub(super) fn apply_title_text_update(
    counters: &mut NetCounters,
    text: bbb_protocol::packets::SetTitleText,
) {
    counters.title.title = Some(text.content);
    counters.title.title_time = title_total_ticks(&counters.title);
    counters.title_text_packets += 1;
}

pub(super) fn apply_subtitle_text_update(
    counters: &mut NetCounters,
    text: bbb_protocol::packets::SetSubtitleText,
) {
    counters.title.subtitle = Some(text.content);
    counters.subtitle_text_packets += 1;
}

pub(super) fn apply_titles_animation_update(
    counters: &mut NetCounters,
    animation: bbb_protocol::packets::SetTitlesAnimation,
) {
    if animation.fade_in >= 0 {
        counters.title.fade_in = animation.fade_in;
    }
    if animation.stay >= 0 {
        counters.title.stay = animation.stay;
    }
    if animation.fade_out >= 0 {
        counters.title.fade_out = animation.fade_out;
    }
    if counters.title.title_time > 0 {
        counters.title.title_time = title_total_ticks(&counters.title);
    }
    counters.titles_animation_packets += 1;
}

fn title_total_ticks(title: &bbb_control::TitleState) -> i32 {
    title
        .fade_in
        .saturating_add(title.stay)
        .saturating_add(title.fade_out)
}

pub(super) fn apply_ticking_state_update(
    counters: &mut NetCounters,
    ticking: bbb_protocol::packets::TickingState,
) {
    counters.ticking.tick_rate = ticking.clamped_tick_rate();
    counters.ticking.frozen = ticking.frozen;
    counters.ticking_state_packets += 1;
}

pub(super) fn apply_ticking_step_update(
    counters: &mut NetCounters,
    step: bbb_protocol::packets::TickingStep,
) {
    counters.ticking.frozen_ticks_to_run = step.tick_steps;
    counters.ticking_step_packets += 1;
}

pub(super) fn apply_set_camera_update(
    counters: &mut NetCounters,
    world: &WorldStore,
    camera: bbb_protocol::packets::SetCamera,
) {
    counters.set_camera_packets += 1;
    let follows_player = counters.player_entity_id == Some(camera.camera_id);
    if follows_player || world.probe_entity(camera.camera_id).is_some() {
        counters.camera = CameraState {
            entity_id: Some(camera.camera_id),
            follows_player,
            entity_known: true,
        };
    }
}

pub(super) fn apply_player_health_update(
    counters: &mut NetCounters,
    health: bbb_protocol::packets::PlayerHealth,
) {
    counters.player_health = Some(PlayerHealth {
        health: health.health,
        food: health.food,
        saturation: health.saturation,
    });
    counters.player_health_packets += 1;
}

pub(super) fn apply_player_experience_update(
    counters: &mut NetCounters,
    experience: bbb_protocol::packets::PlayerExperience,
) {
    counters.player_experience = Some(PlayerExperience {
        progress: experience.progress,
        level: experience.level,
        total: experience.total,
    });
    counters.player_experience_packets += 1;
}

pub(super) fn apply_held_slot_update(
    counters: &mut NetCounters,
    slot: bbb_protocol::packets::SetHeldSlot,
) {
    if (0..=8).contains(&slot.slot) {
        counters.selected_hotbar_slot = slot.slot as u8;
    }
    counters.held_slot_packets += 1;
}

pub(super) fn apply_player_position_update(
    counters: &mut NetCounters,
    update: bbb_protocol::packets::PlayerPositionUpdate,
) {
    let current = counters
        .player_pose
        .map(player_position_state_from_pose)
        .unwrap_or_default();
    let state = update.apply_to_state(current);

    counters.player_pose = Some(PlayerPose {
        position: net_vec3_from_protocol(state.position),
        delta_movement: net_vec3_from_protocol(state.delta_movement),
        y_rot: state.y_rot,
        x_rot: state.x_rot,
        last_teleport_id: update.id,
    });
    counters.player_position_packets += 1;
}

pub(super) fn apply_player_rotation_update(
    counters: &mut NetCounters,
    update: bbb_protocol::packets::PlayerRotationUpdate,
) {
    let current = counters
        .player_pose
        .map(player_position_state_from_pose)
        .unwrap_or_default();
    let state = update.apply_to_state(current);
    let last_teleport_id = counters
        .player_pose
        .map(|pose| pose.last_teleport_id)
        .unwrap_or_default();

    counters.player_pose = Some(PlayerPose {
        position: net_vec3_from_protocol(state.position),
        delta_movement: net_vec3_from_protocol(state.delta_movement),
        y_rot: state.y_rot,
        x_rot: state.x_rot,
        last_teleport_id,
    });
    counters.player_rotation_packets += 1;
}

pub(crate) fn player_position_state_from_pose(player: PlayerPose) -> PlayerPositionState {
    PlayerPositionState {
        position: protocol_vec3_from_net(player.position),
        delta_movement: protocol_vec3_from_net(player.delta_movement),
        y_rot: player.y_rot,
        x_rot: player.x_rot,
    }
}

fn protocol_vec3_from_net(vec: NetVec3) -> bbb_protocol::packets::Vec3d {
    bbb_protocol::packets::Vec3d {
        x: vec.x,
        y: vec.y,
        z: vec.z,
    }
}

fn net_vec3_from_protocol(vec: bbb_protocol::packets::Vec3d) -> NetVec3 {
    NetVec3 {
        x: vec.x,
        y: vec.y,
        z: vec.z,
    }
}

#[cfg(test)]
mod tests {
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
}
