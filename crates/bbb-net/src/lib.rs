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

mod connection;
mod driver;
mod probe;
mod status;
mod types;

use connection::{play_tick_interval, RawConnection};
use driver::{maybe_send_perform_respawn, read_packet_or_drive_connection, ConnectionDrive};
pub use probe::run_offline_probe;
pub use status::status_ping;
pub use types::{
    ChunkProbeSummary, ConnectionOptions, ConnectionState, NetCommand, NetEvent, PlayerMoveCommand,
    ProbeReport, StatusPing, VehicleMoveCommand,
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
                    ConfigurationClientbound::SelectKnownPacks { .. } => {
                        let (id, payload) = packets::encode_select_known_packs_empty();
                        conn.send_packet(id, &payload).await?;
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
                PlayClientbound::DamageEvent(update) => {
                    emit(&events, NetEvent::DamageEvent(update)).await?;
                }
                PlayClientbound::UpdateMobEffect(update) => {
                    emit(&events, NetEvent::UpdateMobEffect(update)).await?;
                }
                PlayClientbound::RemoveMobEffect(update) => {
                    emit(&events, NetEvent::RemoveMobEffect(update)).await?;
                }
                PlayClientbound::MoveEntity(update) => {
                    emit(&events, NetEvent::MoveEntity(update)).await?;
                }
                PlayClientbound::MoveVehicle(update) => {
                    emit(&events, NetEvent::MoveVehicle(update)).await?;
                }
                PlayClientbound::KeepAlive { id } => {
                    let (id, payload) = packets::encode_play_keep_alive(id);
                    conn.send_packet(id, &payload).await?;
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
                PlayClientbound::OpenScreen(update) => {
                    emit(&events, NetEvent::OpenScreen(update)).await?;
                }
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
                    emit(&events, NetEvent::StateChanged { state }).await?;
                }
                PlayClientbound::Login(login) => {
                    emit(&events, NetEvent::Login(login)).await?;
                }
                PlayClientbound::Respawn(respawn) => {
                    player_was_dead = false;
                    emit(&events, NetEvent::Respawn(respawn)).await?;
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
                PlayClientbound::SetTitlesAnimation(animation) => {
                    emit(&events, NetEvent::SetTitlesAnimation(animation)).await?;
                }
                PlayClientbound::TickingState(ticking) => {
                    emit(&events, NetEvent::TickingState(ticking)).await?;
                }
                PlayClientbound::TickingStep(step) => {
                    emit(&events, NetEvent::TickingStep(step)).await?;
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
                PlayClientbound::TabList(update) => {
                    emit(&events, NetEvent::TabList(update)).await?;
                }
                PlayClientbound::LevelChunkWithLight(chunk) => {
                    emit(&events, NetEvent::LevelChunkWithLight(chunk)).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::{
        send_command_suggestion_request, send_pick_item_from_block, send_player_action,
        send_player_command, send_player_input_command, send_set_held_slot_command,
        send_swing_command, send_use_item, send_use_item_on,
    };
    use bbb_protocol::{
        codec::Decoder,
        ids,
        packets::{
            CommandSuggestionRequest, InteractionHand, PlayerAction, PlayerCommand, PlayerHealth,
            PlayerInput, Vec3d,
        },
    };
    use bytes::BytesMut;
    use std::time::Duration;

    #[tokio::test]
    async fn drive_connection_disconnects_when_command_channel_closes_before_play() {
        let (mut conn, server) = raw_connection_pair().await;
        let (tx, mut commands) = mpsc::channel(1);
        drop(tx);
        let mut play_tick = None;
        let mut player_position_state = PlayerPositionState::default();

        let result = timeout(
            Duration::from_secs(1),
            read_packet_or_drive_connection(
                &mut conn,
                ConnectionState::Login,
                &mut play_tick,
                &mut commands,
                &mut player_position_state,
            ),
        )
        .await
        .expect("drive should not hang")
        .unwrap();

        assert!(matches!(result, ConnectionDrive::Disconnect));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn drive_connection_honors_disconnect_command() {
        let (mut conn, server) = raw_connection_pair().await;
        let (tx, mut commands) = mpsc::channel(1);
        tx.send(NetCommand::Disconnect).await.unwrap();
        let mut play_tick = None;
        let mut player_position_state = PlayerPositionState::default();

        let result = timeout(
            Duration::from_secs(1),
            read_packet_or_drive_connection(
                &mut conn,
                ConnectionState::Configuration,
                &mut play_tick,
                &mut commands,
                &mut player_position_state,
            ),
        )
        .await
        .expect("drive should not hang")
        .unwrap();

        assert!(matches!(result, ConnectionDrive::Disconnect));
        server.await.unwrap();
    }

    #[test]
    fn player_move_command_encodes_pos_rot_packet() {
        let command = PlayerMoveCommand {
            state: PlayerPositionState {
                position: Vec3d {
                    x: 1.25,
                    y: 64.5,
                    z: -8.75,
                },
                delta_movement: Vec3d {
                    x: 0.1,
                    y: 0.0,
                    z: -0.2,
                },
                y_rot: 90.0,
                x_rot: -15.0,
            },
            on_ground: true,
            horizontal_collision: true,
        };

        let (packet_id, payload) = command.encode_packet();

        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_f64().unwrap(), 1.25);
        assert_eq!(decoder.read_f64().unwrap(), 64.5);
        assert_eq!(decoder.read_f64().unwrap(), -8.75);
        assert_eq!(decoder.read_f32().unwrap(), 90.0);
        assert_eq!(decoder.read_f32().unwrap(), -15.0);
        assert_eq!(decoder.read_u8().unwrap(), 0b11);
        assert!(decoder.is_empty());
    }

    #[test]
    fn vehicle_move_command_encodes_move_vehicle_packet() {
        let command = VehicleMoveCommand {
            position: Vec3d {
                x: 2.5,
                y: 70.0,
                z: -9.25,
            },
            y_rot: 180.0,
            x_rot: 12.5,
            on_ground: true,
        };

        let (packet_id, payload) = command.encode_packet();

        assert_eq!(packet_id, ids::play::SERVERBOUND_MOVE_VEHICLE);
        assert_eq!(payload.len(), 33);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_f64().unwrap(), 2.5);
        assert_eq!(decoder.read_f64().unwrap(), 70.0);
        assert_eq!(decoder.read_f64().unwrap(), -9.25);
        assert_eq!(decoder.read_f32().unwrap(), 180.0);
        assert_eq!(decoder.read_f32().unwrap(), 12.5);
        assert_eq!(decoder.read_bool().unwrap(), true);
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn send_player_action_encodes_player_action_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("player action should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_ACTION);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            let pos = decoder.read_i64().unwrap();
            assert_eq!(
                bbb_protocol::packets::BlockPos {
                    x: (pos >> 38) as i32,
                    y: ((pos << 52) >> 52) as i32,
                    z: ((pos << 26) >> 38) as i32,
                },
                bbb_protocol::packets::BlockPos { x: 1, y: 64, z: -2 }
            );
            assert_eq!(decoder.read_u8().unwrap(), 4);
            assert_eq!(decoder.read_var_i32().unwrap(), 9);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_player_action(
            &mut conn,
            PlayerAction {
                action: bbb_protocol::packets::PlayerActionKind::StartDestroyBlock,
                pos: bbb_protocol::packets::BlockPos { x: 1, y: 64, z: -2 },
                direction: bbb_protocol::packets::Direction::West,
                sequence: 9,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_player_command_encodes_player_command_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("player command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_COMMAND);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 123);
            assert_eq!(decoder.read_var_i32().unwrap(), 2);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_player_command(
            &mut conn,
            PlayerCommand {
                entity_id: 123,
                action: bbb_protocol::packets::PlayerCommandAction::StopSprinting,
                data: 0,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_player_input_command_encodes_input_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("player input command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_PLAYER_INPUT);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_u8().unwrap(), 0b0111_0001);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_player_input_command(
            &mut conn,
            PlayerInput {
                forward: true,
                backward: false,
                left: false,
                right: false,
                jump: true,
                shift: true,
                sprint: true,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_set_held_slot_command_encodes_carried_item_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("held-slot command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_SET_CARRIED_ITEM);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_i16().unwrap(), 8);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_set_held_slot_command(&mut conn, 12).await.unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_swing_command_encodes_swing_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("swing command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_SWING);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_swing_command(&mut conn, InteractionHand::MainHand)
            .await
            .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_use_item_on_encodes_use_item_on_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("use item on command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_USE_ITEM_ON);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            let pos = decoder.read_i64().unwrap();
            assert_eq!(
                bbb_protocol::packets::BlockPos {
                    x: (pos >> 38) as i32,
                    y: ((pos << 52) >> 52) as i32,
                    z: ((pos << 26) >> 38) as i32,
                },
                bbb_protocol::packets::BlockPos {
                    x: -5,
                    y: 70,
                    z: 12
                }
            );
            assert_eq!(decoder.read_var_i32().unwrap(), 3);
            assert_eq!(decoder.read_f32().unwrap(), 0.25);
            assert_eq!(decoder.read_f32().unwrap(), 0.5);
            assert_eq!(decoder.read_f32().unwrap(), 0.75);
            assert!(!decoder.read_bool().unwrap());
            assert!(!decoder.read_bool().unwrap());
            assert_eq!(decoder.read_var_i32().unwrap(), 4);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_use_item_on(
            &mut conn,
            bbb_protocol::packets::UseItemOn {
                hand: InteractionHand::MainHand,
                hit: bbb_protocol::packets::BlockHitResult {
                    pos: bbb_protocol::packets::BlockPos {
                        x: -5,
                        y: 70,
                        z: 12,
                    },
                    direction: bbb_protocol::packets::Direction::South,
                    cursor_x: 0.25,
                    cursor_y: 0.5,
                    cursor_z: 0.75,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 4,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_use_item_encodes_use_item_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("use item command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_USE_ITEM);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 1);
            assert_eq!(decoder.read_var_i32().unwrap(), 8);
            assert_eq!(decoder.read_f32().unwrap(), 45.0);
            assert_eq!(decoder.read_f32().unwrap(), -20.0);
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_use_item(
            &mut conn,
            bbb_protocol::packets::UseItem {
                hand: InteractionHand::OffHand,
                sequence: 8,
                y_rot: 45.0,
                x_rot: -20.0,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_pick_item_from_block_encodes_pick_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("pick item from block command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_PICK_ITEM_FROM_BLOCK);
            let mut decoder = Decoder::new(&payload);
            let pos = decoder.read_i64().unwrap();
            assert_eq!(
                bbb_protocol::packets::BlockPos {
                    x: (pos >> 38) as i32,
                    y: ((pos << 52) >> 52) as i32,
                    z: ((pos << 26) >> 38) as i32,
                },
                bbb_protocol::packets::BlockPos {
                    x: -5,
                    y: 70,
                    z: 12
                }
            );
            assert!(decoder.read_bool().unwrap());
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_pick_item_from_block(
            &mut conn,
            bbb_protocol::packets::PickItemFromBlock {
                pos: bbb_protocol::packets::BlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                include_data: true,
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn send_command_suggestion_request_encodes_command_suggestion_packet() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("command suggestion request should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_COMMAND_SUGGESTION);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 44);
            assert_eq!(
                decoder.read_string(32500).unwrap(),
                "/give @p minecraft:stone"
            );
            assert!(decoder.is_empty());
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();

        send_command_suggestion_request(
            &mut conn,
            CommandSuggestionRequest {
                id: 44,
                command: "/give @p minecraft:stone".to_string(),
            },
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    #[tokio::test]
    async fn death_health_sends_respawn_command_once_until_alive() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = RawConnection {
                stream,
                read_buf: BytesMut::new(),
                compression_threshold: None,
            };
            let (packet_id, payload) = timeout(Duration::from_secs(1), conn.read_packet())
                .await
                .expect("respawn command should be sent")
                .unwrap();
            assert_eq!(packet_id, ids::play::SERVERBOUND_CLIENT_COMMAND);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_var_i32().unwrap(), 0);
            assert!(decoder.is_empty());

            assert!(
                timeout(Duration::from_millis(100), conn.read_packet())
                    .await
                    .is_err(),
                "second dead health packet must not send another respawn"
            );
        });
        let mut conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();
        let mut player_was_dead = false;

        maybe_send_perform_respawn(
            &mut conn,
            PlayerHealth {
                health: 20.0,
                food: 20,
                saturation: 5.0,
            },
            &mut player_was_dead,
        )
        .await
        .unwrap();
        assert!(!player_was_dead);

        maybe_send_perform_respawn(
            &mut conn,
            PlayerHealth {
                health: 0.0,
                food: 18,
                saturation: 0.0,
            },
            &mut player_was_dead,
        )
        .await
        .unwrap();
        assert!(player_was_dead);

        maybe_send_perform_respawn(
            &mut conn,
            PlayerHealth {
                health: 0.0,
                food: 18,
                saturation: 0.0,
            },
            &mut player_was_dead,
        )
        .await
        .unwrap();

        server.await.unwrap();
    }

    async fn raw_connection_pair() -> (RawConnection, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
        });
        let conn = RawConnection::connect(&addr.to_string(), None)
            .await
            .unwrap();
        (conn, server)
    }
}

#[allow(dead_code)]
fn _keep_encode_packet_reachable(packet_id: i32, payload: &[u8]) -> Vec<u8> {
    encode_packet(packet_id, payload)
}
