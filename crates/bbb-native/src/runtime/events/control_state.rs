use bbb_control::NetCounters;
use bbb_net::NetEvent;
use bbb_world::WorldStore;

use super::client_state::apply_player_look_at_update;

pub(super) fn apply_control_projection_event(
    event: NetEvent,
    counters: &mut NetCounters,
    world: &mut WorldStore,
) -> Option<NetEvent> {
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
            counters.stored_cookie_bytes = counters.stored_cookie_bytes.saturating_add(payload_len);
        }
        NetEvent::CustomReportDetails(details) => {
            world.apply_custom_report_details(details);
            sync_custom_report_detail_counters(counters, world);
        }
        NetEvent::ResetChat => {
            counters.reset_chat_packets += 1;
            counters.last_player_chat = None;
            counters.last_disguised_chat = None;
            counters.last_deleted_chat = None;
        }
        NetEvent::UpdateEnabledFeatures(update) => {
            world.apply_update_enabled_features(update);
            sync_enabled_feature_counters(counters, world);
        }
        NetEvent::CodeOfConduct { text } => {
            counters.last_code_of_conduct_len = text.len();
            counters.code_of_conduct_packets += 1;
        }
        NetEvent::CustomChatCompletions(update) => {
            world.apply_custom_chat_completions(update);
            sync_custom_chat_completion_counters(counters, world);
        }
        NetEvent::CustomPayload(update) => {
            world.apply_custom_payload(update);
            sync_custom_payload_counters(counters, world);
        }
        NetEvent::ServerLinks(links) => {
            world.apply_server_links(links);
            sync_server_link_counters(counters, world);
        }
        NetEvent::LowDiskSpaceWarning => {
            world.apply_low_disk_space_warning();
            sync_client_ui_counters(counters, world);
        }
        NetEvent::MapItemData(update) => {
            world.apply_map_item_data(update);
        }
        NetEvent::MountScreenOpen(update) => {
            world.apply_mount_screen_open(update);
            sync_client_ui_counters(counters, world);
        }
        NetEvent::OpenBook(update) => {
            world.apply_open_book(update);
            sync_client_ui_counters(counters, world);
        }
        NetEvent::OpenSignEditor(update) => {
            world.apply_open_sign_editor(update);
            sync_client_ui_counters(counters, world);
        }
        NetEvent::PlaceGhostRecipe(update) => {
            world.apply_place_ghost_recipe(update);
            sync_client_ui_counters(counters, world);
        }
        NetEvent::ClearDialog => {
            world.apply_clear_dialog();
            sync_client_ui_counters(counters, world);
        }
        NetEvent::ShowDialog(update) => {
            world.apply_show_dialog(update);
            sync_client_ui_counters(counters, world);
        }
        NetEvent::Waypoint(update) => {
            world.apply_waypoint(update);
            sync_waypoint_counters(counters, world);
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
            world.apply_pong_response(update);
            sync_client_ui_counters(counters, world);
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
        NetEvent::DeleteChat(update) => {
            world.apply_delete_chat(update);
            sync_chat_counters(counters, world);
        }
        NetEvent::DisguisedChat(update) => {
            world.apply_disguised_chat(update);
            sync_chat_counters(counters, world);
        }
        NetEvent::PlayerChat(update) => {
            world.apply_player_chat(update);
            sync_chat_counters(counters, world);
        }
        NetEvent::GameRuleValues(update) => {
            counters.last_game_rule_values = Some(bbb_control::GameRuleValuesState {
                values: update.values.len(),
            });
            counters.game_rule_value_packets += 1;
        }
        NetEvent::GameTestHighlightPos(update) => {
            counters.last_game_test_highlight_pos = Some(bbb_control::GameTestHighlightPosState {
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
            world.apply_transfer(transfer);
            sync_transfer_counters(counters, world);
        }
        NetEvent::PacketSeen { .. } => {
            counters.packets_seen += 1;
        }
        NetEvent::SelectAdvancementsTab(update) => {
            world.apply_select_advancements_tab(update);
            let world_counters = world.counters();
            counters.selected_advancements_tab =
                world.selected_advancements_tab().map(str::to_string);
            counters.select_advancements_tab_packets =
                world_counters.select_advancements_tab_packets;
        }
        NetEvent::TagQuery(update) => {
            world.apply_tag_query(update);
            sync_tag_query_counters(counters, world);
        }
        other => return Some(other),
    }

    None
}

pub(super) fn sync_registry_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.registries_seen = world_counters.registries_seen;
    counters.registry_entries_seen = world_counters.registry_entries_seen;
    counters.registry_entries_with_data = world_counters.registry_entries_with_data;
    counters.registry_entry_stubs = world_counters.registry_entry_stubs;
    counters.registry_entry_payload_bytes = world_counters.registry_entry_payload_bytes;
    counters.registry_content_registries_tracked =
        world_counters.registry_content_registries_tracked;
    counters.registry_content_packets_tracked = world_counters.registry_content_packets_tracked;
    counters.registry_content_entries_tracked = world_counters.registry_content_entries_tracked;
    counters.registry_duplicate_entries = world_counters.registry_duplicate_entries;
    counters.registry_duplicate_entry_ids_tracked =
        world_counters.registry_duplicate_entry_ids_tracked;
    counters.last_registry_data_registry = world_counters.last_registry_data_registry.clone();
    counters.last_registry_data_entry_count = world_counters.last_registry_data_entry_count;
}

