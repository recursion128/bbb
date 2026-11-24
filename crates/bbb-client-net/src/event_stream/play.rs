use anyhow::Result;
use bbb_protocol::packets::{self, PlayClientbound, Transfer};

use crate::{
    event_stream::{emit, should_forward_resource_pack_push, EventStreamContext},
    session::PlayPacketStep,
    types::{NetEvent, ServerResourcePackPolicy},
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
        match self
            .session
            .step_play_packet(packet, resource_pack_policy)
            .await?
        {
            PlayPacketStep::Handled
            | PlayPacketStep::ChunkBatchStart
            | PlayPacketStep::ChunkBatchFinished { .. } => {}
            PlayPacketStep::CookieRequest {
                key,
                response_payload_present,
            } => {
                emit(
                    &self.events,
                    NetEvent::CookieRequest {
                        key,
                        response_payload_present,
                    },
                )
                .await?;
            }
            PlayPacketStep::StoreCookie {
                key,
                payload_len,
                stored_cookie_count,
            } => {
                emit(
                    &self.events,
                    NetEvent::StoreCookie {
                        key,
                        payload_len,
                        stored_cookie_count,
                    },
                )
                .await?;
            }
            PlayPacketStep::StartConfiguration => {
                // The canonical world flushes its pending chat acknowledgement
                // and answers with NetCommand::AcknowledgeConfiguration; the
                // event stream pauses inbound packet reads until that answer
                // switches the protocol state to Configuration.
                emit(&self.events, NetEvent::StartConfiguration).await?;
            }
            PlayPacketStep::ResourcePackPush { update, action } => {
                let pack_id = update.id;
                if should_forward_resource_pack_push(update.required, action) {
                    emit(
                        &self.events,
                        NetEvent::Play(PlayClientbound::ResourcePackPush(update)),
                    )
                    .await?;
                }
                emit(
                    &self.events,
                    match action {
                        Some(action) => NetEvent::ResourcePackResponse {
                            id: pack_id,
                            action,
                        },
                        None => NetEvent::ResourcePackPrompt { id: pack_id },
                    },
                )
                .await?;
            }
            PlayPacketStep::Transfer(update) => {
                transfer = Some(update.clone());
                emit(
                    &self.events,
                    NetEvent::Play(PlayClientbound::Transfer(update)),
                )
                .await?;
            }
            PlayPacketStep::PlayerPosition(update) => {
                // The canonical world applies the correction and answers with
                // NetCommand::AcceptTeleport; until then outgoing player moves
                // are gated while the connection keeps reading packets, so a
                // slow canonical apply cannot stall keep-alives or leak stale
                // pre-teleport positions.
                self.pending_teleports += 1;
                emit(&self.events, NetEvent::PlayerPosition { update }).await?;
            }
            PlayPacketStep::PlayerRotation(update) => {
                emit(
                    &self.events,
                    NetEvent::Play(PlayClientbound::PlayerRotation(update)),
                )
                .await?;
            }
            PlayPacketStep::SetHeldSlot(update) => {
                emit(
                    &self.events,
                    NetEvent::Play(PlayClientbound::SetHeldSlot(update)),
                )
                .await?;
            }
            PlayPacketStep::Unknown { packet_id, len } => {
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
            PlayPacketStep::PassThrough(packet) => {
                emit(&self.events, NetEvent::Play(packet)).await?;
            }
        }
        Ok(transfer)
    }

    pub(super) async fn accept_teleport(
        &mut self,
        command: crate::types::AcceptTeleportCommand,
    ) -> Result<()> {
        self.session.accept_teleport(command).await
    }

    pub(super) async fn acknowledge_player_rotation(
        &mut self,
        y_rot: f32,
        x_rot: f32,
    ) -> Result<()> {
        self.session.acknowledge_player_rotation(y_rot, x_rot).await
    }

    pub(super) async fn acknowledge_start_configuration_with_chat(
        &mut self,
        pending_chat_acknowledgement: Option<packets::ChatAcknowledgement>,
    ) -> Result<()> {
        self.session
            .acknowledge_start_configuration_with_chat(pending_chat_acknowledgement)
            .await?;
        self.session.conn.flush().await?;
        self.session.conn.touch_inbound_deadline();
        emit(&self.events, NetEvent::StateChanged { state: self.state }).await
    }

    #[cfg(test)]
    pub(super) async fn acknowledge_start_configuration(&mut self) -> Result<()> {
        self.acknowledge_start_configuration_with_chat(None).await
    }
}
