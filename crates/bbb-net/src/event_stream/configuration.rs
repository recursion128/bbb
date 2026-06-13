use anyhow::{bail, Result};
use bbb_protocol::packets::{self, ConfigurationClientbound, ResourcePackResponseAction};

use crate::{
    connection::play_tick_interval,
    event_stream::{emit, EventStreamContext},
    types::{ConnectionState, NetEvent},
};

impl EventStreamContext {
    pub(super) async fn handle_configuration_packet(
        &mut self,
        packet: ConfigurationClientbound,
    ) -> Result<()> {
        match packet {
            ConfigurationClientbound::Finish => {
                let (id, payload) = packets::encode_configuration_finish();
                self.conn.send_packet(id, &payload).await?;
                self.state = ConnectionState::Play;
                emit(&self.events, NetEvent::StateChanged { state: self.state }).await?;
                let (id, payload) = packets::encode_play_client_information_default();
                self.conn.send_packet(id, &payload).await?;
                self.play_tick = Some(play_tick_interval());
            }
            ConfigurationClientbound::Disconnect { reason, .. } => {
                bail!("configuration disconnected: {reason}");
            }
            ConfigurationClientbound::CustomPayload(update) => {
                emit(&self.events, NetEvent::CustomPayload(update)).await?;
            }
            ConfigurationClientbound::KeepAlive { id } => {
                let (id, payload) = packets::encode_configuration_keep_alive(id);
                self.conn.send_packet(id, &payload).await?;
            }
            ConfigurationClientbound::Ping { id } => {
                let (id, payload) = packets::encode_configuration_pong(id);
                self.conn.send_packet(id, &payload).await?;
            }
            ConfigurationClientbound::RegistryData(registry_data) => {
                emit(&self.events, NetEvent::RegistryData(registry_data)).await?;
            }
            ConfigurationClientbound::UpdateTags(update) => {
                emit(&self.events, NetEvent::UpdateTags(update)).await?;
            }
            ConfigurationClientbound::ResetChat => {
                emit(&self.events, NetEvent::ResetChat).await?;
            }
            ConfigurationClientbound::ResourcePackPush(update) => {
                let (id, payload) = packets::encode_configuration_resource_pack_response(
                    update.id,
                    ResourcePackResponseAction::Declined,
                );
                self.conn.send_packet(id, &payload).await?;
                emit(&self.events, NetEvent::ResourcePackPush(update)).await?;
            }
            ConfigurationClientbound::ResourcePackPop(update) => {
                emit(&self.events, NetEvent::ResourcePackPop(update)).await?;
            }
            ConfigurationClientbound::UpdateEnabledFeatures(update) => {
                emit(&self.events, NetEvent::UpdateEnabledFeatures(update)).await?;
            }
            ConfigurationClientbound::SelectKnownPacks { .. } => {
                let (id, payload) = packets::encode_select_known_packs_empty();
                self.conn.send_packet(id, &payload).await?;
            }
            ConfigurationClientbound::CookieRequest(request) => {
                let payload = self.server_cookies.get(&request.key).map(Vec::as_slice);
                let payload_present = payload.is_some();
                let (id, response) =
                    packets::encode_configuration_cookie_response(&request.key, payload);
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
            ConfigurationClientbound::StoreCookie(cookie) => {
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
            ConfigurationClientbound::CustomReportDetails(details) => {
                emit(&self.events, NetEvent::CustomReportDetails(details)).await?;
            }
            ConfigurationClientbound::ServerLinks(links) => {
                emit(&self.events, NetEvent::ServerLinks(links)).await?;
            }
            ConfigurationClientbound::Transfer(transfer) => {
                emit(&self.events, NetEvent::Transfer(transfer)).await?;
            }
            ConfigurationClientbound::ClearDialog => {
                emit(&self.events, NetEvent::ClearDialog).await?;
            }
            ConfigurationClientbound::ShowDialog(update) => {
                emit(&self.events, NetEvent::ShowDialog(update)).await?;
            }
            ConfigurationClientbound::CodeOfConduct { text } => {
                let (id, payload) = packets::encode_configuration_accept_code_of_conduct();
                self.conn.send_packet(id, &payload).await?;
                emit(&self.events, NetEvent::CodeOfConduct { text }).await?;
            }
            ConfigurationClientbound::Unknown { .. } => {}
        }
        Ok(())
    }
}
