use anyhow::Result;
use bbb_protocol::packets::{ConfigurationClientbound, KnownPack, Transfer};

use crate::{
    event_stream::{emit, should_forward_resource_pack_push, EventStreamContext},
    session::ConfigurationPacketStep,
    types::{NetEvent, ServerResourcePackPolicy},
};

impl EventStreamContext {
    pub(super) async fn handle_configuration_packet(
        &mut self,
        packet: ConfigurationClientbound,
    ) -> Result<Option<Transfer>> {
        self.handle_configuration_packet_with_resource_pack_policy(
            packet,
            ServerResourcePackPolicy::Decline,
            &[],
        )
        .await
    }

    pub(super) async fn handle_configuration_packet_with_resource_pack_policy(
        &mut self,
        packet: ConfigurationClientbound,
        resource_pack_policy: ServerResourcePackPolicy,
        available_known_packs: &[KnownPack],
    ) -> Result<Option<Transfer>> {
        let mut transfer = None;
        match self
            .session
            .step_configuration_packet(packet, resource_pack_policy)
            .await?
        {
            ConfigurationPacketStep::Finish => {
                emit(&self.events, NetEvent::FinishConfiguration).await?;
            }
            ConfigurationPacketStep::CustomPayload(update) => {
                emit(&self.events, NetEvent::CustomPayload(update)).await?;
            }
            ConfigurationPacketStep::ResourcePackPush { update, action } => {
                let pack_id = update.id;
                if should_forward_resource_pack_push(update.required, action) {
                    emit(&self.events, NetEvent::ResourcePackPush(update)).await?;
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
            ConfigurationPacketStep::SelectKnownPacks { known_packs } => {
                let selected_packs: Vec<KnownPack> = known_packs
                    .iter()
                    .filter(|offered| available_known_packs.contains(offered))
                    .cloned()
                    .collect();
                self.session
                    .send_select_known_packs(&selected_packs)
                    .await?;
                emit(
                    &self.events,
                    NetEvent::SelectKnownPacks {
                        known_packs,
                        selected_packs,
                    },
                )
                .await?;
            }
            ConfigurationPacketStep::CookieRequest {
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
            ConfigurationPacketStep::StoreCookie {
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
            ConfigurationPacketStep::Transfer(update) => {
                transfer = Some(update.clone());
                emit(&self.events, NetEvent::Transfer(update)).await?;
            }
            ConfigurationPacketStep::CodeOfConduct { text } => {
                emit(&self.events, NetEvent::CodeOfConduct { text }).await?;
            }
            ConfigurationPacketStep::Unknown { packet_id, len } => {
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
            ConfigurationPacketStep::PassThrough(packet) => match packet {
                ConfigurationClientbound::RegistryData(registry_data) => {
                    emit(&self.events, NetEvent::RegistryData(registry_data)).await?;
                }
                ConfigurationClientbound::UpdateTags(update) => {
                    emit(&self.events, NetEvent::UpdateTags(update)).await?;
                }
                ConfigurationClientbound::ResetChat => {
                    emit(&self.events, NetEvent::ResetChat).await?;
                }
                ConfigurationClientbound::ResourcePackPop(update) => {
                    emit(&self.events, NetEvent::ResourcePackPop(update)).await?;
                }
                ConfigurationClientbound::UpdateEnabledFeatures(update) => {
                    emit(&self.events, NetEvent::UpdateEnabledFeatures(update)).await?;
                }
                ConfigurationClientbound::CustomReportDetails(details) => {
                    emit(&self.events, NetEvent::CustomReportDetails(details)).await?;
                }
                ConfigurationClientbound::ServerLinks(links) => {
                    emit(&self.events, NetEvent::ServerLinks(links)).await?;
                }
                ConfigurationClientbound::ClearDialog => {
                    emit(&self.events, NetEvent::ClearDialog).await?;
                }
                ConfigurationClientbound::ShowDialog(update) => {
                    emit(&self.events, NetEvent::ShowDialog(update)).await?;
                }
                _ => unreachable!("handled by step_configuration_packet"),
            },
            ConfigurationPacketStep::Handled => {}
        }
        Ok(transfer)
    }

    pub(super) async fn finish_configuration(&mut self) -> Result<()> {
        self.session.finish_configuration().await?;
        self.session.conn.flush().await?;
        self.session.conn.touch_inbound_deadline();
        emit(&self.events, NetEvent::StateChanged { state: self.state }).await
    }
}
