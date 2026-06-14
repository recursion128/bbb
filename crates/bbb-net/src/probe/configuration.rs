use anyhow::{bail, Result};
use bbb_protocol::packets::{self, ConfigurationClientbound, ResourcePackResponseAction};

use crate::{connection::play_tick_interval, probe::ProbeContext, types::ConnectionState};

impl ProbeContext {
    pub(super) async fn handle_configuration_packet(
        &mut self,
        packet: ConfigurationClientbound,
    ) -> Result<()> {
        match packet {
            ConfigurationClientbound::Finish => {
                let (id, payload) = packets::encode_configuration_finish();
                self.conn.send_packet(id, &payload).await?;
                self.state = ConnectionState::Play;
                let (id, payload) = packets::encode_play_client_information_default();
                self.conn.send_packet(id, &payload).await?;
                self.play_tick = Some(play_tick_interval());
            }
            ConfigurationClientbound::Disconnect { reason, .. } => {
                bail!("configuration disconnected: {reason}");
            }
            ConfigurationClientbound::CustomPayload(payload) => {
                self.world.apply_custom_payload(payload);
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
                self.world.record_registry_data(registry_data);
            }
            ConfigurationClientbound::UpdateTags(update) => {
                self.world.apply_update_tags(update);
            }
            ConfigurationClientbound::ResetChat => {
                self.world.apply_reset_chat();
            }
            ConfigurationClientbound::ResourcePackPush(update) => {
                let (id, payload) = packets::encode_configuration_resource_pack_response(
                    update.id,
                    ResourcePackResponseAction::Declined,
                );
                self.conn.send_packet(id, &payload).await?;
                self.world.apply_resource_pack_push(update);
            }
            ConfigurationClientbound::ResourcePackPop(update) => {
                self.world.apply_resource_pack_pop(update);
            }
            ConfigurationClientbound::UpdateEnabledFeatures(update) => {
                self.world.apply_update_enabled_features(update);
            }
            ConfigurationClientbound::SelectKnownPacks { .. } => {
                let (id, payload) = packets::encode_select_known_packs_empty();
                self.conn.send_packet(id, &payload).await?;
            }
            ConfigurationClientbound::CookieRequest(request) => {
                let payload = self.server_cookies.get(&request.key).map(Vec::as_slice);
                let (id, response) =
                    packets::encode_configuration_cookie_response(&request.key, payload);
                self.conn.send_packet(id, &response).await?;
            }
            ConfigurationClientbound::StoreCookie(cookie) => {
                self.server_cookies.insert(cookie.key, cookie.payload);
            }
            ConfigurationClientbound::CustomReportDetails(details) => {
                self.world.apply_custom_report_details(details);
            }
            ConfigurationClientbound::ServerLinks(links) => {
                self.world.apply_server_links(links);
            }
            ConfigurationClientbound::Transfer(update) => {
                self.world.apply_transfer(update);
            }
            ConfigurationClientbound::ClearDialog => {
                self.world.apply_clear_dialog();
            }
            ConfigurationClientbound::ShowDialog(update) => {
                self.world.apply_show_dialog(update);
            }
            ConfigurationClientbound::CodeOfConduct { .. } => {
                let (id, payload) = packets::encode_configuration_accept_code_of_conduct();
                self.conn.send_packet(id, &payload).await?;
            }
            ConfigurationClientbound::Unknown { .. } => {}
        }
        Ok(())
    }
}