fn sync_enabled_feature_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.update_enabled_features_packets = world_counters.update_enabled_features_packets;
    counters.enabled_features = world.enabled_feature_list();
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

fn control_waypoint_vec3i(pos: bbb_world::WaypointVec3iState) -> bbb_control::NetVec3i {
    bbb_control::NetVec3i {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

fn sync_chat_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.player_chat_packets = world_counters.player_chat_packets;
    counters.disguised_chat_packets = world_counters.disguised_chat_packets;
    counters.delete_chat_packets = world_counters.delete_chat_packets;
    counters.chat_messages_tracked = world_counters.chat_messages_tracked;
    counters.deleted_chat_messages_tracked = world_counters.deleted_chat_messages_tracked;
    counters.chat_signature_cache_entries = world_counters.chat_signature_cache_entries;
    counters.player_chat_index_mismatches = world_counters.player_chat_index_mismatches;
    counters.chat_unknown_packed_signatures = world_counters.chat_unknown_packed_signatures;
    counters.player_chat_unsigned_content_packets =
        world_counters.player_chat_unsigned_content_packets;
    counters.player_chat_filtered_packets = world_counters.player_chat_filtered_packets;
    counters.player_chat_fully_filtered_packets = world_counters.player_chat_fully_filtered_packets;

    counters.last_player_chat = world
        .client_chat()
        .messages
        .iter()
        .rev()
        .find(|message| message.kind == bbb_world::ChatMessageKind::Player)
        .map(control_chat_line);
    counters.last_disguised_chat = world
        .client_chat()
        .messages
        .iter()
        .rev()
        .find(|message| message.kind == bbb_world::ChatMessageKind::Disguised)
        .map(control_chat_line);
    counters.last_deleted_chat = world
        .client_chat()
        .deleted_messages
        .last()
        .map(control_deleted_chat_line);
}

fn sync_custom_payload_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.custom_payload_packets = world_counters.custom_payload_packets;
    counters.custom_payload_brand_packets = world_counters.custom_payload_brand_packets;
    counters.custom_payload_unknown_packets = world_counters.custom_payload_unknown_packets;
    counters.last_custom_payload = world
        .last_custom_payload()
        .map(control_custom_payload_state);
}

fn sync_custom_report_detail_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.custom_report_detail_packets = world_counters.custom_report_detail_packets;
    counters.custom_report_details = world.custom_report_details().clone();
}

fn sync_custom_chat_completion_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.custom_chat_completion_packets = world_counters.custom_chat_completion_packets;
    counters.last_custom_chat_completion =
        world.last_custom_chat_completion_update().map(|update| {
            bbb_control::CustomChatCompletionState {
                action: update.action.clone(),
                entries: update.entries,
            }
        });
}

fn sync_tag_query_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.tag_query_packets = world_counters.tag_query_packets;
    counters.last_tag_query = world
        .last_tag_query()
        .map(|query| bbb_control::TagQueryState {
            transaction_id: query.transaction_id,
            tag_present: query.tag_present,
            raw_nbt_len: query.raw_nbt_len(),
        });
}

fn control_custom_payload_state(
    state: &bbb_world::CustomPayloadState,
) -> bbb_control::CustomPayloadState {
    bbb_control::CustomPayloadState {
        id: state.id.clone(),
        kind: state.kind.clone(),
        brand: state.brand.clone(),
        raw_payload_len: state.raw_payload_len,
    }
}

