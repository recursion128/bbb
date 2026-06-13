use bbb_control::{
    ActionBarText, CameraState, DefaultSpawn, NetCounters, NetVec3, PlayerAbilities,
    PlayerExperience, PlayerHealth, PlayerPose, SystemChatLine,
};
use bbb_net::{NetCommand, NetEvent};
use bbb_protocol::packets::PlayerPositionState;
use bbb_world::{BlockPos, ChunkPos, WorldStore};
use tokio::sync::mpsc;

use crate::input::queue_vehicle_move_command;

pub(super) fn drain_net_events(
    rx: &mut mpsc::Receiver<NetEvent>,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) -> usize {
    let mut drained = 0;
    while drained < 4096 {
        let event = match rx.try_recv() {
            Ok(event) => event,
            Err(mpsc::error::TryRecvError::Empty) => break,
            Err(mpsc::error::TryRecvError::Disconnected) => {
                counters.connected = false;
                break;
            }
        };
        drained += 1;

        match event {
            NetEvent::Connected => {
                counters.connected = true;
                counters.last_error = None;
            }
            NetEvent::Disconnected { reason } => {
                counters.connected = false;
                counters.last_error = reason;
            }
            NetEvent::StateChanged { state } => {
                counters.state = Some(format!("{state:?}"));
            }
            NetEvent::CompressionSet { threshold } => {
                counters.compression_threshold = Some(threshold);
            }
            NetEvent::PacketSeen { .. } => {
                counters.packets_seen += 1;
            }
            NetEvent::PlayerInfoUpdate(update) => {
                counters.player_info_update_packets += 1;
                world.apply_player_info_update(update);
            }
            NetEvent::PlayerInfoRemove(update) => {
                counters.player_info_remove_packets += 1;
                world.apply_player_info_remove(update);
            }
            NetEvent::ServerData(update) => {
                counters.server_data_packets += 1;
                world.apply_server_data(update);
            }
            NetEvent::ResourcePackPush(update) => {
                counters.resource_pack_push_packets += 1;
                world.apply_resource_pack_push(update);
            }
            NetEvent::ResourcePackPop(update) => {
                counters.resource_pack_pop_packets += 1;
                world.apply_resource_pack_pop(update);
            }
            NetEvent::Cooldown(update) => {
                counters.cooldown_packets += 1;
                world.apply_cooldown(update);
            }
            NetEvent::DamageEvent(update) => {
                counters.damage_event_packets += 1;
                world.apply_damage_event(update);
            }
            NetEvent::UpdateMobEffect(update) => {
                counters.update_mob_effect_packets += 1;
                world.apply_update_mob_effect(update);
            }
            NetEvent::RemoveMobEffect(update) => {
                counters.remove_mob_effect_packets += 1;
                world.apply_remove_mob_effect(update);
            }
            NetEvent::ContainerClose(update) => {
                world.apply_container_close(update);
            }
            NetEvent::ContainerSetContent(update) => {
                world.apply_container_set_content(update);
            }
            NetEvent::ContainerSetData(update) => {
                world.apply_container_set_data(update);
            }
            NetEvent::ContainerSetSlot(update) => {
                world.apply_container_set_slot(update);
            }
            NetEvent::OpenScreen(update) => {
                world.apply_open_screen(update);
            }
            NetEvent::SetCursorItem(update) => {
                world.apply_set_cursor_item(update);
            }
            NetEvent::SetPlayerInventory(update) => {
                world.apply_set_player_inventory(update);
            }
            NetEvent::BlockDestruction(update) => {
                counters.block_destruction_packets += 1;
                world.apply_block_destruction(update);
            }
            NetEvent::BossEvent(update) => {
                counters.boss_event_packets += 1;
                world.apply_boss_event(update);
            }
            NetEvent::ChangeDifficulty(update) => {
                counters.change_difficulty_packets += 1;
                world.apply_change_difficulty(update);
            }
            NetEvent::BlockEvent(event) => {
                counters.block_event_packets += 1;
                world.apply_block_event(event);
            }
            NetEvent::LevelEvent(event) => {
                counters.level_event_packets += 1;
                world.apply_level_event(event);
            }
            NetEvent::InitializeBorder(border) => {
                counters.initialize_border_packets += 1;
                world.apply_initialize_border(border);
            }
            NetEvent::SetBorderCenter(update) => {
                counters.set_border_center_packets += 1;
                world.apply_set_border_center(update);
            }
            NetEvent::SetBorderLerpSize(update) => {
                counters.set_border_lerp_size_packets += 1;
                world.apply_set_border_lerp_size(update);
            }
            NetEvent::SetBorderSize(update) => {
                counters.set_border_size_packets += 1;
                world.apply_set_border_size(update);
            }
            NetEvent::SetBorderWarningDelay(update) => {
                counters.set_border_warning_delay_packets += 1;
                world.apply_set_border_warning_delay(update);
            }
            NetEvent::SetBorderWarningDistance(update) => {
                counters.set_border_warning_distance_packets += 1;
                world.apply_set_border_warning_distance(update);
            }
            NetEvent::ResetScore(update) => {
                counters.reset_score_packets += 1;
                world.apply_reset_score(update);
            }
            NetEvent::SetDisplayObjective(update) => {
                counters.set_display_objective_packets += 1;
                world.apply_set_display_objective(update);
            }
            NetEvent::SetObjective(update) => {
                counters.set_objective_packets += 1;
                world.apply_set_objective(update);
            }
            NetEvent::SetPlayerTeam(update) => {
                counters.set_player_team_packets += 1;
                world.apply_set_player_team(update);
            }
            NetEvent::SetScore(update) => {
                counters.set_score_packets += 1;
                world.apply_set_score(update);
            }
            NetEvent::CommandSuggestions(update) => {
                counters.command_suggestion_packets += 1;
                world.apply_command_suggestions(update);
            }
            NetEvent::TabList(update) => {
                counters.tab_list_packets += 1;
                world.apply_tab_list(update);
            }
            NetEvent::AddEntity(entity) => {
                world.apply_add_entity(entity);
            }
            NetEvent::EntityAnimation(update) => {
                world.apply_entity_animation(update);
            }
            NetEvent::EntityEvent(update) => {
                world.apply_entity_event(update);
            }
            NetEvent::HurtAnimation(update) => {
                world.apply_hurt_animation(update);
            }
            NetEvent::MoveEntity(update) => {
                world.apply_entity_move(update);
            }
            NetEvent::MoveVehicle(update) => {
                counters.move_vehicle_packets += 1;
                if let Some(report) = world.apply_move_vehicle(update) {
                    queue_vehicle_move_command(counters, net_commands, report);
                }
            }
            NetEvent::EntityPositionSync(update) => {
                world.apply_entity_position_sync(update);
            }
            NetEvent::RemoveEntities(update) => {
                world.apply_remove_entities(update);
            }
            NetEvent::RotateHead(update) => {
                world.apply_rotate_head(update);
            }
            NetEvent::SetEntityMotion(update) => {
                world.apply_set_entity_motion(update);
            }
            NetEvent::SetEntityLink(update) => {
                world.apply_set_entity_link(update);
            }
            NetEvent::SetEquipment(update) => {
                world.apply_set_equipment(update);
            }
            NetEvent::TakeItemEntity(update) => {
                world.apply_take_item_entity(update);
                counters.take_item_entity_packets += 1;
            }
            NetEvent::SetPassengers(update) => {
                world.apply_set_passengers(update);
            }
            NetEvent::UpdateAttributes(update) => {
                world.apply_update_attributes(update);
            }
            NetEvent::SetEntityData(update) => {
                world.apply_set_entity_data(update);
            }
            NetEvent::TeleportEntity(update) => {
                world.apply_teleport_entity(update);
            }
            NetEvent::RegistryData {
                registry,
                raw_payload_len,
            } => {
                world.record_registry(registry, raw_payload_len);
                counters.registries_seen = world.counters().registries_seen;
            }
            NetEvent::Login(login) => {
                counters.player_entity_id = Some(login.player_id);
                world.apply_login(&login);
            }
            NetEvent::Respawn(respawn) => {
                world.apply_respawn(&respawn);
            }
            NetEvent::PlayerPosition(update) => {
                apply_player_position_update(counters, update);
            }
            NetEvent::PlayerRotation(update) => {
                apply_player_rotation_update(counters, update);
            }
            NetEvent::PlayerAbilities(abilities) => {
                apply_player_abilities_update(counters, abilities);
            }
            NetEvent::PlayerHealth(health) => {
                apply_player_health_update(counters, health);
            }
            NetEvent::PlayerExperience(experience) => {
                apply_player_experience_update(counters, experience);
            }
            NetEvent::HeldSlot(slot) => {
                apply_held_slot_update(counters, slot);
            }
            NetEvent::SetDefaultSpawnPosition(spawn) => {
                apply_default_spawn_update(counters, spawn);
            }
            NetEvent::SetSimulationDistance(distance) => {
                apply_simulation_distance_update(counters, distance);
            }
            NetEvent::SystemChat(chat) => {
                apply_system_chat_update(counters, chat);
            }
            NetEvent::SetActionBarText(text) => {
                apply_action_bar_update(counters, text);
            }
            NetEvent::SetTitleText(text) => {
                apply_title_text_update(counters, text);
            }
            NetEvent::SetSubtitleText(text) => {
                apply_subtitle_text_update(counters, text);
            }
            NetEvent::SetTitlesAnimation(animation) => {
                apply_titles_animation_update(counters, animation);
            }
            NetEvent::TickingState(ticking) => {
                apply_ticking_state_update(counters, ticking);
            }
            NetEvent::TickingStep(step) => {
                apply_ticking_step_update(counters, step);
            }
            NetEvent::SetCamera(camera) => {
                apply_set_camera_update(counters, world, camera);
            }
            NetEvent::BlockChangedAck(ack) => {
                apply_block_changed_ack(counters, ack);
            }
            NetEvent::BlockEntityData(update) => match world.apply_block_entity_data(update) {
                Ok(_) => {}
                Err(err) => {
                    counters.last_error = Some(err.to_string());
                }
            },
            NetEvent::GameEvent(event) => {
                apply_game_event(counters, event);
            }
            NetEvent::SetTime(time) => {
                apply_world_time_update(counters, time);
            }
            NetEvent::LevelChunkWithLight(chunk) => {
                match world.insert_level_chunk_with_light(chunk) {
                    Ok(pos) => {
                        counters.first_chunk.get_or_insert(pos);
                    }
                    Err(err) => {
                        counters.last_error = Some(err.to_string());
                    }
                }
            }
            NetEvent::LightUpdate(update) => match world.apply_light_update(update) {
                Ok(_) => {}
                Err(err) => {
                    counters.last_error = Some(err.to_string());
                }
            },
            NetEvent::ChunksBiomes(update) => match world.apply_biome_update(update) {
                Ok(_) => {}
                Err(err) => {
                    counters.last_error = Some(err.to_string());
                }
            },
            NetEvent::ForgetLevelChunk(update) => {
                world.forget_chunk(ChunkPos {
                    x: update.pos.x,
                    z: update.pos.z,
                });
            }
            NetEvent::BlockUpdate(update) => {
                world.apply_block_update(update);
            }
            NetEvent::SectionBlocksUpdate(update) => {
                world.apply_section_blocks_update(update);
            }
            NetEvent::SetChunkCacheCenter(update) => {
                counters.chunk_cache_center = Some(ChunkPos {
                    x: update.chunk_x,
                    z: update.chunk_z,
                });
            }
            NetEvent::SetChunkCacheRadius(update) => {
                counters.chunk_cache_radius = Some(update.radius);
            }
        }
    }
    drained
}

