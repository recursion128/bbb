use anyhow::Result;
#[cfg(test)]
use bbb_client_net::ConnectionState;
use bbb_client_net::{ConfigurationPacketStep, ServerResourcePackPolicy};
use bbb_protocol::packets::ConfigurationClientbound;

use crate::probe::ProbeContext;

impl ProbeContext {
    pub(super) async fn handle_configuration_packet(
        &mut self,
        packet: ConfigurationClientbound,
    ) -> Result<()> {
        match self
            .session
            .step_configuration_packet(packet, ServerResourcePackPolicy::Decline)
            .await?
        {
            ConfigurationPacketStep::Finish => {
                self.world.finish_registry_configuration()?;
                if let Some(mut capture) = self.configuration_capture.take() {
                    self.configuration_catalog = Some(capture.finish(&self.world)?);
                }
                self.session.finish_configuration().await?;
                self.session.start_play_tick();
            }
            ConfigurationPacketStep::CustomPayload(payload) => {
                self.world.apply_custom_payload(payload);
            }
            ConfigurationPacketStep::ResourcePackPush { update, action } => {
                let pack_id = update.id;
                self.world.apply_resource_pack_push(update);
                self.world.apply_resource_pack_response(
                    pack_id,
                    action.expect("the headless probe declines valid pack requests"),
                );
            }
            ConfigurationPacketStep::SelectKnownPacks { known_packs } => {
                let (selected_packs, data_source) = match self.known_packs_manager.as_deref() {
                    Some(manager) => {
                        let selected = manager.select_offered(&known_packs);
                        let data_source = manager.data_source_for(&selected)?;
                        (selected, data_source)
                    }
                    None => (
                        Vec::new(),
                        bbb_client_assets::KnownPackDataSource::default(),
                    ),
                };
                self.session
                    .send_select_known_packs(&selected_packs)
                    .await?;
                if let Some(capture) = self.configuration_capture.as_mut() {
                    capture.select_known_packs(selected_packs.len())?;
                }
                self.world.apply_select_known_packs_with_data(
                    known_packs,
                    selected_packs,
                    data_source,
                );
            }
            ConfigurationPacketStep::CookieRequest {
                key,
                response_payload_present,
            } => {
                self.world
                    .apply_cookie_request(key, response_payload_present);
            }
            ConfigurationPacketStep::StoreCookie {
                key,
                payload_len,
                stored_cookie_count,
            } => {
                self.world
                    .apply_store_cookie(key, payload_len, stored_cookie_count);
            }
            ConfigurationPacketStep::Transfer(update) => {
                self.world.apply_transfer(update);
            }
            ConfigurationPacketStep::CodeOfConduct { text } => {
                self.world.apply_code_of_conduct(text);
            }
            ConfigurationPacketStep::Unknown { packet_id, len } => {
                self.record_unsupported_packet(self.state, packet_id, len);
            }
            ConfigurationPacketStep::PassThrough(packet) => match packet {
                ConfigurationClientbound::RegistryData(registry_data) => {
                    if let Some(capture) = self.configuration_capture.as_mut() {
                        capture.registry_data(&registry_data);
                    }
                    self.world.record_registry_data(registry_data);
                }
                ConfigurationClientbound::UpdateTags(update) => {
                    if let Some(capture) = self.configuration_capture.as_mut() {
                        capture.update_tags(&update);
                    }
                    self.world.apply_update_tags(update);
                }
                ConfigurationClientbound::ResetChat => {
                    self.world.apply_reset_chat();
                }
                ConfigurationClientbound::ResourcePackPop(update) => {
                    self.world.apply_resource_pack_pop(update);
                }
                ConfigurationClientbound::UpdateEnabledFeatures(update) => {
                    self.world.apply_update_enabled_features(update);
                }
                ConfigurationClientbound::CustomReportDetails(details) => {
                    self.world.apply_custom_report_details(details);
                }
                ConfigurationClientbound::ServerLinks(links) => {
                    self.world.apply_server_links(links);
                }
                ConfigurationClientbound::ClearDialog => {
                    self.world.apply_clear_dialog();
                    self.world.take_screen_requests();
                }
                ConfigurationClientbound::ShowDialog(update) => {
                    self.world.apply_show_dialog(update);
                    self.world.take_screen_requests();
                }
                _ => unreachable!("handled by step_configuration_packet"),
            },
            ConfigurationPacketStep::Handled => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_client_net::RawConnection;
    use bbb_client_world::{
        code_of_conduct_text_hash, ChunkPos, RegistryPacketEntry, TransferTargetState,
    };
    use bbb_protocol::{
        codec::Decoder,
        ids,
        packets::{
            ChatTypeBound, ChatTypeHolder, CookieRequest, CustomPayload, CustomPayloadBody,
            CustomReportDetails, DialogHolder, DisguisedChat, KnownPack, RegistryData,
            RegistryDataEntry, RegistryTags, ResourcePackPop, ResourcePackPush,
            ResourcePackResponseAction, ServerLinkEntry, ServerLinkKnownType, ServerLinkType,
            ServerLinks, ShowDialog, StoreCookie, TagNetworkPayload, Transfer,
            UpdateEnabledFeatures, UpdateTags,
        },
        NbtSummaryValue,
    };
    use std::{collections::BTreeMap, sync::Arc, time::Duration};
    use tokio::net::TcpListener;
    use tokio::time::timeout;
    use uuid::Uuid;

    fn record_minimal_non_empty_synchronized_registries(
        world: &mut bbb_client_world::WorldStore,
        omitted_registry: &str,
    ) {
        for data_type in bbb_protocol::registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES
        {
            if data_type.validator == Some("RegistryValidator.nonEmpty ()")
                && data_type.resource_id != omitted_registry
            {
                let entry = minimal_sound_variant_nbt(data_type.resource_id)
                    .map(|data| RegistryPacketEntry::with_raw_data("minecraft:base", data))
                    .unwrap_or_else(|| RegistryPacketEntry::stub("minecraft:base"));
                world.record_registry_entries(data_type.resource_id, 0, vec![entry]);
            }
        }
    }

    fn minimal_sound_variant_nbt(registry: &str) -> Option<Vec<u8>> {
        let fields = match registry {
            "minecraft:cat_sound_variant" => [
                "ambient_sound",
                "stray_ambient_sound",
                "hiss_sound",
                "hurt_sound",
                "death_sound",
                "eat_sound",
                "beg_for_food_sound",
                "purr_sound",
                "purreow_sound",
            ]
            .as_slice(),
            "minecraft:chicken_sound_variant" | "minecraft:cow_sound_variant" => {
                ["ambient_sound", "hurt_sound", "death_sound", "step_sound"].as_slice()
            }
            "minecraft:pig_sound_variant" => [
                "ambient_sound",
                "hurt_sound",
                "death_sound",
                "step_sound",
                "eat_sound",
            ]
            .as_slice(),
            "minecraft:wolf_sound_variant" => [
                "ambient_sound",
                "death_sound",
                "growl_sound",
                "hurt_sound",
                "pant_sound",
                "whine_sound",
                "step_sound",
            ]
            .as_slice(),
            _ => return None,
        };
        let write_string = |out: &mut Vec<u8>, value: &str| {
            out.extend_from_slice(&(value.len() as u16).to_be_bytes());
            out.extend_from_slice(value.as_bytes());
        };
        let write_sound_set = |out: &mut Vec<u8>| {
            for field in fields {
                out.push(10);
                write_string(out, field);
                out.push(8);
                write_string(out, "sound_id");
                write_string(out, "minecraft:entity.cat.ambient");
                out.push(0);
            }
        };
        let mut data = vec![10];
        if registry == "minecraft:cow_sound_variant" {
            write_sound_set(&mut data);
        } else {
            data.push(10);
            write_string(&mut data, "adult_sounds");
            write_sound_set(&mut data);
            data.push(0);
            data.push(10);
            write_string(&mut data, "baby_sounds");
            write_sound_set(&mut data);
            data.push(0);
        }
        data.push(0);
        Some(data)
    }

    #[tokio::test]
    async fn probe_configuration_disconnect_is_terminal_and_keeps_state() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.state = ConnectionState::Configuration;

        let error = probe
            .handle_configuration_packet(ConfigurationClientbound::Disconnect {
                reason: "Configuration closed".to_string(),
                styled_reason: Vec::new(),
                raw_reason: vec![1, 2, 3],
            })
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("configuration disconnected: Configuration closed"));
        assert_eq!(probe.state, ConnectionState::Configuration);
    }

