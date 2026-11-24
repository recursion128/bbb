use serde_json::json;
use uuid::Uuid;

use crate::{
    codec::{offline_player_uuid, Encoder, ProtocolError},
    ids,
    packet_catalog::PacketCatalogEntry,
    packets::{
        decode_configuration_clientbound, decode_login_clientbound, decode_pong_response,
        decode_status_response, encode_client_information,
        encode_configuration_brand_custom_payload, encode_configuration_finish, encode_handshake,
        encode_login_acknowledged, encode_login_hello, encode_ping_request,
        encode_select_known_packs, encode_status_request, ClientInformation, ClientIntent,
        ConfigurationClientbound, CustomPayload, CustomPayloadBody, GameProfile,
        GameProfileProperty, KnownPack, LoginClientbound, RegistryTags, TagNetworkPayload,
        UpdateEnabledFeatures, UpdateTags,
    },
    PROTOCOL_VERSION,
};

use super::*;

#[test]
fn handshake_serverbound_decodes_the_vanilla_terminal_packet_strictly() {
    let (packet_id, payload) = encode_handshake("localhost", 25_565, ClientIntent::Login).unwrap();
    assert_eq!(
        decode_client_intention(packet_id, &payload).unwrap(),
        ClientIntention {
            protocol_version: PROTOCOL_VERSION,
            host: "localhost".to_string(),
            port: 25_565,
            intent: ClientIntent::Login,
        }
    );

    let mut bad_intent = Encoder::new();
    bad_intent.write_var_i32(PROTOCOL_VERSION);
    bad_intent.write_string("localhost");
    bad_intent.write_u16(25_565);
    bad_intent.write_var_i32(4);
    assert!(decode_client_intention(packet_id, &bad_intent.into_inner())
        .unwrap_err()
        .to_string()
        .contains("unknown connection intent 4"));

    let mut trailing = payload;
    trailing.push(0);
    assert!(decode_client_intention(packet_id, &trailing)
        .unwrap_err()
        .to_string()
        .contains("trailing bytes after client intention packet"));
    assert!(matches!(
        decode_client_intention(1, &[]),
        Err(ProtocolError::UnknownPacket {
            state: "handshake serverbound",
            id: 1
        })
    ));
}

#[test]
fn status_serverbound_and_clientbound_codecs_share_the_generated_catalog() {
    let (request_id, request) = encode_status_request();
    assert_eq!(
        decode_status_serverbound(request_id, &request).unwrap(),
        StatusServerbound::Request
    );

    let (ping_id, ping) = encode_ping_request(0x1020_3040_5060_7080);
    assert_eq!(
        decode_status_serverbound(ping_id, &ping).unwrap(),
        StatusServerbound::Ping {
            time: 0x1020_3040_5060_7080
        }
    );
    let mut trailing_ping = ping;
    trailing_ping.push(0);
    assert!(decode_status_serverbound(ping_id, &trailing_ping)
        .unwrap_err()
        .to_string()
        .contains("trailing bytes after status ping request packet"));

    let status = json!({
        "description": {"text": "bbb"},
        "enforcesSecureChat": false,
        "players": {"max": 20, "online": 0},
        "version": {"name": "26.1", "protocol": PROTOCOL_VERSION}
    });
    let (status_id, status_payload) = encode_status_response_json(&status).unwrap();
    assert_eq!(status_id, ids::status::CLIENTBOUND_STATUS_RESPONSE);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            &decode_status_response(&status_payload).unwrap()
        )
        .unwrap(),
        status
    );

    let (pong_id, pong_payload) = encode_status_pong(-7);
    assert_eq!(pong_id, ids::status::CLIENTBOUND_PONG_RESPONSE);
    assert_eq!(decode_pong_response(&pong_payload).unwrap(), -7);
}

