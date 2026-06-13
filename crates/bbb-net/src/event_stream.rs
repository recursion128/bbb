use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Context, Result};
use bbb_protocol::{
    frame::encode_packet,
    ids,
    packets::{
        self, ClientIntent, ConfigurationClientbound, LoginClientbound, PlayClientbound,
        PlayerPositionState, ResourcePackResponseAction,
    },
};
use tokio::{sync::mpsc, time::timeout};

use crate::{
    connection::{play_tick_interval, RawConnection},
    driver::{maybe_send_perform_respawn, read_packet_or_drive_connection, ConnectionDrive},
    types::{ConnectionOptions, ConnectionState, NetCommand, NetEvent},
};

pub async fn run_offline_event_stream(
    options: ConnectionOptions,
    events: mpsc::Sender<NetEvent>,
    mut commands: mpsc::Receiver<NetCommand>,
) -> Result<()> {
    let mut conn = timeout(
        options.timeout,
        RawConnection::connect(&options.address, None),
    )
    .await
    .context("offline connect timed out")??;
    let mut state = ConnectionState::Login;
    let mut player_loaded_sent = false;
    let mut player_position_state = PlayerPositionState::default();
    let mut player_was_dead = false;
    let mut play_tick = None;
    let mut server_cookies = BTreeMap::<String, Vec<u8>>::new();

    emit(&events, NetEvent::Connected).await?;
    emit(&events, NetEvent::StateChanged { state }).await?;

    let (id, payload) = packets::encode_handshake(&options.host, options.port, ClientIntent::Login);
    conn.send_packet(id, &payload).await?;
    let (id, payload) = packets::encode_login_hello(&options.username, options.profile_id);
    conn.send_packet(id, &payload).await?;

    loop {
        let drive = read_packet_or_drive_connection(
            &mut conn,
            state,
            &mut play_tick,
            &mut commands,
            &mut player_position_state,
        )
        .await?;
        let ConnectionDrive::Packet(packet_id, payload) = drive else {
            return Ok(());
        };
        tracing::debug!(?state, packet_id, len = payload.len(), "clientbound packet");
        emit_best_effort(
            &events,
            NetEvent::PacketSeen {
                state,
                packet_id,
                len: payload.len(),
            },
        )?;

        match state {
            ConnectionState::Login => match packets::decode_login_clientbound(packet_id, &payload)?
            {
                LoginClientbound::Disconnect { raw_json } => {
                    bail!("login disconnected: {raw_json}")
                }
                LoginClientbound::EncryptionRequest => {
                    bail!("server requested encryption; offline-mode event stream cannot continue")
                }
                LoginClientbound::SetCompression { threshold } => {
                    conn.compression_threshold = Some(threshold);
                    emit(&events, NetEvent::CompressionSet { threshold }).await?;
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
                    let payload_present = payload.is_some();
                    let (id, response) =
                        packets::encode_login_cookie_response(&request.key, payload);
                    conn.send_packet(id, &response).await?;
                    emit(
                        &events,
                        NetEvent::CookieRequest {
                            key: request.key,
                            response_payload_present: payload_present,
                        },
                    )
                    .await?;
                }
                LoginClientbound::LoginFinished { .. } => {
                    let (id, payload) = packets::encode_login_acknowledged();
                    conn.send_packet(id, &payload).await?;
                    state = ConnectionState::Configuration;
                    emit(&events, NetEvent::StateChanged { state }).await?;

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
                        emit(&events, NetEvent::StateChanged { state }).await?;
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
                        emit(
                            &events,
                            NetEvent::RegistryData {
                                registry,
                                raw_payload_len,
                            },
                        )
                        .await?;
                    }
                    ConfigurationClientbound::UpdateTags(update) => {
                        emit(&events, NetEvent::UpdateTags(update)).await?;
                    }
                    ConfigurationClientbound::SelectKnownPacks { .. } => {
                        let (id, payload) = packets::encode_select_known_packs_empty();
                        conn.send_packet(id, &payload).await?;
                    }
                    ConfigurationClientbound::CookieRequest(request) => {
                        let payload = server_cookies.get(&request.key).map(Vec::as_slice);
                        let payload_present = payload.is_some();
                        let (id, response) =
                            packets::encode_configuration_cookie_response(&request.key, payload);
                        conn.send_packet(id, &response).await?;
                        emit(
                            &events,
                            NetEvent::CookieRequest {
                                key: request.key,
                                response_payload_present: payload_present,
                            },
                        )
                        .await?;
                    }
                    ConfigurationClientbound::StoreCookie(cookie) => {
                        let key = cookie.key;
                        let payload_len = cookie.payload.len();
                        server_cookies.insert(key.clone(), cookie.payload);
                        emit(
                            &events,
                            NetEvent::StoreCookie {
                                key,
                                payload_len,
                                stored_cookie_count: server_cookies.len(),
                            },
                        )
                        .await?;
                    }
                    ConfigurationClientbound::CustomReportDetails(details) => {
                        emit(&events, NetEvent::CustomReportDetails(details)).await?;
                    }
                    ConfigurationClientbound::ServerLinks(links) => {
                        emit(&events, NetEvent::ServerLinks(links)).await?;
                    }
                    ConfigurationClientbound::Transfer(transfer) => {
                        emit(&events, NetEvent::Transfer(transfer)).await?;
                    }
                    ConfigurationClientbound::Unknown { .. } => {}
                }
            }
            ConnectionState::Play => match packets::decode_play_clientbound(packet_id, &payload)? {
                PlayClientbound::BundleDelimiter => {}
                PlayClientbound::AddEntity(entity) => {
                    emit(&events, NetEvent::AddEntity(entity)).await?;
                }
                PlayClientbound::EntityAnimation(update) => {
                    emit(&events, NetEvent::EntityAnimation(update)).await?;
                }
                PlayClientbound::AwardStats(_) => {}
                PlayClientbound::BlockDestruction(update) => {
                    emit(&events, NetEvent::BlockDestruction(update)).await?;
                }
                PlayClientbound::BossEvent(update) => {
                    emit(&events, NetEvent::BossEvent(update)).await?;
                }
                PlayClientbound::ChangeDifficulty(update) => {
                    emit(&events, NetEvent::ChangeDifficulty(update)).await?;
                }
                PlayClientbound::Cooldown(update) => {
                    emit(&events, NetEvent::Cooldown(update)).await?;
                }
                PlayClientbound::CustomChatCompletions(update) => {
                    emit(&events, NetEvent::CustomChatCompletions(update)).await?;
                }
                PlayClientbound::CustomPayload(update) => {
                    emit(&events, NetEvent::CustomPayload(update)).await?;
                }
                PlayClientbound::DamageEvent(update) => {
                    emit(&events, NetEvent::DamageEvent(update)).await?;
                }
                PlayClientbound::DebugBlockValue(update) => {
                    emit(&events, NetEvent::DebugBlockValue(update)).await?;
                }
                PlayClientbound::DebugChunkValue(update) => {
                    emit(&events, NetEvent::DebugChunkValue(update)).await?;
                }
                PlayClientbound::DebugEntityValue(update) => {
                    emit(&events, NetEvent::DebugEntityValue(update)).await?;
                }
                PlayClientbound::DebugEvent(update) => {
                    emit(&events, NetEvent::DebugEvent(update)).await?;
                }
                PlayClientbound::DebugSample(update) => {
                    emit(&events, NetEvent::DebugSample(update)).await?;
                }
                PlayClientbound::DeleteChat(update) => {
                    emit(&events, NetEvent::DeleteChat(update)).await?;
                }
                PlayClientbound::DisguisedChat(update) => {
                    emit(&events, NetEvent::DisguisedChat(update)).await?;
                }
                PlayClientbound::UpdateMobEffect(update) => {
                    emit(&events, NetEvent::UpdateMobEffect(update)).await?;
                }
                PlayClientbound::UpdateTags(update) => {
                    emit(&events, NetEvent::UpdateTags(update)).await?;
                }
                PlayClientbound::RemoveMobEffect(update) => {
                    emit(&events, NetEvent::RemoveMobEffect(update)).await?;
                }
                PlayClientbound::MoveEntity(update) => {
                    emit(&events, NetEvent::MoveEntity(update)).await?;
                }
                PlayClientbound::MoveMinecartAlongTrack(update) => {
                    emit(&events, NetEvent::MoveMinecartAlongTrack(update)).await?;
                }
                PlayClientbound::MoveVehicle(update) => {
                    emit(&events, NetEvent::MoveVehicle(update)).await?;
                }
                PlayClientbound::KeepAlive { id } => {
                    let (id, payload) = packets::encode_play_keep_alive(id);
                    conn.send_packet(id, &payload).await?;
                }
                PlayClientbound::LowDiskSpaceWarning => {
                    emit(&events, NetEvent::LowDiskSpaceWarning).await?;
                }
                PlayClientbound::MapItemData(update) => {
                    emit(&events, NetEvent::MapItemData(update)).await?;
                }
                PlayClientbound::MountScreenOpen(update) => {
                    emit(&events, NetEvent::MountScreenOpen(update)).await?;
                }
                PlayClientbound::ChunkBatchStart => {}
                PlayClientbound::ChunkBatchFinished { .. } => {
                    let (id, payload) = packets::encode_play_chunk_batch_received(9.0);
                    conn.send_packet(id, &payload).await?;
                }
                PlayClientbound::ContainerClose(update) => {
                    emit(&events, NetEvent::ContainerClose(update)).await?;
                }
                PlayClientbound::ContainerSetContent(update) => {
                    emit(&events, NetEvent::ContainerSetContent(update)).await?;
                }
                PlayClientbound::ContainerSetData(update) => {
                    emit(&events, NetEvent::ContainerSetData(update)).await?;
                }
                PlayClientbound::ContainerSetSlot(update) => {
                    emit(&events, NetEvent::ContainerSetSlot(update)).await?;
                }
                PlayClientbound::MerchantOffers(update) => {
                    emit(&events, NetEvent::MerchantOffers(update)).await?;
                }
                PlayClientbound::CookieRequest(request) => {
                    let payload = server_cookies.get(&request.key).map(Vec::as_slice);
                    let payload_present = payload.is_some();
                    let (id, response) =
                        packets::encode_play_cookie_response(&request.key, payload);
                    conn.send_packet(id, &response).await?;
                    emit(
                        &events,
                        NetEvent::CookieRequest {
                            key: request.key,
                            response_payload_present: payload_present,
                        },
                    )
                    .await?;
                }
                PlayClientbound::CustomReportDetails(details) => {
                    emit(&events, NetEvent::CustomReportDetails(details)).await?;
                }
                PlayClientbound::ServerLinks(links) => {
                    emit(&events, NetEvent::ServerLinks(links)).await?;
                }
                PlayClientbound::OpenScreen(update) => {
                    emit(&events, NetEvent::OpenScreen(update)).await?;
                }
                PlayClientbound::OpenBook(update) => {
                    emit(&events, NetEvent::OpenBook(update)).await?;
                }
                PlayClientbound::OpenSignEditor(update) => {
                    emit(&events, NetEvent::OpenSignEditor(update)).await?;
                }
                PlayClientbound::Disconnect(disconnect) => {
                    bail!("play disconnected: {}", disconnect.reason)
                }
                PlayClientbound::Ping { id } => {
                    let (id, payload) = packets::encode_play_pong(id);
                    conn.send_packet(id, &payload).await?;
                }
                PlayClientbound::PongResponse(update) => {
                    emit(&events, NetEvent::PongResponse(update)).await?;
                }
                PlayClientbound::PlaceGhostRecipe(update) => {
                    emit(&events, NetEvent::PlaceGhostRecipe(update)).await?;
                }
                PlayClientbound::StartConfiguration => {
                    let (id, payload) = packets::encode_play_configuration_acknowledged();
                    conn.send_packet(id, &payload).await?;
                    state = ConnectionState::Configuration;
                    play_tick = None;
                    emit(&events, NetEvent::StateChanged { state }).await?;
                }
                PlayClientbound::StoreCookie(cookie) => {
                    let key = cookie.key;
                    let payload_len = cookie.payload.len();
                    server_cookies.insert(key.clone(), cookie.payload);
                    emit(
                        &events,
                        NetEvent::StoreCookie {
                            key,
                            payload_len,
                            stored_cookie_count: server_cookies.len(),
                        },
                    )
                    .await?;
                }
                PlayClientbound::Login(login) => {
                    emit(&events, NetEvent::Login(login)).await?;
                }
                PlayClientbound::Respawn(respawn) => {
                    player_was_dead = false;
                    emit(&events, NetEvent::Respawn(respawn)).await?;
                }
                PlayClientbound::PlayerCombatEnd(update) => {
                    emit(&events, NetEvent::PlayerCombatEnd(update)).await?;
                }
                PlayClientbound::PlayerCombatEnter => {
                    emit(&events, NetEvent::PlayerCombatEnter).await?;
                }
                PlayClientbound::PlayerCombatKill(update) => {
                    emit(&events, NetEvent::PlayerCombatKill(update)).await?;
                }
                PlayClientbound::PlayerChat(update) => {
                    emit(&events, NetEvent::PlayerChat(update)).await?;
                }
                PlayClientbound::SetHealth(health) => {
                    maybe_send_perform_respawn(&mut conn, health, &mut player_was_dead).await?;
                    emit(&events, NetEvent::PlayerHealth(health)).await?;
                }
                PlayClientbound::SetExperience(experience) => {
                    emit(&events, NetEvent::PlayerExperience(experience)).await?;
                }
                PlayClientbound::SetHeldSlot(slot) => {
                    emit(&events, NetEvent::HeldSlot(slot)).await?;
                }
                PlayClientbound::SetCursorItem(update) => {
                    emit(&events, NetEvent::SetCursorItem(update)).await?;
                }
                PlayClientbound::SetPlayerInventory(update) => {
                    emit(&events, NetEvent::SetPlayerInventory(update)).await?;
                }
                PlayClientbound::GameEvent(event) => {
                    emit(&events, NetEvent::GameEvent(event)).await?;
                }
                PlayClientbound::GameRuleValues(update) => {
                    emit(&events, NetEvent::GameRuleValues(update)).await?;
                }
                PlayClientbound::GameTestHighlightPos(update) => {
                    emit(&events, NetEvent::GameTestHighlightPos(update)).await?;
                }
                PlayClientbound::SetTime(time) => {
                    emit(&events, NetEvent::SetTime(time)).await?;
                }
                PlayClientbound::BlockChangedAck(ack) => {
                    emit(&events, NetEvent::BlockChangedAck(ack)).await?;
                }
                PlayClientbound::BlockEntityData(update) => {
                    emit(&events, NetEvent::BlockEntityData(update)).await?;
                }
                PlayClientbound::BlockEvent(event) => {
                    emit(&events, NetEvent::BlockEvent(event)).await?;
                }
                PlayClientbound::PlayerLookAt(update) => {
                    emit(&events, NetEvent::PlayerLookAt(update)).await?;
                }
                PlayClientbound::LevelEvent(event) => {
                    emit(&events, NetEvent::LevelEvent(event)).await?;
                }
                PlayClientbound::PlayerPosition(update) => {
                    player_position_state = update.apply_to_state(player_position_state);
                    emit(&events, NetEvent::PlayerPosition(update)).await?;
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
                    emit(&events, NetEvent::PlayerRotation(update)).await?;
                }
                PlayClientbound::PlayerInfoUpdate(update) => {
                    emit(&events, NetEvent::PlayerInfoUpdate(update)).await?;
                }
                PlayClientbound::PlayerInfoRemove(update) => {
                    emit(&events, NetEvent::PlayerInfoRemove(update)).await?;
                }
                PlayClientbound::ServerData(update) => {
                    emit(&events, NetEvent::ServerData(update)).await?;
                }
                PlayClientbound::ResourcePackPush(update) => {
                    let (id, payload) = packets::encode_play_resource_pack_response(
                        update.id,
                        ResourcePackResponseAction::Declined,
                    );
                    conn.send_packet(id, &payload).await?;
                    emit(&events, NetEvent::ResourcePackPush(update)).await?;
                }
                PlayClientbound::ResourcePackPop(update) => {
                    emit(&events, NetEvent::ResourcePackPop(update)).await?;
                }
                PlayClientbound::EntityPositionSync(update) => {
                    emit(&events, NetEvent::EntityPositionSync(update)).await?;
                }
                PlayClientbound::Explosion(update) => {
                    emit(&events, NetEvent::Explosion(update)).await?;
                }
                PlayClientbound::EntityEvent(update) => {
                    emit(&events, NetEvent::EntityEvent(update)).await?;
                }
                PlayClientbound::HurtAnimation(update) => {
                    emit(&events, NetEvent::HurtAnimation(update)).await?;
                }
                PlayClientbound::RemoveEntities(update) => {
                    emit(&events, NetEvent::RemoveEntities(update)).await?;
                }
                PlayClientbound::RotateHead(update) => {
                    emit(&events, NetEvent::RotateHead(update)).await?;
                }
                PlayClientbound::SetEntityMotion(update) => {
                    emit(&events, NetEvent::SetEntityMotion(update)).await?;
                }
                PlayClientbound::SetEntityLink(update) => {
                    emit(&events, NetEvent::SetEntityLink(update)).await?;
                }
                PlayClientbound::SetEquipment(update) => {
                    emit(&events, NetEvent::SetEquipment(update)).await?;
                }
                PlayClientbound::TakeItemEntity(update) => {
                    emit(&events, NetEvent::TakeItemEntity(update)).await?;
                }
                PlayClientbound::SetPassengers(update) => {
                    emit(&events, NetEvent::SetPassengers(update)).await?;
                }
                PlayClientbound::UpdateAttributes(update) => {
                    emit(&events, NetEvent::UpdateAttributes(update)).await?;
                }
                PlayClientbound::SetEntityData(update) => {
                    emit(&events, NetEvent::SetEntityData(update)).await?;
                }
                PlayClientbound::TeleportEntity(update) => {
                    emit(&events, NetEvent::TeleportEntity(update)).await?;
                }
                PlayClientbound::PlayerAbilities(abilities) => {
                    emit(&events, NetEvent::PlayerAbilities(abilities)).await?;
                }
                PlayClientbound::SetDefaultSpawnPosition(spawn) => {
                    emit(&events, NetEvent::SetDefaultSpawnPosition(spawn)).await?;
                }
                PlayClientbound::SetSimulationDistance(distance) => {
                    emit(&events, NetEvent::SetSimulationDistance(distance)).await?;
                }
                PlayClientbound::SystemChat(chat) => {
                    emit(&events, NetEvent::SystemChat(chat)).await?;
                }
                PlayClientbound::SetActionBarText(text) => {
                    emit(&events, NetEvent::SetActionBarText(text)).await?;
                }
                PlayClientbound::SetTitleText(text) => {
                    emit(&events, NetEvent::SetTitleText(text)).await?;
                }
                PlayClientbound::SetSubtitleText(text) => {
                    emit(&events, NetEvent::SetSubtitleText(text)).await?;
                }
                PlayClientbound::ClearTitles(clear) => {
                    emit(&events, NetEvent::ClearTitles(clear)).await?;
                }
                PlayClientbound::SetTitlesAnimation(animation) => {
                    emit(&events, NetEvent::SetTitlesAnimation(animation)).await?;
                }
                PlayClientbound::ShowDialog(update) => {
                    emit(&events, NetEvent::ShowDialog(update)).await?;
                }
                PlayClientbound::Sound(sound) => {
                    emit(&events, NetEvent::Sound(sound)).await?;
                }
                PlayClientbound::SoundEntity(sound) => {
                    emit(&events, NetEvent::SoundEntity(sound)).await?;
                }
                PlayClientbound::StopSound(stop) => {
                    emit(&events, NetEvent::StopSound(stop)).await?;
                }
                PlayClientbound::TickingState(ticking) => {
                    emit(&events, NetEvent::TickingState(ticking)).await?;
                }
                PlayClientbound::TickingStep(step) => {
                    emit(&events, NetEvent::TickingStep(step)).await?;
                }
                PlayClientbound::Transfer(transfer) => {
                    emit(&events, NetEvent::Transfer(transfer)).await?;
                }
                PlayClientbound::UpdateAdvancements(update) => {
                    emit(&events, NetEvent::UpdateAdvancements(update)).await?;
                }
                PlayClientbound::UpdateRecipes(update) => {
                    emit(&events, NetEvent::UpdateRecipes(update)).await?;
                }
                PlayClientbound::Waypoint(update) => {
                    emit(&events, NetEvent::Waypoint(update)).await?;
                }
                PlayClientbound::SetCamera(camera) => {
                    emit(&events, NetEvent::SetCamera(camera)).await?;
                }
                PlayClientbound::InitializeBorder(border) => {
                    emit(&events, NetEvent::InitializeBorder(border)).await?;
                }
                PlayClientbound::SetBorderCenter(update) => {
                    emit(&events, NetEvent::SetBorderCenter(update)).await?;
                }
                PlayClientbound::SetBorderLerpSize(update) => {
                    emit(&events, NetEvent::SetBorderLerpSize(update)).await?;
                }
                PlayClientbound::SetBorderSize(update) => {
                    emit(&events, NetEvent::SetBorderSize(update)).await?;
                }
                PlayClientbound::SetBorderWarningDelay(update) => {
                    emit(&events, NetEvent::SetBorderWarningDelay(update)).await?;
                }
                PlayClientbound::SetBorderWarningDistance(update) => {
                    emit(&events, NetEvent::SetBorderWarningDistance(update)).await?;
                }
                PlayClientbound::ResetScore(update) => {
                    emit(&events, NetEvent::ResetScore(update)).await?;
                }
                PlayClientbound::SetDisplayObjective(update) => {
                    emit(&events, NetEvent::SetDisplayObjective(update)).await?;
                }
                PlayClientbound::SetObjective(update) => {
                    emit(&events, NetEvent::SetObjective(update)).await?;
                }
                PlayClientbound::SetPlayerTeam(update) => {
                    emit(&events, NetEvent::SetPlayerTeam(update)).await?;
                }
                PlayClientbound::SetScore(update) => {
                    emit(&events, NetEvent::SetScore(update)).await?;
                }
                PlayClientbound::CommandSuggestions(update) => {
                    emit(&events, NetEvent::CommandSuggestions(update)).await?;
                }
                PlayClientbound::SelectAdvancementsTab(update) => {
                    emit(&events, NetEvent::SelectAdvancementsTab(update)).await?;
                }
                PlayClientbound::TabList(update) => {
                    emit(&events, NetEvent::TabList(update)).await?;
                }
                PlayClientbound::TagQuery(update) => {
                    emit(&events, NetEvent::TagQuery(update)).await?;
                }
                PlayClientbound::TestInstanceBlockStatus(update) => {
                    emit(&events, NetEvent::TestInstanceBlockStatus(update)).await?;
                }
                PlayClientbound::ClearDialog => {
                    emit(&events, NetEvent::ClearDialog).await?;
                }
                PlayClientbound::LevelChunkWithLight(chunk) => {
                    emit(&events, NetEvent::LevelChunkWithLight(chunk)).await?;
                }
                PlayClientbound::LevelParticles(update) => {
                    emit(&events, NetEvent::LevelParticles(update)).await?;
                }
                PlayClientbound::LightUpdate(update) => {
                    emit(&events, NetEvent::LightUpdate(update)).await?;
                }
                PlayClientbound::ChunksBiomes(update) => {
                    emit(&events, NetEvent::ChunksBiomes(update)).await?;
                }
                PlayClientbound::ForgetLevelChunk(update) => {
                    emit(&events, NetEvent::ForgetLevelChunk(update)).await?;
                }
                PlayClientbound::BlockUpdate(update) => {
                    emit(&events, NetEvent::BlockUpdate(update)).await?;
                }
                PlayClientbound::SectionBlocksUpdate(update) => {
                    emit(&events, NetEvent::SectionBlocksUpdate(update)).await?;
                }
                PlayClientbound::SetChunkCacheCenter(update) => {
                    emit(&events, NetEvent::SetChunkCacheCenter(update)).await?;
                }
                PlayClientbound::SetChunkCacheRadius(update) => {
                    emit(&events, NetEvent::SetChunkCacheRadius(update)).await?;
                }
                PlayClientbound::ProjectilePower(update) => {
                    emit(&events, NetEvent::ProjectilePower(update)).await?;
                }
                PlayClientbound::RecipeBookAdd(update) => {
                    emit(&events, NetEvent::RecipeBookAdd(update)).await?;
                }
                PlayClientbound::RecipeBookRemove(update) => {
                    emit(&events, NetEvent::RecipeBookRemove(update)).await?;
                }
                PlayClientbound::RecipeBookSettings(update) => {
                    emit(&events, NetEvent::RecipeBookSettings(update)).await?;
                }
                PlayClientbound::Unknown { .. } => {}
            },
            ConnectionState::Handshake | ConnectionState::Status => {
                unreachable!("event stream starts at login")
            }
        }
    }
}

async fn emit(events: &mpsc::Sender<NetEvent>, event: NetEvent) -> Result<()> {
    events
        .send(event)
        .await
        .map_err(|_| anyhow!("net event receiver dropped"))
}

fn emit_best_effort(events: &mpsc::Sender<NetEvent>, event: NetEvent) -> Result<()> {
    if events.capacity() <= 1024 {
        return Ok(());
    }

    match events.try_send(event) {
        Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => Ok(()),
        Err(mpsc::error::TrySendError::Closed(_)) => Err(anyhow!("net event receiver dropped")),
    }
}

#[allow(dead_code)]
fn _keep_encode_packet_reachable(packet_id: i32, payload: &[u8]) -> Vec<u8> {
    encode_packet(packet_id, payload)
}
