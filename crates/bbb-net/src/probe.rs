use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use bbb_protocol::{
    ids,
    packets::{
        self, ClientIntent, ConfigurationClientbound, LoginClientbound, PlayClientbound,
        PlayerPositionState, ResourcePackResponseAction,
    },
};
use bbb_world::{BlockPos, ChunkPos, WorldStore};
use tokio::time::timeout;

use crate::{
    connection::{play_tick_interval, RawConnection},
    driver::{maybe_send_perform_respawn, read_packet_or_send_play_tick},
    types::{ChunkProbeSummary, ConnectionOptions, ConnectionState, ProbeReport},
};

pub async fn run_offline_probe(options: ConnectionOptions) -> Result<ProbeReport> {
    timeout(options.timeout, run_offline_probe_inner(options))
        .await
        .context("offline probe timed out")?
}

async fn run_offline_probe_inner(options: ConnectionOptions) -> Result<ProbeReport> {
    let mut conn = RawConnection::connect(&options.address, None).await?;
    let mut state = ConnectionState::Login;
    let mut packets_seen = 0usize;
    let mut world = WorldStore::new();
    let mut player_loaded_sent = false;
    let mut player_position_state = PlayerPositionState::default();
    let mut player_was_dead = false;
    let mut play_tick = None;
    let mut server_cookies = BTreeMap::<String, Vec<u8>>::new();

    let (id, payload) = packets::encode_handshake(&options.host, options.port, ClientIntent::Login);
    conn.send_packet(id, &payload).await?;
    let (id, payload) = packets::encode_login_hello(&options.username, options.profile_id);
    conn.send_packet(id, &payload).await?;

    let first_chunk = loop {
        let (packet_id, payload) =
            read_packet_or_send_play_tick(&mut conn, state, &mut play_tick).await?;
        packets_seen += 1;
        tracing::debug!(?state, packet_id, len = payload.len(), "clientbound packet");

        match state {
            ConnectionState::Login => match packets::decode_login_clientbound(packet_id, &payload)?
            {
                LoginClientbound::Disconnect { raw_json } => {
                    bail!("login disconnected: {raw_json}")
                }
                LoginClientbound::EncryptionRequest => {
                    bail!("server requested encryption; offline-mode probe cannot continue")
                }
                LoginClientbound::SetCompression { threshold } => {
                    conn.compression_threshold = Some(threshold);
                }
                LoginClientbound::CustomQuery { transaction_id } => {
                    let mut response = bbb_protocol::codec::Encoder::new();
                    response.write_var_i32(transaction_id);
                    response.write_bool(false);
                    conn.send_packet(
                        ids::login::SERVERBOUND_CUSTOM_QUERY_ANSWER,
                        &response.into_inner(),
                    )
                    .await?;
                }
                LoginClientbound::CookieRequest(request) => {
                    let payload = server_cookies.get(&request.key).map(Vec::as_slice);
                    let (id, response) =
                        packets::encode_login_cookie_response(&request.key, payload);
                    conn.send_packet(id, &response).await?;
                }
                LoginClientbound::LoginFinished { .. } => {
                    let (id, payload) = packets::encode_login_acknowledged();
                    conn.send_packet(id, &payload).await?;
                    state = ConnectionState::Configuration;

                    let (id, payload) = packets::encode_client_information_default();
                    conn.send_packet(id, &payload).await?;
                }
            },
            ConnectionState::Configuration => {
                match packets::decode_configuration_clientbound(packet_id, &payload)? {
                    ConfigurationClientbound::Finish => {
                        let (id, payload) = packets::encode_configuration_finish();
                        conn.send_packet(id, &payload).await?;
                        state = ConnectionState::Play;
                        let (id, payload) = packets::encode_play_client_information_default();
                        conn.send_packet(id, &payload).await?;
                        play_tick = Some(play_tick_interval());
                    }
                    ConfigurationClientbound::KeepAlive { id } => {
                        let (id, payload) = packets::encode_configuration_keep_alive(id);
                        conn.send_packet(id, &payload).await?;
                    }
                    ConfigurationClientbound::Ping { id } => {
                        let (id, payload) = packets::encode_configuration_pong(id);
                        conn.send_packet(id, &payload).await?;
                    }
                    ConfigurationClientbound::RegistryData {
                        registry,
                        raw_payload_len,
                    } => {
                        world.record_registry(registry, raw_payload_len);
                    }
                    ConfigurationClientbound::UpdateTags(update) => {
                        world.apply_update_tags(update);
                    }
                    ConfigurationClientbound::SelectKnownPacks { .. } => {
                        let (id, payload) = packets::encode_select_known_packs_empty();
                        conn.send_packet(id, &payload).await?;
                    }
                    ConfigurationClientbound::CookieRequest(request) => {
                        let payload = server_cookies.get(&request.key).map(Vec::as_slice);
                        let (id, response) =
                            packets::encode_configuration_cookie_response(&request.key, payload);
                        conn.send_packet(id, &response).await?;
                    }
                    ConfigurationClientbound::StoreCookie(cookie) => {
                        server_cookies.insert(cookie.key, cookie.payload);
                    }
                    ConfigurationClientbound::CustomReportDetails(_)
                    | ConfigurationClientbound::ServerLinks(_) => {}
                    ConfigurationClientbound::Transfer(_) => {}
                    ConfigurationClientbound::Unknown { .. } => {}
                }
            }
            ConnectionState::Play => match packets::decode_play_clientbound(packet_id, &payload)? {
                PlayClientbound::BundleDelimiter => {}
                PlayClientbound::AddEntity(entity) => {
                    world.apply_add_entity(entity);
                }
                PlayClientbound::EntityAnimation(update) => {
                    world.apply_entity_animation(update);
                }
                PlayClientbound::AwardStats(_) => {}
                PlayClientbound::BlockDestruction(update) => {
                    world.apply_block_destruction(update);
                }
                PlayClientbound::BossEvent(update) => {
                    world.apply_boss_event(update);
                }
                PlayClientbound::ChangeDifficulty(update) => {
                    world.apply_change_difficulty(update);
                }
                PlayClientbound::Cooldown(update) => {
                    world.apply_cooldown(update);
                }
                PlayClientbound::CustomChatCompletions(_)
                | PlayClientbound::CustomPayload(_)
                | PlayClientbound::CustomReportDetails(_)
                | PlayClientbound::ServerLinks(_) => {}
                PlayClientbound::DamageEvent(update) => {
                    world.apply_damage_event(update);
                }
                PlayClientbound::DebugBlockValue(_)
                | PlayClientbound::DebugChunkValue(_)
                | PlayClientbound::DebugEntityValue(_)
                | PlayClientbound::DebugEvent(_)
                | PlayClientbound::DebugSample(_) => {}
                PlayClientbound::DeleteChat(update) => {
                    world.apply_delete_chat(update);
                }
                PlayClientbound::DisguisedChat(update) => {
                    world.apply_disguised_chat(update);
                }
                PlayClientbound::UpdateMobEffect(update) => {
                    world.apply_update_mob_effect(update);
                }
                PlayClientbound::UpdateTags(update) => {
                    world.apply_update_tags(update);
                }
                PlayClientbound::RemoveMobEffect(update) => {
                    world.apply_remove_mob_effect(update);
                }
                PlayClientbound::MoveEntity(update) => {
                    world.apply_entity_move(update);
                }
                PlayClientbound::MoveMinecartAlongTrack(update) => {
                    world.apply_move_minecart_along_track(update);
                }
                PlayClientbound::MoveVehicle(update) => {
                    world.apply_move_vehicle(update);
                }
                PlayClientbound::KeepAlive { id } => {
                    let (id, payload) = packets::encode_play_keep_alive(id);
                    conn.send_packet(id, &payload).await?;
                }
                PlayClientbound::LowDiskSpaceWarning | PlayClientbound::MountScreenOpen(_) => {}
                PlayClientbound::ChunkBatchStart => {}
                PlayClientbound::ChunkBatchFinished { .. } => {
                    let (id, payload) = packets::encode_play_chunk_batch_received(9.0);
                    conn.send_packet(id, &payload).await?;
                }
                PlayClientbound::ContainerClose(update) => {
                    world.apply_container_close(update);
                }
                PlayClientbound::ContainerSetContent(update) => {
                    world.apply_container_set_content(update);
                }
                PlayClientbound::ContainerSetData(update) => {
                    world.apply_container_set_data(update);
                }
                PlayClientbound::ContainerSetSlot(update) => {
                    world.apply_container_set_slot(update);
                }
                PlayClientbound::MerchantOffers(update) => {
                    world.apply_merchant_offers(update);
                }
                PlayClientbound::CookieRequest(request) => {
                    let payload = server_cookies.get(&request.key).map(Vec::as_slice);
                    let (id, response) =
                        packets::encode_play_cookie_response(&request.key, payload);
                    conn.send_packet(id, &response).await?;
                }
                PlayClientbound::OpenScreen(update) => {
                    world.apply_open_screen(update);
                }
                PlayClientbound::OpenBook(_)
                | PlayClientbound::OpenSignEditor(_)
                | PlayClientbound::PlaceGhostRecipe(_)
                | PlayClientbound::PongResponse(_) => {}
                PlayClientbound::Disconnect(disconnect) => {
                    bail!("play disconnected: {}", disconnect.reason)
                }
                PlayClientbound::Ping { id } => {
                    let (id, payload) = packets::encode_play_pong(id);
                    conn.send_packet(id, &payload).await?;
                }
                PlayClientbound::StartConfiguration => {
                    let (id, payload) = packets::encode_play_configuration_acknowledged();
                    conn.send_packet(id, &payload).await?;
                    state = ConnectionState::Configuration;
                    play_tick = None;
                }
                PlayClientbound::StoreCookie(cookie) => {
                    server_cookies.insert(cookie.key, cookie.payload);
                }
                PlayClientbound::Login(login) => {
                    world.apply_login(&login);
                }
                PlayClientbound::Respawn(respawn) => {
                    player_was_dead = false;
                    world.apply_respawn(&respawn);
                }
                PlayClientbound::SetHealth(health) => {
                    maybe_send_perform_respawn(&mut conn, health, &mut player_was_dead).await?;
                }
                PlayClientbound::EntityPositionSync(update) => {
                    world.apply_entity_position_sync(update);
                }
                PlayClientbound::Explosion(_) => {}
                PlayClientbound::EntityEvent(update) => {
                    world.apply_entity_event(update);
                }
                PlayClientbound::HurtAnimation(update) => {
                    world.apply_hurt_animation(update);
                }
                PlayClientbound::RemoveEntities(update) => {
                    world.apply_remove_entities(update);
                }
                PlayClientbound::RotateHead(update) => {
                    world.apply_rotate_head(update);
                }
                PlayClientbound::SetEntityMotion(update) => {
                    world.apply_set_entity_motion(update);
                }
                PlayClientbound::SetEntityLink(update) => {
                    world.apply_set_entity_link(update);
                }
                PlayClientbound::SetEquipment(update) => {
                    world.apply_set_equipment(update);
                }
                PlayClientbound::TakeItemEntity(update) => {
                    world.apply_take_item_entity(update);
                }
                PlayClientbound::SetPassengers(update) => {
                    world.apply_set_passengers(update);
                }
                PlayClientbound::UpdateAttributes(update) => {
                    world.apply_update_attributes(update);
                }
                PlayClientbound::SetEntityData(update) => {
                    world.apply_set_entity_data(update);
                }
                PlayClientbound::TeleportEntity(update) => {
                    world.apply_teleport_entity(update);
                }
                PlayClientbound::PlayerAbilities(_) => {}
                PlayClientbound::PlayerChat(update) => {
                    world.apply_player_chat(update);
                }
                PlayClientbound::SetExperience(_) => {}
                PlayClientbound::SetHeldSlot(_) => {}
                PlayClientbound::SetCursorItem(update) => {
                    world.apply_set_cursor_item(update);
                }
                PlayClientbound::SetPlayerInventory(update) => {
                    world.apply_set_player_inventory(update);
                }
                PlayClientbound::SetDefaultSpawnPosition(_) => {}
                PlayClientbound::SetSimulationDistance(_) => {}
                PlayClientbound::SystemChat(_) => {}
                PlayClientbound::PlayerCombatEnd(_)
                | PlayClientbound::PlayerCombatEnter
                | PlayClientbound::PlayerCombatKill(_)
                | PlayClientbound::PlayerLookAt(_) => {}
                PlayClientbound::MapItemData(update) => {
                    world.apply_map_item_data(update);
                }
                PlayClientbound::SetActionBarText(_)
                | PlayClientbound::SetTitleText(_)
                | PlayClientbound::SetSubtitleText(_)
                | PlayClientbound::ClearTitles(_)
                | PlayClientbound::SetTitlesAnimation(_)
                | PlayClientbound::Sound(_)
                | PlayClientbound::SoundEntity(_)
                | PlayClientbound::StopSound(_)
                | PlayClientbound::TickingState(_)
                | PlayClientbound::TickingStep(_)
                | PlayClientbound::Transfer(_)
                | PlayClientbound::SetCamera(_) => {}
                PlayClientbound::InitializeBorder(border) => {
                    world.apply_initialize_border(border);
                }
                PlayClientbound::SetBorderCenter(update) => {
                    world.apply_set_border_center(update);
                }
                PlayClientbound::SetBorderLerpSize(update) => {
                    world.apply_set_border_lerp_size(update);
                }
                PlayClientbound::SetBorderSize(update) => {
                    world.apply_set_border_size(update);
                }
                PlayClientbound::SetBorderWarningDelay(update) => {
                    world.apply_set_border_warning_delay(update);
                }
                PlayClientbound::SetBorderWarningDistance(update) => {
                    world.apply_set_border_warning_distance(update);
                }
                PlayClientbound::ResetScore(update) => {
                    world.apply_reset_score(update);
                }
                PlayClientbound::SetDisplayObjective(update) => {
                    world.apply_set_display_objective(update);
                }
                PlayClientbound::SetObjective(update) => {
                    world.apply_set_objective(update);
                }
                PlayClientbound::SetPlayerTeam(update) => {
                    world.apply_set_player_team(update);
                }
                PlayClientbound::SetScore(update) => {
                    world.apply_set_score(update);
                }
                PlayClientbound::CommandSuggestions(update) => {
                    world.apply_command_suggestions(update);
                }
                PlayClientbound::SelectAdvancementsTab(_)
                | PlayClientbound::TagQuery(_)
                | PlayClientbound::ClearDialog
                | PlayClientbound::ShowDialog(_)
                | PlayClientbound::TestInstanceBlockStatus(_) => {}
                PlayClientbound::TabList(update) => {
                    world.apply_tab_list(update);
                }
                PlayClientbound::BlockChangedAck(_) => {}
                PlayClientbound::BlockEntityData(update) => {
                    world.apply_block_entity_data(update)?;
                }
                PlayClientbound::BlockEvent(event) => {
                    world.apply_block_event(event);
                }
                PlayClientbound::LevelEvent(event) => {
                    world.apply_level_event(event);
                }
                PlayClientbound::GameEvent(_)
                | PlayClientbound::GameRuleValues(_)
                | PlayClientbound::GameTestHighlightPos(_)
                | PlayClientbound::SetTime(_) => {}
                PlayClientbound::PlayerPosition(update) => {
                    player_position_state = update.apply_to_state(player_position_state);
                    let (id, payload) = packets::encode_play_accept_teleportation(update.id);
                    conn.send_packet(id, &payload).await?;
                    let (id, payload) = packets::encode_play_move_player_pos_rot(
                        player_position_state.position.x,
                        player_position_state.position.y,
                        player_position_state.position.z,
                        player_position_state.y_rot,
                        player_position_state.x_rot,
                        false,
                        false,
                    );
                    conn.send_packet(id, &payload).await?;
                    if !player_loaded_sent {
                        let (id, payload) = packets::encode_play_player_loaded();
                        conn.send_packet(id, &payload).await?;
                        player_loaded_sent = true;
                    }
                }
                PlayClientbound::PlayerRotation(update) => {
                    player_position_state = update.apply_to_state(player_position_state);
                }
                PlayClientbound::PlayerInfoUpdate(update) => {
                    world.apply_player_info_update(update);
                }
                PlayClientbound::PlayerInfoRemove(update) => {
                    world.apply_player_info_remove(update);
                }
                PlayClientbound::ServerData(update) => {
                    world.apply_server_data(update);
                }
                PlayClientbound::ResourcePackPush(update) => {
                    let (id, payload) = packets::encode_play_resource_pack_response(
                        update.id,
                        ResourcePackResponseAction::Declined,
                    );
                    conn.send_packet(id, &payload).await?;
                    world.apply_resource_pack_push(update);
                }
                PlayClientbound::ResourcePackPop(update) => {
                    world.apply_resource_pack_pop(update);
                }
                PlayClientbound::LevelChunkWithLight(chunk) => {
                    let pos = world.insert_level_chunk_with_light(chunk)?;
                    break pos;
                }
                PlayClientbound::LevelParticles(_) => {}
                PlayClientbound::LightUpdate(update) => {
                    world.apply_light_update(update)?;
                }
                PlayClientbound::ChunksBiomes(update) => {
                    world.apply_biome_update(update)?;
                }
                PlayClientbound::ForgetLevelChunk(update) => {
                    world.forget_chunk(ChunkPos {
                        x: update.pos.x,
                        z: update.pos.z,
                    });
                }
                PlayClientbound::BlockUpdate(update) => {
                    world.apply_block_update(update);
                }
                PlayClientbound::SectionBlocksUpdate(update) => {
                    world.apply_section_blocks_update(update);
                }
                PlayClientbound::SetChunkCacheCenter(_)
                | PlayClientbound::SetChunkCacheRadius(_)
                | PlayClientbound::ProjectilePower(_)
                | PlayClientbound::Waypoint(_) => {}
                PlayClientbound::RecipeBookAdd(update) => {
                    world.apply_recipe_book_add(update);
                }
                PlayClientbound::RecipeBookRemove(update) => {
                    world.apply_recipe_book_remove(update);
                }
                PlayClientbound::RecipeBookSettings(update) => {
                    world.apply_recipe_book_settings(update);
                }
                PlayClientbound::Unknown { .. } => {}
            },
            ConnectionState::Handshake | ConnectionState::Status => {
                unreachable!("probe starts at login")
            }
        }
    };

    let first_chunk_summary = world
        .probe_chunk(first_chunk)
        .map(ChunkProbeSummary::from_column);
    let first_chunk_center_block = world.probe_block(BlockPos {
        x: first_chunk.x * 16 + 8,
        y: 64,
        z: first_chunk.z * 16 + 8,
    });
    let world_counters = world.counters();

    Ok(ProbeReport {
        reached_state: state,
        compression_threshold: conn.compression_threshold,
        packets_seen,
        registries_seen: world_counters.registries_seen,
        first_chunk: Some(first_chunk),
        first_chunk_summary,
        first_chunk_center_block,
        world_counters,
        world,
    })
}