#[test]
fn offline_login_serverbound_and_clientbound_codecs_preserve_vanilla_fields() {
    let claimed_profile_id = Uuid::from_u128(0x1234);
    let (hello_id, hello) = encode_login_hello("bbb-client", claimed_profile_id).unwrap();
    assert_eq!(
        decode_login_serverbound(hello_id, &hello).unwrap(),
        LoginServerbound::Hello {
            name: "bbb-client".to_string(),
            profile_id: claimed_profile_id,
        }
    );
    assert_eq!(
        offline_player_uuid("bbb-client").to_string(),
        "75d12e42-f08e-3099-9e1b-835c2a69e050"
    );

    let (ack_id, ack) = encode_login_acknowledged();
    assert_eq!(
        decode_login_serverbound(ack_id, &ack).unwrap(),
        LoginServerbound::Acknowledged
    );
    let key = crate::packets::LoginKey {
        encrypted_secret: vec![1, 2, 3],
        encrypted_challenge: vec![4, 5],
    };
    let (key_id, key_payload) = crate::packets::encode_login_key(&key).unwrap();
    assert_eq!(
        decode_login_serverbound(key_id, &key_payload).unwrap(),
        LoginServerbound::Key(key)
    );
    let (query_id, query_payload) = crate::packets::encode_login_custom_query_answer(17);
    assert_eq!(
        decode_login_serverbound(query_id, &query_payload).unwrap(),
        LoginServerbound::CustomQueryAnswer {
            transaction_id: 17,
            payload: None,
        }
    );
    let mut query_with_bytes = Encoder::new();
    query_with_bytes.write_var_i32(18);
    query_with_bytes.write_bool(true);
    query_with_bytes.write_bytes(&[9, 8, 7]);
    assert_eq!(
        decode_login_serverbound(
            ids::login::SERVERBOUND_CUSTOM_QUERY_ANSWER,
            &query_with_bytes.into_inner()
        )
        .unwrap(),
        LoginServerbound::CustomQueryAnswer {
            transaction_id: 18,
            payload: Some(vec![9, 8, 7]),
        }
    );
    let (cookie_id, cookie_payload) =
        crate::packets::encode_login_cookie_response("bbb:session", Some(&[0xaa, 0xbb])).unwrap();
    assert_eq!(
        decode_login_serverbound(cookie_id, &cookie_payload).unwrap(),
        LoginServerbound::CookieResponse {
            key: "bbb:session".to_string(),
            payload: Some(vec![0xaa, 0xbb]),
        }
    );
    assert!(matches!(
        decode_login_serverbound(99, &[]),
        Err(ProtocolError::UnknownPacket {
            state: "login serverbound",
            id: 99
        })
    ));

    let profile = GameProfile {
        uuid: offline_player_uuid("bbb-client"),
        name: "bbb-client".to_string(),
        properties: vec![GameProfileProperty {
            name: "textures".to_string(),
            value: "offline-value".to_string(),
            signature: None,
        }],
    };
    let (finished_id, finished) = encode_login_finished(&profile).unwrap();
    assert_eq!(
        decode_login_clientbound(finished_id, &finished).unwrap(),
        LoginClientbound::LoginFinished { profile }
    );

    let (compression_id, compression) = encode_login_compression(256).unwrap();
    assert_eq!(
        decode_login_clientbound(compression_id, &compression).unwrap(),
        LoginClientbound::SetCompression { threshold: 256 }
    );
    assert!(encode_login_compression(-1).is_err());

    let reason = json!({"text": "incompatible client"});
    let (disconnect_id, disconnect) = encode_login_disconnect_json(&reason).unwrap();
    assert!(matches!(
        decode_login_clientbound(disconnect_id, &disconnect).unwrap(),
        LoginClientbound::Disconnect { reason, .. } if reason == "incompatible client"
    ));
}

