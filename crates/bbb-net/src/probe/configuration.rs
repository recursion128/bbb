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
                self.world
                    .apply_cookie_request(request.key.as_str(), payload.is_some());
                let (id, response) =
                    packets::encode_configuration_cookie_response(&request.key, payload);
                self.conn.send_packet(id, &response).await?;
            }
            ConfigurationClientbound::StoreCookie(cookie) => {
                let key = cookie.key;
                let payload_len = cookie.payload.len();
                self.server_cookies.insert(key.clone(), cookie.payload);
                self.world
                    .apply_store_cookie(key, payload_len, self.server_cookies.len());
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
            ConfigurationClientbound::CodeOfConduct { text } => {
                if self.seen_code_of_conduct {
                    bail!("server sent duplicate Code of Conduct");
                }
                self.seen_code_of_conduct = true;
                let (id, payload) = packets::encode_configuration_accept_code_of_conduct();
                self.conn.send_packet(id, &payload).await?;
                self.world.apply_code_of_conduct(text);
            }
            ConfigurationClientbound::Unknown { .. } => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::RawConnection;
    use bbb_protocol::{
        ids,
        packets::{
            CookieRequest, CustomPayload, CustomPayloadBody, DialogHolder, RegistryTags,
            ResourcePackPop, ResourcePackPush, ShowDialog, StoreCookie, TagNetworkPayload,
            Transfer, UpdateEnabledFeatures, UpdateTags,
        },
    };
    use bbb_world::{ChunkPos, DialogState, ResourcePackState, TransferTargetState};
    use bytes::BytesMut;
    use std::time::Duration;
    use tokio::net::TcpListener;
    use tokio::time::timeout;
    use uuid::Uuid;

    #[tokio::test]
    async fn probe_applies_configuration_packets_to_world() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let pack_id = Uuid::from_u128(0x12345678123456781234567812345678);

        probe
            .handle_configuration_packet(ConfigurationClientbound::CustomPayload(CustomPayload {
                id: "minecraft:brand".to_string(),
                payload: CustomPayloadBody::Brand {
                    brand: "vanilla-26.1".to_string(),
                },
            }))
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::UpdateTags(UpdateTags {
                registries: vec![RegistryTags {
                    registry: "minecraft:block".to_string(),
                    tags: vec![TagNetworkPayload {
                        tag: "minecraft:mineable/pickaxe".to_string(),
                        entries: vec![1, 2, 3],
                    }],
                }],
            }))
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::ResourcePackPush(
                ResourcePackPush {
                    id: pack_id,
                    url: "https://example.invalid/pack.zip".to_string(),
                    hash: "abc123".to_string(),
                    required: false,
                    prompt: Some("Optional pack".to_string()),
                },
            ))
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::UpdateEnabledFeatures(
                UpdateEnabledFeatures {
                    features: vec![
                        "minecraft:vanilla".to_string(),
                        "minecraft:unknown".to_string(),
                    ],
                },
            ))
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::Transfer(Transfer {
                host: "next.example.invalid".to_string(),
                port: 25566,
            }))
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::ShowDialog(ShowDialog {
                dialog: DialogHolder::Reference { registry_id: 7 },
            }))
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::CodeOfConduct {
                text: "Keep the server friendly.".to_string(),
            })
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::StoreCookie(StoreCookie {
                key: "bbb:session".to_string(),
                payload: vec![1, 2, 3],
            }))
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::CookieRequest(CookieRequest {
                key: "bbb:session".to_string(),
            }))
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::CookieRequest(CookieRequest {
                key: "bbb:missing".to_string(),
            }))
            .await
            .unwrap();

        assert_eq!(
            probe.world.server_brand(),
            Some("vanilla-26.1"),
            "configuration custom payload should update world presentation state",
        );
        assert_eq!(
            probe.world.registry_tags("minecraft:block").unwrap().tags
                ["minecraft:mineable/pickaxe"],
            vec![1, 2, 3],
        );
        assert_eq!(
            probe.world.resource_pack(pack_id),
            Some(&ResourcePackState {
                id: pack_id,
                url: "https://example.invalid/pack.zip".to_string(),
                hash: "abc123".to_string(),
                required: false,
                prompt: Some("Optional pack".to_string()),
            })
        );
        assert_eq!(
            probe.world.enabled_feature_list(),
            vec!["minecraft:vanilla".to_string()]
        );
        assert_eq!(
            probe.world.last_transfer(),
            Some(&TransferTargetState {
                host: "next.example.invalid".to_string(),
                port: 25566,
            })
        );
        assert_eq!(
            probe.world.current_dialog(),
            Some(&DialogState {
                holder_kind: "reference".to_string(),
                registry_id: Some(7),
                raw_dialog_payload_len: 0,
            })
        );
        assert_eq!(
            probe.world.last_code_of_conduct().unwrap().text,
            "Keep the server friendly."
        );
        assert_eq!(probe.world.last_cookie_key(), Some("bbb:missing"));
        assert_eq!(probe.world.stored_cookie_count(), 1);

        probe
            .handle_configuration_packet(ConfigurationClientbound::ResourcePackPop(
                ResourcePackPop { id: Some(pack_id) },
            ))
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::ClearDialog)
            .await
            .unwrap();

        let report = probe.finish(9, ChunkPos { x: 0, z: 0 });
        assert!(report.world.resource_packs().is_empty());
        assert!(report.world.current_dialog().is_none());
        assert_eq!(report.world_counters.custom_payload_packets, 1);
        assert_eq!(report.world_counters.update_tags_packets, 1);
        assert_eq!(report.world_counters.resource_pack_push_packets, 1);
        assert_eq!(report.world_counters.resource_pack_pop_packets, 1);
        assert_eq!(report.world_counters.update_enabled_features_packets, 1);
        assert_eq!(report.world_counters.enabled_features_tracked, 1);
        assert_eq!(report.world_counters.enabled_features_ignored, 1);
        assert_eq!(report.world_counters.transfer_packets, 1);
        assert_eq!(report.world_counters.show_dialog_packets, 1);
        assert_eq!(report.world_counters.clear_dialog_packets, 1);
        assert_eq!(report.world_counters.code_of_conduct_packets, 1);
        assert_eq!(
            report.world_counters.last_code_of_conduct_len,
            "Keep the server friendly.".len()
        );
        assert_eq!(report.world_counters.store_cookie_packets, 1);
        assert_eq!(report.world_counters.stored_cookie_bytes, 3);
        assert_eq!(report.world_counters.cookie_request_packets, 2);
        assert_eq!(report.world_counters.cookie_response_hits, 1);
        assert_eq!(report.world_counters.cookie_response_misses, 1);
    }

    #[tokio::test]
    async fn probe_code_of_conduct_auto_accepts_and_updates_world() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_configuration_packet(ConfigurationClientbound::CodeOfConduct {
                text: "Follow the server rules.".to_string(),
            })
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("probe should auto-accept code of conduct")
            .unwrap();
        assert_eq!(
            packet_id,
            ids::configuration::SERVERBOUND_ACCEPT_CODE_OF_CONDUCT
        );
        assert!(payload.is_empty());
        assert_eq!(
            probe.world.last_code_of_conduct().unwrap().text,
            "Follow the server rules."
        );
        assert_eq!(probe.world.counters().code_of_conduct_packets, 1);
    }

    #[tokio::test]
    async fn probe_rejects_duplicate_code_of_conduct_packet() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_configuration_packet(ConfigurationClientbound::CodeOfConduct {
                text: "First rules.".to_string(),
            })
            .await
            .unwrap();
        let err = probe
            .handle_configuration_packet(ConfigurationClientbound::CodeOfConduct {
                text: "Second rules.".to_string(),
            })
            .await
            .unwrap_err();

        assert!(
            err.to_string().contains("duplicate Code of Conduct"),
            "{err:?}"
        );
        assert_eq!(
            probe.world.last_code_of_conduct().unwrap().text,
            "First rules."
        );
        assert_eq!(probe.world.counters().code_of_conduct_packets, 1);
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
            read_buf: BytesMut::with_capacity(8192),
            compression_threshold: None,
        };
        (client, server)
    }
}
