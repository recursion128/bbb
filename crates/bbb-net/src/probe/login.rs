use anyhow::{bail, Result};
use bbb_protocol::{
    ids,
    packets::{self, LoginClientbound},
};

use crate::{probe::ProbeContext, types::ConnectionState};

impl ProbeContext {
    pub(super) async fn handle_login_packet(&mut self, packet: LoginClientbound) -> Result<()> {
        match packet {
            LoginClientbound::Disconnect { raw_json } => {
                bail!("login disconnected: {raw_json}")
            }
            LoginClientbound::EncryptionRequest => {
                bail!("server requested encryption; offline-mode probe cannot continue")
            }
            LoginClientbound::SetCompression { threshold } => {
                self.conn.compression_threshold = Some(threshold);
            }
            LoginClientbound::CustomQuery { transaction_id } => {
                let mut response = bbb_protocol::codec::Encoder::new();
                response.write_var_i32(transaction_id);
                response.write_bool(false);
                self.conn
                    .send_packet(
                        ids::login::SERVERBOUND_CUSTOM_QUERY_ANSWER,
                        &response.into_inner(),
                    )
                    .await?;
            }
            LoginClientbound::CookieRequest(request) => {
                let payload = self.server_cookies.get(&request.key).map(Vec::as_slice);
                self.world
                    .apply_cookie_request(request.key.as_str(), payload.is_some());
                let (id, response) = packets::encode_login_cookie_response(&request.key, payload);
                self.conn.send_packet(id, &response).await?;
            }
            LoginClientbound::LoginFinished { .. } => {
                let (id, payload) = packets::encode_login_acknowledged();
                self.conn.send_packet(id, &payload).await?;
                self.state = ConnectionState::Configuration;
                self.seen_code_of_conduct = false;

                let (id, payload) = packets::encode_client_information_default();
                self.conn.send_packet(id, &payload).await?;
            }
        }
        Ok(())
    }
}
