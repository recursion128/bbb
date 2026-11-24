use anyhow::{bail, Result};
use bbb_client_world::{
    ChatDisconnectReason, ChunkPos, PlayApplyEffects, RemovedPlayerVehicleTeleportRecovery,
    SoundEventState, VehicleMoveReport,
};
use bbb_protocol::packets::{self, ChatAcknowledgement, PlayClientbound};
#[cfg(test)]
use bbb_protocol::recipe_wire_types::VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID;
#[cfg(test)]
use bbb_protocol::stat_types::{VANILLA_STAT_TYPE_BLOCK_MINED_ID, VANILLA_STAT_TYPE_CUSTOM_ID};

use crate::{
    probe::{column_center_block, ProbeContext},
    types::ChunkProbeSummary,
};
#[cfg(test)]
use bbb_client_net::ConnectionState;
use bbb_client_net::{PlayPacketStep, ServerResourcePackPolicy};

/// Buffered play-apply side effects: the probe finishes the serverbound
/// responses after the synchronous world application returns.
struct ProbePlayEffects {
    started_at: std::time::Instant,
    chat_acknowledgement: Option<ChatAcknowledgement>,
    vehicle_move_report: Option<VehicleMoveReport>,
    removed_vehicle_teleport: Option<RemovedPlayerVehicleTeleportRecovery>,
    inserted_chunk_pos: Option<ChunkPos>,
    chat_disconnect: Option<ChatDisconnectReason>,
    /// Positioned sounds are the only visible result of some server
    /// mechanisms, so the probe keeps every one it is handed.
    positioned_sounds: Vec<SoundEventState>,
}

impl ProbePlayEffects {
    fn new(started_at: std::time::Instant) -> Self {
        Self {
            started_at,
            chat_acknowledgement: None,
            vehicle_move_report: None,
            removed_vehicle_teleport: None,
            inserted_chunk_pos: None,
            chat_disconnect: None,
            positioned_sounds: Vec::new(),
        }
    }

    fn monotonic_millis(&self) -> u64 {
        self.started_at
            .elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64
    }
}

impl PlayApplyEffects for ProbePlayEffects {
    fn chat_acknowledgement(&mut self, command: ChatAcknowledgement) {
        self.chat_acknowledgement = Some(command);
    }

    fn chat_validation_epoch_millis(&mut self) -> i64 {
        chrono::Utc::now().timestamp_millis()
    }

    fn chat_monotonic_millis(&mut self) -> u64 {
        self.monotonic_millis()
    }

    fn client_monotonic_millis(&mut self) -> u64 {
        self.monotonic_millis()
    }

    fn chat_disconnect(&mut self, reason: ChatDisconnectReason) {
        self.chat_disconnect = Some(reason);
    }

    fn vehicle_move_report(&mut self, report: VehicleMoveReport) {
        self.vehicle_move_report = Some(report);
    }

    fn removed_player_vehicle_teleport(&mut self, recovery: RemovedPlayerVehicleTeleportRecovery) {
        self.removed_vehicle_teleport = Some(recovery);
    }

    fn chunk_inserted(&mut self, pos: ChunkPos) {
        self.inserted_chunk_pos = Some(pos);
    }

    fn positioned_sound(&mut self, state: &SoundEventState) {
        self.positioned_sounds.push(state.clone());
    }
}