#[test]
fn configuration_bootstrap_codecs_cover_bbb_client_control_order() {
    let information = ClientInformation::default();
    let (information_id, information_payload) = encode_client_information(&information).unwrap();
    assert_eq!(
        decode_configuration_serverbound(information_id, &information_payload).unwrap(),
        ConfigurationServerbound::ClientInformation(information)
    );

    let (brand_id, brand_payload) =
        encode_configuration_brand_custom_payload("bbb-client").unwrap();
    assert_eq!(
        decode_configuration_serverbound(brand_id, &brand_payload).unwrap(),
        ConfigurationServerbound::CustomPayload(CustomPayload {
            id: "minecraft:brand".to_string(),
            payload: CustomPayloadBody::Brand {
                brand: "bbb-client".to_string(),
            },
        })
    );

    let selected = vec![KnownPack {
        namespace: "minecraft".to_string(),
        id: "core".to_string(),
        version: "26.1".to_string(),
    }];
    let (selected_id, selected_payload) = encode_select_known_packs(&selected).unwrap();
    assert_eq!(
        decode_configuration_serverbound(selected_id, &selected_payload).unwrap(),
        ConfigurationServerbound::SelectKnownPacks {
            known_packs: selected.clone(),
        }
    );

    let (finish_id, finish_payload) = encode_configuration_finish();
    assert_eq!(
        decode_configuration_serverbound(finish_id, &finish_payload).unwrap(),
        ConfigurationServerbound::Finish
    );

    let (server_brand_id, server_brand_payload) = encode_configuration_brand("bbb-server").unwrap();
    assert_eq!(
        decode_configuration_clientbound(server_brand_id, &server_brand_payload).unwrap(),
        ConfigurationClientbound::CustomPayload(CustomPayload {
            id: "minecraft:brand".to_string(),
            payload: CustomPayloadBody::Brand {
                brand: "bbb-server".to_string(),
            },
        })
    );

    let features = UpdateEnabledFeatures {
        features: vec!["vanilla".to_string(), "minecraft:vanilla".to_string()],
    };
    let (features_id, features_payload) = encode_configuration_enabled_features(&features).unwrap();
    assert_eq!(
        decode_configuration_clientbound(features_id, &features_payload).unwrap(),
        ConfigurationClientbound::UpdateEnabledFeatures(UpdateEnabledFeatures {
            features: vec!["minecraft:vanilla".to_string()],
        })
    );

    let (packs_id, packs_payload) = encode_configuration_select_known_packs(&selected).unwrap();
    assert_eq!(
        decode_configuration_clientbound(packs_id, &packs_payload).unwrap(),
        ConfigurationClientbound::SelectKnownPacks {
            known_packs: selected,
        }
    );

    let (tags_id, tags_payload) = encode_configuration_empty_tags();
    assert_eq!(
        decode_configuration_clientbound(tags_id, &tags_payload).unwrap(),
        ConfigurationClientbound::UpdateTags(UpdateTags {
            registries: Vec::<RegistryTags>::new(),
        })
    );

    let (server_finish_id, server_finish_payload) = encode_configuration_finish_clientbound();
    assert_eq!(
        decode_configuration_clientbound(server_finish_id, &server_finish_payload).unwrap(),
        ConfigurationClientbound::Finish
    );
}

#[test]
fn configuration_update_tags_encoder_preserves_wire_order() {
    let update = UpdateTags {
        registries: vec![
            RegistryTags {
                registry: "bbb:custom".to_string(),
                tags: vec![
                    TagNetworkPayload {
                        tag: "bbb:second".to_string(),
                        entries: vec![7, 1, 300],
                    },
                    TagNetworkPayload {
                        tag: "bbb:first".to_string(),
                        entries: vec![],
                    },
                ],
            },
            RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:logs".to_string(),
                    entries: vec![5, 6],
                }],
            },
        ],
    };

    let (packet_id, payload) = encode_configuration_update_tags(&update).unwrap();
    assert_eq!(packet_id, ids::configuration::CLIENTBOUND_UPDATE_TAGS);
    assert_eq!(
        decode_configuration_clientbound(packet_id, &payload).unwrap(),
        ConfigurationClientbound::UpdateTags(update)
    );

    let empty = UpdateTags {
        registries: Vec::new(),
    };
    assert_eq!(
        encode_configuration_update_tags(&empty).unwrap(),
        encode_configuration_empty_tags()
    );
}

#[test]
fn configuration_update_tags_encoder_rejects_invalid_ids_and_negative_holders() {
    let invalid_registry = UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:Item".to_string(),
            tags: vec![],
        }],
    };
    assert!(encode_configuration_update_tags(&invalid_registry)
        .unwrap_err()
        .to_string()
        .contains("invalid resource location"));

    let invalid_tag = UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:Logs".to_string(),
                entries: vec![],
            }],
        }],
    };
    assert!(encode_configuration_update_tags(&invalid_tag)
        .unwrap_err()
        .to_string()
        .contains("invalid resource location"));

    let negative_holder = UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:logs".to_string(),
                entries: vec![0, -1],
            }],
        }],
    };
    assert!(encode_configuration_update_tags(&negative_holder)
        .unwrap_err()
        .to_string()
        .contains("negative holder id -1"));
}

#[test]
fn configuration_update_tags_encoder_enforces_decode_budgets() {
    let oversized_resource = UpdateTags {
        registries: vec![RegistryTags {
            registry: format!("minecraft:{}", "a".repeat(32_758)),
            tags: vec![],
        }],
    };
    assert!(matches!(
        encode_configuration_update_tags(&oversized_resource),
        Err(ProtocolError::StringTooLong {
            actual: 32_768,
            max: 32_767
        })
    ));

    let too_many_registries = UpdateTags {
        registries: vec![
            RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![],
            };
            1_025
        ],
    };
    assert!(matches!(
        encode_configuration_update_tags(&too_many_registries),
        Err(ProtocolError::PacketTooLarge(1_025, 1_024))
    ));

    let too_many_tags = UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![
                TagNetworkPayload {
                    tag: "minecraft:logs".to_string(),
                    entries: vec![],
                };
                65_537
            ],
        }],
    };
    assert!(matches!(
        encode_configuration_update_tags(&too_many_tags),
        Err(ProtocolError::PacketTooLarge(65_537, 65_536))
    ));

    let too_many_entries = UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:logs".to_string(),
                entries: vec![0; 1_000_001],
            }],
        }],
    };
    assert!(matches!(
        encode_configuration_update_tags(&too_many_entries),
        Err(ProtocolError::PacketTooLarge(1_000_001, 1_000_000))
    ));
}

