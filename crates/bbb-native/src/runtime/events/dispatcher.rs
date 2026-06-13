use bbb_control::NetCounters;
use bbb_net::{NetCommand, NetEvent};
use bbb_protocol::packets::{ServerLinkEntry, ServerLinkType, ServerLinks};
use bbb_world::{ChunkPos, WorldStore};
use tokio::sync::mpsc;

use crate::input::queue_vehicle_move_command;

use super::client_state::*;
use super::{apply_block_changed_ack, apply_game_event, apply_world_time_update};

pub(in crate::runtime) fn drain_net_events(
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
            NetEvent::CookieRequest {
                key,
                response_payload_present,
            } => {
                counters.last_cookie_key = Some(key);
                counters.cookie_request_packets += 1;
                if response_payload_present {
                    counters.cookie_response_hits += 1;
                } else {
                    counters.cookie_response_misses += 1;
                }
            }
            NetEvent::StoreCookie {
                key,
                payload_len,
                stored_cookie_count,
            } => {
                counters.last_cookie_key = Some(key);
                counters.store_cookie_packets += 1;
                counters.stored_cookie_count = stored_cookie_count;
                counters.stored_cookie_bytes =
                    counters.stored_cookie_bytes.saturating_add(payload_len);
            }
            NetEvent::CustomReportDetails(details) => {
                counters.custom_report_details = details.details;
                counters.custom_report_detail_packets += 1;
            }
            NetEvent::CustomChatCompletions(update) => {
                counters.last_custom_chat_completion =
                    Some(bbb_control::CustomChatCompletionState {
                        action: update.action.as_str().to_string(),
                        entries: update.entries.len(),
                    });
                counters.custom_chat_completion_packets += 1;
            }
            NetEvent::CustomPayload(update) => {
                counters.last_custom_payload = Some(custom_payload_state(update));
                counters.custom_payload_packets += 1;
            }
            NetEvent::ServerLinks(links) => {
                apply_server_links_update(counters, links);
            }
            NetEvent::LowDiskSpaceWarning => {
                counters.low_disk_space_warnings += 1;
            }
            NetEvent::MapItemData(update) => {
                world.apply_map_item_data(update);
            }
            NetEvent::MountScreenOpen(update) => {
                counters.last_mount_screen = Some(bbb_control::MountScreenState {
                    container_id: update.container_id,
                    inventory_columns: update.inventory_columns,
                    entity_id: update.entity_id,
                });
                counters.mount_screen_open_packets += 1;
            }
            NetEvent::OpenBook(update) => {
                counters.last_open_book_hand = Some(
                    match update.hand {
                        bbb_protocol::packets::InteractionHand::MainHand => "main_hand",
                        bbb_protocol::packets::InteractionHand::OffHand => "off_hand",
                    }
                    .to_string(),
                );
                counters.open_book_packets += 1;
            }
            NetEvent::OpenSignEditor(update) => {
                counters.last_open_sign_editor = Some(bbb_control::OpenSignEditorState {
                    pos: bbb_world::BlockPos {
                        x: update.pos.x,
                        y: update.pos.y,
                        z: update.pos.z,
                    },
                    is_front_text: update.is_front_text,
                });
                counters.open_sign_editor_packets += 1;
            }
            NetEvent::PlaceGhostRecipe(update) => {
                counters.last_ghost_recipe = Some(bbb_control::GhostRecipeState {
                    container_id: update.container_id,
                    recipe_display_type_id: update.recipe_display_type.id(),
                    recipe_display_type: update.recipe_display_type.as_str().to_string(),
                    recipe_display_body_len: update.recipe_display_body.len(),
                });
                counters.ghost_recipe_packets += 1;
            }
            NetEvent::ClearDialog => {
                counters.clear_dialog_packets += 1;
            }
            NetEvent::ShowDialog(update) => {
                counters.last_show_dialog = Some(show_dialog_state(update));
                counters.show_dialog_packets += 1;
            }
            NetEvent::Waypoint(update) => {
                counters.last_waypoint = Some(waypoint_state(update));
                counters.waypoint_packets += 1;
            }
            NetEvent::PlayerCombatEnd(update) => {
                counters.last_player_combat = Some(bbb_control::PlayerCombatState {
                    kind: "end".to_string(),
                    duration: Some(update.duration),
                    player_id: None,
                    message: None,
                });
                counters.player_combat_end_packets += 1;
            }
            NetEvent::PlayerCombatEnter => {
                counters.last_player_combat = Some(bbb_control::PlayerCombatState {
                    kind: "enter".to_string(),
                    duration: None,
                    player_id: None,
                    message: None,
                });
                counters.player_combat_enter_packets += 1;
            }
            NetEvent::PlayerCombatKill(update) => {
                counters.last_player_combat = Some(bbb_control::PlayerCombatState {
                    kind: "kill".to_string(),
                    duration: None,
                    player_id: Some(update.player_id),
                    message: Some(update.message),
                });
                counters.player_combat_kill_packets += 1;
            }
            NetEvent::PlayerLookAt(update) => {
                apply_player_look_at_update(counters, update);
            }
            NetEvent::PongResponse(update) => {
                counters.last_pong_response_time = Some(update.time);
                counters.pong_response_packets += 1;
            }
            NetEvent::Explosion(update) => {
                counters.last_explosion = Some(bbb_control::ExplosionState {
                    center: net_vec3(update.center),
                    radius: update.radius,
                    block_count: update.block_count,
                    player_knockback: update.player_knockback.map(net_vec3),
                    raw_effect_payload_len: update.raw_effect_payload.len(),
                });
                counters.explosion_packets += 1;
            }
            NetEvent::LevelParticles(update) => {
                counters.last_level_particles = Some(bbb_control::LevelParticlesState {
                    override_limiter: update.override_limiter,
                    always_show: update.always_show,
                    position: net_vec3(update.position),
                    offset: net_vec3(update.offset),
                    max_speed: update.max_speed,
                    count: update.count,
                    particle_type_id: update.particle.particle_type_id,
                    raw_options_len: update.particle.raw_options.len(),
                });
                counters.level_particles_packets += 1;
            }
            NetEvent::ProjectilePower(update) => {
                counters.last_projectile_power = Some(bbb_control::ProjectilePowerState {
                    entity_id: update.entity_id,
                    acceleration_power: update.acceleration_power,
                });
                counters.projectile_power_packets += 1;
            }
            NetEvent::DebugBlockValue(update) => {
                counters.last_debug_block_value = Some(bbb_control::DebugBlockValueState {
                    pos: control_block_pos(update.pos),
                    raw_update_payload_len: update.raw_update_payload.len(),
                });
                counters.debug_block_value_packets += 1;
            }
            NetEvent::DebugChunkValue(update) => {
                counters.last_debug_chunk_value = Some(bbb_control::DebugChunkValueState {
                    pos: control_chunk_pos(update.pos),
                    raw_update_payload_len: update.raw_update_payload.len(),
                });
                counters.debug_chunk_value_packets += 1;
            }
            NetEvent::DebugEntityValue(update) => {
                counters.last_debug_entity_value = Some(bbb_control::DebugEntityValueState {
                    entity_id: update.entity_id,
                    raw_update_payload_len: update.raw_update_payload.len(),
                });
                counters.debug_entity_value_packets += 1;
            }
            NetEvent::DebugEvent(update) => {
                counters.last_debug_event = Some(bbb_control::DebugEventState {
                    raw_event_payload_len: update.raw_event_payload.len(),
                });
                counters.debug_event_packets += 1;
            }
            NetEvent::DebugSample(update) => {
                counters.last_debug_sample = Some(bbb_control::DebugSampleState {
                    sample_len: update.sample.len(),
                    sample_type: update.sample_type.as_str().to_string(),
                });
                counters.debug_sample_packets += 1;
            }
            NetEvent::GameRuleValues(update) => {
                counters.last_game_rule_values = Some(bbb_control::GameRuleValuesState {
                    values: update.values.len(),
                });
                counters.game_rule_value_packets += 1;
            }
            NetEvent::GameTestHighlightPos(update) => {
                counters.last_game_test_highlight_pos =
                    Some(bbb_control::GameTestHighlightPosState {
                        absolute_pos: control_block_pos(update.absolute_pos),
                        relative_pos: control_block_pos(update.relative_pos),
                    });
                counters.game_test_highlight_pos_packets += 1;
            }
            NetEvent::TestInstanceBlockStatus(update) => {
                counters.last_test_instance_block_status =
                    Some(bbb_control::TestInstanceBlockStatusState {
                        status: update.status,
                        size: update.size.map(control_vec3i),
                    });
                counters.test_instance_block_status_packets += 1;
            }
            NetEvent::Sound(update) => {
                counters.last_sound = Some(bbb_control::ClientSoundState {
                    sound: sound_holder_state(update.sound),
                    source: update.source.as_str().to_string(),
                    position: net_vec3(update.position),
                    volume: update.volume,
                    pitch: update.pitch,
                    seed: update.seed,
                });
                counters.sound_packets += 1;
            }
            NetEvent::SoundEntity(update) => {
                counters.last_sound_entity = Some(bbb_control::ClientSoundEntityState {
                    sound: sound_holder_state(update.sound),
                    source: update.source.as_str().to_string(),
                    entity_id: update.entity_id,
                    volume: update.volume,
                    pitch: update.pitch,
                    seed: update.seed,
                });
                counters.sound_entity_packets += 1;
            }
            NetEvent::StopSound(update) => {
                counters.last_stop_sound = Some(bbb_control::StopSoundState {
                    source: update.source.map(|source| source.as_str().to_string()),
                    name: update.name,
                });
                counters.stop_sound_packets += 1;
            }
            NetEvent::Transfer(transfer) => {
                counters.last_transfer = Some(bbb_control::TransferTarget {
                    host: transfer.host,
                    port: transfer.port,
                });
                counters.transfer_packets += 1;
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
            NetEvent::SelectAdvancementsTab(update) => {
                counters.selected_advancements_tab = update.tab;
                counters.select_advancements_tab_packets += 1;
            }
            NetEvent::TagQuery(update) => {
                counters.last_tag_query = Some(bbb_control::TagQueryState {
                    transaction_id: update.transaction_id,
                    tag_present: update.tag_present,
                    raw_nbt_len: update.raw_nbt.len(),
                });
                counters.tag_query_packets += 1;
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
            NetEvent::ClearTitles(clear) => {
                apply_clear_titles_update(counters, clear);
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

fn apply_server_links_update(counters: &mut NetCounters, links: ServerLinks) {
    let mut invalid_entries = 0usize;
    let server_links = links
        .links
        .into_iter()
        .filter_map(|entry| {
            if is_allowed_untrusted_uri(&entry.url) {
                Some(server_link_state(entry))
            } else {
                invalid_entries += 1;
                None
            }
        })
        .collect();

    counters.server_links = server_links;
    counters.server_link_packets += 1;
    counters.server_link_invalid_entries = counters
        .server_link_invalid_entries
        .saturating_add(invalid_entries);
}

fn server_link_state(entry: ServerLinkEntry) -> bbb_control::ServerLinkState {
    match entry.link_type {
        ServerLinkType::Known(kind) => {
            let known_type = kind.vanilla_name();
            bbb_control::ServerLinkState {
                label: format!("known_server_link.{known_type}"),
                url: entry.url,
                known_type: Some(known_type.to_string()),
            }
        }
        ServerLinkType::Custom { label } => bbb_control::ServerLinkState {
            label,
            url: entry.url,
            known_type: None,
        },
    }
}

fn sound_holder_state(
    sound: bbb_protocol::packets::SoundEventHolder,
) -> bbb_control::SoundHolderState {
    match sound {
        bbb_protocol::packets::SoundEventHolder::Reference { registry_id } => {
            bbb_control::SoundHolderState {
                kind: "reference".to_string(),
                registry_id: Some(registry_id),
                location: None,
                fixed_range: None,
            }
        }
        bbb_protocol::packets::SoundEventHolder::Direct {
            location,
            fixed_range,
        } => bbb_control::SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(location),
            fixed_range,
        },
    }
}

fn custom_payload_state(
    payload: bbb_protocol::packets::CustomPayload,
) -> bbb_control::CustomPayloadState {
    match payload.payload {
        bbb_protocol::packets::CustomPayloadBody::Brand { brand } => {
            bbb_control::CustomPayloadState {
                id: payload.id,
                kind: "brand".to_string(),
                brand: Some(brand),
                raw_payload_len: 0,
            }
        }
        bbb_protocol::packets::CustomPayloadBody::Unknown { raw_payload } => {
            bbb_control::CustomPayloadState {
                id: payload.id,
                kind: "unknown".to_string(),
                brand: None,
                raw_payload_len: raw_payload.len(),
            }
        }
    }
}

fn show_dialog_state(dialog: bbb_protocol::packets::ShowDialog) -> bbb_control::ShowDialogState {
    match dialog.dialog {
        bbb_protocol::packets::DialogHolder::Reference { registry_id } => {
            bbb_control::ShowDialogState {
                holder_kind: "reference".to_string(),
                registry_id: Some(registry_id),
                raw_dialog_payload_len: 0,
            }
        }
        bbb_protocol::packets::DialogHolder::Direct { raw_dialog_payload } => {
            bbb_control::ShowDialogState {
                holder_kind: "direct".to_string(),
                registry_id: None,
                raw_dialog_payload_len: raw_dialog_payload.len(),
            }
        }
    }
}

fn waypoint_state(
    packet: bbb_protocol::packets::TrackedWaypointPacket,
) -> bbb_control::WaypointState {
    let data = packet.waypoint.data;
    bbb_control::WaypointState {
        operation: packet.operation.as_str().to_string(),
        identifier_kind: packet.waypoint.identifier.kind().to_string(),
        identifier: packet.waypoint.identifier.value_string(),
        icon_style: packet.waypoint.icon.style,
        icon_color_rgb: packet.waypoint.icon.color_rgb,
        waypoint_kind: data.kind().to_string(),
        position: match data {
            bbb_protocol::packets::WaypointData::Position(pos) => Some(control_waypoint_vec3i(pos)),
            _ => None,
        },
        chunk: match data {
            bbb_protocol::packets::WaypointData::Chunk(pos) => Some(control_chunk_pos(pos)),
            _ => None,
        },
        azimuth: match data {
            bbb_protocol::packets::WaypointData::Azimuth(angle) => Some(angle),
            _ => None,
        },
    }
}

fn net_vec3(vec: bbb_protocol::packets::Vec3d) -> bbb_control::NetVec3 {
    bbb_control::NetVec3 {
        x: vec.x,
        y: vec.y,
        z: vec.z,
    }
}

fn control_block_pos(pos: bbb_protocol::packets::BlockPos) -> bbb_world::BlockPos {
    bbb_world::BlockPos {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

fn control_chunk_pos(pos: bbb_protocol::packets::ChunkPos) -> bbb_world::ChunkPos {
    bbb_world::ChunkPos { x: pos.x, z: pos.z }
}

fn control_vec3i(pos: bbb_protocol::packets::Vec3i) -> bbb_control::NetVec3i {
    bbb_control::NetVec3i {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

fn control_waypoint_vec3i(pos: bbb_protocol::packets::WaypointVec3i) -> bbb_control::NetVec3i {
    bbb_control::NetVec3i {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

fn is_allowed_untrusted_uri(uri: &str) -> bool {
    if uri
        .chars()
        .any(|ch| ch.is_ascii_control() || ch.is_whitespace())
    {
        return false;
    }
    let Some((scheme, _)) = uri.split_once(':') else {
        return false;
    };
    if scheme.is_empty() {
        return false;
    }
    let mut chars = scheme.chars();
    if !chars.next().is_some_and(|ch| ch.is_ascii_alphabetic()) {
        return false;
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')) {
        return false;
    }
    matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https")
}
