use anyhow::Result;
#[cfg(test)]
use bbb_client_net::ConnectionState;
use bbb_client_net::LoginPacketStep;
use bbb_protocol::packets::LoginClientbound;

use crate::probe::ProbeContext;

impl ProbeContext {
    pub(super) async fn handle_login_packet(&mut self, packet: LoginClientbound) -> Result<()> {
        match self.session.step_login_packet(packet).await? {
            LoginPacketStep::CookieRequest {
                key,
                response_payload_present,
            } => {
                self.world
                    .apply_cookie_request(key, response_payload_present);
            }
            LoginPacketStep::EnteredConfiguration => {
                self.world.begin_registry_configuration();
                self.world.clear_client_level();
            }
            LoginPacketStep::Unknown { packet_id, len } => {
                self.record_unsupported_packet(self.state, packet_id, len);
            }
            LoginPacketStep::CompressionSet { .. } | LoginPacketStep::Handled => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_client_net::RawConnection;
    use bbb_client_world::ChunkPos;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID;
    use bbb_protocol::{
        codec::Decoder,
        ids,
        packets::{
            AddEntity, ClientChatVisibility, ClientInformation, ClientMainHand,
            ClientParticleStatus, CookieRequest, GameProfile, Vec3d as ProtocolVec3d,
        },
    };
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
    async fn probe_login_disconnect_and_early_encryption_fail_without_advancing_state() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        let error = probe
            .handle_login_packet(LoginClientbound::Disconnect {
                reason: "bye".to_string(),
                styled_reason: Vec::new(),
                raw_reason: r#"{"text":"bye"}"#.to_string(),
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("login disconnected"));
        assert_eq!(probe.state, ConnectionState::Login);

        let error = probe
            .handle_login_packet(LoginClientbound::EncryptionRequest(
                bbb_protocol::packets::EncryptionRequest {
                    server_id: String::new(),
                    public_key: vec![1, 2, 3],
                    challenge: vec![4, 5, 6],
                    should_authenticate: true,
                },
            ))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("before sending login hello"));
        assert_eq!(probe.state, ConnectionState::Login);

        probe.conn.mark_login_hello_sent().unwrap();
        let error = probe
            .handle_login_packet(LoginClientbound::EncryptionRequest(
                bbb_protocol::packets::EncryptionRequest {
                    server_id: String::new(),
                    public_key: vec![1, 2, 3],
                    challenge: vec![4, 5, 6],
                    should_authenticate: true,
                },
            ))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("offline-only"));
        assert_eq!(probe.state, ConnectionState::Login);
    }

    #[tokio::test]
    async fn probe_login_finished_sends_brand_and_configured_client_information() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.client_information = ClientInformation {
            language: "zh_cn".to_string(),
            view_distance: 12,
            chat_visibility: ClientChatVisibility::Hidden,
            chat_colors: false,
            displayed_skin_parts: 0x15,
            main_hand: ClientMainHand::Left,
            text_filtering_enabled: true,
            allows_listing: true,
            particle_status: ClientParticleStatus::Decreased,
        };
        probe.world.apply_add_entity(AddEntity {
            id: 55,
            uuid: uuid::Uuid::from_u128(55),
            entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
            position: ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        probe.world.set_local_using_item(true);
        assert_eq!(probe.world.entity_count(), 1);
        assert!(probe.world.local_player().interaction.using_item);

        probe
            .handle_login_packet(LoginClientbound::LoginFinished {
                profile: GameProfile {
                    uuid: uuid::Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678),
                    name: "bbb-client".to_string(),
                    properties: Vec::new(),
                },
            })
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("login acknowledgement should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::login::SERVERBOUND_LOGIN_ACKNOWLEDGED);
        assert!(payload.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("brand custom payload should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::configuration::SERVERBOUND_CUSTOM_PAYLOAD);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:brand");
        assert_eq!(decoder.read_string(32767).unwrap(), "bbb-client");
        assert!(decoder.is_empty());

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("client information should be sent")
            .unwrap();
        assert_eq!(
            packet_id,
            ids::configuration::SERVERBOUND_CLIENT_INFORMATION
        );
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(16).unwrap(), "zh_cn");
        assert_eq!(decoder.read_i8().unwrap(), 12);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert!(!decoder.read_bool().unwrap());
        assert_eq!(decoder.read_u8().unwrap(), 0x15);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(decoder.read_bool().unwrap());
        assert!(decoder.read_bool().unwrap());
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert!(decoder.is_empty());
        assert_eq!(probe.state, ConnectionState::Configuration);
        assert!(!probe.seen_code_of_conduct);
        assert_eq!(probe.world.entity_count(), 0);
        assert!(!probe.world.local_player().interaction.using_item);
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

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
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
        let server = RawConnection::from_stream(server_stream, None);
        (client, server)
    }
}
