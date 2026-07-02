use anyhow::{bail, Context, Result};
use bbb_protocol::packets::{self, PlayClientbound};
use tokio::sync::oneshot;

use crate::{
    driver::send_chat_acknowledgement,
    event_stream::{emit, EventStreamContext},
    resource_pack::response_action_for_push,
    types::{ConnectionState, NetEvent},
};

impl EventStreamContext {
    pub(super) async fn handle_play_packet(&mut self, packet: PlayClientbound) -> Result<()> {
        match packet {
            PlayClientbound::BundleDelimiter => {}
            PlayClientbound::KeepAlive { id } => {
                let (id, payload) = packets::encode_play_keep_alive(id);
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::Ping { id } => {
                let (id, payload) = packets::encode_play_pong(id);
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::ChunkBatchStart => {
                self.chunk_batch_size.on_batch_start();
            }
            PlayClientbound::ChunkBatchFinished { batch_size } => {
                let desired_chunks_per_tick = self.chunk_batch_size.on_batch_finished(batch_size);
                let (id, payload) =
                    packets::encode_play_chunk_batch_received(desired_chunks_per_tick);
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::CookieRequest(request) => {
                let payload = self.server_cookies.get(&request.key).map(Vec::as_slice);
                let payload_present = payload.is_some();
                let (id, response) = packets::encode_play_cookie_response(&request.key, payload);
                self.conn.send_packet(id, &response).await?;
                emit(
                    &self.events,
                    NetEvent::CookieRequest {
                        key: request.key,
                        response_payload_present: payload_present,
                    },
                )
                .await?;
            }
            PlayClientbound::StoreCookie(cookie) => {
                let key = cookie.key;
                let payload_len = cookie.payload.len();
                self.server_cookies.insert(key.clone(), cookie.payload);
                emit(
                    &self.events,
                    NetEvent::StoreCookie {
                        key,
                        payload_len,
                        stored_cookie_count: self.server_cookies.len(),
                    },
                )
                .await?;
            }
            PlayClientbound::StartConfiguration => {
                let (ack_tx, ack_rx) = oneshot::channel();
                emit(
                    &self.events,
                    NetEvent::StartConfiguration {
                        pending_chat_acknowledgement: ack_tx,
                    },
                )
                .await?;
                if let Some(command) = ack_rx
                    .await
                    .context("chat acknowledgement flush dropped during play reconfiguration")?
                {
                    send_chat_acknowledgement(&mut self.conn, command).await?;
                }
                self.acknowledge_start_configuration().await?;
            }
            PlayClientbound::Disconnect(disconnect) => {
                bail!("play disconnected: {}", disconnect.reason)
            }
            PlayClientbound::ResourcePackPush(update) => {
                let pack_id = update.id;
                let action = response_action_for_push(&update);
                let (id, payload) = packets::encode_play_resource_pack_response(pack_id, action);
                self.conn.send_packet(id, &payload).await?;
                emit(
                    &self.events,
                    NetEvent::Play(PlayClientbound::ResourcePackPush(update)),
                )
                .await?;
                emit(
                    &self.events,
                    NetEvent::ResourcePackResponse {
                        id: pack_id,
                        action,
                    },
                )
                .await?;
            }
            PlayClientbound::PlayerPosition(update) => {
                self.player_position_state = update.apply_to_state(self.player_position_state);
                emit(
                    &self.events,
                    NetEvent::Play(PlayClientbound::PlayerPosition(update)),
                )
                .await?;
                let (id, payload) = packets::encode_play_accept_teleportation(update.id);
                self.conn.send_packet(id, &payload).await?;
                let (id, payload) = packets::encode_play_move_player_pos_rot(
                    self.player_position_state.position.x,
                    self.player_position_state.position.y,
                    self.player_position_state.position.z,
                    self.player_position_state.y_rot,
                    self.player_position_state.x_rot,
                    false,
                    false,
                );
                self.conn.send_packet(id, &payload).await?;
                if !self.player_loaded_sent {
                    let (id, payload) = packets::encode_play_player_loaded();
                    self.conn.send_packet(id, &payload).await?;
                    self.player_loaded_sent = true;
                }
            }
            PlayClientbound::PlayerRotation(update) => {
                self.player_position_state = update.apply_to_state(self.player_position_state);
                emit(
                    &self.events,
                    NetEvent::Play(PlayClientbound::PlayerRotation(update)),
                )
                .await?;
                let (id, payload) = packets::encode_play_move_player_rot(
                    self.player_position_state.y_rot,
                    self.player_position_state.x_rot,
                    false,
                    false,
                );
                self.conn.send_packet(id, &payload).await?;
            }
            PlayClientbound::Unknown { packet_id, len } => {
                emit(
                    &self.events,
                    NetEvent::UnsupportedPacket {
                        state: self.state,
                        packet_id,
                        len,
                    },
                )
                .await?;
            }
            packet => {
                emit(&self.events, NetEvent::Play(packet)).await?;
            }
        }
        Ok(())
    }

    async fn acknowledge_start_configuration(&mut self) -> Result<()> {
        let (id, payload) = packets::encode_play_configuration_acknowledged();
        self.conn.send_packet(id, &payload).await?;
        self.state = ConnectionState::Configuration;
        self.play_tick = None;
        self.seen_code_of_conduct = false;
        emit(&self.events, NetEvent::StateChanged { state: self.state }).await
    }
}