#[test]
fn bootstrap_server_codecs_reference_generated_direction_entries() {
    assert_entry(
        ids::handshake::SERVERBOUND_PACKETS,
        ids::handshake::SERVERBOUND_CLIENT_INTENTION,
        "ClientIntentionPacket",
    );
    for (entries, id, class_name) in [
        (
            ids::status::SERVERBOUND_PACKETS,
            ids::status::SERVERBOUND_STATUS_REQUEST,
            "ServerboundStatusRequestPacket",
        ),
        (
            ids::status::SERVERBOUND_PACKETS,
            ids::status::SERVERBOUND_PING_REQUEST,
            "ServerboundPingRequestPacket",
        ),
        (
            ids::status::CLIENTBOUND_PACKETS,
            ids::status::CLIENTBOUND_STATUS_RESPONSE,
            "ClientboundStatusResponsePacket",
        ),
        (
            ids::status::CLIENTBOUND_PACKETS,
            ids::status::CLIENTBOUND_PONG_RESPONSE,
            "ClientboundPongResponsePacket",
        ),
        (
            ids::login::SERVERBOUND_PACKETS,
            ids::login::SERVERBOUND_HELLO,
            "ServerboundHelloPacket",
        ),
        (
            ids::login::SERVERBOUND_PACKETS,
            ids::login::SERVERBOUND_LOGIN_ACKNOWLEDGED,
            "ServerboundLoginAcknowledgedPacket",
        ),
        (
            ids::login::CLIENTBOUND_PACKETS,
            ids::login::CLIENTBOUND_LOGIN_FINISHED,
            "ClientboundLoginFinishedPacket",
        ),
        (
            ids::login::CLIENTBOUND_PACKETS,
            ids::login::CLIENTBOUND_LOGIN_COMPRESSION,
            "ClientboundLoginCompressionPacket",
        ),
        (
            ids::configuration::SERVERBOUND_PACKETS,
            ids::configuration::SERVERBOUND_CLIENT_INFORMATION,
            "ServerboundClientInformationPacket",
        ),
        (
            ids::configuration::SERVERBOUND_PACKETS,
            ids::configuration::SERVERBOUND_SELECT_KNOWN_PACKS,
            "ServerboundSelectKnownPacks",
        ),
        (
            ids::configuration::SERVERBOUND_PACKETS,
            ids::configuration::SERVERBOUND_FINISH_CONFIGURATION,
            "ServerboundFinishConfigurationPacket",
        ),
        (
            ids::configuration::CLIENTBOUND_PACKETS,
            ids::configuration::CLIENTBOUND_UPDATE_ENABLED_FEATURES,
            "ClientboundUpdateEnabledFeaturesPacket",
        ),
        (
            ids::configuration::CLIENTBOUND_PACKETS,
            ids::configuration::CLIENTBOUND_SELECT_KNOWN_PACKS,
            "ClientboundSelectKnownPacks",
        ),
        (
            ids::configuration::CLIENTBOUND_PACKETS,
            ids::configuration::CLIENTBOUND_FINISH_CONFIGURATION,
            "ClientboundFinishConfigurationPacket",
        ),
    ] {
        assert_entry(entries, id, class_name);
    }
}

fn assert_entry(entries: &[PacketCatalogEntry], id: i32, class_name: &str) {
    assert!(entries
        .iter()
        .any(|entry| entry.id == id && entry.class_name == class_name));
}

#[test]
fn login_finished_rejects_vanilla_profile_bounds_before_writing() {
    let mut profile = GameProfile {
        uuid: Uuid::nil(),
        name: "bbb-client".to_string(),
        properties: Vec::new(),
    };
    profile.properties = (0..17)
        .map(|index| GameProfileProperty {
            name: format!("property-{index}"),
            value: String::new(),
            signature: None,
        })
        .collect();
    assert!(matches!(
        encode_login_finished(&profile),
        Err(ProtocolError::PacketTooLarge(17, 16))
    ));

    profile.properties.truncate(1);
    profile.properties[0].name = "a".repeat(65);
    assert_eq!(
        encode_login_finished(&profile).unwrap_err().to_string(),
        "game profile property name has 65 UTF-16 code units, maximum is 64"
    );
}