fn apply_world_time_update(counters: &mut NetCounters, time: bbb_protocol::packets::PlayTime) {
    let day_time = time
        .clock_updates
        .first()
        .map(|clock| clock.total_ticks)
        .unwrap_or(time.game_time);
    counters.world_time = Some(bbb_control::WorldTime {
        game_time: time.game_time,
        day_time,
        clock_updates: time.clock_updates.len(),
    });
    counters.world_time_packets += 1;
}

fn apply_game_event(counters: &mut NetCounters, event: bbb_protocol::packets::GameEvent) {
    counters.weather.last_game_event_id = Some(event.event_id);
    counters.weather.last_game_event_param = event.param;
    counters.game_event_packets += 1;

    match event.event_id {
        1 => {
            counters.weather.raining = true;
            counters.weather.rain_level = counters.weather.rain_level.max(1.0);
        }
        2 => {
            counters.weather.raining = false;
            counters.weather.rain_level = 0.0;
            counters.weather.thunder_level = 0.0;
        }
        7 => {
            counters.weather.rain_level = event.param.clamp(0.0, 1.0);
            counters.weather.raining = counters.weather.rain_level > 0.0;
        }
        8 => {
            counters.weather.thunder_level = event.param.clamp(0.0, 1.0);
        }
        _ => {}
    }
}