    #[tokio::test]
    async fn probe_finish_configuration_does_not_resend_client_information() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.state = ConnectionState::Configuration;

        probe
            .handle_configuration_packet(ConfigurationClientbound::Finish)
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("finish configuration should be sent")
            .unwrap();
        assert_eq!(
            packet_id,
            ids::configuration::SERVERBOUND_FINISH_CONFIGURATION
        );
        assert!(payload.is_empty());
        assert!(timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err());
        assert_eq!(probe.state, ConnectionState::Play);
        assert!(probe.play_tick.is_some());
    }

    #[tokio::test]
    async fn probe_selects_local_core_and_materializes_registry_stubs() {
        let (client, mut server) = raw_connection_pair().await;
        let manager = Arc::new(crate::test_support::known_packs_manager("probe"));
        let mut probe = ProbeContext::new(client);
        probe.state = ConnectionState::Configuration;
        probe.known_packs_manager = Some(Arc::clone(&manager));
        probe.world.begin_registry_configuration();

        probe
            .handle_configuration_packet(ConfigurationClientbound::SelectKnownPacks {
                known_packs: vec![KnownPack {
                    namespace: "minecraft".to_string(),
                    id: "core".to_string(),
                    version: "26.1".to_string(),
                }],
            })
            .await
            .unwrap();
        let (packet_id, payload) = server.read_packet().await.unwrap();
        assert_eq!(
            packet_id,
            ids::configuration::SERVERBOUND_SELECT_KNOWN_PACKS
        );
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_string(32_767).unwrap(), "minecraft");
        assert_eq!(decoder.read_string(32_767).unwrap(), "core");
        assert_eq!(decoder.read_string(32_767).unwrap(), "26.1");
        assert!(decoder.is_empty());

        probe
            .handle_configuration_packet(ConfigurationClientbound::RegistryData(RegistryData {
                registry: "minecraft:damage_type".to_string(),
                entries: vec![RegistryDataEntry {
                    id: "minecraft:base".to_string(),
                    raw_data: None,
                }],
                raw_payload_len: 0,
            }))
            .await
            .unwrap();
        record_minimal_non_empty_synchronized_registries(&mut probe.world, "minecraft:damage_type");
        probe
            .handle_configuration_packet(ConfigurationClientbound::Finish)
            .await
            .unwrap();

        assert_eq!(probe.world.known_packs().selected.len(), 1);
        assert!(probe
            .world
            .registry_entry_by_holder_id("minecraft:damage_type", 0)
            .unwrap()
            .known_pack_data()
            .is_some());
        assert_eq!(probe.state, ConnectionState::Play);
    }

    #[tokio::test]
    async fn probe_rejects_invalid_registry_collection_before_finish_ack() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.state = ConnectionState::Configuration;
        probe.world.record_registry_entries(
            "minecraft:wolf_variant",
            0,
            vec![RegistryPacketEntry::stub("minecraft:pale")],
        );
        probe.world.begin_registry_configuration();
        record_minimal_non_empty_synchronized_registries(
            &mut probe.world,
            "minecraft:wolf_variant",
        );
        probe
            .handle_configuration_packet(ConfigurationClientbound::RegistryData(RegistryData {
                registry: "minecraft:wolf_variant".to_string(),
                raw_payload_len: 0,
                entries: Vec::new(),
            }))
            .await
            .unwrap();

        let err = probe
            .handle_configuration_packet(ConfigurationClientbound::Finish)
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Registry must be non-empty: minecraft:wolf_variant"
        );
        assert_eq!(probe.state, ConnectionState::Configuration);
        assert!(probe.play_tick.is_none());
        assert_eq!(
            probe
                .world
                .registry_resource_id_by_holder_id("minecraft:wolf_variant", 0),
            Some("minecraft:pale")
        );
        assert!(timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn probe_applies_configuration_packets_to_world() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let pack_id = Uuid::from_u128(0x12345678123456781234567812345678);
        let report_details = BTreeMap::from([
            ("Region".to_string(), "local".to_string()),
            ("Server".to_string(), "configuration-probe".to_string()),
        ]);

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
            .handle_configuration_packet(ConfigurationClientbound::RegistryData(RegistryData {
                registry: "minecraft:chat_type".to_string(),
                raw_payload_len: 96,
                entries: vec![
                    RegistryDataEntry {
                        id: "minecraft:chat".to_string(),
                        raw_data: Some(vec![10; 24]),
                    },
                    RegistryDataEntry {
                        id: "minecraft:raw".to_string(),
                        raw_data: None,
                    },
                ],
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
                    prompt: Some(bbb_protocol::RichTextComponent::plain("Optional pack")),
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
            .handle_configuration_packet(ConfigurationClientbound::SelectKnownPacks {
                known_packs: vec![KnownPack {
                    namespace: "minecraft".to_string(),
                    id: "core".to_string(),
                    version: "26.1".to_string(),
                }],
            })
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::Transfer(Transfer {
                host: "next.example.invalid".into(),
                port: 25566,
            }))
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::CustomReportDetails(
                CustomReportDetails {
                    details: report_details.clone(),
                },
            ))
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::ServerLinks(ServerLinks {
                links: vec![
                    ServerLinkEntry {
                        link_type: ServerLinkType::Known(ServerLinkKnownType::Support),
                        url: "https://example.invalid/support".to_string(),
                    },
                    ServerLinkEntry {
                        link_type: ServerLinkType::Custom {
                            label: "Rules".to_string(),
                        },
                        url: "ftp://example.invalid/rules".to_string(),
                    },
                ],
            }))
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::ShowDialog(ShowDialog {
                dialog: DialogHolder::ContextFree {
                    dialog: NbtSummaryValue::Compound(Vec::new()),
                },
            }))
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::CodeOfConduct {
                text: "Keep the server friendly.".to_string(),
            })
            .await
            .unwrap();
        probe.world.apply_disguised_chat(DisguisedChat {
            message: "previous notice".to_string(),
            message_styled: Vec::new(),
            message_component: None,
            chat_type: ChatTypeBound {
                chat_type: ChatTypeHolder::Registry { id: 0 },
                name: "Server".to_string(),
                name_styled: Vec::new(),
                name_component: None,
                target_name: None,
                target_name_styled: None,
                target_name_component: None,
            },
        });
        assert_eq!(probe.world.counters().chat_messages_tracked, 1);
        probe
            .handle_configuration_packet(ConfigurationClientbound::ResetChat)
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

        let (packet_id, payload) = server.read_packet().await.unwrap();
        assert_eq!(packet_id, ids::configuration::SERVERBOUND_RESOURCE_PACK);
        assert_eq!(payload.len(), 17);
        let (packet_id, payload) = server.read_packet().await.unwrap();
        assert_eq!(
            packet_id,
            ids::configuration::SERVERBOUND_SELECT_KNOWN_PACKS
        );
        assert_eq!(payload, [0]);
        for (expected_key, expected_payload) in
            [("bbb:session", Some(&[1, 2, 3][..])), ("bbb:missing", None)]
        {
            let (packet_id, payload) = server.read_packet().await.unwrap();
            assert_eq!(packet_id, ids::configuration::SERVERBOUND_COOKIE_RESPONSE);
            let mut decoder = Decoder::new(&payload);
            assert_eq!(decoder.read_string(32767).unwrap(), expected_key);
            assert_eq!(decoder.read_bool().unwrap(), expected_payload.is_some());
            if let Some(expected_payload) = expected_payload {
                let len = decoder.read_len().unwrap();
                assert_eq!(
                    decoder.read_exact(len, "cookie payload").unwrap(),
                    expected_payload
                );
            }
            assert!(decoder.is_empty());
        }

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
        let registry_packet = &probe.world.registries().registries[0];
        assert_eq!(registry_packet.name, "minecraft:chat_type");
        assert_eq!(registry_packet.raw_payload_len, 96);
        assert_eq!(
            registry_packet.entries,
            vec![
                RegistryPacketEntry::with_raw_data("minecraft:chat", vec![10; 24]),
                RegistryPacketEntry::stub("minecraft:raw"),
            ]
        );
        assert!(probe.world.client_chat().messages.is_empty());
        let pack = probe
            .world
            .resource_pack(pack_id)
            .expect("resource pack should be tracked");
        assert_eq!(pack.id, pack_id);
        assert_eq!(pack.url, "https://example.invalid/pack.zip");
        assert_eq!(pack.hash, "abc123");
        assert!(!pack.required);
        assert_eq!(pack.prompt, None);
        assert_eq!(
            pack.last_response.as_ref().map(|response| response.action),
            Some(ResourcePackResponseAction::Declined)
        );
        assert_eq!(
            probe.world.enabled_feature_list(),
            vec!["minecraft:vanilla".to_string()]
        );
        assert_eq!(probe.world.known_packs().offered.len(), 1);
        assert_eq!(probe.world.known_packs().offered[0].namespace, "minecraft");
        assert_eq!(probe.world.known_packs().offered[0].id, "core");
        assert_eq!(probe.world.known_packs().offered[0].version, "26.1");
        assert!(probe.world.known_packs().selected.is_empty());
        assert_eq!(
            probe.world.last_transfer(),
            Some(&TransferTargetState {
                host: "next.example.invalid".to_string(),
                port: 25566,
            })
        );
        assert_eq!(probe.world.custom_report_details(), &report_details);
        assert_eq!(probe.world.server_links().len(), 1);
        assert_eq!(
            probe.world.server_links()[0].known_type.as_deref(),
            Some("support")
        );
        assert_eq!(
            probe.world.server_links()[0].url,
            "https://example.invalid/support"
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
            .handle_configuration_packet(ConfigurationClientbound::ResourcePackPop(
                ResourcePackPop { id: Some(pack_id) },
            ))
            .await
            .unwrap();
        probe
            .handle_configuration_packet(ConfigurationClientbound::ClearDialog)
            .await
            .unwrap();

        let report = probe.finish(18, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
        assert!(report.world.resource_packs().is_empty());
        assert_eq!(report.packets_seen, 18);
        assert_eq!(report.registries_seen, 1);
        assert_eq!(report.registry_entries_seen, 2);
        assert_eq!(report.registry_entries_with_data, 1);
        assert_eq!(report.registry_entry_stubs, 1);
        assert_eq!(report.registry_entry_payload_bytes, 24);
        assert_eq!(report.registry_content_registries_tracked, 1);
        assert_eq!(report.registry_content_packets_tracked, 1);
        assert_eq!(report.registry_content_entries_tracked, 2);
        assert_eq!(
            report.last_registry_data_registry.as_deref(),
            Some("minecraft:chat_type")
        );
        assert_eq!(report.last_registry_data_entry_count, 2);
        assert_eq!(report.world_counters.custom_payload_packets, 1);
        assert_eq!(report.world_counters.reset_chat_packets, 1);
        assert_eq!(report.world_counters.chat_messages_tracked, 0);
        assert_eq!(report.world_counters.update_tags_packets, 1);
        assert_eq!(report.world_counters.resource_pack_push_packets, 1);
        assert_eq!(report.world_counters.resource_pack_response_packets, 1);
        assert_eq!(
            report.world_counters.resource_pack_response_updates_applied,
            1
        );
        assert_eq!(
            report.world_counters.resource_pack_response_updates_ignored,
            0
        );
        assert_eq!(report.world_counters.resource_pack_pop_packets, 2);
        assert_eq!(report.world_counters.resource_pack_pop_updates_applied, 1);
        assert_eq!(report.world_counters.resource_pack_pop_updates_ignored, 1);
        assert_eq!(report.world_counters.update_enabled_features_packets, 1);
        assert_eq!(report.world_counters.enabled_features_tracked, 1);
        assert_eq!(report.world_counters.enabled_features_ignored, 1);
        assert_eq!(report.world_counters.select_known_packs_packets, 1);
        assert_eq!(report.world_counters.known_packs_offered, 1);
        assert_eq!(report.world_counters.known_packs_selected, 0);
        assert_eq!(report.world_counters.transfer_packets, 1);
        assert_eq!(report.world_counters.custom_report_detail_packets, 1);
        assert_eq!(report.world_counters.custom_report_details_tracked, 2);
        assert_eq!(report.world_counters.server_link_packets, 1);
        assert_eq!(report.world_counters.server_links_tracked, 1);
        assert_eq!(report.world_counters.server_link_invalid_entries, 1);
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
    async fn probe_configuration_resource_pack_invalid_url_updates_world_and_response() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        let pack_id = Uuid::from_u128(0x22222222_3333_4444_5555_666666666666);

        probe
            .handle_configuration_packet(ConfigurationClientbound::ResourcePackPush(
                ResourcePackPush {
                    id: pack_id,
                    url: "ftp://example.invalid/pack.zip".to_string(),
                    hash: "abc123".to_string(),
                    required: false,
                    prompt: Some(bbb_protocol::RichTextComponent::plain("Optional pack")),
                },
            ))
            .await
            .unwrap();

        let pack = probe
            .world
            .resource_pack(pack_id)
            .expect("resource pack should be tracked");
        assert_eq!(
            pack.last_response.as_ref().map(|response| response.action),
            Some(ResourcePackResponseAction::InvalidUrl)
        );

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("resource pack response should be sent")
            .unwrap();
        assert_eq!(packet_id, ids::configuration::SERVERBOUND_RESOURCE_PACK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_uuid().unwrap(), pack_id);
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            ResourcePackResponseAction::InvalidUrl.ordinal()
        );
        assert!(decoder.is_empty());
    }

    #[tokio::test]
    async fn probe_code_of_conduct_updates_world_without_unmatched_auto_accept() {
        let (client, mut server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);

        probe
            .handle_configuration_packet(ConfigurationClientbound::CodeOfConduct {
                text: "Follow the server rules.".to_string(),
            })
            .await
            .unwrap();

        assert!(timeout(Duration::from_millis(50), server.read_packet())
            .await
            .is_err());
        assert_eq!(
            probe.world.last_code_of_conduct().unwrap().text,
            "Follow the server rules."
        );
        assert_eq!(probe.world.counters().code_of_conduct_packets, 1);
    }

    #[tokio::test]
    async fn probe_code_of_conduct_auto_accepts_matching_hash() {
        let (client, mut server) = raw_connection_pair().await;
        let text = "Follow the server rules.";
        let mut probe = ProbeContext::new(client);
        probe.accepted_code_of_conduct_hash = Some(code_of_conduct_text_hash(text));

        probe
            .handle_configuration_packet(ConfigurationClientbound::CodeOfConduct {
                text: text.to_string(),
            })
            .await
            .unwrap();

        let (packet_id, payload) = timeout(Duration::from_secs(1), server.read_packet())
            .await
            .expect("probe should auto-accept matching code-of-conduct hash")
            .unwrap();
        assert_eq!(
            packet_id,
            ids::configuration::SERVERBOUND_ACCEPT_CODE_OF_CONDUCT
        );
        assert!(payload.is_empty());
        assert_eq!(probe.world.last_code_of_conduct().unwrap().text, text);
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

    #[tokio::test]
    async fn probe_configuration_unknown_packets_update_report_diagnostics() {
        let (client, _server) = raw_connection_pair().await;
        let mut probe = ProbeContext::new(client);
        probe.state = ConnectionState::Configuration;

        probe
            .handle_configuration_packet(ConfigurationClientbound::Unknown {
                packet_id: 0x7f,
                len: 12,
            })
            .await
            .unwrap();

        let report = probe.finish(1, ChunkPos { x: 0, z: 0 }, Vec::new(), None, None);
        assert_eq!(report.unsupported_packets, 1);
        assert_eq!(
            report.last_unsupported_packet_state.as_deref(),
            Some("Configuration")
        );
        assert_eq!(report.last_unsupported_packet_id, Some(0x7f));
        assert_eq!(report.last_unsupported_packet_len, Some(12));
        assert_eq!(report.world_counters.custom_payload_packets, 0);
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