fn sync_client_ui_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.low_disk_space_warnings = world_counters.low_disk_space_warnings;
    counters.clear_dialog_packets = world_counters.clear_dialog_packets;
    counters.show_dialog_packets = world_counters.show_dialog_packets;
    counters.mount_screen_open_packets = world_counters.mount_screen_open_packets;
    counters.open_book_packets = world_counters.open_book_packets;
    counters.open_sign_editor_packets = world_counters.open_sign_editor_packets;
    counters.ghost_recipe_packets = world_counters.ghost_recipe_packets;
    counters.pong_response_packets = world_counters.pong_response_packets;

    counters.last_mount_screen =
        world
            .last_mount_screen()
            .map(|state| bbb_control::MountScreenState {
                container_id: state.container_id,
                inventory_columns: state.inventory_columns,
                entity_id: state.entity_id,
            });
    counters.last_open_book_hand = world.last_open_book().map(|state| state.hand.clone());
    counters.last_open_sign_editor =
        world
            .last_open_sign_editor()
            .map(|state| bbb_control::OpenSignEditorState {
                pos: state.pos,
                is_front_text: state.is_front_text,
            });
    counters.last_show_dialog = world
        .current_dialog()
        .map(|state| bbb_control::ShowDialogState {
            holder_kind: state.holder_kind.clone(),
            registry_id: state.registry_id,
            raw_dialog_payload_len: state.raw_dialog_payload_len,
        });
    counters.last_ghost_recipe =
        world
            .last_ghost_recipe()
            .map(|state| bbb_control::GhostRecipeState {
                container_id: state.container_id,
                recipe_display_type_id: state.recipe_display_type_id,
                recipe_display_type: state.recipe_display_type.clone(),
                recipe_display_body_len: state.recipe_display_body_len,
            });
    counters.last_pong_response_time = world.last_pong_response().map(|state| state.time);
}

fn sync_waypoint_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.waypoint_packets = world_counters.waypoint_packets;
    counters.waypoints_tracked = world_counters.waypoints_tracked;
    counters.waypoint_updates_applied = world_counters.waypoint_updates_applied;
    counters.waypoint_updates_ignored = world_counters.waypoint_updates_ignored;
    counters.waypoint_untracks_ignored = world_counters.waypoint_untracks_ignored;
    counters.last_waypoint = world.last_waypoint_event().map(control_waypoint_event);
}

fn control_waypoint_event(event: &bbb_world::WaypointEventState) -> bbb_control::WaypointState {
    let waypoint = &event.waypoint;
    bbb_control::WaypointState {
        operation: event.operation.clone(),
        identifier_kind: waypoint.identifier_kind.clone(),
        identifier: waypoint.identifier.clone(),
        icon_style: waypoint.icon_style.clone(),
        icon_color_rgb: waypoint.icon_color_rgb,
        waypoint_kind: waypoint.data.kind.clone(),
        position: waypoint.data.position.map(control_waypoint_vec3i),
        chunk: waypoint.data.chunk,
        azimuth: waypoint.data.azimuth,
    }
}

fn sync_transfer_counters(counters: &mut NetCounters, world: &WorldStore) {
    counters.transfer_packets = world.counters().transfer_packets;
    counters.last_transfer = world.last_transfer().map(control_transfer_target);
}

fn control_transfer_target(state: &bbb_world::TransferTargetState) -> bbb_control::TransferTarget {
    bbb_control::TransferTarget {
        host: state.host.clone(),
        port: state.port,
    }
}

fn sync_server_link_counters(counters: &mut NetCounters, world: &WorldStore) {
    let world_counters = world.counters();
    counters.server_link_packets = world_counters.server_link_packets;
    counters.server_link_invalid_entries = world_counters.server_link_invalid_entries;
    counters.server_links = world
        .server_links()
        .iter()
        .map(|link| bbb_control::ServerLinkState {
            label: link.label.clone(),
            url: link.url.clone(),
            known_type: link.known_type.clone(),
        })
        .collect();
}

fn control_chat_line(message: &bbb_world::ChatMessageState) -> bbb_control::ClientChatLine {
    bbb_control::ClientChatLine {
        kind: message.kind.as_str().to_string(),
        content: message.content.clone(),
        sender: message.sender.map(|sender| sender.to_string()),
        sender_name: message.sender_name.clone(),
        target_name: message.target_name.clone(),
        global_index: message.global_index,
        message_index: message.message_index,
        chat_type_id: message.chat_type.registry_id,
        signature_checksum: message
            .signature
            .as_ref()
            .map(|signature| signature.checksum),
        unsigned_content_present: message.unsigned_content.is_some(),
        filter_mask: message.filter_mask.clone(),
        validation_state: message.validation_state.as_str().to_string(),
    }
}

fn control_deleted_chat_line(
    deleted: &bbb_world::DeletedChatState,
) -> bbb_control::DeletedChatLine {
    bbb_control::DeletedChatLine {
        signature_checksum: deleted
            .signature
            .as_ref()
            .map(|signature| signature.checksum),
        cache_id: deleted.cache_id,
        resolved: deleted.resolved,
    }
}