fn apply_block_changed_ack(
    counters: &mut NetCounters,
    ack: bbb_protocol::packets::BlockChangedAck,
) {
    counters.block_changed_ack_packets += 1;
    counters.last_block_changed_ack_sequence = Some(ack.sequence);
}

fn apply_player_abilities_update(
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

fn apply_default_spawn_update(
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

fn apply_simulation_distance_update(
    counters: &mut NetCounters,
    distance: bbb_protocol::packets::SetSimulationDistance,
) {
    counters.simulation_distance = Some(distance.distance);
    counters.simulation_distance_packets += 1;
}

fn apply_system_chat_update(counters: &mut NetCounters, chat: bbb_protocol::packets::SystemChat) {
    counters.last_system_chat = Some(SystemChatLine {
        content: chat.content,
        overlay: chat.overlay,
    });
    counters.system_chat_packets += 1;
}

fn apply_action_bar_update(
    counters: &mut NetCounters,
    text: bbb_protocol::packets::SetActionBarText,
) {
    counters.last_action_bar = Some(ActionBarText {
        content: text.content,
        display_ticks: 60,
    });
    counters.action_bar_packets += 1;
}

fn apply_title_text_update(counters: &mut NetCounters, text: bbb_protocol::packets::SetTitleText) {
    counters.title.title = Some(text.content);
    counters.title.title_time = title_total_ticks(&counters.title);
    counters.title_text_packets += 1;
}

fn apply_subtitle_text_update(
    counters: &mut NetCounters,
    text: bbb_protocol::packets::SetSubtitleText,
) {
    counters.title.subtitle = Some(text.content);
    counters.subtitle_text_packets += 1;
}

fn apply_titles_animation_update(
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

fn apply_ticking_state_update(
    counters: &mut NetCounters,
    ticking: bbb_protocol::packets::TickingState,
) {
    counters.ticking.tick_rate = ticking.clamped_tick_rate();
    counters.ticking.frozen = ticking.frozen;
    counters.ticking_state_packets += 1;
}

fn apply_ticking_step_update(counters: &mut NetCounters, step: bbb_protocol::packets::TickingStep) {
    counters.ticking.frozen_ticks_to_run = step.tick_steps;
    counters.ticking_step_packets += 1;
}

fn apply_set_camera_update(
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

fn apply_player_health_update(
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

fn apply_player_experience_update(
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

fn apply_held_slot_update(counters: &mut NetCounters, slot: bbb_protocol::packets::SetHeldSlot) {
    if (0..=8).contains(&slot.slot) {
        counters.selected_hotbar_slot = slot.slot as u8;
    }
    counters.held_slot_packets += 1;
}

fn apply_player_position_update(
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

fn apply_player_rotation_update(
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
    use crate::runtime::clear_color_for_day_time;
    use bbb_protocol::packets::{
        AddEntity, BlockPos as ProtocolBlockPos, CommonPlayerSpawnInfo, PlayLogin, SetPassengers,
        Vec3d as ProtocolVec3d, PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_X, PLAYER_RELATIVE_X_ROT,
        PLAYER_RELATIVE_Y_ROT,
    };
    use uuid::Uuid;

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

    #[test]
    fn block_changed_ack_updates_snapshot_counters() {
        let mut counters = NetCounters::default();

        apply_block_changed_ack(
            &mut counters,
            bbb_protocol::packets::BlockChangedAck { sequence: 17 },
        );

        assert_eq!(counters.block_changed_ack_packets, 1);
        assert_eq!(counters.last_block_changed_ack_sequence, Some(17));
    }

    #[test]
    fn take_item_entity_event_updates_snapshot_counter() {
        let (tx, mut rx) = mpsc::channel(1);
        tx.try_send(NetEvent::TakeItemEntity(
            bbb_protocol::packets::TakeItemEntity {
                item_id: 10,
                player_id: 20,
                amount: 3,
            },
        ))
        .unwrap();
        drop(tx);

        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        assert_eq!(
            drain_net_events(&mut rx, &mut world, &mut counters, &None),
            1
        );
        assert_eq!(counters.take_item_entity_packets, 1);
        assert_eq!(world.counters().take_item_entities_received, 1);
        assert_eq!(world.counters().take_item_entities_applied, 0);
    }

    #[test]
    fn command_suggestions_event_updates_world_and_counters() {
        let (tx, mut rx) = mpsc::channel(1);
        tx.try_send(NetEvent::CommandSuggestions(
            bbb_protocol::packets::CommandSuggestions {
                id: 7,
                start: 1,
                length: 4,
                suggestions: vec![
                    bbb_protocol::packets::CommandSuggestion {
                        text: "give".to_string(),
                        tooltip: Some("Run give".to_string()),
                    },
                    bbb_protocol::packets::CommandSuggestion {
                        text: "gamemode".to_string(),
                        tooltip: None,
                    },
                ],
            },
        ))
        .unwrap();

        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        assert_eq!(
            drain_net_events(&mut rx, &mut world, &mut counters, &None),
            1
        );
        assert_eq!(counters.command_suggestion_packets, 1);
        assert_eq!(world.counters().command_suggestion_packets, 1);
        assert_eq!(world.counters().command_suggestion_entries_tracked, 2);

        let result = world.command_suggestions_by_id(7).unwrap();
        assert_eq!(result.start, 1);
        assert_eq!(result.length, 4);
        assert_eq!(result.suggestions.len(), 2);
        assert_eq!(result.suggestions[0].text, "give");
        assert_eq!(result.suggestions[0].tooltip.as_deref(), Some("Run give"));
        assert_eq!(world.last_command_suggestions(), Some(result));
    }

    #[test]
    fn block_destruction_event_updates_world_and_counter() {
        let (tx, mut rx) = mpsc::channel(1);
        tx.try_send(NetEvent::BlockDestruction(
            bbb_protocol::packets::BlockDestruction {
                id: 4,
                pos: ProtocolBlockPos {
                    x: 12,
                    y: 64,
                    z: -5,
                },
                progress: 6,
            },
        ))
        .unwrap();

        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        assert_eq!(
            drain_net_events(&mut rx, &mut world, &mut counters, &None),
            1
        );
        assert_eq!(counters.block_destruction_packets, 1);
        assert_eq!(world.counters().block_destructions_received, 1);
        assert_eq!(world.counters().block_destructions_tracked, 1);
        assert_eq!(world.block_destruction(4).unwrap().progress, 6);
    }

    #[test]
    fn block_and_level_events_update_world_and_counters() {
        let (tx, mut rx) = mpsc::channel(2);
        tx.try_send(NetEvent::BlockEvent(bbb_protocol::packets::BlockEvent {
            pos: ProtocolBlockPos {
                x: 12,
                y: 65,
                z: -5,
            },
            b0: 2,
            b1: 9,
            block_id: 54,
        }))
        .unwrap();
        tx.try_send(NetEvent::LevelEvent(bbb_protocol::packets::LevelEvent {
            event_type: 1001,
            pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
            data: 42,
            global: true,
        }))
        .unwrap();

        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        assert_eq!(
            drain_net_events(&mut rx, &mut world, &mut counters, &None),
            2
        );
        assert_eq!(counters.block_event_packets, 1);
        assert_eq!(counters.level_event_packets, 1);

        let world_counters = world.counters();
        assert_eq!(world_counters.block_events_received, 1);
        assert_eq!(world_counters.block_events_tracked, 1);
        assert_eq!(world_counters.level_events_received, 1);
        assert_eq!(world_counters.level_events_tracked, 1);

        let block_event = world.block_events().first().unwrap();
        assert_eq!(
            block_event.pos,
            BlockPos {
                x: 12,
                y: 65,
                z: -5
            }
        );
        assert_eq!(block_event.b0, 2);
        assert_eq!(block_event.b1, 9);
        assert_eq!(block_event.block_id, 54);

        let level_event = world.level_events().first().unwrap();
        assert_eq!(level_event.event_type, 1001);
        assert_eq!(level_event.pos, BlockPos { x: 3, y: 4, z: 5 });
        assert_eq!(level_event.data, 42);
        assert!(level_event.global);
    }

    #[test]
    fn border_events_update_world_and_counters() {
        let (tx, mut rx) = mpsc::channel(6);
        tx.try_send(NetEvent::InitializeBorder(
            bbb_protocol::packets::InitializeBorder {
                new_center_x: 1.0,
                new_center_z: 2.0,
                old_size: 100.0,
                new_size: 200.0,
                lerp_time: 40,
                new_absolute_max_size: 500,
                warning_blocks: 6,
                warning_time: 7,
            },
        ))
        .unwrap();
        tx.try_send(NetEvent::SetBorderCenter(
            bbb_protocol::packets::SetBorderCenter {
                new_center_x: 3.0,
                new_center_z: 4.0,
            },
        ))
        .unwrap();
        tx.try_send(NetEvent::SetBorderLerpSize(
            bbb_protocol::packets::SetBorderLerpSize {
                old_size: 200.0,
                new_size: 300.0,
                lerp_time: 50,
            },
        ))
        .unwrap();
        tx.try_send(NetEvent::SetBorderSize(
            bbb_protocol::packets::SetBorderSize { size: 250.0 },
        ))
        .unwrap();
        tx.try_send(NetEvent::SetBorderWarningDelay(
            bbb_protocol::packets::SetBorderWarningDelay { warning_delay: 9 },
        ))
        .unwrap();
        tx.try_send(NetEvent::SetBorderWarningDistance(
            bbb_protocol::packets::SetBorderWarningDistance { warning_blocks: 8 },
        ))
        .unwrap();

        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        assert_eq!(
            drain_net_events(&mut rx, &mut world, &mut counters, &None),
            6
        );
        assert_eq!(counters.initialize_border_packets, 1);
        assert_eq!(counters.set_border_center_packets, 1);
        assert_eq!(counters.set_border_lerp_size_packets, 1);
        assert_eq!(counters.set_border_size_packets, 1);
        assert_eq!(counters.set_border_warning_delay_packets, 1);
        assert_eq!(counters.set_border_warning_distance_packets, 1);

        let border = world.world_border();
        assert_eq!(border.center_x, 3.0);
        assert_eq!(border.center_z, 4.0);
        assert_eq!(border.size, 250.0);
        assert_eq!(border.lerp_target, 250.0);
        assert_eq!(border.lerp_time, 0);
        assert_eq!(border.absolute_max_size, 500);
        assert_eq!(border.warning_blocks, 8);
        assert_eq!(border.warning_time, 9);
    }

    #[test]
    fn scoreboard_events_update_world_and_counters() {
        let (tx, mut rx) = mpsc::channel(6);
        tx.try_send(NetEvent::SetObjective(
            bbb_protocol::packets::SetObjective {
                objective_name: "kills".to_string(),
                method: bbb_protocol::packets::SetObjectiveMethod::Add,
                parameters: Some(bbb_protocol::packets::SetObjectiveParameters {
                    display_name: "Kills".to_string(),
                    render_type: bbb_protocol::packets::ObjectiveRenderType::Integer,
                    number_format: Some(vec![9]),
                }),
            },
        ))
        .unwrap();
        tx.try_send(NetEvent::SetDisplayObjective(
            bbb_protocol::packets::SetDisplayObjective {
                slot: bbb_protocol::packets::ScoreboardDisplaySlot::Sidebar,
                objective_name: Some("kills".to_string()),
            },
        ))
        .unwrap();
        tx.try_send(NetEvent::SetScore(bbb_protocol::packets::SetScore {
            owner: "Steve".to_string(),
            objective_name: "kills".to_string(),
            score: 4,
            display: Some("Four".to_string()),
            number_format: None,
        }))
        .unwrap();
        tx.try_send(NetEvent::SetScore(bbb_protocol::packets::SetScore {
            owner: "Alex".to_string(),
            objective_name: "kills".to_string(),
            score: 1,
            display: None,
            number_format: None,
        }))
        .unwrap();
        tx.try_send(NetEvent::SetPlayerTeam(
            bbb_protocol::packets::SetPlayerTeam {
                name: "red".to_string(),
                method: bbb_protocol::packets::PlayerTeamMethod::Add,
                parameters: Some(bbb_protocol::packets::PlayerTeamParameters {
                    display_name: "Red Team".to_string(),
                    options: 0b11,
                    nametag_visibility: bbb_protocol::packets::TeamVisibility::Always,
                    collision_rule: bbb_protocol::packets::TeamCollisionRule::Never,
                    color: bbb_protocol::packets::ChatFormatting::Red,
                    player_prefix: "[R]".to_string(),
                    player_suffix: "!".to_string(),
                }),
                players: vec!["Steve".to_string()],
            },
        ))
        .unwrap();
        tx.try_send(NetEvent::ResetScore(bbb_protocol::packets::ResetScore {
            owner: "Alex".to_string(),
            objective_name: Some("kills".to_string()),
        }))
        .unwrap();

        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        assert_eq!(
            drain_net_events(&mut rx, &mut world, &mut counters, &None),
            6
        );
        assert_eq!(counters.set_objective_packets, 1);
        assert_eq!(counters.set_display_objective_packets, 1);
        assert_eq!(counters.set_score_packets, 2);
        assert_eq!(counters.set_player_team_packets, 1);
        assert_eq!(counters.reset_score_packets, 1);

        let scoreboard = world.scoreboard();
        let objective = scoreboard.objectives.get("kills").unwrap();
        assert_eq!(objective.display_name, "Kills");
        assert_eq!(objective.render_type, "integer");
        assert_eq!(objective.number_format, Some(vec![9]));
        assert_eq!(
            scoreboard.display_slots.get("sidebar").map(String::as_str),
            Some("kills")
        );

        let steve_scores = scoreboard.scores.get("Steve").unwrap();
        let steve_kills = steve_scores.get("kills").unwrap();
        assert_eq!(steve_kills.value, 4);
        assert_eq!(steve_kills.display.as_deref(), Some("Four"));
        assert!(!scoreboard.scores.contains_key("Alex"));

        let team = scoreboard.teams.get("red").unwrap();
        assert!(team.players.contains("Steve"));
        let parameters = team.parameters.as_ref().unwrap();
        assert_eq!(parameters.display_name, "Red Team");
        assert_eq!(parameters.color, "red");
    }

    #[test]
    fn hud_session_events_update_world_and_counters() {
        let boss_id = Uuid::from_u128(1);
        let (tx, mut rx) = mpsc::channel(4);
        tx.try_send(NetEvent::BossEvent(bbb_protocol::packets::BossEvent {
            id: boss_id,
            operation: bbb_protocol::packets::BossEventOperation::Add {
                name: "Ender Dragon".to_string(),
                progress: 0.75,
                color: bbb_protocol::packets::BossBarColor::Purple,
                overlay: bbb_protocol::packets::BossBarOverlay::Progress,
                flags: bbb_protocol::packets::BossEventFlags {
                    darken_screen: true,
                    play_music: false,
                    create_world_fog: true,
                },
            },
        }))
        .unwrap();
        tx.try_send(NetEvent::BossEvent(bbb_protocol::packets::BossEvent {
            id: boss_id,
            operation: bbb_protocol::packets::BossEventOperation::UpdateProgress { progress: 0.25 },
        }))
        .unwrap();
        tx.try_send(NetEvent::TabList(bbb_protocol::packets::TabList {
            header: Some("Welcome".to_string()),
            footer: None,
        }))
        .unwrap();
        tx.try_send(NetEvent::ChangeDifficulty(
            bbb_protocol::packets::ChangeDifficulty {
                difficulty: bbb_protocol::packets::Difficulty::Hard,
                locked: true,
            },
        ))
        .unwrap();

        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        assert_eq!(
            drain_net_events(&mut rx, &mut world, &mut counters, &None),
            4
        );
        assert_eq!(counters.boss_event_packets, 2);
        assert_eq!(counters.tab_list_packets, 1);
        assert_eq!(counters.change_difficulty_packets, 1);

        let boss = world.boss_bars().get(&boss_id).unwrap();
        assert_eq!(boss.name, "Ender Dragon");
        assert_eq!(boss.progress, 0.25);
        assert_eq!(boss.color, "purple");
        assert_eq!(boss.overlay, "progress");
        assert!(boss.darken_screen);
        assert!(boss.create_world_fog);
        assert_eq!(world.tab_list().header.as_deref(), Some("Welcome"));
        assert_eq!(world.tab_list().footer, None);
        assert_eq!(world.difficulty().difficulty, "hard");
        assert!(world.difficulty().difficulty_locked);

        let world_counters = world.counters();
        assert_eq!(world_counters.boss_event_packets, 2);
        assert_eq!(world_counters.boss_bars_tracked, 1);
        assert_eq!(world_counters.tab_list_packets, 1);
        assert_eq!(world_counters.change_difficulty_packets, 1);
    }

    #[test]
    fn player_info_events_update_world_and_counters() {
        let profile_id = Uuid::from_u128(1);
        let removed_profile_id = Uuid::from_u128(2);
        let (tx, mut rx) = mpsc::channel(2);
        tx.try_send(NetEvent::PlayerInfoUpdate(
            bbb_protocol::packets::PlayerInfoUpdate {
                actions: vec![
                    bbb_protocol::packets::PlayerInfoAction::AddPlayer,
                    bbb_protocol::packets::PlayerInfoAction::InitializeChat,
                    bbb_protocol::packets::PlayerInfoAction::UpdateGameMode,
                    bbb_protocol::packets::PlayerInfoAction::UpdateListed,
                    bbb_protocol::packets::PlayerInfoAction::UpdateLatency,
                    bbb_protocol::packets::PlayerInfoAction::UpdateDisplayName,
                    bbb_protocol::packets::PlayerInfoAction::UpdateListOrder,
                    bbb_protocol::packets::PlayerInfoAction::UpdateHat,
                ],
                entries: vec![
                    bbb_protocol::packets::PlayerInfoEntry {
                        profile_id,
                        profile: Some(bbb_protocol::packets::GameProfile {
                            uuid: profile_id,
                            name: "Ada".to_string(),
                            properties: vec![bbb_protocol::packets::GameProfileProperty {
                                name: "textures".to_string(),
                                value: "skin".to_string(),
                                signature: Some("signature".to_string()),
                            }],
                        }),
                        listed: true,
                        latency: 42,
                        game_mode: bbb_protocol::packets::GameType::Creative,
                        display_name: Some("Ada Lovelace".to_string()),
                        show_hat: true,
                        list_order: 3,
                        chat_session: Some(bbb_protocol::packets::PlayerInfoChatSession {
                            session_id: Uuid::from_u128(3),
                            expires_at_epoch_millis: 99,
                            public_key: vec![1, 2],
                            key_signature: vec![3, 4],
                        }),
                    },
                    bbb_protocol::packets::PlayerInfoEntry {
                        profile_id: removed_profile_id,
                        profile: Some(bbb_protocol::packets::GameProfile {
                            uuid: removed_profile_id,
                            name: "Removed".to_string(),
                            properties: Vec::new(),
                        }),
                        listed: true,
                        latency: 7,
                        game_mode: bbb_protocol::packets::GameType::Survival,
                        display_name: None,
                        show_hat: false,
                        list_order: 0,
                        chat_session: None,
                    },
                ],
            },
        ))
        .unwrap();
        tx.try_send(NetEvent::PlayerInfoRemove(
            bbb_protocol::packets::PlayerInfoRemove {
                profile_ids: vec![removed_profile_id],
            },
        ))
        .unwrap();

        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        assert_eq!(
            drain_net_events(&mut rx, &mut world, &mut counters, &None),
            2
        );
        assert_eq!(counters.player_info_update_packets, 1);
        assert_eq!(counters.player_info_remove_packets, 1);

        let entry = world.player_info_entry(profile_id).unwrap();
        assert_eq!(entry.profile.uuid, profile_id);
        assert_eq!(entry.profile.name, "Ada");
        assert_eq!(entry.profile.properties.len(), 1);
        assert!(entry.listed);
        assert_eq!(entry.latency, 42);
        assert_eq!(entry.game_mode, "creative");
        assert_eq!(entry.display_name.as_deref(), Some("Ada Lovelace"));
        assert!(entry.show_hat);
        assert_eq!(entry.list_order, 3);
        assert!(entry.chat_session_present);
        assert!(world.listed_players().contains(&profile_id));
        assert!(world.player_info_entry(removed_profile_id).is_none());
        assert!(!world.listed_players().contains(&removed_profile_id));

        let world_counters = world.counters();
        assert_eq!(world_counters.player_info_update_packets, 1);
        assert_eq!(world_counters.player_info_remove_packets, 1);
        assert_eq!(world_counters.player_info_entries_tracked, 1);
        assert_eq!(world_counters.listed_players_tracked, 1);
    }

    #[test]
    fn server_presentation_events_update_world_and_counters() {
        let pack_id = Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678);
        let (tx, mut rx) = mpsc::channel(3);
        tx.try_send(NetEvent::ServerData(bbb_protocol::packets::ServerData {
            motd: "Native test server".to_string(),
            icon_bytes: Some(vec![1, 2, 3, 4]),
        }))
        .unwrap();
        tx.try_send(NetEvent::ResourcePackPush(
            bbb_protocol::packets::ResourcePackPush {
                id: pack_id,
                url: "https://example.invalid/pack.zip".to_string(),
                hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
                required: true,
                prompt: Some("Install pack?".to_string()),
            },
        ))
        .unwrap();
        tx.try_send(NetEvent::ResourcePackPop(
            bbb_protocol::packets::ResourcePackPop { id: None },
        ))
        .unwrap();

        let mut world = WorldStore::new();
        let mut counters = NetCounters::default();

        assert_eq!(
            drain_net_events(&mut rx, &mut world, &mut counters, &None),
            3
        );
        assert_eq!(counters.server_data_packets, 1);
        assert_eq!(counters.resource_pack_push_packets, 1);
        assert_eq!(counters.resource_pack_pop_packets, 1);

        let server_data = world.server_data().unwrap();
        assert_eq!(server_data.motd, "Native test server");
        assert_eq!(server_data.icon_byte_len(), Some(4));
        assert!(world.resource_packs().is_empty());

        let world_counters = world.counters();
        assert_eq!(world_counters.server_data_packets, 1);
        assert_eq!(world_counters.resource_pack_push_packets, 1);
        assert_eq!(world_counters.resource_pack_pop_packets, 1);
        assert_eq!(world_counters.resource_packs_tracked, 0);
    }

    #[test]
    fn entity_status_events_update_world_and_counters() {
        let entity_id = 55;
        let (tx, mut rx) = mpsc::channel(4);
        tx.try_send(NetEvent::Cooldown(bbb_protocol::packets::Cooldown {
            cooldown_group: "minecraft:ender_pearl".to_string(),
            duration: 20,
        }))
        .unwrap();
        tx.try_send(NetEvent::DamageEvent(bbb_protocol::packets::DamageEvent {
            entity_id,
            source_type_id: 5,
            source_cause_id: -1,
            source_direct_id: 42,
            source_position: Some(bbb_protocol::packets::Vec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            }),
        }))
        .unwrap();
        tx.try_send(NetEvent::UpdateMobEffect(
            bbb_protocol::packets::UpdateMobEffect {
                entity_id,
                effect_id: 3,
                amplifier: 2,
                duration_ticks: 400,
                flags: bbb_protocol::packets::MobEffectFlags {
                    raw: 0b1011,
                    ambient: true,
                    visible: true,
                    show_icon: false,
                    blend: true,
                },
            },
        ))
        .unwrap();
        tx.try_send(NetEvent::RemoveMobEffect(
            bbb_protocol::packets::RemoveMobEffect {
                entity_id,
                effect_id: 99,
            },
        ))
        .unwrap();

        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(entity_id));
        let mut counters = NetCounters::default();

        assert_eq!(
            drain_net_events(&mut rx, &mut world, &mut counters, &None),
            4
        );
        assert_eq!(counters.cooldown_packets, 1);
        assert_eq!(counters.damage_event_packets, 1);
        assert_eq!(counters.update_mob_effect_packets, 1);
        assert_eq!(counters.remove_mob_effect_packets, 1);

        let cooldown = world.cooldown("minecraft:ender_pearl").unwrap();
        assert_eq!(cooldown.duration, 20);

        let damage = world.entity_last_damage(entity_id).unwrap();
        assert_eq!(damage.source_type_id, 5);
        assert_eq!(damage.source_cause_id, -1);
        assert_eq!(damage.source_direct_id, 42);
        assert_eq!(
            damage.source_position,
            Some(bbb_protocol::packets::Vec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            })
        );

        let effect = world.entity_effect(entity_id, 3).unwrap();
        assert_eq!(effect.amplifier, 2);
        assert_eq!(effect.duration_ticks, 400);
        assert!(effect.ambient);
        assert!(effect.visible);
        assert!(!effect.show_icon);
        assert!(effect.blend);
        assert!(world.entity_effect(entity_id, 99).is_none());

        let world_counters = world.counters();
        assert_eq!(world_counters.cooldown_packets, 1);
        assert_eq!(world_counters.cooldowns_tracked, 1);
        assert_eq!(world_counters.damage_event_packets, 1);
        assert_eq!(world_counters.damage_events_applied, 1);
        assert_eq!(world_counters.update_mob_effect_packets, 1);
        assert_eq!(world_counters.remove_mob_effect_packets, 1);
        assert_eq!(world_counters.active_mob_effects_tracked, 1);
    }

    #[test]
    fn move_vehicle_event_updates_world_and_queues_ack() {
        let (event_tx, mut event_rx) = mpsc::channel(1);
        let (command_tx, mut command_rx) = mpsc::channel(1);
        let commands = Some(command_tx);
        let mut world = WorldStore::new();
        world.apply_login(&protocol_play_login(99));
        world.apply_add_entity(protocol_add_entity(10));
        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![99],
        }));

        event_tx
            .try_send(NetEvent::MoveVehicle(bbb_protocol::packets::MoveVehicle {
                position: ProtocolVec3d {
                    x: 5.0,
                    y: 66.0,
                    z: -7.0,
                },
                y_rot: 45.0,
                x_rot: -5.0,
            }))
            .unwrap();

        let mut counters = NetCounters::default();
        assert_eq!(
            drain_net_events(&mut event_rx, &mut world, &mut counters, &commands),
            1
        );

        assert_eq!(counters.move_vehicle_packets, 1);
        assert_eq!(counters.move_vehicle_commands_queued, 1);
        assert_eq!(world.counters().vehicle_moves_snapped, 1);
        let vehicle = world.probe_entity(10).unwrap();
        assert_eq!(
            vehicle.position,
            bbb_world::EntityVec3 {
                x: 5.0,
                y: 66.0,
                z: -7.0,
            }
        );
        match command_rx.try_recv().unwrap() {
            NetCommand::MoveVehicle(command) => {
                assert_eq!(command.position.x, 5.0);
                assert_eq!(command.position.y, 66.0);
                assert_eq!(command.position.z, -7.0);
                assert_eq!(command.y_rot, 45.0);
                assert_eq!(command.x_rot, -5.0);
                assert!(!command.on_ground);
            }
            other => panic!("expected move vehicle command, got {other:?}"),
        }
    }

    #[test]
    fn world_time_and_weather_update_snapshot_and_clear_color() {
        let mut counters = NetCounters::default();

        apply_world_time_update(
            &mut counters,
            bbb_protocol::packets::PlayTime {
                game_time: 123,
                clock_updates: vec![bbb_protocol::packets::ClockUpdate {
                    clock_id: 0,
                    total_ticks: 6000,
                    partial_tick: 0.0,
                    rate: 1.0,
                }],
            },
        );
        apply_game_event(
            &mut counters,
            bbb_protocol::packets::GameEvent {
                event_id: 7,
                param: 0.5,
            },
        );

        assert_eq!(
            counters.world_time,
            Some(bbb_control::WorldTime {
                game_time: 123,
                day_time: 6000,
                clock_updates: 1,
            })
        );
        assert!(counters.weather.raining);
        assert_eq!(counters.weather.rain_level, 0.5);
        assert_eq!(counters.world_time_packets, 1);
        assert_eq!(counters.game_event_packets, 1);

        let day = clear_color_for_day_time(6000, 0.0, 0.0);
        let night = clear_color_for_day_time(18000, 0.0, 0.0);
        let storm = clear_color_for_day_time(6000, 1.0, 1.0);
        assert!(day.b > night.b);
        assert!(storm.r < day.r);
        assert!(storm.g < day.g);
        assert!(storm.b < day.b);
    }

    fn protocol_play_login(player_id: i32) -> PlayLogin {
        PlayLogin {
            player_id,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        }
    }

    fn protocol_add_entity(id: i32) -> AddEntity {
        AddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: 7,
            position: ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: -10.0,
            y_rot: 20.0,
            y_head_rot: 30.0,
            data: 99,
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
}
