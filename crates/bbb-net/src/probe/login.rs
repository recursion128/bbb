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
                let payload_present = payload.is_some();
                let (id, response) = packets::encode_login_cookie_response(&request.key, payload);
                self.conn.send_packet(id, &response).await?;
                self.world
                    .apply_cookie_request(request.key, payload_present);
            }
            LoginClientbound::LoginFinished { .. } => {
                let (id, payload) = packets::encode_login_acknowledged();
                self.conn.send_packet(id, &payload).await?;
                self.state = ConnectionState::Configuration;
                self.seen_code_of_conduct = false;

                let (id, payload) = packets::encode_client_information_default();
                self.conn.send_packet(id, &payload).await?;
            }
            LoginClientbound::Unknown { packet_id, len } => {
                self.record_unsupported_packet(self.state, packet_id, len);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::RawConnection;
    use bbb_protocol::{codec::Decoder, packets::CookieRequest};
    use bbb_world::ChunkPos;
    use bytes::BytesMut;
    use std::time::Duration;
    use tokio::net::TcpListener;
    use tokio::time::timeout;

    #[tokio::test]
    async fn probe_login_cookie_request_responds_and_updates_world() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_login_packet(LoginClientbound::CookieRequest(CookieRequest {
                key: "bbb:missing".to_string(),
            }))
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("login cookie response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::login::SERVERBOUND_COOKIE_RESPONSE);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(32767).unwrap(), "bbb:missing");
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());

        assert_eq!(probe.world.last_cookie_key(), Some("bbb:missing"));
        assert_eq!(probe.world.counters().cookie_request_packets, 1);
        assert_eq!(probe.world.counters().cookie_response_hits, 0);
        assert_eq!(probe.world.counters().cookie_response_misses, 1);
    }

    #[tokio::test]
    async fn probe_login_unknown_packets_update_report_diagnostics() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.state = ConnectionState::Login;

        probe
            .handle_login_packet(LoginClientbound::Unknown {
                packet_id: 0x7d,
                len: 5,
            })
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 });
        assert_eq!(report.unsupported_packets, 1);
        assert_eq!(
            report.last_unsupported_packet_state.as_deref(),
            Some("Login")
        );
        assert_eq!(report.last_unsupported_packet_id, Some(0x7d));
        assert_eq!(report.last_unsupported_packet_len, Some(5));
        assert_eq!(report.world_counters.cookie_request_packets, 0);
    }

    async fn raw_connection_pair() -> (RawConnection, RawConnection) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client = tokio::spawn(async move {
            RawConnection::connect(&addr.to_string(), None)
                .await
                .unwrap()
        });
        let (server_stream, _) = listener.accept().await.unwrap();
        let client = client.await.unwrap();
        let server = RawConnection {
            stream: server_stream,
            read_buf: BytesMut::new(),
            compression_threshold: None,
        };
        (client, server)
    }
}