#[test]
fn configuration_serverbound_and_clientbound_26_1_packets_round_trip() {
    use std::collections::BTreeMap;

    use crate::{
        packets::{
            encode_configuration_cookie_response, encode_configuration_custom_click_action,
            CookieRequest, CustomClickAction, CustomReportDetails, DialogHolder, ResourcePackPop,
            ShowDialog, StoreCookie, Transfer,
        },
        NbtSummaryEntry, NbtSummaryValue,
    };

    let (cookie_resp_id, cookie_resp_payload) =
        encode_configuration_cookie_response("bbb:token", Some(&[1, 2, 3])).unwrap();
    assert_eq!(
        decode_configuration_serverbound(cookie_resp_id, &cookie_resp_payload).unwrap(),
        ConfigurationServerbound::CookieResponse {
            key: "bbb:token".to_string(),
            payload: Some(vec![1, 2, 3]),
        }
    );

    let click = CustomClickAction {
        id: "bbb:confirm".to_string(),
        payload: Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "accepted".into(),
            value: NbtSummaryValue::Byte(1),
        }])),
    };
    let (click_id, click_payload) = encode_configuration_custom_click_action(&click).unwrap();
    assert_eq!(
        decode_configuration_serverbound(click_id, &click_payload).unwrap(),
        ConfigurationServerbound::CustomClickAction(click)
    );

    let cookie_req = CookieRequest {
        key: "bbb:token".to_string(),
    };
    let (id, payload) = encode_configuration_cookie_request(&cookie_req).unwrap();
    assert_eq!(id, ids::configuration::CLIENTBOUND_COOKIE_REQUEST);
    assert_eq!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::CookieRequest(cookie_req)
    );

    let (id, payload) = encode_configuration_ping(-42);
    assert_eq!(id, ids::configuration::CLIENTBOUND_PING);
    assert_eq!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::Ping { id: -42 }
    );

    let (id, payload) = encode_configuration_reset_chat();
    assert_eq!(id, ids::configuration::CLIENTBOUND_RESET_CHAT);
    assert_eq!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::ResetChat
    );

    for pop in [
        ResourcePackPop { id: None },
        ResourcePackPop {
            id: Some(Uuid::from_u128(0xfeed_face)),
        },
    ] {
        let (id, payload) = encode_configuration_resource_pack_pop(&pop);
        assert_eq!(id, ids::configuration::CLIENTBOUND_RESOURCE_PACK_POP);
        assert_eq!(
            decode_configuration_clientbound(id, &payload).unwrap(),
            ConfigurationClientbound::ResourcePackPop(pop)
        );
    }

    let store_cookie = StoreCookie {
        key: "bbb:token".to_string(),
        payload: vec![7, 8, 9],
    };
    let (id, payload) = encode_configuration_store_cookie(&store_cookie).unwrap();
    assert_eq!(id, ids::configuration::CLIENTBOUND_STORE_COOKIE);
    assert_eq!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::StoreCookie(store_cookie)
    );

    let transfer = Transfer {
        host: "play.example.org".into(),
        port: 25_565,
    };
    let (id, payload) = encode_configuration_transfer(&transfer).unwrap();
    assert_eq!(id, ids::configuration::CLIENTBOUND_TRANSFER);
    assert_eq!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::Transfer(transfer)
    );

    let mut details_map = BTreeMap::new();
    details_map.insert("region".to_string(), "ap-east".to_string());
    let report_details = CustomReportDetails {
        details: details_map,
    };
    let (id, payload) = encode_configuration_custom_report_details(&report_details).unwrap();
    assert_eq!(id, ids::configuration::CLIENTBOUND_CUSTOM_REPORT_DETAILS);
    assert_eq!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::CustomReportDetails(report_details)
    );

    let (id, payload) = encode_configuration_clear_dialog();
    assert_eq!(id, ids::configuration::CLIENTBOUND_CLEAR_DIALOG);
    assert_eq!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::ClearDialog
    );

    let dialog = ShowDialog {
        dialog: DialogHolder::ContextFree {
            dialog: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "type".into(),
                value: NbtSummaryValue::String("minecraft:notice".into()),
            }]),
        },
    };
    let (id, payload) = encode_configuration_show_dialog(&dialog).unwrap();
    assert_eq!(id, ids::configuration::CLIENTBOUND_SHOW_DIALOG);
    assert_eq!(
        decode_configuration_clientbound(id, &payload).unwrap(),
        ConfigurationClientbound::ShowDialog(dialog)
    );
}