impl ProbeContext {
    pub(super) async fn handle_play_packet(
        &mut self,
        packet: PlayClientbound,
    ) -> Result<Option<ChunkPos>> {
        if let PlayClientbound::SetHeldSlot(update) = &packet {
            self.session.observe_play_set_held_slot(update.slot);
        }
        let mut effects = ProbePlayEffects::new(self.started_at);
        let connection_packet =
            self.world
                .apply_play_packet(packet, &mut self.level_event_sound_random, &mut effects);
        // The probe has no screens to open.
        self.world.take_screen_requests();
        self.world.poll_queued_light_updates();
        if let Some(pos) = effects.inserted_chunk_pos {
            self.chunk_delivery.on_chunk();
            if self.first_chunk_summary.is_none() {
                if let Some(column) = self.world.probe_chunk(pos) {
                    self.first_chunk_summary = Some(ChunkProbeSummary::from_column(column));
                    self.first_chunk_center_block =
                        self.world.probe_block(column_center_block(pos));
                }
            }
        }
        self.positioned_sounds
            .append(&mut effects.positioned_sounds);

        if let Some(reason) = effects.chat_disconnect {
            bail!("invalid signed-chat stream: {reason:?}");
        }

        if let Some(command) = effects.chat_acknowledgement.take() {
            self.session.send_play_chat_acknowledgement(command).await?;
        }
        if let Some(report) = effects.vehicle_move_report.take() {
            self.session
                .send_play_move_vehicle(packets::MoveVehicleCommand {
                    position: report.position,
                    y_rot: report.y_rot,
                    x_rot: report.x_rot,
                    on_ground: report.on_ground,
                })
                .await?;
        }
        if let Some(recovery) = effects.removed_vehicle_teleport.take() {
            self.session
                .send_play_player_pos_rot(recovery.player_pose.position_state(), false, false)
                .await?;
        }

        let Some(packet) = connection_packet else {
            return Ok(effects.inserted_chunk_pos);
        };
        match self
            .session
            .step_play_packet(packet, ServerResourcePackPolicy::Decline)
            .await?
        {
            PlayPacketStep::Handled => {}
            PlayPacketStep::PlayerRotation(_) => {
                let pose = self.world.local_player_pose().ok_or_else(|| {
                    anyhow::anyhow!("canonical player pose missing after rotation correction")
                })?;
                self.session
                    .acknowledge_player_rotation(pose.y_rot, pose.x_rot)
                    .await?;
            }
            PlayPacketStep::ChunkBatchStart => {
                self.chunk_delivery.on_batch_start();
            }
            PlayPacketStep::ChunkBatchFinished { batch_size } => {
                self.chunk_delivery.on_batch_finished(batch_size);
            }
            PlayPacketStep::CookieRequest {
                key,
                response_payload_present,
            } => {
                self.world
                    .apply_cookie_request(key, response_payload_present);
            }
            PlayPacketStep::StoreCookie {
                key,
                payload_len,
                stored_cookie_count,
            } => {
                self.world
                    .apply_store_cookie(key, payload_len, stored_cookie_count);
            }
            PlayPacketStep::StartConfiguration => {
                self.session
                    .acknowledge_start_configuration_with_chat(
                        self.world.take_pending_player_chat_acknowledgement(),
                    )
                    .await?;
                self.player_loaded_sent = false;
                self.world.begin_registry_configuration();
                self.world.clear_client_level();
            }
            PlayPacketStep::ResourcePackPush { update, action } => {
                // The world push apply already ran in apply_play_packet.
                self.world.apply_resource_pack_response(
                    update.id,
                    action.expect("the headless probe declines valid pack requests"),
                );
            }
            PlayPacketStep::PlayerPosition(update) => {
                self.player_position_state = self
                    .world
                    .local_player_pose()
                    .map(|pose| pose.position_state())
                    .ok_or_else(|| {
                        anyhow::anyhow!("canonical player pose missing after position correction")
                    })?;
                self.session
                    .send_teleport_acknowledgement(update.id)
                    .await?;
                if !self.player_loaded_sent {
                    self.session.send_play_player_loaded().await?;
                    self.player_loaded_sent = true;
                }
            }
            PlayPacketStep::Transfer(transfer) => {
                // `apply_play_packet` already recorded the transfer target on `self.world`.
                if let Some(options) = self.transfer_options.clone() {
                    self.session
                        .reconnect_for_transfer(&options, transfer)
                        .await?;
                    self.world.begin_registry_configuration();
                    self.world.clear_client_level();
                    self.player_loaded_sent = false;
                } else {
                    self.session.conn.exit_play_phase();
                }
            }
            PlayPacketStep::Unknown { packet_id, len } => {
                self.record_unsupported_packet(self.state, packet_id, len);
            }
            other => {
                debug_assert!(
                    false,
                    "unexpected connection packet returned by apply_play_packet: {other:?}"
                );
            }
        }
        Ok(effects.inserted_chunk_pos)
    }
}

#[cfg(test)]
mod tests;
