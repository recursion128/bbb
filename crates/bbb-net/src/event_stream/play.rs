use anyhow::{bail, Result};
use bbb_protocol::packets::{self, PlayClientbound, Transfer};

use crate::{
    event_stream::{emit, EventStreamContext},
    resource_pack::response_action_for_push_with_policy,
    types::{ConnectionState, NetEvent, ServerResourcePackPolicy},
};

impl EventStreamContext {
    pub(super) async fn handle_play_packet(
        &mut self,
        packet: PlayClientbound,
    ) -> Result<Option<Transfer>> {
        self.handle_play_packet_with_resource_pack_policy(packet, ServerResourcePackPolicy::Decline)
            .await
    }

    pub(super) async fn handle_play_packet_with_resource_pack_policy(
        &mut self,
        packet: PlayClientbound,
        resource_pack_policy: ServerResourcePackPolicy,
    ) -> Result<Option<Transfer>> {
        let mut transfer = None;
        match packet {
            PlayClientbound::BundleDelimiter => {}
            PlayClientbound::KeepAlive { id } => {
                // Vanilla answers keep-alives on the connection thread
                // (ClientCommonPacketListenerImpl.handleKeepAlive has no
                // ensureRunningOnSameThread), so a stalled render loop must
                // not cost the session its liveness.
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
                let (id, response) = packets::encode_play_cookie_response(&request.key, payload)?;
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
                // The canonical world flushes its pending chat acknowledgement
                // and answers with NetCommand::AcknowledgeConfiguration; the
                // connection keeps reading packets until that answer arrives.
                emit(&self.events, NetEvent::StartConfiguration).await?;
            }
            PlayClientbound::Disconnect(disconnect) => {
                bail!("play disconnected: {}", disconnect.reason)
            }
            PlayClientbound::ResourcePackPush(update) => {
                let pack_id = update.id;
                let action = response_action_for_push_with_policy(&update, resource_pack_policy);
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
            PlayClientbound::Transfer(update) => {
                transfer = Some(update.clone());
                emit(
                    &self.events,
                    NetEvent::Play(PlayClientbound::Transfer(update)),
                )
                .await?;
            }
            PlayClientbound::PlayerPosition(update) => {
                // The canonical world applies the correction and answers with
                // NetCommand::AcceptTeleport; until then outgoing player moves
                // are gated while the connection keeps reading packets, so a
                // slow canonical apply cannot stall keep-alives or leak stale
                // pre-teleport positions.
                self.pending_teleports += 1;
                emit(&self.events, NetEvent::PlayerPosition { update }).await?;
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
            PlayClientbound::SetHeldSlot(update) => {
                if (0..=8).contains(&update.slot) {
                    self.conn
                        .remember_serverbound_carried_hotbar_slot(update.slot as u8);
                }
                emit(
                    &self.events,
                    NetEvent::Play(PlayClientbound::SetHeldSlot(update)),
                )
                .await?;
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
        Ok(transfer)
    }

    pub(super) async fn accept_teleport(
        &mut self,
        command: crate::types::AcceptTeleportCommand,
    ) -> Result<()> {
        let (id, payload) = packets::encode_play_accept_teleportation(command.teleport_id);
        self.conn.send_packet(id, &payload).await?;
        self.player_position_state = command.state;
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
        self.pending_teleports = self.pending_teleports.saturating_sub(1);
        Ok(())
    }

    pub(super) async fn acknowledge_start_configuration(&mut self) -> Result<()> {
        let (id, payload) = packets::encode_play_configuration_acknowledged();
        self.conn.send_packet(id, &payload).await?;
        self.state = ConnectionState::Configuration;
        self.seen_code_of_conduct = false;
        // Reconfiguration abandons the play session's teleport handshake; a
        // stale pending count must not gate moves in the next play state.
        self.pending_teleports = 0;
        emit(&self.events, NetEvent::StateChanged { state: self.state }).await
    }
}
