use anyhow::{bail, Result};
use bbb_protocol::packets::{self, LoginClientbound};

use crate::{
    event_stream::{emit, EventStreamContext},
    types::{ConnectionState, NetEvent},
};

impl EventStreamContext {
    pub(super) async fn handle_login_packet(&mut self, packet: LoginClientbound) -> Result<()> {
        match packet {
            LoginClientbound::Disconnect { reason, .. } => {
                bail!("login disconnected: {reason}")
            }
            LoginClientbound::EncryptionRequest(_) => {
                bail!("server requested encryption; offline-mode event stream cannot continue")
            }
            LoginClientbound::SetCompression { threshold } => {
                self.conn.set_compression_threshold(threshold);
                emit(&self.events, NetEvent::CompressionSet { threshold }).await?;
            }
            LoginClientbound::CustomQuery { transaction_id } => {
                let (id, response) = packets::encode_login_custom_query_answer(transaction_id);
                self.conn.send_packet(id, &response).await?;
            }
            LoginClientbound::CookieRequest(request) => {
                let payload = self.server_cookies.get(&request.key).map(Vec::as_slice);
                let payload_present = payload.is_some();
                let (id, response) = packets::encode_login_cookie_response(&request.key, payload);
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
            LoginClientbound::LoginFinished { .. } => {
                let (id, payload) = packets::encode_login_acknowledged();
                self.conn.send_packet(id, &payload).await?;
                self.state = ConnectionState::Configuration;
                self.seen_code_of_conduct = false;
                emit(&self.events, NetEvent::StateChanged { state: self.state }).await?;

                let (id, payload) =
                    packets::encode_configuration_brand_custom_payload("bbb-native");
                self.conn.send_packet(id, &payload).await?;

                let (id, payload) = packets::encode_client_information(&self.client_information);
                self.conn.send_packet(id, &payload).await?;
            }
            LoginClientbound::Unknown { packet_id, len } => {
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
        }
        Ok(())
    }
}
